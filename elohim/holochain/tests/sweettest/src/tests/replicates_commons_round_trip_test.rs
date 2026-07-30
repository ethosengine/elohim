//! @dna-scope: mishpat
//! Sweettest — replicates-commons conductor round-trip (Slice 2b T1 HARD GATE).
//!
//! Proves the notarization leg of the provide loop: Agent A authors a
//! `replicates-commons` Commitment with a caller-supplied `signed_at`; the
//! mishpat coordinator validates + create_entry's it; after exchange_peer_info
//! + await_consistency Agent B reads it back via the NEW `get_commitment`
//! extern. The returned `action_hash` is exactly what the elohim-storage
//! post-commit projection writes into `mishpat_commitments.dht_anchor_hash`
//! (NON-NULL), so a 32-byte action_hash here == a notarized, bounds-checkable
//! row in storage. Spec §6.5 + the Slice-2b shared contract.
//!
//! `#[ignore]` — requires a packed mishpat.dna from the Jenkins pipeline (the
//! DNA sweettest stage runs `--run-ignored all`). Local:
//! `just pack && cargo test --test replicates_commons_round_trip_test -- --ignored`.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, two_agent_conductors},
    fixtures::network_seed,
};
use holo_hash::{ActionHash, EntryHash};
use holochain::sweettest::{await_consistency, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

const MISHPAT_DNA: &str = "mishpat";
const MISHPAT_ZOME: &str = "mishpat";

// ---------------------------------------------------------------------------
// Local mirrors — field names MUST match the coordinator's serde structs.
// CreateCommitmentInput gains `signed_at` in this task (was sys_time()-internal).
// ---------------------------------------------------------------------------

/// Mirror of `mishpat::commitments::CreateCommitmentInput` (post-Slice-2b).
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateCommitmentInput {
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
}

/// Mirror of `mishpat::commitments::CommitmentOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
}

/// Mirror of the wire shape returned by the NEW `get_commitment` extern.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct GetCommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
}

/// Well-formed `replicates-commons` content-variant payload (per the Slice-2b
/// schema oneOf): reach == "commons", bounds with rate_per_minute + reach_ceiling,
/// NO ratio_attestation (content variant).
fn replicates_commons_content_payload() -> String {
    serde_json::json!({
        "action": "replicates-commons",
        "variant": "content",
        "head_ref": "epr:lamad-spa-head-cid",
        "reach": "commons",
        "bounds": {
            "rate_per_minute": 60,
            "reach_ceiling": "commons"
        }
    })
    .to_string()
}

/// A well-formed `replicates-commons` Commitment is accepted by the coordinator,
/// notarized on the DHT, and readable by peer B via `get_commitment`. The
/// returned action_hash is the future `dht_anchor_hash` — a 32-byte hash here
/// means the storage projection writes a NON-NULL anchor.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed mishpat.dna artifact from Jenkins pipeline"]
async fn replicates_commons_notarized_and_readable_by_peer() -> Result<()> {
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;

    let mishpat_dna = load_dna(MISHPAT_DNA, &network_seed(MISHPAT_DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("mishpat-app-alice-rc", a1.clone(), &[mishpat_dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("mishpat-app-bob-rc", a2.clone(), &[mishpat_dna])
        .await?;

    let cell_a = app_a.cells().first().expect("mishpat cell A").clone();
    let cell_b = app_b.cells().first().expect("mishpat cell B").clone();

    // --- Agent A authors the replicates-commons Commitment with explicit signed_at. ---
    let input = CreateCommitmentInput {
        action: "replicates-commons".to_string(),
        payload_json: replicates_commons_content_payload(),
        signed_at: "2026-06-10T00:00:00Z".to_string(),
    };

    let alice_output: CommitmentOutput = ca
        .call(&cell_a.zome(MISHPAT_ZOME), "create_commitment", input)
        .await;

    assert_eq!(
        alice_output.action_hash.get_raw_32().len(),
        32,
        "create_commitment must return a 32-byte ActionHash (the future dht_anchor_hash)"
    );

    // --- Exchange peer info then await DHT consistency (per _sweettest_cross_agent_consistency). ---
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency(60, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after create_commitment: {e}"))?;

    // --- Bob reads it back by entry_hash (the storage `cid`) via get_commitment. ---
    // The `cid` storage uses is the base64 entry_hash; get_commitment takes that
    // string and resolves the notarized record. Some(...) here proves DHT gossip
    // propagated the Commitment AND that the action_hash is a real anchor.
    let cid = alice_output.entry_hash.to_string();
    let bob_view: Option<GetCommitmentOutput> = cb
        .call(&cell_b.zome(MISHPAT_ZOME), "get_commitment", cid.clone())
        .await;

    let got = bob_view.ok_or_else(|| {
        anyhow::anyhow!(
            "Bob must read Alice's replicates-commons commitment by entry_hash after \
             exchange_peer_info + await_consistency. None == DHT gossip did not propagate."
        )
    })?;

    assert_eq!(got.action, "replicates-commons");
    assert_eq!(
        got.signed_at, "2026-06-10T00:00:00Z",
        "caller signed_at must round-trip"
    );
    assert_eq!(
        got.action_hash, alice_output.action_hash,
        "get_commitment action_hash (== dht_anchor_hash) must be byte-identical across peers"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&got.payload_json).expect("payload_json must be valid JSON");
    assert_eq!(parsed["variant"], "content");
    assert_eq!(parsed["reach"], "commons");

    Ok(())
}
