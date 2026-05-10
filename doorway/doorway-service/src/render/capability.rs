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

/// Static fallback used when no compute budget is available (e.g.
/// `STORAGE_COMPUTE_URL` unset or unreachable). When a budget *is* available,
/// `max_concurrent_renders` is derived as
/// `min(probe_cpu_count, ceiling_max_cores, allocation_cpu_cores)` — the
/// operator-as-pod model from
/// `project_storage_as_pod_operator_sets_virtual_limits`.
const DEFAULT_MAX_CONCURRENT: u32 = 8;
const DEFAULT_AUTH_MODES: &[&str] = &["anonymous", "doorway-hosted"];

/// Per-node compute budget extracted from elohim-storage's
/// `/api/v1/compute/dashboard`. Each field is `0` when unknown / not
/// declared (consumers ignore zeros when taking the min).
#[derive(Debug, Clone, Default)]
pub struct ComputeBudget {
    /// `computeMetrics.cpuTotalCores` — actual hardware probe.
    pub cpu_total_cores: u32,
    /// `constitutionalLimits.ceilingLimit.computeMaxCores` — operator hard cap.
    pub ceiling_max_cores: u32,
    /// `allocations.allocationBlocks[0].cpuCores` — operator virtual allocation
    /// for this workload.
    pub allocation_cpu_cores: u32,
}

impl ComputeBudget {
    /// Take the min of the non-zero fields. Returns `None` when every field
    /// is unknown (zero) — caller should fall back to `DEFAULT_MAX_CONCURRENT`.
    pub fn min_cpu_budget(&self) -> Option<u32> {
        [
            self.cpu_total_cores,
            self.ceiling_max_cores,
            self.allocation_cpu_cores,
        ]
        .into_iter()
        .filter(|v| *v > 0)
        .min()
    }
}

/// Fetch the storage compute dashboard and extract the SSR-relevant CPU fields.
///
/// Returns `None` (honest degradation) when the URL is unreachable, the
/// response is non-2xx, or the body fails to parse. Mirrors the shape of
/// `views::load_render_capability_from_url` in elohim-storage.
pub async fn fetch_compute_budget(dashboard_url: &str) -> Option<ComputeBudget> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;
    let resp = match client.get(dashboard_url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                url = %dashboard_url,
                error = %e,
                "compute dashboard unreachable — falling back to DEFAULT_MAX_CONCURRENT"
            );
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!(
            url = %dashboard_url,
            status = %resp.status(),
            "compute dashboard returned non-success — falling back to DEFAULT_MAX_CONCURRENT"
        );
        return None;
    }
    let body: serde_json::Value = resp.json().await.ok()?;
    let cpu_total_cores = body
        .pointer("/computeMetrics/cpuTotalCores")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let ceiling_max_cores = body
        .pointer("/constitutionalLimits/ceilingLimit/computeMaxCores")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .round() as u32;
    let allocation_cpu_cores = body
        .pointer("/allocations/allocationBlocks/0/cpuCores")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
        .round() as u32;
    Some(ComputeBudget {
        cpu_total_cores,
        ceiling_max_cores,
        allocation_cpu_cores,
    })
}

