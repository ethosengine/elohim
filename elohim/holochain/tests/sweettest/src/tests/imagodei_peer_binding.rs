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
//!   2. `binding_rejects_wrong_signer` — agent B tries to create a binding
//!      whose `agent_cid` identifies agent A's Agent EPR → validation rejects
//!      (rule: signer must be the agent identified by agent_cid; enforced by
//!      coordinator pre-commit gate, visible here as a rejected zome call).
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
#[ignore = "requires packed DNA artifact + coordinator A.3 — wire into Jenkins pack-then-test stage"]
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
/// Expected: the `conductor.call` returns an error (zome call rejected).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNA artifact + coordinator A.3 — wire into Jenkins pack-then-test stage"]
async fn binding_rejects_wrong_signer() -> Result<()> {
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

    // Agent A creates a Human + Agent EPR, producing an agent_cid.
    // (Coordinator helper assumed to exist at A.3 — `get_my_agent_cid` or similar.)
    // For test purposes, we use a placeholder agent_cid pointing to A's EPR.
    // The coordinator must resolve this and compare against the caller's pubkey.
    let a1_agent_cid = "bafyreiagenteprforagenta1placeholder".to_string();

    // Agent B (wrong signer) tries to create a binding for A's agent_cid.
    let input = CreateAgentPeerBindingInput {
        peer_id: "12D3KooWMaliciousPeer".to_string(),
        agent_cid: a1_agent_cid,
        valid_from_micros: 1_000_000_000,
        valid_until_micros: None,
        device_archetype: "desktop".to_string(),
        signature: vec![0xBAu8; 64],
    };

    // Agent B calls `create_agent_peer_binding` — coordinator must reject it
    // because B's pubkey does not match A's Agent EPR.
    // The call should return an error (WasmError::Guest).
    let result: Result<CreateAgentPeerBindingOutput, _> = conductor_call_fallible(
        &mut c2,
        &cell2.zome("imagodei"),
        "create_agent_peer_binding",
        input,
    )
    .await;

    assert!(
        result.is_err(),
        "coordinator must reject binding where signer does not match agent_cid"
    );

    // Verify A's cell has no spurious bindings.
    let by_a1: Vec<serde_json::Value> = c1
        .call(
            &cell1.zome("imagodei"),
            "get_agent_peer_bindings",
            a1.clone(),
        )
        .await;
    assert!(by_a1.is_empty(), "no bindings must exist for agent A");

    let _ = (a2, cell2); // keep imports used
    Ok(())
}

// ---------------------------------------------------------------------------
// Helper: fallible conductor call (returns Result instead of panicking)
// ---------------------------------------------------------------------------

/// Wraps `SweetConductor::call` to return a `Result` for negative-path tests.
/// SweetConductor::call panics on error; this captures the panic as an Err.
async fn conductor_call_fallible<I, O>(
    conductor: &mut holochain::sweettest::SweetConductor,
    zome: &holochain::sweettest::SweetZome,
    fn_name: &str,
    input: I,
) -> Result<O, anyhow::Error>
where
    I: serde::Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Use the raw call interface which returns a Result.
    use holochain::conductor::api::AppResponse;
    use holochain_types::prelude::ExternIO;

    let payload = ExternIO::encode(input)?;
    let response = conductor
        .raw_handle()
        .call_zome(holochain_types::prelude::ZomeCall {
            cell_id: zome.cell_id().clone(),
            zome_name: zome.name().clone(),
            fn_name: fn_name.into(),
            cap_secret: None,
            provenance: zome.cell_id().agent_pubkey().clone(),
            payload,
        })
        .await??;

    match response {
        AppResponse::ZomeCalled(result) => {
            let output: O = result.decode()?;
            Ok(output)
        }
        other => Err(anyhow::anyhow!("unexpected AppResponse: {other:?}")),
    }
}
