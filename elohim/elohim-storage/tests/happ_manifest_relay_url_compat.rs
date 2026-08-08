//! Admin-seam wire compatibility: the conductor's app manifest carries `relay_url`.
//!
//! Regression guard for the alpha iroh partition (2026-08-07). The forked iroh
//! conductor (`elohim/holochain-conductor`, branch `elohim-0.6.3` @ 6d0814266)
//! emits `relay_url` in the `AppManifestV0` wire shape returned inside every
//! `AppInfo` from `list_apps`. A client whose `holochain_types` does not know
//! that field breaks `happ_manager::ensure_happ_installed`, `hc_client`'s ping,
//! and `signing`'s cell discovery — the whole storage<->conductor admin seam —
//! fleet-wide.
//!
//! HOW THAT BREAKAGE MANIFESTS DEPENDS ON THE CLIENT FAMILY, and the pin moved
//! between the two on 2026-08-08:
//!
//! - On the **0.7-dev line** (the old `holochain_client =0.9.0-dev.24` pin),
//!   `AppManifestV0` carries `#[serde(deny_unknown_fields)]` and renamed
//!   `signal_url` -> `relay_url` only at `holochain_types 0.7.0-dev.23`. A client
//!   below that rejects the WHOLE response with
//!   `Deserialize("unknown field relay_url, expected one of name, description,
//!    roles, allow_deferred_memproofs, bootstrap_url, signal_url")`.
//! - On the **0.6.3 line we now pin** (the conductor's own family, taken as a git
//!   dep on the fork), `AppManifestV0` has NO `deny_unknown_fields` and declares
//!   `relay_url` natively with `#[serde(default)]`
//!   (`crates/holochain_types/src/app/app_manifest/app_manifest_v0.rs:70`).
//!   Unknown fields are tolerated, so the loud-rejection failure mode does not
//!   exist here at all — the field is simply known and carried.
//!
//! What this file guards, in both worlds, is the thing that actually matters:
//! the client type KNOWS `relay_url` and ROUND-TRIPS it, rather than dropping
//! the conductor's iroh relay hint on the floor. serde field handling is
//! format-independent, so JSON exercises the same path as the MessagePack wire.
//! The manifest below is the shape a fork conductor actually returns for the
//! `elohim` hApp.

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

/// Records the manifest strictness contract of the client family we pin.
///
/// On the 0.6.3 family (the conductor's own — see the module docs) `AppManifestV0`
/// does NOT carry `#[serde(deny_unknown_fields)]`, so an unrecognised key is
/// ignored rather than rejected. This is asserted, not merely tolerated, for two
/// reasons:
///
/// 1. It is the property that makes this family robust across conductor drift —
///    a future conductor-side manifest field cannot take down the admin seam the
///    way `relay_url` did on the strict 0.7-dev line.
/// 2. It pins the difference so nobody re-imports the old assumption. The
///    previous version of this test asserted the OPPOSITE (that unknown fields
///    are rejected) and was correct only for `holochain_types 0.7.0-dev.23`.
///    If this test ever starts failing, the client family has moved back onto a
///    `deny_unknown_fields` line — check the pins in Cargo.toml before "fixing"
///    the test.
#[test]
fn app_manifest_tolerates_unknown_fields_on_the_pinned_family() {
    let json = r#"{
      "manifest_version": "0",
      "name": "elohim",
      "roles": [],
      "not_a_real_manifest_field": "x"
    }"#;

    let manifest: AppManifest = serde_json::from_str(json).expect(
        "the pinned 0.6.3 client family has no deny_unknown_fields on AppManifestV0, \
         so an unrecognised key must be ignored, not rejected",
    );
    let AppManifest::V0(v0) = manifest;
    assert_eq!(v0.name, "elohim");
    assert_eq!(
        v0.relay_url, None,
        "an unknown key must not be mistaken for relay_url"
    );
}
