use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;

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
        handle_response(resp).await
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
        handle_response(resp).await
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
        handle_response(resp).await
    }

    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response(resp).await
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
        handle_response(resp).await
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .delete(&url)
            .send()
            .await
            .with_context(|| format!("Failed to connect to {url}"))?;
        handle_response_empty(resp).await
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
        handle_response_empty(resp).await
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
        handle_response_empty(resp).await
    }
}

async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<serde_json::Value>(&body)
            && let Some(msg) = err.get("message").and_then(|m| m.as_str())
        {
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        anyhow::bail!("API error ({}): {}", status, body);
    }
    resp.json::<T>()
        .await
        .context("Failed to parse API response")
}

async fn handle_response_empty(resp: reqwest::Response) -> Result<()> {
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<serde_json::Value>(&body)
            && let Some(msg) = err.get("message").and_then(|m| m.as_str())
        {
            anyhow::bail!("API error ({}): {}", status, msg);
        }
        anyhow::bail!("API error ({}): {}", status, body);
    }
    Ok(())
}
