//! @dna-scope: mishpat
//! Sweettest baseline — mishpat (governance).
//!
//! Baseline (§2.1.3): bootstrap-steward creates a governance entry; a second
//! agent reads it; validation rejects unauthorized bootstrap-only creates.

use anyhow::Result;
use elohim_sweettest::common::{
    conductors::{load_dna, single_agent_conductor},
    fixtures::network_seed,
};

const DNA: &str = "mishpat";

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_steward_is_configured() -> Result<()> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("mishpat-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().unwrap().clone();

    let who: Option<holo_hash::AgentPubKey> = conductor
        .call(&cell.zome("mishpat"), "get_bootstrap_steward", ())
        .await;
    assert_eq!(who, Some(agent));
    Ok(())
}

// proposal_round_trips_across_agents removed — Stage F TODO: re-write as cross-DNA
// mishpat+elohim test once elohim::content_store::get_proposal_by_id is exposed and
// the harness installs both DNAs together. `get_proposal_by_id` was deliberately
// removed from the mishpat coordinator in #1231; proposals now live on the elohim DNA
// as `governance-action:proposal` Content entries. The CI runner uses
// `--run-ignored all` so #[ignore] alone would not prevent the test from running
// against the missing zome function. See: dna/mishpat/zomes/mishpat/src/lib.rs:272.

// =============================================================================
// Holochain Evolution Epic Task 2 — migrates-lineage / sunsets-lineage
// commitment arms + signature quorum (also gating revokes-commitment when the
// target is a lineage commitment). @dna-scope: mishpat.
//
// Single-conductor (self-signed) — no cross-agent DHT consistency needed, so
// (like `bootstrap_steward_is_configured` above) these are NOT `#[ignore]`d.
// =============================================================================

use base64::{engine::general_purpose::STANDARD, Engine as _};
use holo_hash::{ActionHash, AgentPubKey, EntryHash};
use holochain::sweettest::{SweetCell, SweetConductor};
use holochain_serialized_bytes::prelude::*;
use holochain_types::prelude::Signature;
use serde::{Deserialize, Serialize};

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

/// Single-conductor mishpat cell fixture — mirrors `bootstrap_steward_is_configured`'s
/// setup (single_agent_conductor + load_dna + setup_app_for_agent) so the
/// lineage-arm tests below don't repeat the conductor/DNA/app wiring.
async fn mishpat_cell() -> Result<(SweetConductor, SweetCell, AgentPubKey)> {
    let (mut conductor, agent) = single_agent_conductor().await?;
    let dna = load_dna(DNA, &network_seed(DNA), Some(agent.clone())).await?;
    let app = conductor
        .setup_app_for_agent("mishpat-app", agent.clone(), &[dna])
        .await?;
    let cell = app.cells().first().expect("mishpat cell").clone();
    Ok((conductor, cell, agent))
}

/// Well-formed `migrates-lineage` payload signed by `alice` over `cid`. Every
/// field name is verbatim from the epic spec §3 (Codex E, adopted).
fn migrates_lineage_payload(cid: &str, sig_b64: &str, alice: &AgentPubKey) -> serde_json::Value {
    serde_json::json!({
        "action": "migrates-lineage",
        "role": "node_registry",
        "from_dna_hash": "uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH",
        "to_dna_hash": "uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1",
        "release_cid": "uhCEkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "constitution_root": "bafyroot",
        "roster_cid": "bafyroster",
        "signing_payload_cid": cid,
        "signatures": [{"agent": alice.to_string(), "signature": sig_b64}],
        "evidence": {"soak": ["bafysoak"], "forecast": "bafyf", "deliberation": "bafyd"},
        "window": {"opens_at": "2026-09-04T00:00:00Z", "revert_until": "2026-09-11T00:00:00Z"}
    })
}

/// Signs `cid`'s UTF-8 bytes with `alice`'s conductor keystore key and returns
/// the base64-standard-encoded 64-byte signature. Uses the RAW (literal-bytes)
/// keystore signing path — `MetaLairClient::sign` — matching the zome's
/// `verify_signature_raw` counterpart (see commitments.rs's lineage-arm note:
/// the msgpack-serializing `verify_signature` would check against the encoded
/// bytes, not the literal bytes actually signed here).
async fn sign_cid(conductor: &SweetConductor, alice: &AgentPubKey, cid: &str) -> Result<String> {
    let sig: Signature = conductor
        .keystore()
        .sign(alice.clone(), cid.as_bytes().to_vec().into())
        .await?;
    Ok(STANDARD.encode(sig.0))
}

/// `migrates-lineage` with >=1 verified signature is accepted; the same
/// payload with an empty `signatures` array is refused, naming the field.
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_requires_signature() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let good = migrates_lineage_payload(cid, &sig_b64, &alice);

    let out: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: good.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await;
    assert!(
        !out.entry_hash.to_string().is_empty(),
        "well-formed migrates-lineage must be accepted and return a non-empty entry_hash"
    );

    let mut bad = good.clone();
    bad["signatures"] = serde_json::json!([]);
    let err = conductor
        .call_fallible::<_, CommitmentOutput>(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: bad.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("signatures"),
        "refusal must name the 'signatures' field: {err}"
    );
    Ok(())
}

