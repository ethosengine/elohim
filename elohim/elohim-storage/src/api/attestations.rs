//! Attestation API controller
//!
//! Routes served by this module:
//!
//! ## Unified attestation projection (Category A — DHT source of truth)
//! - `POST   /api/v1/attestations/unified`         — upsert projection row (for signal replay /admin backfill; regular creation goes via zome)
//! - `GET    /api/v1/attestations/unified/{id}`    — fetch by CID
//! - `GET    /api/v1/attestations/unified`         — list by subjectCid / issuerCid
//! - `POST   /api/v1/attestations/unified/{id}/revoke` — record revocation on projection
//!
//! ## Legacy content-attestation routes (pre-consolidation)
//! - `GET    /api/v1/attestations`                 — list by contentId or attestorId
//! - `GET    /api/v1/attestations/{id}`            — get by id
//! - `POST   /api/v1/attestations`                 — create
//! - `POST   /api/v1/attestations/{id}/revoke`     — revoke

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;

use crate::db::content_attestations;
use crate::db::models::NewContentAttestation;
use crate::db::{attestations as unified_attestations, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::{
    AttestationView, ContentAttestationView, CreateAttestationInputView, RevokeAttestationInputView,
};

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Query param types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AttestationQuery {
    pub content_id: Option<String>,
    pub attestor_id: Option<String>,
}

/// Query params for the unified projection list endpoint.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UnifiedAttestationQuery {
    /// Filter by subject CID.
    pub subject_cid: Option<String>,
    /// Filter by issuer CID.
    pub issuer_cid: Option<String>,
    /// Filter by attestation kind (e.g. "attestation:governance-vote").
    pub kind: Option<String>,
}

/// Input for recording a revocation on the projection.
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RevokeUnifiedInput {
    pub reason: String,
    pub revoked_at: String,
}

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

