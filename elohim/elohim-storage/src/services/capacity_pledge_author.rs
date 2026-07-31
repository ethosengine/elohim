//! Conductor-first authoring for commons capacity pledges.
//!
//! A capacity pledge is the `capacity` variant of the existing
//! `replicates-commons` Mishpat Commitment. It is a Class-A promise: the generic
//! `mishpat::create_commitment` coordinator notarizes it, the normal
//! `CommitmentCommitted` signal projects it, and the eager projection below
//! closes the request/signal race for the HTTP caller.
//!
//! The HTTP surface is deliberately last and thin. It accepts the canonical
//! typed payload, resolves the node steward's `agent_cid`, and delegates here;
//! no operational capacity measurement is silently converted into consent.

use std::sync::Arc;

use diesel::SqliteConnection;
use elohim_views::replicates_commons::ReplicatesContentPayload;
use elohim_views::MishpatCommitmentView;

use crate::db::context::AppContext;
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::mishpat_projection::CommitmentProjection;
use crate::services::conductor_writes::{
    self, CreateCommitmentOutput, CreateMishpatCommitmentInput,
};

/// Serialize authoring with the persistent exact-pledge check. This closes the
/// same-process race where two explicit requests arrive before either eager
/// projection is visible.
static CAPACITY_PLEDGE_AUTHOR_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Resolve the pledging node's canonical agent identity. Candidate order is the
/// repository-wide write convention: active local session, then this
/// conductor cell's own key. A transport id is never written into an agent-CID
/// join column.
pub fn resolve_provider(conn: &mut SqliteConnection, hc: &HcClient) -> Option<String> {
    crate::services::salvage_commitment_author::resolve_self_agent_cid(conn, hc)
}

/// Build the exact snake_case DHT payload for the already-typed capacity
/// request. The authenticated provider is injected server-side: callers may
/// choose the byte budget, but cannot claim another agent's identity.
pub fn build_capacity_payload(
    provider: &str,
    payload: &ReplicatesContentPayload,
) -> Result<String, StorageError> {
    crate::services::replicates_commons_validator::validate_typed_for_creation_commons(payload)
        .map_err(|e| StorageError::InvalidInput(e.to_string()))?;

    let ReplicatesContentPayload::Capacity {
        commons_bytes,
        bounds,
        ratio_attestation,
    } = payload
    else {
        return Err(StorageError::InvalidInput(
            "capacity pledge endpoint requires variant 'capacity'".to_string(),
        ));
    };

    Ok(serde_json::json!({
        "action": "replicates-commons",
        "variant": "capacity",
        "commons_bytes": commons_bytes,
        "reach": "commons",
        "bounds": {
            "rate_per_minute": bounds.rate_per_minute,
            "reach_ceiling": bounds.reach_ceiling,
        },
        "ratio_attestation": {
            "commons_pct": ratio_attestation.commons_pct,
            "dwelling_pct": ratio_attestation.dwelling_pct,
            "collective_pct": ratio_attestation.collective_pct,
            "free_pct": ratio_attestation.free_pct,
            "effective_ratio_cid": ratio_attestation.effective_ratio_cid,
        },
        "provider": provider,
    })
    .to_string())
}

/// True when a live projected row already represents this exact pledge.
///
/// The persistent check makes an explicit retry idempotent across process
/// restarts. A changed byte budget or ratio attestation is a new immutable
/// promise; withdrawing the prior promise remains the existing
/// `revokes-commitment` lifecycle.
fn is_exact_live_pledge(
    row: &crate::db::models::MishpatCommitment,
    provider: &str,
    payload: &ReplicatesContentPayload,
) -> bool {
    let ReplicatesContentPayload::Capacity {
        commons_bytes,
        bounds,
        ratio_attestation,
    } = payload
    else {
        return false;
    };
    if row.action != "replicates-commons"
        || row.provider != provider
        || !row.recipient.is_empty()
        || row.revoked_at.is_some()
    {
        return false;
    }

    let Ok(projected) = serde_json::from_str::<serde_json::Value>(&row.bounds_json) else {
        return false;
    };
    projected.get("commons_bytes").and_then(|v| v.as_u64()) == Some(*commons_bytes)
        && projected.get("rate_per_minute").and_then(|v| v.as_u64())
            == Some(u64::from(bounds.rate_per_minute))
        && projected.get("reach_ceiling").and_then(|v| v.as_str())
            == Some(bounds.reach_ceiling.as_str())
        && projected
            .pointer("/ratio_attestation/commons_pct")
            .and_then(|v| v.as_u64())
            == Some(u64::from(ratio_attestation.commons_pct))
        && projected
            .pointer("/ratio_attestation/dwelling_pct")
            .and_then(|v| v.as_u64())
            == Some(u64::from(ratio_attestation.dwelling_pct))
        && projected
            .pointer("/ratio_attestation/collective_pct")
            .and_then(|v| v.as_u64())
            == Some(u64::from(ratio_attestation.collective_pct))
        && projected
            .pointer("/ratio_attestation/free_pct")
            .and_then(|v| v.as_u64())
            == Some(u64::from(ratio_attestation.free_pct))
        && projected
            .pointer("/ratio_attestation/effective_ratio_cid")
            .and_then(|v| v.as_str())
            == Some(ratio_attestation.effective_ratio_cid.as_str())
}

