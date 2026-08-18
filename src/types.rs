// Copyright (c) 2026 Alibaba Cloud
//
// SPDX-License-Identifier: Apache-2.0
//

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};

/// Release Manifest `schemaVersion` currently accepted by artifact-server.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Rekor v1 log service type string used on the wire.
pub const LOG_SERVICE_REKOR_V1: &str = "rekor-v1";

/// One `{type, value}` item from a Release Manifest.
///
/// `value` follows the measurement type: it may be a JSON string or a JSON object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    #[serde(rename = "type")]
    pub type_: String,
    pub value: Value,
}

impl Measurement {
    pub fn new(type_: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            type_: type_.into(),
            value: value.into(),
        }
    }

    /// Build a measurement whose `value` is a JSON string.
    pub fn text(type_: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(type_, Value::String(value.into()))
    }
}

/// Signed-payload Release Manifest: `{schemaVersion, measurements[]}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    pub measurements: Vec<Measurement>,
}

impl ReleaseManifest {
    pub fn new(measurements: impl IntoIterator<Item = Measurement>) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            measurements: measurements.into_iter().collect(),
        }
    }

    pub fn with_schema_version(mut self, version: impl Into<String>) -> Self {
        self.schema_version = version.into();
        self
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.schema_version.trim().is_empty() {
            return Err(Error::InvalidRequest(
                "schemaVersion is required".to_string(),
            ));
        }
        if self.measurements.is_empty() {
            return Err(Error::InvalidRequest(
                "measurements must not be empty".to_string(),
            ));
        }
        for (i, meas) in self.measurements.iter().enumerate() {
            if meas.type_.trim().is_empty() {
                return Err(Error::InvalidRequest(format!(
                    "measurements[{i}].type is required"
                )));
            }
            if meas.value.is_null() {
                return Err(Error::InvalidRequest(format!(
                    "measurements[{i}].value is required"
                )));
            }
        }
        Ok(())
    }
}

/// Optional target transparency log. Not part of the DSSE payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogService {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl LogService {
    pub fn new(type_: impl Into<String>, url: impl Into<Option<String>>) -> Self {
        Self {
            type_: type_.into(),
            url: url.into(),
        }
    }

    pub fn rekor_v1(url: impl Into<String>) -> Self {
        Self {
            type_: LOG_SERVICE_REKOR_V1.to_string(),
            url: Some(url.into()),
        }
    }
}

/// `POST /api/v1/transparency/resolve` request body.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveRequest {
    pub release_manifest: ReleaseManifest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_services: Option<Vec<LogService>>,
}

impl ResolveRequest {
    pub fn new(release_manifest: ReleaseManifest) -> Self {
        Self {
            release_manifest,
            log_services: None,
        }
    }

    pub fn with_log_services(mut self, services: impl Into<Vec<LogService>>) -> Self {
        self.log_services = Some(services.into());
        self
    }

    pub fn with_log_service(self, service: LogService) -> Self {
        self.with_log_services(vec![service])
    }

    pub(crate) fn validate(&self) -> Result<()> {
        self.release_manifest.validate()
    }
}

/// Materials that verify the signed payload inside a log entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryVerifier {
    #[serde(rename = "type")]
    pub type_: String,
    pub content: String,
}

/// Transparency-log signing public key for SET / inclusion proof.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogVerifier {
    pub public_key_pem: String,
}

/// One transparency log entry returned by resolve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(rename = "type")]
    pub type_: String,
    pub url: String,
    /// Opaque entry body; structure is defined by `type`.
    pub log_entry: Value,
    pub entry_verifier: EntryVerifier,
    pub log_verifier: LogVerifier,
}

/// Successful resolve response. `status` is `"resolved"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolveResponse {
    #[serde(default)]
    pub request_id: Option<String>,
    pub status: String,
    pub release_manifest: ReleaseManifest,
    #[serde(default)]
    pub log_entries: Vec<LogEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_serializes_to_wire_shape() {
        let req = ResolveRequest::new(ReleaseManifest::new([
            Measurement::text("tdx.td-shim", "582f8ed2..."),
            Measurement::text("container.image.cmaas-runtime", "sha256:abc123..."),
        ]))
        .with_log_service(LogService::rekor_v1("https://rekor.example.com"));

        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(
            value,
            json!({
                "release_manifest": {
                    "schemaVersion": "1.0.0",
                    "measurements": [
                        {"type": "tdx.td-shim", "value": "582f8ed2..."},
                        {"type": "container.image.cmaas-runtime", "value": "sha256:abc123..."}
                    ]
                },
                "log_services": [
                    {"type": "rekor-v1", "url": "https://rekor.example.com"}
                ]
            })
        );
    }

    #[test]
    fn response_deserializes_from_wire_shape() {
        let raw = json!({
            "request_id": "req-1",
            "status": "resolved",
            "release_manifest": {
                "schemaVersion": "1.0.0",
                "measurements": [
                    {"type": "tdx.td-shim", "value": "582f8ed2..."}
                ]
            },
            "log_entries": [{
                "type": "rekor-v1",
                "url": "https://rekor.sigstore.dev",
                "log_entry": {"body": "opaque"},
                "entry_verifier": {
                    "type": "public_key",
                    "content": "-----BEGIN PUBLIC KEY-----\nMIIB\n-----END PUBLIC KEY-----\n"
                },
                "log_verifier": {
                    "public_key_pem": "-----BEGIN PUBLIC KEY-----\nMIIB\n-----END PUBLIC KEY-----\n"
                }
            }]
        });

        let resp: ResolveResponse = serde_json::from_value(raw).unwrap();
        assert_eq!(resp.status, "resolved");
        assert_eq!(resp.request_id.as_deref(), Some("req-1"));
        assert_eq!(resp.log_entries.len(), 1);
        assert_eq!(resp.log_entries[0].type_, "rekor-v1");
        assert_eq!(resp.log_entries[0].entry_verifier.type_, "public_key");
    }

    #[test]
    fn measurement_value_can_be_object() {
        let meas = Measurement::new("prot.fw", json!({"hash": "aa", "version": "1"}));
        let value = serde_json::to_value(&meas).unwrap();
        assert_eq!(
            value,
            json!({"type": "prot.fw", "value": {"hash": "aa", "version": "1"}})
        );
    }

    #[test]
    fn empty_measurements_fail_validation() {
        let err = ReleaseManifest::new([]).validate().unwrap_err();
        assert!(err.is_invalid_request());
    }
}
