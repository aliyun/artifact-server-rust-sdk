// Copyright (c) 2026 Alibaba Cloud
//
// SPDX-License-Identifier: Apache-2.0
//

//! Rust SDK for Artifact Server **Transparency Resolve**.
//!
//! Calls the public anonymous API:
//!
//! ```text
//! POST /api/v1/transparency/resolve
//! ```
//!
//! The caller assembles a [`ReleaseManifest`] (`schemaVersion` + `measurements[]`)
//! from attestation evidence. The server checks that every `{type,value}` is
//! registered and not effectively revoked, then returns transparency log entries
//! (lazy-publishing them when missing).
//!
//! # Example
//!
//! ```no_run
//! use artifact_resolve_sdk::{Client, Measurement, ReleaseManifest};
//!
//! # async fn run() -> artifact_resolve_sdk::Result<()> {
//! let client = Client::pre()?;
//! let manifest = ReleaseManifest::new([Measurement::text(
//!     "tdx.td-shim",
//!     "582f8ed2...",
//! )]);
//! let response = client.resolve_manifest(manifest, None).await?;
//! assert_eq!(response.status, "resolved");
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod types;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL};
pub use error::{Error, Result};
pub use types::{
    EntryVerifier, LogEntry, LogService, LogVerifier, Measurement, ReleaseManifest, ResolveRequest,
    ResolveResponse, LOG_SERVICE_REKOR_V1, SCHEMA_VERSION,
};

#[cfg(feature = "blocking")]
pub mod blocking;