/// Author one explicit capacity pledge and eagerly project the notarized entry.
pub async fn author_capacity_pledge(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    payload: ReplicatesContentPayload,
    hc: &Arc<HcClient>,
) -> Result<MishpatCommitmentView, StorageError> {
    let _guard = CAPACITY_PLEDGE_AUTHOR_LOCK.lock().await;
    let provider = resolve_provider(conn, hc).ok_or_else(|| {
        StorageError::Auth("no canonical agent_cid available for the pledging steward".to_string())
    })?;
    let payload_json = build_capacity_payload(&provider, &payload)?;

    if let Some(existing) = crate::db::mishpat_commitments::list_all(conn)?
        .into_iter()
        .find(|row| is_exact_live_pledge(row, &provider, &payload))
    {
        return Ok(crate::services::mishpat_commitment_facing::to_view(
            existing,
        ));
    }

    // Midnight UTC is deterministic within the retry window. If the conductor
    // accepted the first call but the client lost the response, retrying today
    // recreates the same EntryHash; eager projection plus the persistent exact
    // check handles later retries.
    let signed_at = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let bytes = conductor_writes::call_create_commitment(
        hc,
        CreateMishpatCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload_json.clone(),
            signed_at,
        },
    )
    .await?;
    let output: CreateCommitmentOutput = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Serialization(format!(
            "capacity pledge: decode CreateCommitmentOutput: {e}"
        ))
    })?;
    let cid = format!("{}", output.entry_hash);
    let action_hash = format!("{}", output.action_hash);

    let new_row = match crate::mishpat_projection::parse_commitment_payload(
        "replicates-commons",
        &payload_json,
        &cid,
        &action_hash,
    )
    .map_err(|e| StorageError::Internal(format!("capacity pledge projection: {e}")))?
    {
        CommitmentProjection::Upsert(row) => row,
        _ => {
            return Err(StorageError::Internal(
                "capacity pledge parsed to a non-commitment projection".to_string(),
            ))
        }
    };

    // Mirror before consuming the row, exactly like signals.rs. Both writes are
    // idempotent with the eventual post-commit signal.
    if let Some(mirror) = crate::mishpat_projection::replication_mirror_for(&new_row) {
        crate::db::rea_commitments::mirror_replication_commitment(conn, ctx.h_app_id(), &mirror)?;
    }
    let row = crate::db::mishpat_commitments::upsert_with_anchor(conn, new_row)?;
    Ok(crate::services::mishpat_commitment_facing::to_view(row))
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_views::replicates_commons::{
        CommonsBounds, CommonsRatioAttestation, ReplicatesContentPayload,
    };

    fn valid_capacity() -> ReplicatesContentPayload {
        let provenance = crate::services::constitutional_ratio_registry::effective_ratios();
        let ratios = provenance.ratios;
        ReplicatesContentPayload::Capacity {
            commons_bytes: 64 * 1024 * 1024,
            bounds: CommonsBounds {
                rate_per_minute: 12,
                reach_ceiling: "commons".to_string(),
            },
            ratio_attestation: CommonsRatioAttestation {
                commons_pct: ratios.commons_pct,
                dwelling_pct: ratios.dwelling_pct,
                collective_pct: ratios.collective_pct,
                free_pct: ratios.free_pct,
                effective_ratio_cid: provenance.manifest_cid,
            },
        }
    }

    #[test]
    fn builder_injects_provider_and_preserves_capacity_contract() {
        let json = build_capacity_payload("uhCAk-steward", &valid_capacity()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["action"], "replicates-commons");
        assert_eq!(value["variant"], "capacity");
        assert_eq!(value["provider"], "uhCAk-steward");
        assert_eq!(value["commons_bytes"], 64 * 1024 * 1024);
        assert_eq!(value["bounds"]["reach_ceiling"], "commons");
        assert!(value["ratio_attestation"]["effective_ratio_cid"].is_string());
    }

    #[test]
    fn builder_rejects_content_variant() {
        let payload = ReplicatesContentPayload::Content {
            head_ref: "bafy-head".to_string(),
            closure_rule: None,
            reach: "commons".to_string(),
            bounds: CommonsBounds {
                rate_per_minute: 12,
                reach_ceiling: "commons".to_string(),
            },
        };
        assert!(matches!(
            build_capacity_payload("uhCAk-steward", &payload),
            Err(StorageError::InvalidInput(_))
        ));
    }
}