/// Auto-derive a render-capability claim. Honest by construction:
/// only bundles on disk whose renderer is referenced in storage's manifest
/// can appear in the claim. Override may reduce the claim but never inflate.
///
/// `compute_budget` carries the live per-node compute-as-pod view from
/// elohim-storage. When present and non-zero, the default for
/// `max_concurrent_renders` becomes
/// `min(cpu_total_cores, ceiling_max_cores, allocation_cpu_cores)` rather
/// than the static `DEFAULT_MAX_CONCURRENT`. Override (`override.toml`)
/// still wins and may only reduce.
///
/// Returns `Ok(None)` when:
/// - No bundles on disk match a manifest renderer (storage-only doorway)
/// - Override hides every bundle
/// - Manifest fetch fails (caller can retry on next derive)
pub async fn derive_capability(
    bundles_dir: &std::path::Path,
    storage_manifest_url: &str,
    override_path: Option<&std::path::Path>,
    compute_budget: Option<&ComputeBudget>,
) -> Result<Option<RenderCapabilityProfile>, CapabilityDeriverError> {
    let on_disk = scan_bundles(bundles_dir).await?;
    let manifest_renderers = match fetch_manifest_renderers(storage_manifest_url).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "manifest fetch failed — publishing renderCapability=null"
            );
            return Ok(None);
        }
    };
    let renderer_set: std::collections::HashSet<RendererKind> =
        manifest_renderers.iter().cloned().collect();

    // Filter on-disk bundles to those whose renderer is in the manifest set.
    let mut bundles: Vec<BundleEntry> = on_disk
        .into_iter()
        .filter(|b| renderer_set.contains(&b.renderer))
        .collect();

    let override_cfg = load_override(override_path).await;

    // Apply override: hide bundles
    if !override_cfg.bundles_hidden.is_empty() {
        bundles.retain(|b| !override_cfg.bundles_hidden.contains(&b.name));
    }

    if bundles.is_empty() {
        return Ok(None);
    }

    // Compute distinct renderers from filtered bundles (dedup, preserve order).
    let renderers: Vec<RendererKind> = {
        let mut seen = std::collections::HashSet::new();
        bundles
            .iter()
            .filter_map(|b| {
                if seen.insert(b.renderer.clone()) {
                    Some(b.renderer.clone())
                } else {
                    None
                }
            })
            .collect()
    };

    // Auth modes: override-restricted or default (anonymous + doorway-hosted).
    // Per spec, anonymous must always be present.
    let auth_modes: Vec<String> = match override_cfg.auth_modes {
        Some(modes) => {
            let mut m = modes;
            if !m.iter().any(|x| x == "anonymous") {
                m.insert(0, "anonymous".to_string());
            }
            m
        }
        None => DEFAULT_AUTH_MODES.iter().map(|s| s.to_string()).collect(),
    };

    // Default derives from the operator-as-pod budget when available; falls
    // back to DEFAULT_MAX_CONCURRENT only when probes are entirely unknown.
    // Override always wins.
    let derived_default = compute_budget
        .and_then(|b| b.min_cpu_budget())
        .unwrap_or(DEFAULT_MAX_CONCURRENT);
    let max_concurrent_renders = override_cfg.max_concurrent.unwrap_or(derived_default);
    let memory_budget_mib = override_cfg.memory_budget_mib;

    Ok(Some(RenderCapabilityProfile {
        bundles,
        renderers,
        auth_modes,
        max_concurrent_renders,
        memory_budget_mib,
    }))
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

#[derive(serde::Deserialize)]
struct ManifestResponse {
    routes: Vec<ManifestRoute>,
}

#[derive(serde::Deserialize)]
struct ManifestRoute {
    #[serde(default)]
    render: Option<String>,
}

/// Fetch elohim-storage's manifest and extract the unique set of renderers
/// declared by SSR-eligible routes. Errors propagate so the caller can
/// publish `renderCapability: null` and retry.
pub async fn fetch_manifest_renderers(
    storage_manifest_url: &str,
) -> Result<Vec<RendererKind>, CapabilityDeriverError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| CapabilityDeriverError::ManifestFetch(e.to_string()))?;
    let resp = client
        .get(storage_manifest_url)
        .send()
        .await
        .map_err(|e| CapabilityDeriverError::ManifestFetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(CapabilityDeriverError::ManifestFetch(format!(
            "HTTP {}",
            resp.status()
        )));
    }
    let manifest: ManifestResponse = resp
        .json()
        .await
        .map_err(|e| CapabilityDeriverError::ManifestFetch(e.to_string()))?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for route in manifest.routes {
        if let Some(rstr) = route.render {
            if let Ok(kind) =
                serde_json::from_value::<RendererKind>(serde_json::Value::String(rstr))
            {
                if seen.insert(kind.clone()) {
                    out.push(kind);
                }
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct RenderOverride {
    pub bundles_hidden: Vec<String>,
    pub max_concurrent: Option<u32>,
    pub auth_modes: Option<Vec<String>>,
    pub memory_budget_mib: Option<u32>,
}

#[derive(serde::Deserialize)]
struct OverrideRoot {
    #[serde(default)]
    render: RenderOverride,
}

/// Parse override TOML text. Returns `RenderOverride::default()` on empty input.
pub fn parse_override(text: &str) -> Result<RenderOverride, CapabilityDeriverError> {
    if text.trim().is_empty() {
        return Ok(RenderOverride::default());
    }
    let root: OverrideRoot = toml::from_str(text)
        .map_err(|e| CapabilityDeriverError::OverrideMalformed(e.to_string()))?;
    Ok(root.render)
}

/// Load override from a path. Missing file or malformed contents → default
/// (honest degradation: no override applied; warning logged).
pub async fn load_override(path: Option<&std::path::Path>) -> RenderOverride {
    let Some(path) = path else {
        return RenderOverride::default();
    };
    let contents = match tokio::fs::read_to_string(path).await {
        Ok(c) => c,
        Err(_) => {
            tracing::debug!(
                path = %path.display(),
                "override file not present — using defaults"
            );
            return RenderOverride::default();
        }
    };
    match parse_override(&contents) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "override file malformed — ignoring (using derived claim verbatim)"
            );
            RenderOverride::default()
        }
    }
}

