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
//! HOW THAT BREAKAGE MANIFESTS DEPENDS ON THE CLIENT FAMILY, and the pin has now
//! moved across three of them:
//!
//! - On the **0.7-dev line** (the old `holochain_client =0.9.0-dev.24` pin),
//!   `AppManifestV0` carries `#[serde(deny_unknown_fields)]` and renamed
//!   `signal_url` -> `relay_url` only at `holochain_types 0.7.0-dev.23`. A client
//!   below that rejects the WHOLE response with
//!   `Deserialize("unknown field relay_url, expected one of name, description,
//!    roles, allow_deferred_memproofs, bootstrap_url, signal_url")`.
//! - On the **0.6.3 line** (the conductor fork, taken as a git dep),
//!   `AppManifestV0` had NO `deny_unknown_fields` and declared `relay_url`
//!   natively with `#[serde(default)]`. Unknown fields were tolerated, so the
//!   loud-rejection failure mode did not exist there at all.
//! - On the **0.7.0 finals we now pin**, `AppManifestV0` is
//!   `#[serde(rename_all = "snake_case", deny_unknown_fields)]` again, and
//!   `relay_url` is a first-class field while `signal_url` is gone entirely
//!   (`holochain_types-0.7.0/src/app/app_manifest/app_manifest_v0.rs:37,70`).
//!   So strictness is BACK: a conductor-side manifest field this client family
//!   does not know will take the admin seam down exactly the way `relay_url`
//!   once did. Conductor-side manifest additions and this pin must move together.
//!
//! What this file guards, in every one of those worlds, is the thing that matters:
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
/// This assertion has now inverted TWICE, which is the whole reason it is a test
/// and not a comment:
///
/// - `holochain_types 0.7.0-dev.23` (old pin): STRICT — unknown keys rejected.
/// - the 0.6.3 conductor fork (interim pin): TOLERANT — unknown keys ignored.
/// - `holochain_types 0.7.0` (current pin): STRICT again —
///   `#[serde(rename_all = "snake_case", deny_unknown_fields)]` at
///   `app_manifest_v0.rs:37`.
///
/// Asserting the CURRENT direction is what keeps the seam honest, because
/// strictness is a live operational risk, not a detail: under 0.7 a conductor
/// that emits any manifest field this client family does not know takes down
/// `happ_manager::ensure_happ_installed`, `hc_client`'s ping and `signing`'s
/// cell discovery, fleet-wide — the original 2026-08-07 alpha partition. The
/// practical rule that follows: a conductor-fork manifest addition and this
/// client pin must land in the same batch.
///
/// If this test ever starts failing, the client family has moved OFF a
/// `deny_unknown_fields` line — check the pins in Cargo.toml before "fixing"
/// the test, and flip the assertion rather than deleting it.
#[test]
fn app_manifest_rejects_unknown_fields_on_the_pinned_family() {
    let json = r#"{
      "manifest_version": "0",
      "name": "elohim",
      "roles": [],
      "not_a_real_manifest_field": "x"
    }"#;

    let err = serde_json::from_str::<AppManifest>(json).expect_err(
        "the pinned 0.7.0 client family carries deny_unknown_fields on AppManifestV0, \
         so an unrecognised key must be rejected, not ignored",
    );
    let msg = err.to_string();
    assert!(
        msg.contains("not_a_real_manifest_field"),
        "the rejection must name the offending field (this is the diagnostic that \
         identified the 2026-08-07 admin-seam break); got: {msg}"
    );
}

/// The strictness above must NOT extend to the fields the conductor really sends.
///
/// Guards the other half of the contract: `relay_url` and `bootstrap_url` are
/// known keys on 0.7.0, so a manifest carrying both is accepted. Together with
/// the test above this pins "strict, and knows the right things" — strictness
/// alone would also be satisfied by a client that rejects everything.
#[test]
fn app_manifest_accepts_the_fields_the_conductor_actually_sends() {
    let manifest: AppManifest = serde_json::from_str(FORK_CONDUCTOR_APP_MANIFEST_JSON)
        .expect("relay_url + bootstrap_url are known fields on the 0.7.0 family");
    let AppManifest::V0(v0) = manifest;
    assert_eq!(
        v0.relay_url.as_deref(),
        Some("https://doorway-alpha.elohim.host")
    );
}
