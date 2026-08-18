// Copyright (c) 2026 Alibaba Cloud
//
// SPDX-License-Identifier: Apache-2.0
//

/// Convenience alias used by the SDK.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the resolve client.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Local request validation failed before any HTTP call.
    #[error("invalid resolve request: {0}")]
    InvalidRequest(String),

    /// The server returned a structured API error.
    #[error("{error_code}: {error_message} (http {status})")]
    Api {
        status: u16,
        error_code: String,
        error_message: String,
        request_id: Option<String>,
    },

    /// Transport or HTTP-client failure.
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    /// The response body could not be interpreted.
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

impl Error {
    pub fn error_code(&self) -> Option<&str> {
        match self {
            Self::Api { error_code, .. } => Some(error_code),
            _ => None,
        }
    }

    pub fn request_id(&self) -> Option<&str> {
        match self {
            Self::Api { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::Http(err) => err.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    pub fn is_invalid_request(&self) -> bool {
        matches!(self, Self::InvalidRequest(_)) || self.error_code() == Some("invalid_request")
    }

    pub fn is_measurement_not_found(&self) -> bool {
        self.error_code() == Some("measurement_not_found")
    }

    pub fn is_measurement_revoked(&self) -> bool {
        self.error_code() == Some("measurement_revoked")
    }

    pub fn is_unsupported_log_service(&self) -> bool {
        self.error_code() == Some("unsupported_log_service")
    }

    pub fn is_policy_violation(&self) -> bool {
        self.error_code() == Some("policy_violation")
    }

    pub fn is_publish_failed(&self) -> bool {
        self.error_code() == Some("publish_failed")
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct ErrorBody {
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}
