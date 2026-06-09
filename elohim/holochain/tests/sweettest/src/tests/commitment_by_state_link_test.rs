//! @dna-scope: mishpat
//! Sweettest — CommitmentByState link cross-conductor round-trip (Slice 2b T11).
//!
//! Proves the lifecycle-truth leg of the provide loop: a commitment's state
//! transition (proposed → active) is recorded as an immutable `CommitmentByState`
//! link on the Mishpat DHT, readable off the commitment anchor by a peer WITHOUT
//! replaying every EconomicEvent. The SQL `state` column in elohim-storage is
//! reframed as a write-through cache; THIS link is the truth.
//!
//! Flow:
//!   1. Agent A authors a `replicates-commons` Commitment (the live anchor).
//!   2. Agent A calls `create_commitment_state_link` with state="active",
//!      event_hash = the commitment's own action_hash (a stand-in for the
//!      graduating EconomicEvent's hash — any valid ActionHash works as the
//!      link target; the projection passes the real graduating event hash).
//!   3. exchange_peer_info + await_consistency (per _sweettest_cross_agent_consistency).
//!   4. Agent B reads `get_commitment_state_links(commitment_cid)` and sees the
//!      "active" transition with the round-tripped signed_at — proving DHT gossip
//!      replicated the link.
//!
//! `#[ignore]` — requires a packed mishpat.dna from the Jenkins pipeline (the
//! DNA sweettest stage runs `--run-ignored all`). Local:
//! `just pack && cargo test --test commitment_by_state_link_test -- --ignored`.

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
// ---------------------------------------------------------------------------

/// Mirror of `mishpat::commitments::CreateCommitmentInput`.
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

/// Mirror of `mishpat::commitments::CreateCommitmentStateLinkInput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CreateCommitmentStateLinkInput {
    pub commitment_cid: String,
    pub state: String,
    pub event_hash: String,
    pub signed_at: String,
}

/// Mirror of `mishpat::commitments::CommitmentStateLinkOutput`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CommitmentStateLinkOutput {
    pub link_action_hash: ActionHash,
}

/// Mirror of `mishpat::commitments::CommitmentStateLink`.
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct CommitmentStateLink {
    pub state: String,
    pub signed_at: String,
    pub event_hash: String,
}

/// Well-formed `replicates-commons` content-variant payload.
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

/// A `CommitmentByState` link recording proposed→active is notarized on the DHT
/// and readable by peer B off the commitment anchor — the lifecycle is DHT-truth,
/// replay-safe and peer-observable. None/empty == the link did not replicate.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "Requires packed mishpat.dna artifact from Jenkins pipeline"]
async fn commitment_by_state_link_notarized_and_readable_by_peer() -> Result<()> {
    let [(mut ca, a1), (mut cb, a2)] = two_agent_conductors().await?;

    let mishpat_dna = load_dna(MISHPAT_DNA, &network_seed(MISHPAT_DNA), Some(a1.clone())).await?;

    let app_a = ca
        .setup_app_for_agent("mishpat-app-alice-cbs", a1.clone(), &[mishpat_dna.clone()])
        .await?;
    let app_b = cb
        .setup_app_for_agent("mishpat-app-bob-cbs", a2.clone(), &[mishpat_dna])
        .await?;

    let cell_a = app_a.cells().first().expect("mishpat cell A").clone();
    let cell_b = app_b.cells().first().expect("mishpat cell B").clone();

    // --- Agent A authors the replicates-commons Commitment (the live anchor). ---
    let input = CreateCommitmentInput {
        action: "replicates-commons".to_string(),
        payload_json: replicates_commons_content_payload(),
        signed_at: "2026-06-11T00:00:00Z".to_string(),
    };
    let commitment: CommitmentOutput = ca
        .call(&cell_a.zome(MISHPAT_ZOME), "create_commitment", input)
        .await;

    let commitment_cid = commitment.entry_hash.to_string();

    // --- Agent A authors the CommitmentByState link (proposed → active). ---
    // The graduating event's ActionHash is the link target. Here we use the
    // commitment's own action_hash as a valid ActionHash stand-in; the storage
    // graduation projection passes the real ProvideAnnounce event hash.
    let link_input = CreateCommitmentStateLinkInput {
        commitment_cid: commitment_cid.clone(),
        state: "active".to_string(),
        event_hash: commitment.action_hash.to_string(),
        signed_at: "2026-06-11T10:00:00Z".to_string(),
    };
    let link_out: CommitmentStateLinkOutput = ca
        .call(
            &cell_a.zome(MISHPAT_ZOME),
            "create_commitment_state_link",
            link_input,
        )
        .await;
    assert_eq!(
        link_out.link_action_hash.get_raw_32().len(),
        32,
        "create_commitment_state_link must return a 32-byte link ActionHash"
    );

    // --- Exchange peer info then await DHT consistency. ---
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        while !SweetConductor::exchange_peer_info([&ca, &cb]).await {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for peer info exchange"))?;

    await_consistency(60, [&cell_a, &cell_b])
        .await
        .map_err(|e| anyhow::anyhow!("DHT consistency timeout after state link: {e}"))?;

    // --- Bob reads the CommitmentByState links off the commitment anchor. ---
    let links: Vec<CommitmentStateLink> = cb
        .call(
            &cell_b.zome(MISHPAT_ZOME),
            "get_commitment_state_links",
            commitment_cid.clone(),
        )
        .await;

    assert!(
        !links.is_empty(),
        "Bob must read Alice's CommitmentByState link off the commitment anchor after \
         exchange_peer_info + await_consistency. Empty == DHT gossip did not replicate the link."
    );

    let active = links
        .iter()
        .find(|l| l.state == "active")
        .expect("an 'active' lifecycle link must be present");
    assert_eq!(
        active.signed_at, "2026-06-11T10:00:00Z",
        "the link tag's signed_at must round-trip across peers"
    );
    assert_eq!(
        active.event_hash,
        commitment.action_hash.to_string(),
        "the link target (graduating event hash) must round-trip across peers"
    );

    Ok(())
}
