// Copyright (c) 2026 Alibaba Cloud
//
// SPDX-License-Identifier: Apache-2.0
//

use std::time::Duration;

use reqwest::{Client as HttpClient, StatusCode};
use serde_json::Value;

use crate::error::{Error, ErrorBody, Result};
use crate::types::{LogService, ReleaseManifest, ResolveRequest, ResolveResponse};

/// Public pre-environment OpenAPI host used by the anonymous resolve path.
pub const DEFAULT_BASE_URL: &str = "https://attest-pre.aliyuncs.com";

const RESOLVE_PATH: &str = "/api/v1/transparency/resolve";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const USER_AGENT: &str = concat!("artifact-resolve-sdk/", env!("CARGO_PKG_VERSION"));

/// Async client for `POST /api/v1/transparency/resolve`.
///
/// Resolve is a public anonymous API: the client does not sign with an AccessKey.
#[derive(Clone, Debug)]
pub struct Client {
    http: HttpClient,
    base_url: String,
}

impl Client {
    /// Client pointed at the public pre environment (`https://attest-pre.aliyuncs.com`).
    pub fn pre() -> Result<Self> {
        ClientBuilder::new().build()
    }

    /// Client pointed at an arbitrary artifact-server base URL
    /// (for example `http://127.0.0.1:8080`).
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        ClientBuilder::new().base_url(base_url).build()
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Resolve a fully assembled request.
    pub async fn resolve(&self, request: &ResolveRequest) -> Result<ResolveResponse> {
        request.validate()?;
        self.send(request).await
    }

    /// Resolve a Release Manifest, optionally targeting specific log services.
    pub async fn resolve_manifest(
        &self,
        manifest: ReleaseManifest,
        log_services: Option<Vec<LogService>>,
    ) -> Result<ResolveResponse> {
        let mut request = ResolveRequest::new(manifest);
        request.log_services = log_services;
        self.resolve(&request).await
    }

    async fn send(&self, request: &ResolveRequest) -> Result<ResolveResponse> {
        let url = format!("{}{RESOLVE_PATH}", self.base_url);
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(request)
            .send()
            .await?;
        decode_response(response).await
    }
}

/// Builder for [`Client`].
#[derive(Debug)]
pub struct ClientBuilder {
    base_url: String,
    timeout: Duration,
    http: Option<HttpClient>,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
            http: None,
        }
    }
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Inject a preconfigured reqwest client (tests, custom TLS, proxies).
    pub fn http_client(mut self, http: HttpClient) -> Self {
        self.http = Some(http);
        self
    }

    pub(crate) fn base_url_str(&self) -> &str {
        &self.base_url
    }

    pub(crate) fn timeout_value(&self) -> Duration {
        self.timeout
    }

    pub fn build(self) -> Result<Client> {
        let base_url = self.base_url.trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(Error::InvalidRequest("base_url is required".to_string()));
        }
        let http = match self.http {
            Some(http) => http,
            None => HttpClient::builder()
                .timeout(self.timeout)
                .user_agent(USER_AGENT)
                .build()?,
        };
        Ok(Client { http, base_url })
    }
}

pub(crate) async fn decode_response(response: reqwest::Response) -> Result<ResolveResponse> {
    let status = response.status();
    let body = response.bytes().await?;
    decode_body(status, &body)
}

pub(crate) fn decode_body(status: StatusCode, body: &[u8]) -> Result<ResolveResponse> {
    if status.is_success() {
        return serde_json::from_slice(body).map_err(|err| {
            Error::Unexpected(format!("invalid resolve response: {err}; body={body:?}"))
        });
    }

    if let Ok(parsed) = serde_json::from_slice::<ErrorBody>(body) {
        if parsed.error_code.is_some() || parsed.error_message.is_some() {
            return Err(Error::Api {
                status: status.as_u16(),
                error_code: parsed.error_code.unwrap_or_else(|| "unknown".to_string()),
                error_message: parsed
                    .error_message
                    .unwrap_or_else(|| "no error_message".to_string()),
                request_id: parsed.request_id,
            });
        }
    }

    let fallback = match serde_json::from_slice::<Value>(body) {
        Ok(value) => value.to_string(),
        Err(_) => String::from_utf8_lossy(body).into_owned(),
    };
    Err(Error::Unexpected(format!(
        "http {} with unexpected body: {fallback}",
        status.as_u16()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Measurement, ReleaseManifest};
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn resolve_posts_wire_body_and_parses_success() {
        let server = MockServer::start().await;
        let request = ResolveRequest::new(ReleaseManifest::new([Measurement::text(
            "tdx.td-shim",
            "582f8ed2...",
        )]));

        Mock::given(method("POST"))
            .and(path(RESOLVE_PATH))
            .and(header("accept", "application/json"))
            .and(header("content-type", "application/json"))
            .and(body_json(&request))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "request_id": "abc",
                "status": "resolved",
                "release_manifest": request.release_manifest,
                "log_entries": [{
                    "type": "rekor-v1",
                    "url": "https://rekor.example.com",
                    "log_entry": {"ok": true},
                    "entry_verifier": {"type": "public_key", "content": "PEM"},
                    "log_verifier": {"public_key_pem": "PEM"}
                }]
            })))
            .mount(&server)
            .await;

        let client = Client::new(server.uri()).unwrap();
        let resp = client.resolve(&request).await.unwrap();
        assert_eq!(resp.status, "resolved");
        assert_eq!(resp.log_entries[0].url, "https://rekor.example.com");
        assert_eq!(resp.request_id.as_deref(), Some("abc"));
    }

    #[tokio::test]
    async fn resolve_maps_measurement_revoked() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(RESOLVE_PATH))
            .respond_with(ResponseTemplate::new(409).set_body_json(json!({
                "error_code": "measurement_revoked",
                "error_message": "release manifest contains revoked measurements"
            })))
            .mount(&server)
            .await;

        let client = Client::new(server.uri()).unwrap();
        let err = client
            .resolve_manifest(
                ReleaseManifest::new([Measurement::text("tdx.td-shim", "deadbeef")]),
                None,
            )
            .await
            .unwrap_err();
        assert!(err.is_measurement_revoked());
        assert_eq!(err.status(), Some(409));
    }

    #[tokio::test]
    async fn resolve_rejects_empty_manifest_locally() {
        let client = Client::new("http://127.0.0.1:1").unwrap();
        let err = client
            .resolve(&ResolveRequest::new(ReleaseManifest::new([])))
            .await
            .unwrap_err();
        assert!(err.is_invalid_request());
    }
}
