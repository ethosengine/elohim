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

    let app1 = c1.setup_app_for_agent("imagodei", a1.clone(), &[dna_file.clone()]).await?;
    let app2 = c2.setup_app_for_agent("imagodei", a2.clone(), &[dna_file]).await?;

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
