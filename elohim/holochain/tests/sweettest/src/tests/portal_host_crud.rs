//! Sweettest — Recovery Protocol Phase 2 Milestone M5: PortalHost CRUD.
//!
//! These tests exercise the `add_portal_host`, `get_my_portal_hosts`, and
//! `remove_portal_host` coordinator functions introduced in M5 (Task 3).
//!
//! All DNA-bound scenarios are `#[ignore]` until the imagodei DNA artifact is
//! packed by the pipeline. See M4 tests for the same pattern.
//!
//! Scenarios:
//!   1. `portal_host_add_remove_roundtrip` — happy path: add, list, remove, verify empty.
//!   2. `portal_host_rejects_non_https` — integrity validator rejects `http://` URLs.
//!
//! Setup pattern mirrors `recovery_m4.rs`:
//!   1. Spin up a single-conductor/single-agent harness via `single_agent_conductor`.
//!   2. Load the imagodei DNA via `load_dna`.
//!   3. Install the DNA with `conductor.setup_app_for_agent`.
//!   4. Create a Human entry via `create_human` (required by the PortalHost
//!      coordinator pre-commit gate: `get_my_human_record`).
//!   5. Drive the PortalHost coordinator entry points.
//!   6. Assert on return values and list state.
//!
//! Local mirror structs (no path-dep on WASM coordinator crates):
//!   `CreateHumanInput`, `AddPortalHostInput`, `PortalHostReach`.
//!   Field names and variants must match the coordinator / integrity types
//!   exactly so msgpack round-tripping through the conductor is correct.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};
use holo_hash::ActionHash;

// ============================================================================
// Module-level constants
// ============================================================================

const DNA: &str = "imagodei";
/// Coordinator zome name as declared in imagodei/dna.yaml.
const ZOME: &str = "imagodei";

// ============================================================================
// Local I/O mirrors
//
// No path-dep on the coordinator crate (WASM-only). Mirror the exact field
// names / variant names so msgpack round-tripping is correct.
// Pattern mirrors `common/fixtures.rs::NodeRegistration`.
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct CreateHumanInput {
    id: String,
    display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bio: Option<String>,
    affinities: Vec<String>,
    profile_reach: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    location: Option<String>,
}

/// Mirror of `imagodei_integrity::PortalHostReach`.
/// Variants must match exactly — serde(rename_all) is NOT set on the
/// integrity enum, so PascalCase is the wire format.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
enum PortalHostReach {
    Trusted,
    Intimate,
    Public,
}

/// Mirror of `imagodei::portal_host::AddPortalHostInput`.
/// `#[serde(rename_all = "camelCase")]` is set on the coordinator struct,
/// so `host_url` → "hostUrl", `label` → "label", `reach` → "reach".
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct AddPortalHostInput {
    host_url: String,
    label: Option<String>,
    reach: Option<PortalHostReach>,
}

/// Partial mirror of `imagodei_integrity::PortalHost` — only the fields
/// asserted on in these tests.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct PortalHostWire {
    host_url: String,
    label: Option<String>,
}

// ============================================================================
// Helper: build a minimal valid CreateHumanInput
// ============================================================================

fn human_input(id: &str, display_name: &str) -> CreateHumanInput {
    CreateHumanInput {
        id: id.to_string(),
        display_name: display_name.to_string(),
        bio: None,
        affinities: vec![],
        profile_reach: "community".to_string(),
        location: None,
    }
}

// ============================================================================
// Scenario 1: Happy-path add → list → remove → verify empty
// ============================================================================