#[cfg(test)]
mod derive_tests {
    use super::*;
    use crate::render::types::RendererKind;
    use std::fs;
    use tempfile::TempDir;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    async fn mock_manifest_with_angular(server: &MockServer) {
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "routes": [{ "path": "/lamad/concept/{id}", "render": "angular-ssr" }]
            })))
            .mount(server)
            .await;
    }

    fn write_lamad_bundle(dir: &std::path::Path) {
        fs::write(
            dir.join("lamad.bundle.mjs"),
            r#"/* @elohim-bundle {"name":"lamad-app","version":"1.0.3","renderer":"angular-ssr"} */"#,
        )
        .unwrap();
    }

    fn write_qahal_bundle(dir: &std::path::Path) {
        fs::write(
            dir.join("qahal.bundle.mjs"),
            r#"/* @elohim-bundle {"name":"qahal-app","version":"0.2.0","renderer":"angular-ssr"} */"#,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn derives_full_profile_when_disk_and_manifest_align() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let manifest_url = format!("{}/admin/manifest", server.uri());
        let result = derive_capability(bundles.path(), &manifest_url, None, None)
            .await
            .expect("derive ok");
        let profile = result.expect("non-null");
        assert_eq!(profile.bundles.len(), 1);
        assert_eq!(profile.bundles[0].name, "lamad-app");
        assert_eq!(profile.renderers, vec![RendererKind::AngularSsr]);
        assert!(profile.auth_modes.contains(&"anonymous".to_string()));
        assert!(profile.auth_modes.contains(&"doorway-hosted".to_string()));
        assert_eq!(profile.max_concurrent_renders, DEFAULT_MAX_CONCURRENT);
        assert!(profile.memory_budget_mib.is_none());
    }

    #[tokio::test]
    async fn returns_none_when_no_bundles_on_disk() {
        let bundles = TempDir::new().unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            None,
            None,
        )
        .await
        .expect("derive ok");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn returns_none_when_manifest_fetch_fails() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let result = derive_capability(bundles.path(), "http://127.0.0.1:1/never", None, None)
            .await
            .expect("derive ok (Ok(None) on manifest failure)");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn returns_none_when_bundle_renderer_not_in_manifest() {
        let bundles = TempDir::new().unwrap();
        // Bundle with renderer angular-ssr, but manifest declares only react-rsc
        write_lamad_bundle(bundles.path());
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "routes": [{ "path": "/foo", "render": "react-rsc" }]
            })))
            .mount(&server)
            .await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            None,
            None,
        )
        .await
        .expect("derive ok");
        assert!(
            result.is_none(),
            "lamad bundle filtered out — no renderable bundles"
        );
    }

    #[tokio::test]
    async fn override_reduces_max_concurrent() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        fs::write(&override_file, "[render]\nmax_concurrent = 1\n").unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            None,
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.max_concurrent_renders, 1);
    }

    #[tokio::test]
    async fn override_hides_bundle() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        write_qahal_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        fs::write(
            &override_file,
            r#"[render]
bundles_hidden = ["qahal-app"]
"#,
        )
        .unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            None,
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.bundles.len(), 1);
        assert_eq!(profile.bundles[0].name, "lamad-app");
    }

    #[tokio::test]
    async fn override_hiding_all_bundles_returns_none() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        fs::write(
            &override_file,
            r#"[render]
bundles_hidden = ["lamad-app"]
"#,
        )
        .unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            None,
        )
        .await
        .expect("derive ok");
        assert!(
            result.is_none(),
            "all bundles hidden — empty claim degrades to null"
        );
    }

    #[tokio::test]
    async fn override_restricting_auth_modes_keeps_anonymous_required() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        // Operator restricts to doorway-hosted ONLY (forgets anonymous);
        // deriver must add anonymous since spec requires it.
        fs::write(
            &override_file,
            r#"[render]
auth_modes = ["doorway-hosted"]
"#,
        )
        .unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            None,
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert!(profile.auth_modes.contains(&"anonymous".to_string()));
        assert!(profile.auth_modes.contains(&"doorway-hosted".to_string()));
    }

    #[tokio::test]
    async fn override_restricting_to_anonymous_only_works() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        fs::write(
            &override_file,
            r#"[render]
auth_modes = ["anonymous"]
"#,
        )
        .unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            None,
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.auth_modes, vec!["anonymous".to_string()]);
    }

    #[tokio::test]
    async fn compute_budget_drives_max_concurrent_default() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        // Probes report 8 CPUs, operator ceiling is 2, allocation is 4.
        // min(8, 2, 4) = 2, so default max_concurrent_renders should be 2.
        let budget = ComputeBudget {
            cpu_total_cores: 8,
            ceiling_max_cores: 2,
            allocation_cpu_cores: 4,
        };
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            None,
            Some(&budget),
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.max_concurrent_renders, 2);
    }

    #[tokio::test]
    async fn override_still_wins_over_compute_budget() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        fs::write(&override_file, "[render]\nmax_concurrent = 1\n").unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        // Budget would resolve to 4, but override pins to 1.
        let budget = ComputeBudget {
            cpu_total_cores: 8,
            ceiling_max_cores: 4,
            allocation_cpu_cores: 8,
        };
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            Some(&budget),
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.max_concurrent_renders, 1);
    }

    #[tokio::test]
    async fn budget_with_all_zeros_falls_back_to_default() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let budget = ComputeBudget::default();
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            None,
            Some(&budget),
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.max_concurrent_renders, DEFAULT_MAX_CONCURRENT);
    }

    #[tokio::test]
    async fn fetch_compute_budget_extracts_cpu_fields() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/api/v1/compute/dashboard"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "computeMetrics": { "cpuTotalCores": 12 },
                "constitutionalLimits": {
                    "ceilingLimit": { "computeMaxCores": 6.0 }
                },
                "allocations": {
                    "allocationBlocks": [
                        { "cpuCores": 8.0 }
                    ]
                }
            })))
            .mount(&server)
            .await;
        let url = format!("{}/api/v1/compute/dashboard", server.uri());
        let budget = fetch_compute_budget(&url).await.expect("budget present");
        assert_eq!(budget.cpu_total_cores, 12);
        assert_eq!(budget.ceiling_max_cores, 6);
        assert_eq!(budget.allocation_cpu_cores, 8);
        assert_eq!(budget.min_cpu_budget(), Some(6));
    }

    #[tokio::test]
    async fn fetch_compute_budget_unreachable_returns_none() {
        let result = fetch_compute_budget("http://127.0.0.1:1/never").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn override_memory_budget_propagates() {
        let bundles = TempDir::new().unwrap();
        write_lamad_bundle(bundles.path());
        let override_file = bundles.path().join("override.toml");
        fs::write(
            &override_file,
            r#"[render]
memory_budget_mib = 768
"#,
        )
        .unwrap();
        let server = MockServer::start().await;
        mock_manifest_with_angular(&server).await;
        let result = derive_capability(
            bundles.path(),
            &format!("{}/admin/manifest", server.uri()),
            Some(override_file.as_path()),
            None,
        )
        .await
        .expect("derive ok");
        let profile = result.unwrap();
        assert_eq!(profile.memory_budget_mib, Some(768));
    }
}

