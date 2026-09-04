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

/// Mirror of `mishpat::GetCommitmentOutput` (lib.rs's `get_commitment` reader).
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
struct GetCommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub action: String,
    pub payload_json: String,
    pub signed_at: String,
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

// =============================================================================
// Task 2 fix round 3 — quorum-validator refusal-path coverage (review finding 1)
// and the sunsets-lineage identity-field symmetry fix (review finding 2).
// =============================================================================

/// The SAME agent appearing twice in `signatures` is refused before either
/// copy is even verified — `validate_lineage_signatures`'s duplicate-signer
/// guard, not a quorum-counting quirk (two identical entries would otherwise
/// silently count as two signers).
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_duplicate_signer() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let mut bad = migrates_lineage_payload(cid, &sig_b64, &alice);
    bad["signatures"] = serde_json::json!([
        {"agent": alice.to_string(), "signature": sig_b64},
        {"agent": alice.to_string(), "signature": sig_b64},
    ]);
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
        err.contains("duplicate signer"),
        "refusal must name the duplicate-signer condition: {err}"
    );
    Ok(())
}

/// A `signature` field that is not valid base64 at all is refused, naming
/// the decode failure — distinct from a well-formed-but-wrong-length or
/// well-formed-but-non-verifying signature.
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_non_base64_signature() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let mut bad = migrates_lineage_payload(cid, "unused", &alice);
    bad["signatures"] = serde_json::json!([
        {"agent": alice.to_string(), "signature": "not-valid-base64!!!"},
    ]);
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
        err.contains("signature not base64"),
        "refusal must name the base64-decode failure: {err}"
    );
    Ok(())
}

/// A `signature` that decodes as valid base64 but is the wrong byte length
/// (32 bytes, not the 64-byte ed25519 signature length) is refused, naming
/// the length requirement.
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_wrong_length_signature() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let short_sig_b64 = STANDARD.encode([0u8; 32]);
    let mut bad = migrates_lineage_payload(cid, "unused", &alice);
    bad["signatures"] = serde_json::json!([
        {"agent": alice.to_string(), "signature": short_sig_b64},
    ]);
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
        err.contains("signature must be 64 bytes"),
        "refusal must name the 64-byte length requirement: {err}"
    );
    Ok(())
}

/// `required_signatures: 2` with only one verified signature present is
/// refused as a quorum shortfall, naming the exact count (not just "not
/// enough signatures").
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_requires_quorum_of_required_signatures() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let mut bad = migrates_lineage_payload(cid, &sig_b64, &alice);
    bad["required_signatures"] = serde_json::json!(2);
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
        err.contains("quorum unmet: 1 of 2"),
        "refusal must name the exact shortfall: {err}"
    );
    Ok(())
}

/// A `window.opens_at` without the `Z` UTC suffix is refused — the
/// lexicographic-comparison precondition the validator's comment documents
/// (a fixed-offset timestamp would sort out of chronological order against
/// a `Z`-suffixed one).
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_non_z_opens_at() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let mut bad = migrates_lineage_payload(cid, &sig_b64, &alice);
    bad["window"] = serde_json::json!({
        "opens_at": "2026-09-04T00:00:00+00:00",
        "revert_until": "2026-09-11T00:00:00Z"
    });
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
        err.contains("RFC3339 UTC") && err.contains("Z"),
        "refusal must name the Z-suffix requirement: {err}"
    );
    Ok(())
}

/// `from_dna_hash == to_dna_hash` is refused for `migrates-lineage` — a
/// migration that doesn't actually move DNA lineage is not a migration.
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_equal_dna_hashes() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let mut bad = migrates_lineage_payload(cid, &sig_b64, &alice);
    bad["to_dna_hash"] = bad["from_dna_hash"].clone();
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
        err.contains("from_dna_hash and to_dna_hash must differ"),
        "refusal must name the identical-hash condition: {err}"
    );
    Ok(())
}

/// Review finding 2 (plan-mandated symmetry, spec §3): `sunsets-lineage`
/// must apply the SAME `from_dna_hash`/`to_dna_hash` identity checks as
/// `migrates-lineage` — `from_dna_hash == to_dna_hash` is refused with the
/// identical message.
#[tokio::test(flavor = "multi_thread")]
async fn sunsets_lineage_commitment_rejects_equal_dna_hashes() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let bad = serde_json::json!({
        "action": "sunsets-lineage",
        "role": "node_registry",
        "from_dna_hash": "uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH",
        "to_dna_hash": "uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH",
        "migration_commitment_cid": "uhCEkAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "signing_payload_cid": cid,
        "signatures": [{"agent": alice.to_string(), "signature": sig_b64}],
        "evidence": {"convergence": ["bafyconv"], "soak": ["bafysoak"], "deliberation": "bafyd"},
        "window": {"sunsets_at": "2026-09-11T00:00:00Z"}
    });
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
        err.contains("from_dna_hash and to_dna_hash must differ"),
        "sunsets-lineage must apply the same identity check as migrates-lineage: {err}"
    );
    Ok(())
}

