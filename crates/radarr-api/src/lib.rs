//! Thin, hand-written client for the Radarr v3 HTTP API.
//!
//! [`RadarrClient`] handles authentication (`X-Api-Key`), JSON encoding and
//! error reporting; callers pick the endpoint path and the response type.
//! Commonly shared resource types live in [`types`].

pub mod client;
pub mod types;

pub use client::RadarrClient;