#[cfg(test)]
mod override_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parses_full_override() {
        let toml_str = r#"
[render]
bundles_hidden = ["qahal-app"]
max_concurrent = 2
auth_modes = ["anonymous"]
memory_budget_mib = 512
"#;
        let parsed = parse_override(toml_str).expect("parses");
        assert_eq!(parsed.bundles_hidden, vec!["qahal-app".to_string()]);
        assert_eq!(parsed.max_concurrent, Some(2));
        assert_eq!(parsed.auth_modes, Some(vec!["anonymous".to_string()]));
        assert_eq!(parsed.memory_budget_mib, Some(512));
    }

    #[test]
    fn empty_override_returns_default() {
        let parsed = parse_override("").expect("parses");
        assert!(parsed.bundles_hidden.is_empty());
        assert!(parsed.max_concurrent.is_none());
        assert!(parsed.auth_modes.is_none());
        assert!(parsed.memory_budget_mib.is_none());
    }

    #[test]
    fn whitespace_only_returns_default() {
        let parsed = parse_override("   \n\t\n  ").expect("parses");
        assert!(parsed.bundles_hidden.is_empty());
    }

    #[test]
    fn missing_render_section_returns_default() {
        // Valid TOML but no [render] section
        let parsed = parse_override("[other]\nfoo = 1\n").expect("parses");
        assert!(parsed.bundles_hidden.is_empty());
    }

    #[test]
    fn malformed_toml_returns_error() {
        let result = parse_override("[render\nbroken");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CapabilityDeriverError::OverrideMalformed(_)
        ));
    }

    #[tokio::test]
    async fn missing_file_returns_default() {
        let result = load_override(Some(std::path::Path::new(
            "/nonexistent/path/override.toml",
        )))
        .await;
        assert!(result.bundles_hidden.is_empty());
        assert!(result.max_concurrent.is_none());
    }

    #[tokio::test]
    async fn none_path_returns_default() {
        let result = load_override(None).await;
        assert!(result.bundles_hidden.is_empty());
    }

    #[tokio::test]
    async fn loads_valid_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("override.toml");
        fs::write(&path, "[render]\nmax_concurrent = 4\n").unwrap();
        let result = load_override(Some(path.as_path())).await;
        assert_eq!(result.max_concurrent, Some(4));
    }

    #[tokio::test]
    async fn malformed_file_falls_back_to_default() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("broken.toml");
        fs::write(&path, "[render\nthis is broken").unwrap();
        // Honest degradation: no panic, no error — just defaults
        let result = load_override(Some(path.as_path())).await;
        assert!(result.bundles_hidden.is_empty());
    }
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