fn extract_id(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let id = trimmed.split('/').next()?;
    if id.is_empty() {
        None
    } else {
        Some(id)
    }
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    _ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let (path_only, _query) = resource_path.split_once('?').unwrap_or((resource_path, ""));
    // Capture the query string as an owned String before any potential moves of `req`.
    let query_str_owned: String = req.uri().query().unwrap_or("").to_string();
    let query_str = query_str_owned.as_str();

    // ---------- Unified projection routes (Category A, DHT-backed) ----------
    if path_only.starts_with("/unified") {
        let sub = path_only.strip_prefix("/unified").unwrap_or("");
        return handle_unified(req, method, sub, query_str, pool).await;
    }

    // ---------- Legacy content-attestation routes ---------------------------
    match (&method, path_only) {
        // POST /api/v1/attestations/ → create
        (&Method::POST, "/" | "") => {
            // TODO(p2p-coherence): Populate dht_anchor_hash from post-commit signal.
            // Currently null for direct storage writes. Backfill needed for pre-coherence data.
            let input: CreateAttestationInputView = parse_body(req).await?;
            let id = uuid::Uuid::new_v4().to_string();
            let evidence_json = input
                .evidence
                .as_ref()
                .map(|v| serde_json::to_string(&v.0))
                .transpose()
                .map_err(|e| StorageError::InvalidInput(format!("Invalid evidence JSON: {}", e)))?;
            let grantor_json = input
                .grantor
                .as_ref()
                .map(|v| serde_json::to_string(&v.0))
                .transpose()
                .map_err(|e| StorageError::InvalidInput(format!("Invalid grantor JSON: {}", e)))?;
            let new = NewContentAttestation {
                id: &id,
                content_id: &input.content_id,
                attestor_presence_id: &input.attestor_presence_id,
                scope: &input.scope,
                attestation_type: &input.attestation_type,
                evidence: evidence_json.as_deref(),
                grantor: grantor_json.as_deref(),
            };
            let mut conn = get_conn(pool)?;
            let result = content_attestations::create_attestation(&mut conn, &new)?;
            Ok(response::ok(&ContentAttestationView::from(result)))
        }

        // POST /api/v1/attestations/{id}/revoke
        (&Method::POST, p) if p.ends_with("/revoke") => {
            let id_part = p.strip_suffix("/revoke").unwrap_or("");
            let id = extract_id(id_part)
                .ok_or_else(|| StorageError::InvalidInput("Attestation ID required".to_string()))?;
            let _input: RevokeAttestationInputView = parse_body(req).await?;
            let mut conn = get_conn(pool)?;
            let result = content_attestations::revoke_attestation(&mut conn, id)?;
            Ok(response::ok(&ContentAttestationView::from(result)))
        }

        // GET /api/v1/attestations?contentId=X or ?attestorId=X
        (&Method::GET, "/" | "") => {
            let params: AttestationQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let mut conn = get_conn(pool)?;
            if let Some(content_id) = &params.content_id {
                let results =
                    content_attestations::query_attestations_for_content(&mut conn, content_id)?;
                let views: Vec<ContentAttestationView> = results
                    .into_iter()
                    .map(ContentAttestationView::from)
                    .collect();
                Ok(response::ok(&views))
            } else if let Some(attestor_id) = &params.attestor_id {
                let results =
                    content_attestations::query_attestations_by_attestor(&mut conn, attestor_id)?;
                let views: Vec<ContentAttestationView> = results
                    .into_iter()
                    .map(ContentAttestationView::from)
                    .collect();
                Ok(response::ok(&views))
            } else {
                Ok(response::bad_request(
                    "contentId or attestorId query param is required",
                ))
            }
        }

        // GET /api/v1/attestations/{id}
        (&Method::GET, p) => {
            let id = extract_id(p)
                .ok_or_else(|| StorageError::InvalidInput("Attestation ID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let result = content_attestations::get_attestation(&mut conn, id)?;
            Ok(response::from_option(
                Ok(result.map(ContentAttestationView::from)),
                &format!("Attestation {} not found", id),
            ))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown attestations route: {} {}",
            method, resource_path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Unified projection handlers (Category A — source of truth: Holochain DHT)
// ---------------------------------------------------------------------------

/// Handle `/api/v1/attestations/unified[/{id}[/revoke]]` routes.
async fn handle_unified(
    req: Request<Incoming>,
    method: Method,
    sub_path: &str,
    query_str: &str,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    match (&method, sub_path) {
        // GET /api/v1/attestations/unified?subjectCid=X[&kind=Y] or ?issuerCid=X
        (&Method::GET, "/" | "") => {
            let params: UnifiedAttestationQuery =
                serde_urlencoded::from_str(query_str).unwrap_or_default();
            let mut conn = get_conn(pool)?;

            if let Some(subject_cid) = &params.subject_cid {
                let rows = unified_attestations::list_by_subject(
                    &mut conn,
                    subject_cid,
                    params.kind.as_deref(),
                )?;
                let views: Vec<AttestationView> =
                    rows.into_iter().map(AttestationView::from).collect();
                Ok(response::ok(&views))
            } else if let Some(issuer_cid) = &params.issuer_cid {
                let rows = unified_attestations::list_by_issuer(&mut conn, issuer_cid)?;
                let views: Vec<AttestationView> =
                    rows.into_iter().map(AttestationView::from).collect();
                Ok(response::ok(&views))
            } else {
                Ok(response::bad_request(
                    "subjectCid or issuerCid query param is required",
                ))
            }
        }

        // GET /api/v1/attestations/unified/{id}
        (&Method::GET, p) if !p.contains('/') || p == "/" => {
            let id = extract_id(p)
                .ok_or_else(|| StorageError::InvalidInput("Attestation CID required".to_string()))?;
            let mut conn = get_conn(pool)?;
            let row = unified_attestations::get_by_id(&mut conn, id)?;
            Ok(response::from_option(
                Ok(row.map(AttestationView::from)),
                &format!("Attestation {} not found", id),
            ))
        }

        // POST /api/v1/attestations/unified/{id}/revoke — record revocation on projection
        // Note: this does NOT revoke the DHT entry. It records the revocation signal.
        (&Method::POST, p) if p.ends_with("/revoke") => {
            let id_part = p.strip_suffix("/revoke").unwrap_or("");
            let id = extract_id(id_part.trim_start_matches('/'))
                .ok_or_else(|| StorageError::InvalidInput("Attestation CID required".to_string()))?;
            let input: RevokeUnifiedInput = parse_body(req).await?;
            let mut conn = get_conn(pool)?;
            // Fetch, apply revocation fields, upsert back.
            let mut row = unified_attestations::get_by_id(&mut conn, id)?
                .ok_or_else(|| StorageError::NotFound(format!("Attestation {} not found", id)))?;
            row.revocation_reason = Some(input.reason);
            row.revoked_at = Some(input.revoked_at);
            unified_attestations::upsert(&mut conn, &row)?;
            Ok(response::ok(&AttestationView::from(row)))
        }

        _ => Ok(response::not_found(&format!(
            "Unknown unified attestation route: {} /unified{}",
            method, sub_path
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::AttestationRow;
    use diesel::prelude::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    fn make_attestation_row(id: &str, subject: &str, issuer: &str) -> AttestationRow {
        AttestationRow {
            id: id.to_string(),
            dht_anchor_hash: b"fakehash0000000000000000000000000".to_vec(),
            attestation_kind: "attestation:humanness".to_string(),
            subject_cid: subject.to_string(),
            subject_kind: "agent".to_string(),
            issuer_cid: issuer.to_string(),
            parent_governance_action_cid: None,
            vote_value: None,
            vote_weight: None,
            proof_class: "witness".to_string(),
            proof_evidence_json: "{}".to_string(),
            evidence_json: "{}".to_string(),
            expires_at: None,
            supersedes_cid: None,
            revocation_reason: None,
            revoked_at: None,
            created_at: "2026-05-12T10:00:00Z".to_string(),
            manifest_ref: "imagodei".to_string(),
            title: "Test attestation".to_string(),
            description: None,
        }
    }

    #[test]
    fn attestation_view_from_row_encodes_dht_hash_as_hex() {
        let row = make_attestation_row("cid-001", "subject-001", "issuer-001");
        let view = AttestationView::from(row.clone());
        assert_eq!(view.id, "cid-001");
        assert_eq!(
            view.dht_anchor_hash,
            hex::encode(&row.dht_anchor_hash),
            "dht_anchor_hash must be hex-encoded"
        );
        assert_eq!(view.attestation_kind, "attestation:humanness");
        assert!(view.revocation_reason.is_none());
        assert!(view.revoked_at.is_none());
    }

    #[test]
    fn revocation_fields_propagate_to_view() {
        let mut row = make_attestation_row("cid-002", "subject-002", "issuer-002");
        row.revocation_reason = Some("test revocation".to_string());
        row.revoked_at = Some("2026-05-13T08:00:00Z".to_string());
        let view = AttestationView::from(row);
        assert_eq!(view.revocation_reason.as_deref(), Some("test revocation"));
        assert_eq!(view.revoked_at.as_deref(), Some("2026-05-13T08:00:00Z"));
    }

    #[test]
    fn list_by_subject_returns_rows_as_views() {
        let mut conn = setup();
        let row1 = make_attestation_row("cid-003", "subject-abc", "issuer-001");
        let row2 = make_attestation_row("cid-004", "subject-abc", "issuer-002");
        unified_attestations::upsert(&mut conn, &row1).unwrap();
        unified_attestations::upsert(&mut conn, &row2).unwrap();

        let rows =
            unified_attestations::list_by_subject(&mut conn, "subject-abc", None).unwrap();
        assert_eq!(rows.len(), 2);

        let views: Vec<AttestationView> = rows.into_iter().map(AttestationView::from).collect();
        assert!(views.iter().any(|v| v.id == "cid-003"));
        assert!(views.iter().any(|v| v.id == "cid-004"));
    }
}
