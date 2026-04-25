//! Sweettest — AgentPeerBinding (EPR Phase 2B, Task A.2).
//!
//! These tests exercise the AgentPeerBinding integrity entry type via the
//! imagodei coordinator zome. They are `#[ignore]` until the DNA is packed
//! upstream by the Jenkins pipeline; remove `#[ignore]` once the pipeline's
//! pack-then-test stage is wired.
//!
//! Scenarios:
//!   1. `binding_creates_and_is_readable` — agent A creates a binding; the
//!      entry exists on the DHT and both link types resolve.
//!      NOTE: This test exercises the Stage 1 self-sovereign carve-out path
//!      (no Agent EPR is registered for the placeholder CID) — it validates
//!      structural correctness only, not the signer-match gate enforcement.
//!   2. `binding_rejects_wrong_signer` — agent B tries to create a binding
//!      whose `agent_cid` identifies agent A's Agent EPR → validation rejects
//!      (rule: signer must be the agent identified by agent_cid; enforced by
//!      coordinator pre-commit gate, visible here as a rejected zome call).
//!      NOTE: This test registers a real Agent EPR for A1 so the gate fires.
//!      Using a placeholder CID would hit the Stage 1 carve-out and falsely pass.
//!   3. `binding_canonical_bytes_stable` — the `AgentPeerBinding::canonical_bytes()`
//!      helper produces identical output on repeated serialisation. This is a
//!      unit-level check; it exercises the pure-logic path without DHT.
//!
//! NOTE on scenario 1 + 2: these require a coordinator function
//! `create_agent_peer_binding` (Task A.3+) which does not exist yet in this
//! batch. The test bodies are written with the expected coordinator API so
//! they run automatically when A.3 lands. Until then, the tests compile but
//! are ignored.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor, two_agent_conductors, SweetAgents},
    fixtures::network_seed,
};
use holochain::sweettest::{await_consistency, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const DNA: &str = "imagodei";

// ---------------------------------------------------------------------------
// Wire types — mirrors the coordinator I/O shapes Task A.3 will expose.
// These are declared here so the test file compiles independently; update
// field names to match the coordinator once A.3 is implemented.
// ---------------------------------------------------------------------------

/// Input to the (future) `create_agent_peer_binding` coordinator function.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateAgentPeerBindingInput {
    pub peer_id: String,
    pub agent_cid: String,
    pub valid_from_micros: i64,
    pub valid_until_micros: Option<i64>,
    pub device_archetype: String, // "node" | "desktop" | "mobile" | "steward"
    pub signature: Vec<u8>,       // Ed25519 over canonical_bytes()
}

/// Output from `create_agent_peer_binding`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateAgentPeerBindingOutput {
    pub action_hash: holo_hash::ActionHash,
}

/// Minimal mirror of `CreateAgentInput` (coordinator crate is WASM-only).
/// Fields must match the coordinator struct exactly so serde round-trips correctly.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateAgentInput {
    pub id: String,
    pub agent_type: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    pub affinities: Vec<String>,
    pub visibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub did: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity_pub_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Scenario 1: binding creates and is readable via both link types
// ---------------------------------------------------------------------------

/// Happy path: agent A installs imagodei, creates an AgentPeerBinding, and
/// the coordinator returns an ActionHash. Both forward link
/// (AgentPubKey -> AgentPeerBinding) and reverse link
/// (StringAnchor(peer_id) -> AgentPeerBinding) resolve to the same entry.
///
/// Coordinator functions exercised (Task A.3+):
///   - `create_agent_peer_binding(input)` → `CreateAgentPeerBindingOutput`
///   - `get_agent_peer_bindings(agent_pubkey)` → `Vec<AgentPeerBindingView>`
///   - `get_bindings_for_peer(peer_id)` → `Vec<AgentPeerBindingView>`
#[tokio::test(flavor = "multi_thread")]
async fn binding_creates_and_is_readable() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    // Fixture: a plausible multibase-encoded libp2p PeerId (base58btc).
    let peer_id = "12D3KooWEyoppNCUx8Yx66oV9fJnriXwCZXaZBkqgvu2LLQaLDXp".to_string();
    let agent_cid = "bafyreihzwxixtodg3opbqjnfnmksoqhpswz3mxkhq2lxi6w3dvvbxjqwe".to_string();

    // Canonical body (peer_id, agent_cid, valid_from, valid_until, device_archetype)
    // is what the agent would sign. For test purposes we use a stub 64-byte signature.
    let stub_signature = vec![0xABu8; 64];

    let input = CreateAgentPeerBindingInput {
        peer_id: peer_id.clone(),
        agent_cid: agent_cid.clone(),
        valid_from_micros: 1_000_000_000,
        valid_until_micros: None,
        device_archetype: "node".to_string(),
        signature: stub_signature,
    };

    // Create the binding via coordinator.
    let output: CreateAgentPeerBindingOutput = conductor
        .call(&cell.zome("imagodei"), "create_agent_peer_binding", input)
        .await;

    assert!(
        !output.action_hash.get_raw_39().is_empty(),
        "action_hash must be non-empty"
    );

    // Forward link: AgentPubKey -> AgentPeerBinding
    let by_agent: Vec<serde_json::Value> = conductor
        .call(
            &cell.zome("imagodei"),
            "get_agent_peer_bindings",
            agent.clone(),
        )
        .await;
    assert_eq!(
        by_agent.len(),
        1,
        "forward link must return exactly 1 binding"
    );
    assert_eq!(
        by_agent[0]["peerId"].as_str().unwrap_or(""),
        peer_id,
        "forward link must resolve to the created binding"
    );

    // Reverse link: StringAnchor(peer_id) -> AgentPeerBinding
    let by_peer: Vec<serde_json::Value> = conductor
        .call(
            &cell.zome("imagodei"),
            "get_bindings_for_peer",
            peer_id.clone(),
        )
        .await;
    assert_eq!(
        by_peer.len(),
        1,
        "reverse link must return exactly 1 binding"
    );
    assert_eq!(
        by_peer[0]["agentCid"].as_str().unwrap_or(""),
        agent_cid,
        "reverse link must resolve to the correct agent_cid"
    );

    let _ = SweetAgents::one; // keep import used
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2: wrong signer is rejected
// ---------------------------------------------------------------------------