/// Minor fix: a `revokes-commitment` naming a `target_action` OUTSIDE the two
/// known lineage actions is NOT gated by the quorum rule (the tightened
/// `target_action` match, not a bare non-empty-string check) — an unrelated
/// commitment class using the same field name for its own purposes must not
/// be accidentally quorum-gated.
#[tokio::test(flavor = "multi_thread")]
async fn revokes_commitment_ignores_non_lineage_target_action() -> Result<()> {
    let (conductor, cell, _alice) = mishpat_cell().await?;
    let payload = serde_json::json!({
        "action": "revokes-commitment",
        "target_cid": "some-non-lineage-commitment-cid",
        "target_action": "delegates-compute",
        "reason": "pin removed",
        "signed_at": "2026-09-05T00:00:00Z",
        "signatures": []
    });
    let out: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "revokes-commitment".into(),
                payload_json: payload.to_string(),
                signed_at: "2026-09-05T00:00:00Z".into(),
            },
        )
        .await;
    assert!(
        !out.entry_hash.to_string().is_empty(),
        "target_action naming a non-lineage action must not trigger the quorum gate"
    );
    Ok(())
}

/// Minor fix: `required_signatures` present but not an integer (e.g. a
/// string) is refused rather than silently defaulting to 1 — a caller who
/// mistypes the field must not accidentally get the weakest quorum.
#[tokio::test(flavor = "multi_thread")]
async fn migrates_lineage_commitment_rejects_non_integer_required_signatures() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi";
    let sig_b64 = sign_cid(&conductor, &alice, cid).await?;
    let mut bad = migrates_lineage_payload(cid, &sig_b64, &alice);
    bad["required_signatures"] = serde_json::json!("two");
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
        err.contains("required_signatures must be a non-negative integer"),
        "refusal must name the required_signatures type requirement: {err}"
    );
    Ok(())
}

// =============================================================================
// Holochain Evolution Epic Task 11 part 2a — create_lineage_commitment: the
// CALLING agent signs signing_payload_cid with its own key IN-ZOME (via
// sign_raw), satisfying the SAME validate_lineage_signatures gate
// create_commitment enforces. Exists because @holochain/client cannot sign
// raw bytes with the agent's real conductor-held key from JS.
// =============================================================================

/// A well-formed `migrates-lineage` payload with `signatures: []` — the
/// caller never supplies a signature; `create_lineage_commitment` appends its
/// own in-zome.
#[tokio::test(flavor = "multi_thread")]
async fn create_lineage_commitment_self_signs_and_validates() -> Result<()> {
    let (conductor, cell, alice) = mishpat_cell().await?;
    let cid = "bafy-test-create-lineage-commitment-self-signs";
    let mut unsigned = migrates_lineage_payload(cid, "", &alice);
    unsigned["signatures"] = serde_json::json!([]);

    let out: CommitmentOutput = conductor
        .call(
            &cell.zome("mishpat"),
            "create_lineage_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: unsigned.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await;
    assert!(
        !out.entry_hash.to_string().is_empty(),
        "create_lineage_commitment must self-sign in-zome and produce a valid commitment"
    );

    // Fetch the record back and inspect the projected signature — proves the
    // signature that landed on the DHT is the calling agent's own.
    let fetched: Option<GetCommitmentOutput> = conductor
        .call(
            &cell.zome("mishpat"),
            "get_commitment",
            out.entry_hash.to_string(),
        )
        .await;
    let fetched = fetched.expect("commitment must be readable back by its entry_hash cid");
    let signed_payload: serde_json::Value = serde_json::from_str(&fetched.payload_json)?;
    let signatures = signed_payload["signatures"]
        .as_array()
        .expect("signatures must be an array");
    assert_eq!(
        signatures.len(),
        1,
        "exactly one self-signature must be appended: {signatures:?}"
    );
    assert_eq!(
        signatures[0]["agent"].as_str(),
        Some(alice.to_string().as_str()),
        "the appended signature must be attributed to the calling agent"
    );

    // A second call by the SAME agent — on the payload already carrying
    // alice's signature (the record just fetched back) — is refused. A
    // double self-sign is a caller bug, not a silent no-op.
    let dup_err = conductor
        .call_fallible::<_, CommitmentOutput>(
            &cell.zome("mishpat"),
            "create_lineage_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: fetched.payload_json.clone(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(
        dup_err.contains("already signed"),
        "a second self-sign by the same agent must be refused, naming why: {dup_err}"
    );

    // Plain create_commitment on the ORIGINAL unsigned payload is refused by
    // the SAME validator create_lineage_commitment satisfies — proving
    // create_lineage_commitment's earlier success came from satisfying the
    // validator (via a real in-zome signature), not from bypassing it.
    let unsigned_err = conductor
        .call_fallible::<_, CommitmentOutput>(
            &cell.zome("mishpat"),
            "create_commitment",
            CreateCommitmentInput {
                action: "migrates-lineage".into(),
                payload_json: unsigned.to_string(),
                signed_at: "2026-09-04T00:00:00Z".into(),
            },
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(
        unsigned_err.contains("signatures"),
        "plain create_commitment on the unsigned payload must be refused by the same \
         validator: {unsigned_err}"
    );
    Ok(())
}
