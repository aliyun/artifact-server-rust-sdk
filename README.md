# artifact-resolve-sdk

English | [简体中文](README.zh-CN.md)

Artifact Server provides a client SDK for its anonymous APIs.

## Usage

```toml
[dependencies]
artifact-resolve-sdk = { path = "../artifact-resolve-sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust
use artifact_resolve_sdk::{Client, LogService, Measurement, ReleaseManifest};

#[tokio::main]
async fn main() -> artifact_resolve_sdk::Result<()> {
    let client = Client::pre()?; // https://attest-pre.aliyuncs.com
    // Talk to a local server: Client::new("http://127.0.0.1:8080")?

    let manifest = ReleaseManifest::new([
        Measurement::text("tdx.td-shim", "582f8ed2..."),
        Measurement::text("container.image.cmaas-runtime", "sha256:abc123..."),
    ]);

    let response = client
        .resolve_manifest(
            manifest,
            Some(vec![LogService::rekor_v1("https://rekor.example.com")]),
        )
        .await?;

    assert_eq!(response.status, "resolved");
    for entry in response.log_entries {
        println!("{} {}", entry.type_, entry.url);
    }
    Ok(())
}
```

Enable the `blocking` feature for synchronous calls:

```toml
artifact-resolve-sdk = { path = "../artifact-resolve-sdk", features = ["blocking"] }
```

```rust
let client = artifact_resolve_sdk::blocking::Client::pre()?;
let response = client.resolve_manifest(manifest, None)?;
```

## Error codes

| HTTP | `error_code` | SDK helper |
| --- | --- | --- |
| 400 | `invalid_request` | `Error::is_invalid_request` |
| 404 | `measurement_not_found` | `Error::is_measurement_not_found` |
| 409 | `measurement_revoked` | `Error::is_measurement_revoked` |
| 400 | `unsupported_log_service` | `Error::is_unsupported_log_service` |
| 403 | `policy_violation` | `Error::is_policy_violation` |
| 503 | `publish_failed` | `Error::is_publish_failed` |

## Examples

```bash
# Public pre-release gateway
cargo run --example resolve

# Talk to a local artifact-server
ARTIFACT_SERVER_URL=http://127.0.0.1:8080 cargo run --example resolve
```