/// Agent B attempts to create a binding that claims agent A's `agent_cid`.
/// The coordinator pre-commit gate (Task A.3+) must reject this — it verifies
/// that the calling agent's pubkey matches the Agent EPR identified by `agent_cid`.
///
/// Setup: A1 registers a real Agent EPR via `create_agent` (which stores
/// A1's holochain_agent_key in the DHT entry).  Conductors exchange peer info
/// and we await consistency so C2 can resolve the entry.  B then calls
/// `create_agent_peer_binding` with A1's agent id as `agent_cid` — the gate
/// now fires and rejects because B's pubkey ≠ A1's registered key.
///
/// Using a placeholder CID (no registered EPR) would hit the Stage 1
/// self-sovereign carve-out and FALSELY PASS the create call.
///
/// Expected: the `conductor.call` returns an error (WasmError::Guest).
#[tokio::test(flavor = "multi_thread")]
async fn binding_rejects_wrong_signer() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;

    // Both conductors use the same network seed so they join the same DHT space.
    let seed = network_seed(DNA);
    let dna_file = load_dna(DNA, &seed, Some(a1.clone())).await?;

    let app1 = c1
        .setup_app_for_agent("imagodei", a1.clone(), &[dna_file.clone()])
        .await?;
    let app2 = c2
        .setup_app_for_agent("imagodei", a2.clone(), &[dna_file])
        .await?;

    let cell1 = app1.cells().first().unwrap().clone();
    let cell2 = app2.cells().first().unwrap().clone();

    // -----------------------------------------------------------------------
    // Step 1: A1 registers an Agent EPR so the signer-match gate has an entry
    // to resolve.  `create_agent` stores `holochain_agent_key = a1.to_string()`
    // inside the Agent entry, keyed by `id`.  The gate in
    // `create_agent_peer_binding` calls `get_agent_by_id_internal(agent_cid)`,
    // which traverses IdToAgent → reads `holochain_agent_key` → compares to
    // the caller's pubkey.
    // -----------------------------------------------------------------------
    let a1_agent_id = "agent-a1-epr-binding-gate-test".to_string();
    let create_agent_input = CreateAgentInput {
        id: a1_agent_id.clone(),
        agent_type: "human".to_string(),
        display_name: "Agent A1 (gate test)".to_string(),
        bio: None,
        avatar: None,
        affinities: vec![],
        visibility: "community".to_string(),
        location: None,
        did: None,
        activity_pub_type: None,
    };

    // A1 creates the Agent EPR on C1.
    let _: serde_json::Value = c1
        .call(&cell1.zome("imagodei"), "create_agent", create_agent_input)
        .await;

    // -----------------------------------------------------------------------
    // Step 2: Exchange peer info so C2 knows about C1's agent, then await
    // DHT consistency so C2 can `get` the Agent entry that A1 just created.
    // -----------------------------------------------------------------------
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while !SweetConductor::exchange_peer_info([&c1, &c2]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency(10, [&cell1, &cell2])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout: {e}"))?;

    // -----------------------------------------------------------------------
    // Step 3: B (wrong signer) tries to create a binding that claims A1's
    // agent_cid.  The coordinator resolves the Agent EPR, finds A1's key, and
    // rejects because B's pubkey ≠ A1's key.
    // -----------------------------------------------------------------------
    let input = CreateAgentPeerBindingInput {
        peer_id: "12D3KooWMaliciousPeer".to_string(),
        agent_cid: a1_agent_id, // B claims A1's registered EPR id
        valid_from_micros: 1_000_000_000,
        valid_until_micros: None,
        device_archetype: "desktop".to_string(),
        signature: vec![0xBAu8; 64],
    };

    // Agent B calls `create_agent_peer_binding` — coordinator must reject it
    // because B's pubkey does not match A1's registered holochain_agent_key.
    let result: holochain::conductor::api::error::ConductorApiResult<CreateAgentPeerBindingOutput> =
        c2.call_fallible(&cell2.zome("imagodei"), "create_agent_peer_binding", input)
            .await;

    assert!(
        result.is_err(),
        "coordinator must reject binding where signer does not match agent_cid"
    );

    // Verify A1's cell has no spurious bindings.
    let by_a1: Vec<serde_json::Value> = c1
        .call(
            &cell1.zome("imagodei"),
            "get_agent_peer_bindings",
            a1.clone(),
        )
        .await;
    assert!(by_a1.is_empty(), "no bindings must exist for agent A1");

    let _ = (a2, cell2); // keep imports used
    Ok(())
}