/// A signature by an agent who never signed the payload (a forged/garbage
/// signature over the right bytes but the wrong keypair — here simulated by
/// reusing alice's signature under a DIFFERENT declared agent) is refused —
/// the quorum rule actually verifies, it does not just count entries.
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_signature_that_does_not_verify() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    // Sign a DIFFERENT string than the declared signing_payload_cid — the
    // signature is well-formed (64 bytes, valid base64) but does not verify
    // over `cid`.
    let wrong_sig_b64 = sign_cid(&conductor, &alice, "not-the-signing-payload-cid").await?;
    let bad = migrates_lineage_payload(cid, &wrong_sig_b64, &alice);

    let err = conductor
        .call_fallible::<_, CommitmentOutput>(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: bad.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("does not verify"),
        "a signature that does not verify over signing_payload_cid must be refused: {err}"
    );
    Ok(())
}

/// `sunsets-lineage` mirrors `migrates-lineage`'s quorum rule: a well-formed
/// payload with a verified signature is accepted; missing `window.sunsets_at`
/// is refused, naming the field.
#[tokio::test(flavor = "multi_thread")]
async fn sunsets_lineage_commitment_requires_signature_and_sunsets_at() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let good = serde_json::json!({
        "action": "sunsets-lineage",
        "role": "node_registry",
        "from_dna_hash": "uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH",
        "to_dna_hash": "uhC0kEKiIscIk5BDdethLGMFGLnvSvP2gRP5o74v0vAvoRnEzbiJ1",
        "migration_commitment_cid": "uhCEkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "signing_payload_cid": cid,
        "signatures": [{"agent": alice.to_string(), "signature": sig_b64}],
        "evidence": {"convergence": ["bafyconv"], "soak": ["bafysoak"], "deliberation": "bafyd"},
        "window": {"sunsets_at": "2026-09-11T00:00:00Z"}
    });
    let out: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "sunsets-lineage".into(),
                payload_json: good.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await;
    assert!(!out.entry_hash.to_string().is_empty());

    let mut bad = good.clone();
    bad["window"] = serde_json::json!({});
    let err = conductor
        .call_fallible::<_, CommitmentOutput>(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "sunsets-lineage".into(),
                payload_json: bad.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("sunsets_at"),
        "refusal must name the 'window.sunsets_at' field: {err}"
    );
    Ok(())
}

/// `revokes-commitment` against a lineage target (`target_action` present)
/// requires the same quorum as authoring the lineage commitment: refused with
/// zero signatures, accepted with one verified signature (k=1 default).
#[tokio::test(flavor = "multi_thread")]
async fn revokes_commitment_on_lineage_target_requires_quorum() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let migration = migrates_lineage_payload(cid, &sig_b64, &alice);
    let migrated: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: migration.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await;
    let target_cid = migrated.entry_hash.to_string();

    let revoke_cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbz00";
    let revoke_sig_b64 = sign_cid(&conductor, &alice, revoke_cid).await?;

    let no_sigs = serde_json::json!({
        "action": "revokes-commitment",
        "target_cid": target_cid,
        "target_action": "migrates-lineage",
        "reason": "release recalled",
        "signed_at": "2026-09-05T00:00:00Z",
        "signing_payload_cid": revoke_cid,
        "signatures": []
    });
    let err = conductor
        .call_fallible::<_, CommitmentOutput>(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "revokes-commitment".into(),
                payload_json: no_sigs.to_string(),
                signed_at: "2026-09-05T00:00:00Z".into(),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("signatures"),
        "revoking a lineage commitment with zero signatures must be refused: {err}"
    );

    let quorum_met = serde_json::json!({
        "action": "revokes-commitment",
        "target_cid": target_cid,
        "target_action": "migrates-lineage",
        "reason": "release recalled",
        "signed_at": "2026-09-05T00:00:00Z",
        "signing_payload_cid": revoke_cid,
        "signatures": [{"agent": alice.to_string(), "signature": revoke_sig_b64}]
    });
    let out: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "revokes-commitment".into(),
                payload_json: quorum_met.to_string(),
                signed_at: "2026-09-05T00:00:00Z".into(),
            },
        )
        .await;
    assert!(
        !out.entry_hash.to_string().is_empty(),
        "revoking a lineage commitment with a verified quorum must be accepted"
    );
    Ok(())
}

/// Legacy `revokes-commitment` payloads (no `target_action`) are unaffected
/// by the quorum rule — backward compatibility with the existing non-lineage
/// revoke path (`replicates_commons_substrate_correct_test.rs`'s `revokes_payload`).
#[tokio::test(flavor = "multi_thread")]
async fn revokes_commitment_without_target_action_is_unaffected() -> Result<()> {
    let (conductor, cell, _alice) = mishpat_cell().await?;
    let legacy = serde_json::json!({
        "action": "revokes-commitment",
        "target_cid": "some-non-lineage-commitment-cid",
        "reason": "pin removed",
        "signed_at": "2026-09-05T00:00:00Z"
    });
    let out: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "revokes-commitment".into(),
                payload_json: legacy.to_string(),
                signed_at: "2026-09-05T00:00:00Z".into(),
            },
        )
        .await;
    assert!(!out.entry_hash.to_string().is_empty());
    Ok(())
}
