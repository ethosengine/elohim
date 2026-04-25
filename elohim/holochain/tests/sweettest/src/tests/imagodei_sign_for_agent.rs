//! Sweettest — `sign_for_agent` coordinator function (EPR Phase 2B, Task C.1).
//!
//! Exercises the imagodei coordinator function that elohim-storage calls
//! from `/api/v1/signal/emit` (Task C.2) to sign EPR Envelope canonical bytes.
//!
//! Scenarios:
//!   1. `sign_succeeds_for_self` — a single agent signs arbitrary bytes under
//!      their own agent_cid (Stage 1 carve-out path: no Agent EPR registered);
//!      output carries a 64-byte signature and 32-byte pubkey matching the
//!      caller's `agent_initial_pubkey`. Validates the lair signing path and
//!      the wire-shape contract.
//!   2. `sign_signature_verifies` — same as (1) plus we verify the returned
//!      signature against the returned pubkey using ed25519-dalek. This is the
//!      load-bearing correctness check: if the bytes signed in lair are not
//!      what we send across the wire, downstream `EprService::ingest` will
//!      reject the resulting Envelope.
//!   3. `sign_rejects_wrong_signer` — agent B tries to sign with agent A's
//!      `agent_cid` (where A has registered an Agent EPR). The pre-commit
//!      signer-match gate fires → zome call errors. Mirrors the gate test
//!      pattern in `imagodei_peer_binding.rs`.
//!
//! NOTE on `#[ignore]`: these tests pack a real imagodei DNA. They are
//! `#[ignore]` until the Jenkins pipeline's pack-then-test stage runs them
//! upstream. Run locally with `cargo test --test imagodei_sign_for_agent --
//! --ignored`.

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
// Wire types — must match imagodei/src/sign_for_agent.rs (coordinator crate
// is WASM-only so we mirror the I/O struct here).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct SignForAgentInput {
    pub agent_cid: String,
    pub canonical_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct SignForAgentOutput {
    pub signature: Vec<u8>,
    pub signer_pubkey: Vec<u8>,
}

/// Mirror of `CreateAgentInput` (coordinator crate is WASM-only). Fields
/// must match the coordinator struct exactly.
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
// Scenario 1: happy path under Stage 1 self-sovereign carve-out
// ---------------------------------------------------------------------------

/// Single agent signs arbitrary bytes under a placeholder agent_cid for which
/// no Agent EPR is registered. The Stage 1 carve-out (see
/// `project_bootstrap_to_elohim_security_gradient`) permits this — lair signs
/// because it holds the caller's key. The signature is 64 bytes, the pubkey
/// is 32 bytes.
#[ignore = "requires packed DNA — run via Jenkins pack-then-test or `cargo test -- --ignored`"]
#[tokio::test(flavor = "multi_thread")]
async fn sign_succeeds_for_self() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    let input = SignForAgentInput {
        agent_cid: "bafy-stage1-carve-out-placeholder".to_string(),
        canonical_bytes: b"test bytes for signing".to_vec(),
    };

    let output: SignForAgentOutput = conductor
        .call(&cell.zome("imagodei"), "sign_for_agent", input)
        .await;

    assert_eq!(output.signature.len(), 64, "ed25519 signature is 64 bytes");
    assert_eq!(output.signer_pubkey.len(), 32, "ed25519 pubkey is 32 bytes");

    // The signer pubkey returned must equal the caller's agent_initial_pubkey
    // (raw 32-byte form, stripped of the 3-byte holo_hash prefix + 4-byte loc).
    // AgentPubKey::get_raw_32() is what the coordinator returns.
    let agent_raw_32 = agent.get_raw_32().to_vec();
    assert_eq!(
        output.signer_pubkey, agent_raw_32,
        "signer_pubkey must match the calling agent's pubkey"
    );

    let _ = SweetAgents::one;
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 2: signature actually verifies under the returned pubkey
// ---------------------------------------------------------------------------

/// Same as (1) but additionally verifies the returned signature against the
/// returned pubkey using ed25519-dalek. This catches the load-bearing failure
/// mode where lair signs *something*, but not the bytes we sent — which would
/// silently produce Envelopes that fail downstream `EprService::ingest`
/// signature verification.
#[ignore = "requires packed DNA — run via Jenkins pack-then-test or `cargo test -- --ignored`"]
#[tokio::test(flavor = "multi_thread")]
async fn sign_signature_verifies() -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("imagodei-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("cell installed").clone();

    let canonical_bytes = b"epr envelope canonical cbor stand-in".to_vec();
    let input = SignForAgentInput {
        agent_cid: "bafy-stage1-carve-out-placeholder".to_string(),
        canonical_bytes: canonical_bytes.clone(),
    };

    let output: SignForAgentOutput = conductor
        .call(&cell.zome("imagodei"), "sign_for_agent", input)
        .await;

    let pubkey_bytes: [u8; 32] = output
        .signer_pubkey
        .as_slice()
        .try_into()
        .expect("pubkey is 32 bytes");
    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_bytes).expect("returned pubkey is valid ed25519");

    let sig_bytes: [u8; 64] = output
        .signature
        .as_slice()
        .try_into()
        .expect("signature is 64 bytes");
    let signature = Signature::from_bytes(&sig_bytes);

    verifying_key
        .verify(&canonical_bytes, &signature)
        .expect("signature must verify against returned pubkey + canonical_bytes");

    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario 3: signer-match gate rejects cross-agent signing requests
// ---------------------------------------------------------------------------

/// Agent A registers an Agent EPR. Agent B then calls `sign_for_agent` with
/// A's `agent_cid`. The pre-commit gate looks up the Agent EPR, finds A's
/// `holochain_agent_key`, compares to B's caller pubkey, mismatch → reject.
///
/// Mirrors `imagodei_peer_binding.rs::binding_rejects_wrong_signer`.
///
/// Using a placeholder CID (no registered EPR) would hit the Stage 1
/// carve-out and falsely pass — that's why scenario (1) uses a placeholder
/// while this scenario registers a real EPR.
#[ignore = "requires packed DNA — run via Jenkins pack-then-test or `cargo test -- --ignored`"]
#[tokio::test(flavor = "multi_thread")]
async fn sign_rejects_wrong_signer() -> Result<()> {
    let [(mut c1, a1), (mut c2, a2)] = two_agent_conductors().await?;

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

    // A1 registers an Agent EPR identifying A1's pubkey.
    let a1_agent_id = "agent-a1-sign-for-agent-gate-test".to_string();
    let create_agent_input = CreateAgentInput {
        id: a1_agent_id.clone(),
        agent_type: "human".to_string(),
        display_name: "A1 (sign gate test)".to_string(),
        bio: None,
        avatar: None,
        affinities: vec![],
        visibility: "community".to_string(),
        location: None,
        did: None,
        activity_pub_type: None,
    };
    let _: serde_json::Value = c1
        .call(&cell1.zome("imagodei"), "create_agent", create_agent_input)
        .await;

    // Exchange peer info and wait for the Agent EPR to propagate to C2's DHT view.
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

    // B (on C2) tries to sign with A1's agent_cid.
    let input = SignForAgentInput {
        agent_cid: a1_agent_id.clone(),
        canonical_bytes: b"unauthorized bytes".to_vec(),
    };

    let result: holochain::conductor::api::error::ConductorApiResult<SignForAgentOutput> = c2
        .call_fallible(&cell2.zome("imagodei"), "sign_for_agent", input)
        .await;

    assert!(
        result.is_err(),
        "signer-match gate must reject B signing with A1's agent_cid"
    );

    let _ = (a2, cell1); // keep unused-binding lint happy
    Ok(())
}