#[cfg(test)]
mod manifest_tests {
    use super::*;
    use crate::render::types::RendererKind;
    use wiremock::{matchers, Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn fetches_ssr_eligible_renderers_from_manifest() {
        let server = MockServer::start().await;
        let manifest = serde_json::json!({
            "routes": [
                { "path": "/lamad/concept/{id}", "render": "angular-ssr" },
                { "path": "/lamad/path/{slug}", "render": "angular-ssr" },
                { "path": "/api/content/{id}" }
            ]
        });
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(manifest))
            .mount(&server)
            .await;
        let url = format!("{}/admin/manifest", server.uri());
        let renderers = fetch_manifest_renderers(&url).await.expect("fetch ok");
        assert!(renderers.contains(&RendererKind::AngularSsr));
        assert_eq!(renderers.len(), 1, "deduped angular-ssr appears once");
    }

    #[tokio::test]
    async fn dedupes_multiple_renderer_kinds() {
        let server = MockServer::start().await;
        let manifest = serde_json::json!({
            "routes": [
                { "path": "/a", "render": "angular-ssr" },
                { "path": "/b", "render": "react-rsc" },
                { "path": "/c", "render": "angular-ssr" },
                { "path": "/d" }
            ]
        });
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(manifest))
            .mount(&server)
            .await;
        let url = format!("{}/admin/manifest", server.uri());
        let renderers = fetch_manifest_renderers(&url).await.expect("fetch ok");
        assert_eq!(renderers.len(), 2);
        assert!(renderers.contains(&RendererKind::AngularSsr));
        assert!(renderers.contains(&RendererKind::ReactRsc));
    }

    #[tokio::test]
    async fn manifest_unreachable_returns_error() {
        let result = fetch_manifest_renderers("http://127.0.0.1:1/never").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CapabilityDeriverError::ManifestFetch(_)
        ));
    }

    #[tokio::test]
    async fn manifest_5xx_returns_error() {
        let server = MockServer::start().await;
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let url = format!("{}/admin/manifest", server.uri());
        let result = fetch_manifest_renderers(&url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn skips_unknown_renderer_kinds_silently() {
        let server = MockServer::start().await;
        let manifest = serde_json::json!({
            "routes": [
                { "path": "/a", "render": "angular-ssr" },
                { "path": "/b", "render": "future-renderer-kind" }
            ]
        });
        Mock::given(matchers::method("GET"))
            .and(matchers::path("/admin/manifest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(manifest))
            .mount(&server)
            .await;
        let url = format!("{}/admin/manifest", server.uri());
        let renderers = fetch_manifest_renderers(&url).await.expect("fetch ok");
        // Only the known one is included; unknowns drop silently.
        assert_eq!(renderers.len(), 1);
        assert!(renderers.contains(&RendererKind::AngularSsr));
    }
}
