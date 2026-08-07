//! Admin-seam wire compatibility: the conductor's app manifest carries `relay_url`.
//!
//! Regression guard for the alpha iroh partition (2026-08-07). The forked iroh
//! conductor (`elohim/holochain-conductor`, tag `fixt-0.6.3-6-g6d0814266`) emits
//! `relay_url` in the `AppManifestV0` wire shape returned inside every `AppInfo`
//! from `list_apps`. Upstream renamed `signal_url` -> `relay_url` in the 0.7 line
//! at `holochain_types 0.7.0-dev.23`; every earlier 0.7-dev release still expects
//! `signal_url` AND carries `#[serde(deny_unknown_fields)]` on `AppManifestV0`,
//! so an older client rejects the whole response with:
//!
//! ```text
//! Deserialize("unknown field relay_url, expected one of name, description,
//!  roles, allow_deferred_memproofs, bootstrap_url, signal_url")
//! ```
//!
//! That rejection breaks `happ_manager::ensure_happ_installed`, `hc_client`'s
//! ping, and `signing`'s cell discovery — the whole storage<->conductor admin
//! seam — fleet-wide.
//!
//! `deny_unknown_fields` is format-independent in serde, so JSON exercises the
//! exact rejection path the MessagePack conductor wire hits. The manifest below
//! is the shape a fork conductor actually returns for the `elohim` hApp.

use holochain_types::app::AppManifest;

/// The app-manifest wire shape emitted by the forked iroh conductor.
const FORK_CONDUCTOR_APP_MANIFEST_JSON: &str = r#"{
  "manifest_version": "0",
  "name": "elohim",
  "description": null,
  "roles": [
    {
      "name": "elohim",
      "provisioning": { "strategy": "create", "deferred": false },
      "dna": {
        "path": "elohim.dna",
        "modifiers": {},
        "installed_hash": null,
        "clone_limit": 0
      }
    }
  ],
  "allow_deferred_memproofs": false,
  "bootstrap_url": "https://doorway-alpha.elohim.host/bootstrap",
  "relay_url": "https://doorway-alpha.elohim.host"
}"#;

/// The pinned client must accept `relay_url` in the app manifest.
///
/// Fails with `unknown field relay_url` on any `holochain_types` older than
/// 0.7.0-dev.23 — which is precisely the alpha admin-seam break.
#[test]
fn app_manifest_from_iroh_conductor_deserializes_with_relay_url() {
    let manifest: AppManifest = serde_json::from_str(FORK_CONDUCTOR_APP_MANIFEST_JSON)
        .expect("conductor app manifest with relay_url must deserialize");

    let AppManifest::V0(v0) = manifest;
    assert_eq!(v0.name, "elohim");
    assert_eq!(v0.roles.len(), 1);
    assert_eq!(v0.roles[0].name, "elohim");
    assert_eq!(
        v0.bootstrap_url.as_deref(),
        Some("https://doorway-alpha.elohim.host/bootstrap")
    );
    assert_eq!(
        v0.relay_url.as_deref(),
        Some("https://doorway-alpha.elohim.host"),
        "relay_url must round-trip; the field is the conductor's iroh relay hint"
    );
}

/// A conductor that omits `relay_url` (tx5 lane, or a pre-iroh install) must
/// still deserialize — the field is `#[serde(default)]`, so the fix is additive
/// and staging/prod (still on tx5) are unaffected.
#[test]
fn app_manifest_without_relay_url_still_deserializes() {
    let json = r#"{
      "manifest_version": "0",
      "name": "elohim",
      "description": null,
      "roles": [],
      "allow_deferred_memproofs": false,
      "bootstrap_url": null
    }"#;

    let manifest: AppManifest =
        serde_json::from_str(json).expect("manifest without relay_url must deserialize");
    let AppManifest::V0(v0) = manifest;
    assert_eq!(v0.relay_url, None);
}

/// `deny_unknown_fields` is still in force for genuinely unknown fields — the
/// fix is a field addition, not a blanket loosening of the manifest contract.
#[test]
fn app_manifest_still_rejects_genuinely_unknown_fields() {
    let json = r#"{
      "manifest_version": "0",
      "name": "elohim",
      "roles": [],
      "not_a_real_manifest_field": "x"
    }"#;

    let err = serde_json::from_str::<AppManifest>(json)
        .expect_err("unknown manifest fields must still be rejected");
    assert!(
        err.to_string().contains("not_a_real_manifest_field"),
        "unexpected error: {err}"
    );
}
