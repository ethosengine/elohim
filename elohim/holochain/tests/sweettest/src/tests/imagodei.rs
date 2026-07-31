//! @dna-scope: imagodei
//! Sweettest baseline — imagodei (identity).
//!
//! Baseline scenarios (§2.1.3):
//! 1. Bootstrap-steward creates an identity record.
//! 2. Second agent joins and can see the identity via coordinator `get`.
//! 3. Validation rejects bootstrap-only actions from non-steward agents.
//!
//! Run via the Jenkins DNA Integration (bootstrap-steward) stage; the packed
//! DNA artifact is produced by the preceding Build DNA stage.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors, SweetAgents},
    fixtures::network_seed,
};

const DNA: &str = "imagodei";

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_steward_is_identifiable() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    let who: Option<holo_hash::AgentPubKey> = conductor
        .call(&cell.zome("imagodei"), "get_bootstrap_steward", ())
        .await;
    assert_eq!(who, Some(agent.clone()));

    let is_me: bool = conductor
        .call(&cell.zome("imagodei"), "is_bootstrap_steward", ())
        .await;
    assert!(is_me, "installing agent should be the bootstrap steward");
    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn second_agent_is_not_bootstrap_steward() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;
    let dna_file = load_dna(DNA, &network_seed(DNA), Some(a1.clone())).await?;

    let app1 = c1
        .setup_app_for_agent("imagodei", a1.clone(), &[dna_file.clone()])
        .await?;
    let app2 = c2
        .setup_app_for_agent("imagodei", a2.clone(), &[dna_file])
        .await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    let is_steward_1: bool = c1
        .call(&cell1.zome("imagodei"), "is_bootstrap_steward", ())
        .await;
    let is_steward_2: bool = c2
        .call(&cell2.zome("imagodei"), "is_bootstrap_steward", ())
        .await;

    assert!(is_steward_1);
    assert!(!is_steward_2, "second agent must NOT be bootstrap steward");
    let _ = SweetAgents::one; // keep import used even if helpers evolve
    Ok(())
}

// --- Scenario 3: intentionally absent ---
//
// The plan (wave1-sweettest-bodies §2.1) predicted: "there is no integrity
// validator in imagodei that rejects an action *because* the author is not the
// bootstrap steward." This prediction has been verified by reading all three
// source files in `imagodei_integrity/src/`:
//
//   - lib.rs          — dispatches to per-entry validate_* functions; zero
//                       references to `bootstrap_steward`, `progenitor`, or
//                       `is_bootstrap`.
//   - stewardship.rs  — `validate_stewardship_grant` gates on structural
//                       validity (empty IDs, enum membership, delegation depth)
//                       only; no author-identity check.
//   - recovery_v2.rs  — no bootstrap-steward references.
//
// The absence is deliberate, not a gap. The authority frame is documented in:
//   genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md
//
// Recommendation (b) from that spec was adopted: the bootstrap steward is the
// initial `constitutional`-tier steward at DNA install time, but authority is
// NOT exclusive to that pubkey. Gating integrity validators on the bootstrap
// pubkey would calcify exclusive authority — the opposite of the graduated-
// authority principle in `project_stewardship_philosophy.md`.
//
// The relevant design constraints from bootstrap_steward.rs (lines 28-37):
//
//   > Authority is **not** exclusive to this pubkey at any point; this module
//   > exposes only **identity**. Authority checks MUST go through the
//   > stewardship-grant resolution layer … Callers seeking "is this agent
//   > allowed to X?" must not use `is_bootstrap_steward` as a capability gate.
//
// When `StewardshipGrant`-based validators are added to the four ported DNAs
// (mishpat, node-registry, lamad, and imagodei itself) in a future wave, the
// gating condition will be "holds a StewardshipGrant at tier X with matching
// scope" — a check the bootstrap steward passes trivially at install time and
// that later-attested stewards also pass. At that point a cross-agent rejection
// test should be added here exercising the tier-based gate, not the bootstrap
// pubkey directly.
//
// See also: §7 (deferred work) of the authority frame design doc.

// =============================================================================
// HostedAgentBinding (doorway-federation-failover T2.2)
// =============================================================================
//
// Category A2 — a link on existing integrity surface (`LinkTypes::PeerToBinding`
// on a disjoint `StringAnchor("hosted_at", …)` base), so the DNA hash does not
// move. This test proves the coordinator round-trip: an agent binds itself to a
// home doorway, and the binding resolves back by agent pubkey.

#[tokio::test(flavor = "multi_thread")]
async fn hosted_agent_binding_round_trips() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-hosted-binding", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    // No binding before the write — resolution must be honestly empty, not an error.
    let before: Option<serde_json::Value> = conductor
        .call(
            &cell.zome("imagodei"),
            "get_hosted_agent_binding",
            agent.clone(),
        )
        .await;
    assert!(before.is_none(), "expected no binding before create");

    // Typed input mirror (the coordinator crate is WASM-only, so sweettests
    // mirror its input struct — same convention as imagodei_peer_binding.rs).
    // `rename_all = "camelCase"` must match the coordinator exactly or serde
    // silently drops fields over the msgpack zome-call boundary.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateHostedAgentBindingInput {
        doorway_id: String,
        doorway_url: String,
        installed_app_id: String,
    }

    let _created: serde_json::Value = conductor
        .call(
            &cell.zome("imagodei"),
            "create_hosted_agent_binding",
            CreateHostedAgentBindingInput {
                doorway_id: "alpha-a".to_string(),
                doorway_url: "https://doorway-alpha.elohim.host".to_string(),
                installed_app_id: "elohim-hosted-sweettest".to_string(),
            },
        )
        .await;

    let after: Option<serde_json::Value> = conductor
        .call(
            &cell.zome("imagodei"),
            "get_hosted_agent_binding",
            agent.clone(),
        )
        .await;
    let binding = after.expect("binding resolves after create");
    assert_eq!(binding["doorwayId"], "alpha-a");
    assert_eq!(binding["doorwayUrl"], "https://doorway-alpha.elohim.host");
    assert_eq!(binding["installedAppId"], "elohim-hosted-sweettest");
    assert!(
        binding["boundAtMicros"].as_i64().unwrap_or(0) > 0,
        "boundAtMicros must be stamped"
    );

    // Anchor-base disjointness: the hosted binding must NOT leak into the
    // AgentPeerBinding reverse index that shares LinkTypes::PeerToBinding.
    let peer_bindings: Vec<serde_json::Value> = conductor
        .call(
            &cell.zome("imagodei"),
            "get_bindings_for_peer",
            "alpha-a".to_string(),
        )
        .await;
    assert!(
        peer_bindings.is_empty(),
        "hosted binding leaked into the peer_binding anchor namespace: {peer_bindings:?}"
    );

    Ok(())
}