/// M5 Scenario 1: Portal host add/remove round-trip.
///
/// A Human is created, a PortalHost is added, the listing confirms the URL,
/// and after removal the list is empty.
///
/// Coordinator functions exercised:
///   - `add_portal_host`       → returns ActionHash
///   - `get_my_portal_hosts`   → Vec<(ActionHash, PortalHost)>
///   - `remove_portal_host`    → ()
///
/// Pre-commit gate: `get_my_human_record` in the coordinator requires an
/// `AgentKeyToHuman` link created by `create_human`. Without a Human the
/// `add_portal_host` call returns Err("No Human entry for this agent").
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn portal_host_add_remove_roundtrip() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), None).await?;

    let app = conductor
        .setup_app_for_agent("portal-host-roundtrip", agent.clone(), &[dna])
        .await?;
    let (cell,) = app.into_tuple();

    // Pre-req: create a Human so the PortalHost coordinator gate passes.
    let _human_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_human",
            human_input("human-matthew-m5-portal", "Matthew"),
        )
        .await;

    // Add a portal host.
    let add_input = AddPortalHostInput {
        host_url: "https://matthew.steward.example/account".to_string(),
        label: Some("Main steward".to_string()),
        reach: Some(PortalHostReach::Trusted),
    };
    let action_hash: ActionHash = conductor
        .call(&cell.zome(ZOME), "add_portal_host", add_input)
        .await;

    // Returned ActionHash must be non-empty (36-byte raw form).
    assert!(!action_hash.get_raw_36().is_empty());

    // List: exactly one portal host, correct URL.
    let hosts: Vec<(ActionHash, PortalHostWire)> = conductor
        .call(&cell.zome(ZOME), "get_my_portal_hosts", ())
        .await;
    assert_eq!(hosts.len(), 1, "expected 1 portal host after add");
    assert_eq!(
        hosts[0].1.host_url,
        "https://matthew.steward.example/account"
    );
    assert_eq!(hosts[0].1.label.as_deref(), Some("Main steward"));

    // Remove by URL.
    let _: () = conductor
        .call(
            &cell.zome(ZOME),
            "remove_portal_host",
            "https://matthew.steward.example/account".to_string(),
        )
        .await;

    // List must be empty after removal.
    let hosts_after: Vec<(ActionHash, PortalHostWire)> = conductor
        .call(&cell.zome(ZOME), "get_my_portal_hosts", ())
        .await;
    assert!(
        hosts_after.is_empty(),
        "expected empty portal host list after remove"
    );

    Ok(())
}

// ============================================================================
// Scenario 2: Validator rejects non-HTTPS URLs
// ============================================================================

/// M5 Scenario 2: Portal host validator rejects `http://` URLs.
///
/// The integrity-zome validator (`validate_create_portal_host`) enforces that
/// `host_url` must start with `https://`. Passing an `http://` URL must
/// produce a `ValidateCallbackResult::Invalid` response, which the conductor
/// surfaces as an error to the caller.
///
/// This test asserts the error path — `conductor.call_fallible` is used so
/// the Err variant is returned rather than panicking.
///
/// Validator rule (integrity/portal_host.rs Rule 1):
///   "PortalHost host_url must begin with https://"
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact — wire into Jenkins pack-then-test stage"]
async fn portal_host_rejects_non_https() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), None).await?;

    let app = conductor
        .setup_app_for_agent("portal-host-nonhttps", agent.clone(), &[dna])
        .await?;
    let (cell,) = app.into_tuple();

    // Pre-req: create a Human so the coordinator gate passes and the
    // call reaches the entry-creation step (and thus the validator).
    let _human_hash: ActionHash = conductor
        .call(
            &cell.zome(ZOME),
            "create_human",
            human_input("human-matthew-m5-nonhttps", "Matthew"),
        )
        .await;

    // Attempt to add a portal host with a plain http:// URL.
    let add_input = AddPortalHostInput {
        host_url: "http://insecure.example/account".to_string(),
        label: None,
        reach: Some(PortalHostReach::Trusted),
    };
    let result: std::result::Result<ActionHash, _> = conductor
        .call_fallible(&cell.zome(ZOME), "add_portal_host", add_input)
        .await;

    assert!(
        result.is_err(),
        "expected validator rejection for http:// URL, but call succeeded"
    );

    Ok(())
}

// ============================================================================
// Compile-time shape guard (NOT #[ignore]'d)
//
// Validates that the local mirror structs are structurally consistent with
// the coordinator wire format. No conductor required — just serde round-trip.
// ============================================================================

#[tokio::test(flavor = "multi_thread")]
async fn portal_host_wire_shape_guard() -> Result<()> {
    // AddPortalHostInput — camelCase on the wire.
    let input = AddPortalHostInput {
        host_url: "https://doorway.elohim.host".to_string(),
        label: Some("Primary doorway".to_string()),
        reach: Some(PortalHostReach::Trusted),
    };
    let json = serde_json::to_value(&input)?;
    assert_eq!(json["hostUrl"].as_str().unwrap_or(""), "https://doorway.elohim.host");
    assert_eq!(json["label"].as_str().unwrap_or(""), "Primary doorway");
    // reach variant name must be PascalCase (no rename on integrity enum).
    assert_eq!(json["reach"].as_str().unwrap_or(""), "Trusted");

    // PortalHostReach variants are plain strings.
    let reach_json = serde_json::to_string(&PortalHostReach::Trusted)?;
    assert_eq!(reach_json, "\"Trusted\"");

    Ok(())
}
