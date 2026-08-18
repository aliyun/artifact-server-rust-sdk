// Copyright (c) 2026 Alibaba Cloud
//
// SPDX-License-Identifier: Apache-2.0
//

use artifact_resolve_sdk::{Client, LogService, Measurement, ReleaseManifest, ResolveRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("ARTIFACT_SERVER_URL")
        .unwrap_or_else(|_| "https://attest-pre.aliyuncs.com".to_string());
    let client = Client::new(base_url)?;

    let request = ResolveRequest::new(ReleaseManifest::new([Measurement::text(
        "container.image.image_rs",
        "sha256:c64c687cbea9300178b30c95835354e34c4e4febc4badfe27102879de0483b5e",
    )]));

    let request = match std::env::var("REKOR_URL") {
        Ok(url) if !url.is_empty() => request.with_log_service(LogService::rekor_v1(url)),
        _ => request,
    };

    match client.resolve(&request).await {
        Ok(response) => {
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
    Ok(())
}
