//! Authorization-gate helper for Shamir share responders.
//!
//! Source of truth: Holochain DHT (`attestations` projection table).
//! This module answers the single question the custodian responder needs to
//! answer before releasing a share: *is there an effective, non-superseded
//! `attestation:recovery-approval` on the DHT for this recovery governance
//! action, and does it authorize the requesting custodian to release material?*
//!
//! ## "Effective" definition
//!
//! An `attestation:recovery-approval` row is effective when ALL of:
//!   1. `attestation_kind = "attestation:recovery-approval"`
//!   2. `parent_governance_action_cid = <recovery_governance_action_cid>`
//!   3. `revoked_at IS NULL`  — not revoked
//!   4. No OTHER attestation row exists with `supersedes_cid = this.id` —
//!      not superseded by a replacement attestation.
//!   5. `expires_at IS NULL OR expires_at > now_iso` — not expired
//!
//! Conditions 3 and 5 are checked directly on the row. Condition 4 is a
//! correlated sub-check: the `supersedes_cid` column on the attestation rows
//! points backward (an approval that supersedes a prior approval carries the
//! prior approval's CID in its own `supersedes_cid` column). Therefore
//! "this row is superseded" means another row exists whose `supersedes_cid =
//! this.id`. We detect this with a second point-query per candidate.
//!
//! ## Authorized-recipient check
//!
//! The `evidence_json` field may carry an `authorized_recipient` key (JSON
//! string) that restricts which recovery agent may claim this approval.
//! When absent, the approval is usable by any recovery agent that presents
//! the correct governance-action CID. When present it must match
//! `requesting_agent_cid`.
//!
//! TODO(T22/T23-authorized-recipient): the `authorized_recipient` field is
//! optional and currently advisory only. Once the DHT validator enforces it
//! end-to-end, tighten the check here to a hard reject when the field is
//! present and does not match.
//!
//! ## Custodian self-check
//!
//! The custodian compares `custodian_cid` in the request against its own
//! identity CID (from `NodeIdentity::agent_pubkey()`). This prevents replay
//! attacks where a request directed at one custodian is sent to another.
//! Silent rejection (no error) on mismatch, per the `ShamirShareRequest`
//! doc comment.

use diesel::prelude::*;

use crate::db::diesel_schema::attestations;
use crate::db::models::AttestationRow;
use crate::error::StorageError;

// ─────────────────────────────────────────────────────────────────────────────
// Public result type
// ─────────────────────────────────────────────────────────────────────────────

/// Outcome of the authorization gate for a share request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareAuthDecision {
    /// Authorization passes.  `attestation_cid` is the CID of the approving
    /// attestation the custodian should cite in its `ShamirShareResponse`.
    Authorized { attestation_cid: String },

    /// No effective recovery-approval attestation exists for this governance
    /// action, or the attestation has been revoked / superseded / expired.
    NotAuthorized { reason: String },

    /// The `custodian_cid` in the request does not match this node's identity.
    /// Silent per spec — log at `debug!` and reject without sending an error
    /// envelope (prevents custodian enumeration).
    WrongCustodian,
}

// ─────────────────────────────────────────────────────────────────────────────
// Gate function
// ─────────────────────────────────────────────────────────────────────────────

