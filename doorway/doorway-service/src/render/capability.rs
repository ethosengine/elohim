//! Capability deriver: bundles ∩ manifest, with override.
//!
//! Tasks 11-14 fill in `scan_bundles`, `fetch_manifest_renderers`,
//! `parse_override`/`load_override`, and the `derive_capability` orchestrator.
//! This file is the stub that compiles end-to-end so the module aggregator
//! has something to re-export.

use crate::render::types::{BundleEntry, RenderCapabilityProfile, RendererKind};
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum CapabilityDeriverError {
    #[error("bundles directory unreadable: {0}")]
    BundleDirRead(String),
    #[error("manifest fetch failed: {0}")]
    ManifestFetch(String),
    #[error("override config malformed: {0}")]
    OverrideMalformed(String),
}

/// Auto-derive a render-capability claim. Honest by construction:
/// only bundles on disk whose renderer is referenced in storage's manifest
/// can appear in the claim. Override may reduce the claim but never inflate.
///
/// Tasks 11-14 implement this. The stub returns Ok(None) so the module compiles.
pub async fn derive_capability(
    _bundles_dir: &std::path::Path,
    _storage_manifest_url: &str,
    _override_path: Option<&std::path::Path>,
) -> Result<Option<RenderCapabilityProfile>, CapabilityDeriverError> {
    Ok(None)
}

#[derive(Deserialize)]
struct BundleHeader {
    name: String,
    version: String,
    renderer: RendererKind,
}

/// Scan a directory for `*.bundle.mjs` files and parse the
/// `@elohim-bundle {...}` JSON header. Files without a header (or with a
/// malformed header) are skipped silently. Missing directory is honest-
/// degradation: empty result, no error.
pub async fn scan_bundles(
    dir: &std::path::Path,
) -> Result<Vec<BundleEntry>, CapabilityDeriverError> {
    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CapabilityDeriverError::BundleDirRead(e.to_string())),
    };
    let mut bundles = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| CapabilityDeriverError::BundleDirRead(e.to_string()))?
    {
        let path = entry.path();
        let name_ok = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with(".bundle.mjs"))
            .unwrap_or(false);
        if !name_ok {
            continue;
        }
        let contents = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(bundle) = parse_bundle_header(&contents) {
            bundles.push(bundle);
        }
    }
    Ok(bundles)
}

/// Parse a bundle's `@elohim-bundle {...}` header. Returns None when the
/// banner is missing or malformed (caller skips silently).
fn parse_bundle_header(contents: &str) -> Option<BundleEntry> {
    let marker = "@elohim-bundle";
    let start = contents.find(marker)? + marker.len();
    let rest = &contents[start..];
    let json_start = rest.find('{')?;
    let json_end = rest[json_start..].find("*/")?;
    let json_str = rest[json_start..json_start + json_end].trim();
    let header: BundleHeader = serde_json::from_str(json_str).ok()?;
    Some(BundleEntry {
        name: header.name,
        version: header.version,
        renderer: header.renderer,
        digest: None,
    })
}

#[cfg(test)]
mod scan_tests {
    use super::*;
    use crate::render::types::RendererKind;
    use std::fs;
    use tempfile::TempDir;

    fn write_bundle(dir: &std::path::Path, name: &str, header: &str) {
        let path = dir.join(format!("{name}.bundle.mjs"));
        fs::write(path, header).expect("write bundle stub");
    }

    #[tokio::test]
    async fn scans_bundles_with_protocol_manifest_header() {
        let tmp = TempDir::new().unwrap();
        write_bundle(
            tmp.path(),
            "lamad",
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */
            export function bootstrap() {}"#,
        );
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].name, "lamad-app");
        assert_eq!(bundles[0].version, "1.0.3");
        assert_eq!(bundles[0].renderer, RendererKind::AngularSsr);
    }

    #[tokio::test]
    async fn scans_multiple_bundles() {
        let tmp = TempDir::new().unwrap();
        write_bundle(
            tmp.path(),
            "lamad",
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */"#,
        );
        write_bundle(
            tmp.path(),
            "qahal",
            r#"/* @elohim-bundle {"name":"qahal-app","version":"0.2.0","renderer":"angular-ssr"} */"#,
        );
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert_eq!(bundles.len(), 2);
        let names: Vec<&str> = bundles.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"lamad-app"));
        assert!(names.contains(&"qahal-app"));
    }

    #[tokio::test]
    async fn skips_bundles_without_header() {
        let tmp = TempDir::new().unwrap();
        write_bundle(tmp.path(), "no-header", "export const x = 1;");
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert!(bundles.is_empty());
    }

    #[tokio::test]
    async fn skips_non_mjs_files() {
        let tmp = TempDir::new().unwrap();
        // .js (not .mjs), .bundle.txt, etc. should not match
        std::fs::write(
            tmp.path().join("lamad.bundle.js"),
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */"#,
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("plain.mjs"),
            r#"/* @elohim-bundle {"name":"plain","version":"1.0.0","renderer":"angular-ssr"} */"#,
        )
        .unwrap();
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert!(
            bundles.is_empty(),
            "only files ending in .bundle.mjs should match"
        );
    }

    #[tokio::test]
    async fn returns_empty_when_dir_missing() {
        let bundles = scan_bundles(std::path::Path::new(
            "/nonexistent/path/that/does/not/exist",
        ))
        .await;
        // Missing dir is honest-degradation: empty, not error
        assert!(bundles.unwrap().is_empty());
    }

    #[tokio::test]
    async fn malformed_header_skipped_silently() {
        let tmp = TempDir::new().unwrap();
        write_bundle(
            tmp.path(),
            "broken",
            r#"/* @elohim-bundle {malformed json} */"#,
        );
        let bundles = scan_bundles(tmp.path()).await.expect("scan succeeds");
        assert!(bundles.is_empty());
    }
}
