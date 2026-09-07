use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

/// Time allowed to establish a TCP/TLS connection to the Radarr server.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Overall time allowed for a single request (some Radarr endpoints are slow).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone)]
pub struct RadarrClient {
    http: reqwest::Client,
    base_url: String,
}

impl RadarrClient {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Api-Key",
            HeaderValue::from_str(api_key).context("Invalid API key format")?,
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            // Never follow redirects: a redirect would drop the X-Api-Key header
            // or silently send it to another host.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("Failed to create HTTP client")?;

        let base_url = base_url.trim_end_matches('/').to_string();

        Ok(Self { http, base_url })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response("GET", &url, resp).await
    }

    pub async fn get_with_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .query(query)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response("GET", &url, resp).await
    }

    /// Fetch a resource whose body is plain text (e.g. a log file) rather than JSON.
    pub async fn get_text(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(api_error("GET", &url, status, &body));
        }
        resp.text()
            .await
            .with_context(|| format!("Failed to read response body from GET {url}"))
    }

    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response("POST", &url, resp).await
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response("POST", &url, resp).await
    }

    pub async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .put(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response("PUT", &url, resp).await
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response_empty("DELETE", &url, resp).await
    }

    pub async fn delete_with_query(&self, path: &str, query: &[(&str, &str)]) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .delete(&url)
            .query(query)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response_empty("DELETE", &url, resp).await
    }

    pub async fn delete_with_body<B: serde::Serialize>(&self, path: &str, body: &B) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .delete(&url)
            .json(body)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response_empty("DELETE", &url, resp).await
    }
}

/// Percent-encode `segment` so it is safe to embed as a single path segment in a URL.
///
/// Everything outside the RFC 3986 unreserved set is escaped, `/` included, so a
/// value containing a slash cannot escape its position in the path.
pub fn encode_path_segment(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for byte in segment.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn api_error(method: &str, url: &str, status: reqwest::StatusCode, body: &str) -> anyhow::Error {
    if let Ok(err) = serde_json::from_str::<serde_json::Value>(body)
        && let Some(msg) = err.get("message").and_then(|m| m.as_str())
    {
        return anyhow::anyhow!("API error ({status} for {method} {url}): {msg}");
    }
    anyhow::anyhow!("API error ({status} for {method} {url}): {body}")
}

/// Truncate a response body for inclusion in an error message.
fn body_preview(body: &str) -> String {
    const MAX: usize = 200;
    if body.chars().count() <= MAX {
        return body.to_string();
    }
    let truncated: String = body.chars().take(MAX).collect();
    format!("{truncated}…")
}

async fn handle_response<T: DeserializeOwned>(
    method: &str,
    url: &str,
    resp: reqwest::Response,
) -> Result<T> {
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<none>")
        .to_string();

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(api_error(method, url, status, &body));
    }

    let body = resp
        .text()
        .await
        .with_context(|| format!("Failed to read response body from {method} {url}"))?;

    serde_json::from_str::<T>(&body).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse API response from {method} {url} (content-type: {content_type}): {e}; body starts with: {}",
            body_preview(&body)
        )
    })
}

async fn handle_response_empty(method: &str, url: &str, resp: reqwest::Response) -> Result<()> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(api_error(method, url, status, &body));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_unreserved_characters_verbatim() {
        assert_eq!(encode_path_segment("radarr.txt"), "radarr.txt");
        assert_eq!(encode_path_segment("a-b_c~d.0"), "a-b_c~d.0");
    }

    #[test]
    fn escapes_separators_and_spaces() {
        assert_eq!(encode_path_segment("../etc/passwd"), "..%2Fetc%2Fpasswd");
        assert_eq!(encode_path_segment("my log.txt"), "my%20log.txt");
        assert_eq!(encode_path_segment("a?b#c"), "a%3Fb%23c");
    }

    #[test]
    fn escapes_non_ascii_as_utf8_bytes() {
        assert_eq!(encode_path_segment("é"), "%C3%A9");
    }

    #[test]
    fn body_preview_truncates_long_bodies() {
        let long = "x".repeat(500);
        let preview = body_preview(&long);
        assert_eq!(preview.chars().count(), 201);
        assert!(preview.ends_with('…'));
        assert_eq!(body_preview("short"), "short");
    }

    #[test]
    fn api_error_includes_method_and_url() {
        let err = api_error(
            "DELETE",
            "http://host/api/v3/queue/1",
            reqwest::StatusCode::NOT_FOUND,
            r#"{"message":"nope"}"#,
        );
        let msg = err.to_string();
        assert!(msg.contains("DELETE"), "{msg}");
        assert!(msg.contains("http://host/api/v3/queue/1"), "{msg}");
        assert!(msg.contains("nope"), "{msg}");
    }
}
