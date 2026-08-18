// Copyright (c) 2026 Alibaba Cloud
//
// SPDX-License-Identifier: Apache-2.0
//

//! Blocking wrapper around [`crate::Client`].
//!
//! Enable with `features = ["blocking"]`.

use std::time::Duration;

use reqwest::blocking::Client as HttpClient;

use crate::client::{decode_body, ClientBuilder};
use crate::error::{Error, Result};
use crate::types::{LogService, ReleaseManifest, ResolveRequest, ResolveResponse};

const RESOLVE_PATH: &str = "/api/v1/transparency/resolve";
const USER_AGENT: &str = concat!("artifact-resolve-sdk/", env!("CARGO_PKG_VERSION"));

/// Blocking client for `POST /api/v1/transparency/resolve`.
#[derive(Clone, Debug)]
pub struct Client {
    http: HttpClient,
    base_url: String,
}

impl Client {
    pub fn pre() -> Result<Self> {
        Builder::new().build()
    }

    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        Builder::new().base_url(base_url).build()
    }

    pub fn builder() -> Builder {
        Builder::new()
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn resolve(&self, request: &ResolveRequest) -> Result<ResolveResponse> {
        request.validate()?;
        let url = format!("{}{RESOLVE_PATH}", self.base_url);
        let response = self
            .http
            .post(&url)
            .header(reqwest::header::ACCEPT, "application/json")
            .json(request)
            .send()
            .map_err(Error::from)?;
        decode_body(response.status(), &response.bytes()?)
    }

    pub fn resolve_manifest(
        &self,
        manifest: ReleaseManifest,
        log_services: Option<Vec<LogService>>,
    ) -> Result<ResolveResponse> {
        let mut request = ResolveRequest::new(manifest);
        request.log_services = log_services;
        self.resolve(&request)
    }
}

/// Builder for the blocking [`Client`].
#[derive(Debug)]
pub struct Builder {
    inner: ClientBuilder,
}

impl Builder {
    pub fn new() -> Self {
        Self {
            inner: ClientBuilder::new(),
        }
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self.inner.base_url(base_url);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn build(self) -> Result<Client> {
        let base_url = self.inner.base_url_str().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(Error::InvalidRequest("base_url is required".to_string()));
        }
        let http = HttpClient::builder()
            .timeout(self.inner.timeout_value())
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Client { http, base_url })
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