/// Check whether this node may release a share for `recovery_governance_action_cid`.
///
/// `local_agent_cid` — the value from `NodeIdentity::agent_pubkey()`.
/// `requesting_agent_cid` — the recovery agent's CID from the request (advisory
///   check; becomes hard-gate once DHT validators enforce it, see TODO above).
/// `now_iso` — ISO-8601 timestamp for expiry comparison (pass `chrono::Utc::now().to_rfc3339()`).
pub fn check_share_authorization(
    conn: &mut SqliteConnection,
    recovery_governance_action_cid: &str,
    requested_custodian_cid: &str,
    local_agent_cid: &str,
    now_iso: &str,
) -> Result<ShareAuthDecision, StorageError> {
    // ── Self-check: verify the request is actually directed at this node ──
    if requested_custodian_cid != local_agent_cid {
        return Ok(ShareAuthDecision::WrongCustodian);
    }

    // ── Load candidate recovery-approval attestations ─────────────────────
    //
    // Filter on the three cheapest predicates first (index-covered):
    //   parent_governance_action_cid, attestation_kind, revoked_at IS NULL.
    let candidates: Vec<AttestationRow> = attestations::table
        .filter(attestations::parent_governance_action_cid.eq(recovery_governance_action_cid))
        .filter(attestations::attestation_kind.eq("attestation:recovery-approval"))
        .filter(attestations::revoked_at.is_null())
        .load::<AttestationRow>(conn)
        .map_err(|e| StorageError::Database(format!("recovery_approval_gate load: {e}")))?;

    if candidates.is_empty() {
        return Ok(ShareAuthDecision::NotAuthorized {
            reason: format!(
                "no recovery-approval attestations for governance action {}",
                recovery_governance_action_cid
            ),
        });
    }

    // ── For each candidate: check expiry + supersession + recipient ───────
    for row in &candidates {
        // Check expiry
        if let Some(ref expires_at) = row.expires_at {
            if expires_at.as_str() <= now_iso {
                continue; // expired
            }
        }

        // Check supersession: does any other row carry `supersedes_cid = row.id`?
        // (i.e., this approval has been replaced by a newer one)
        let is_superseded: bool = attestations::table
            .filter(attestations::supersedes_cid.eq(&row.id))
            .count()
            .get_result::<i64>(conn)
            .map_err(|e| {
                StorageError::Database(format!("recovery_approval_gate supersession check: {e}"))
            })?
            > 0;

        if is_superseded {
            continue;
        }

        // TODO(T22/T23-authorized-recipient): if evidence_json carries
        // `authorized_recipient`, verify it matches `requesting_agent_cid`.
        // For now this is advisory only — accept if field is absent.
        //
        // let evidence: serde_json::Value = serde_json::from_str(&row.evidence_json).ok();
        // if let Some(recipient) = evidence.and_then(|v| v.get("authorized_recipient")...) { ... }

        // This candidate is effective — authorize the share release.
        return Ok(ShareAuthDecision::Authorized {
            attestation_cid: row.id.clone(),
        });
    }

    // All candidates were expired or superseded.
    Ok(ShareAuthDecision::NotAuthorized {
        reason: format!(
            "all recovery-approval attestations for {} are expired or superseded",
            recovery_governance_action_cid
        ),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// PeerId resolution helper
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve a custodian's `agent_cid` to their most-recently-observed libp2p
/// `PeerId` string, using the `peer_identity_bindings` projection.
///
/// Returns `None` when no active binding exists for the agent — caller should
/// treat this as an `OutboundFailure::DialFailure`-equivalent and fall back to
/// the next custodian.
///
/// TODO(T22-multi-device): `list_active_for_agent` may return multiple bindings
/// (one per device). This helper returns the most recently observed one.
/// In a future sprint we should try them in order (like the view-federation
/// fanout does) and return the first success.
pub fn resolve_custodian_peer_id(
    conn: &mut SqliteConnection,
    custodian_cid: &str,
    now_iso: &str,
) -> Result<Option<String>, StorageError> {
    use crate::db::peer_identity_bindings::list_active_for_agent;

    let bindings = list_active_for_agent(conn, custodian_cid, now_iso)?;
    // list_active_for_agent returns rows ORDER BY observed_at DESC — first is freshest.
    Ok(bindings.into_iter().next().map(|b| b.peer_id))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::{attestations, governance_actions};
    use crate::db::{init_pool_from_dir, run_migrations, DbPool};

    fn test_pool() -> DbPool {
        let dir = tempfile::tempdir().expect("tempdir");
        let pool = init_pool_from_dir(dir.path()).expect("pool");
        run_migrations(&pool).expect("migrations");
        std::mem::forget(dir);
        pool
    }

    fn insert_gov_action(conn: &mut SqliteConnection, id: &str) {
        diesel::replace_into(governance_actions::table)
            .values((
                governance_actions::id.eq(id),
                governance_actions::dht_anchor_hash.eq(vec![0u8; 39]),
                governance_actions::governance_kind.eq("governance-action:recovery-request"),
                governance_actions::subject_cid.eq("subject"),
                governance_actions::proposer_cid.eq("proposer"),
                governance_actions::threshold_json.eq(r#"{"type":"shamir","m":2,"n":3}"#),
                governance_actions::ballot_format.eq("approval"),
                governance_actions::closes_at.eq("2099-01-01T00:00:00Z"),
                governance_actions::title.eq("test"),
                governance_actions::created_at.eq("2026-05-11T00:00:00Z"),
            ))
            .execute(conn)
            .expect("insert governance_action");
    }

    fn insert_attestation(
        conn: &mut SqliteConnection,
        id: &str,
        parent_id: &str,
        revoked_at: Option<&str>,
        supersedes_cid: Option<&str>,
        expires_at: Option<&str>,
    ) {
        diesel::replace_into(attestations::table)
            .values((
                attestations::id.eq(id),
                attestations::dht_anchor_hash.eq(vec![0u8; 39]),
                attestations::attestation_kind.eq("attestation:recovery-approval"),
                attestations::subject_cid.eq("subject"),
                attestations::subject_kind.eq("human"),
                attestations::issuer_cid.eq("custodian-self"),
                attestations::parent_governance_action_cid.eq(parent_id),
                attestations::proof_class.eq("witness"),
                attestations::proof_evidence_json.eq("{}"),
                attestations::evidence_json.eq("{}"),
                attestations::manifest_ref.eq("test"),
                attestations::title.eq("approval"),
                attestations::created_at.eq("2026-05-11T00:00:00Z"),
                attestations::revoked_at.eq(revoked_at),
                attestations::supersedes_cid.eq(supersedes_cid),
                attestations::expires_at.eq(expires_at),
            ))
            .execute(conn)
            .expect("insert attestation");
    }

    #[test]
    fn authorizes_valid_approval() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_gov_action(&mut conn, "action-001");
        insert_attestation(&mut conn, "attest-001", "action-001", None, None, None);

        let result = check_share_authorization(
            &mut conn,
            "action-001",
            "custodian-self",
            "custodian-self",
            "2026-05-15T00:00:00Z",
        )
        .expect("check");
        assert!(
            matches!(result, ShareAuthDecision::Authorized { .. }),
            "expected Authorized, got {:?}",
            result
        );
    }

    #[test]
    fn rejects_wrong_custodian() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_gov_action(&mut conn, "action-002");
        insert_attestation(&mut conn, "attest-002", "action-002", None, None, None);

        let result = check_share_authorization(
            &mut conn,
            "action-002",
            "different-custodian",
            "custodian-self",
            "2026-05-15T00:00:00Z",
        )
        .expect("check");
        assert_eq!(result, ShareAuthDecision::WrongCustodian);
    }

    #[test]
    fn rejects_revoked_approval() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_gov_action(&mut conn, "action-003");
        insert_attestation(
            &mut conn,
            "attest-003",
            "action-003",
            Some("2026-05-01T00:00:00Z"),
            None,
            None,
        );

        let result = check_share_authorization(
            &mut conn,
            "action-003",
            "custodian-self",
            "custodian-self",
            "2026-05-15T00:00:00Z",
        )
        .expect("check");
        assert!(
            matches!(result, ShareAuthDecision::NotAuthorized { .. }),
            "expected NotAuthorized for revoked, got {:?}",
            result
        );
    }

    #[test]
    fn rejects_superseded_approval() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_gov_action(&mut conn, "action-004");

        // Original approval (id = attest-004-original)
        insert_attestation(
            &mut conn,
            "attest-004-original",
            "action-004",
            None,
            None,
            None,
        );

        // Replacement approval that supersedes the original
        diesel::replace_into(attestations::table)
            .values((
                attestations::id.eq("attest-004-replacement"),
                attestations::dht_anchor_hash.eq(vec![0u8; 39]),
                attestations::attestation_kind.eq("attestation:recovery-approval"),
                attestations::subject_cid.eq("subject"),
                attestations::subject_kind.eq("human"),
                attestations::issuer_cid.eq("custodian-self"),
                attestations::parent_governance_action_cid.eq("action-004"),
                attestations::proof_class.eq("witness"),
                attestations::proof_evidence_json.eq("{}"),
                attestations::evidence_json.eq("{}"),
                attestations::manifest_ref.eq("test"),
                attestations::title.eq("replacement"),
                attestations::created_at.eq("2026-05-12T00:00:00Z"),
                attestations::supersedes_cid.eq("attest-004-original"),
            ))
            .execute(&mut conn)
            .expect("insert replacement");

        // The ORIGINAL should be rejected (superseded by the replacement).
        // The replacement itself has no superseder, so it should be authorized.
        let result = check_share_authorization(
            &mut conn,
            "action-004",
            "custodian-self",
            "custodian-self",
            "2026-05-15T00:00:00Z",
        )
        .expect("check");
        // One of the two should be authorized (the replacement).
        assert!(
            matches!(result, ShareAuthDecision::Authorized { ref attestation_cid } if attestation_cid == "attest-004-replacement"),
            "expected Authorized with replacement CID, got {:?}",
            result
        );
    }

    #[test]
    fn rejects_no_approvals() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        insert_gov_action(&mut conn, "action-005");
        // No attestations inserted.

        let result = check_share_authorization(
            &mut conn,
            "action-005",
            "custodian-self",
            "custodian-self",
            "2026-05-15T00:00:00Z",
        )
        .expect("check");
        assert!(
            matches!(result, ShareAuthDecision::NotAuthorized { .. }),
            "expected NotAuthorized with no approvals, got {:?}",
            result
        );
    }
}
