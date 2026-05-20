//! Diesel models and queries for the EPR storage layer (Phase 2a).
//!
//! See spec §8 + Integrator Compatibility Contract §4.
//!
//! SQLite adaptation notes:
//! - Timestamps stored as TEXT (ISO-8601) — diesel `sqlite` feature has no chrono mapping.
//! - Binary data stored as Vec<u8> — maps to SQLite BLOB / diesel Binary.
//! - Connection type is SqliteConnection (not PgConnection).

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::diesel_schema::{epr_atoms, epr_claims, epr_coupling, epr_supersedence};

// ---------------------------------------------------------------------------
// epr_atoms
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_atoms)]
pub struct EprAtom {
    pub cid: String,
    pub kind: String,
    pub schema_ref: String,
    pub schema_key: String,
    pub reach: String,
    /// ISO-8601 timestamp string (e.g. "2026-04-22T00:00:00Z")
    pub issued_at: String,
    pub signer_cid: String,
    pub supersedes: Option<String>,
    pub canonical_bytes: Vec<u8>,
    pub payload_bytes: Vec<u8>,
    pub proof_bytes: Vec<u8>,
    pub proof_algorithm: String,
    /// UTC ISO-8601 timestamp at which resolver-backed Ed25519 verify succeeded.
    /// `None` for atoms ingested before A.7 or whose signer has no timeline entry.
    pub verified_at: Option<String>,
    /// blake3-128-prefix of the 32-byte ed25519 pubkey that signed this atom
    /// (first 16 bytes of the hash = 32 hex chars). `None` when `verified_at` is `None`.
    pub verified_signer_fingerprint: Option<String>,
}

// ---------------------------------------------------------------------------
// epr_coupling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_coupling)]
pub struct EprCouplingRow {
    pub epr_cid: String,
    pub leg: String,
    pub target_cid: String,
}

// ---------------------------------------------------------------------------
// epr_claims
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_claims)]
pub struct EprClaimRow {
    pub epr_cid: String,
    pub claim_cid: String,
}

// ---------------------------------------------------------------------------
// epr_supersedence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = epr_supersedence)]
pub struct EprSupersedenceRow {
    pub predecessor: String,
    pub successor: String,
    pub attested_by: String,
    /// ISO-8601 timestamp string (e.g. "2026-04-22T00:00:00Z")
    pub attested_at: String,
}

// ---------------------------------------------------------------------------
// Query helpers
// ---------------------------------------------------------------------------

pub fn insert_atom(conn: &mut SqliteConnection, atom: &EprAtom) -> QueryResult<usize> {
    diesel::insert_into(epr_atoms::table)
        .values(atom)
        .execute(conn)
}

pub fn insert_coupling_rows(
    conn: &mut SqliteConnection,
    rows: &[EprCouplingRow],
) -> QueryResult<usize> {
    diesel::insert_into(epr_coupling::table)
        .values(rows)
        .execute(conn)
}

pub fn insert_claim_rows(conn: &mut SqliteConnection, rows: &[EprClaimRow]) -> QueryResult<usize> {
    diesel::insert_into(epr_claims::table)
        .values(rows)
        .execute(conn)
}

pub fn fetch_atom_by_cid(conn: &mut SqliteConnection, cid: &str) -> QueryResult<Option<EprAtom>> {
    epr_atoms::table.find(cid).first::<EprAtom>(conn).optional()
}

pub fn fetch_coupling_for_atom(
    conn: &mut SqliteConnection,
    cid: &str,
) -> QueryResult<Vec<EprCouplingRow>> {
    epr_coupling::table
        .filter(epr_coupling::epr_cid.eq(cid))
        .load::<EprCouplingRow>(conn)
}

pub fn fetch_claims_for_atom(
    conn: &mut SqliteConnection,
    cid: &str,
) -> QueryResult<Vec<EprClaimRow>> {
    epr_claims::table
        .filter(epr_claims::epr_cid.eq(cid))
        .load::<EprClaimRow>(conn)
}

pub fn fetch_superseded_by(
    conn: &mut SqliteConnection,
    predecessor: &str,
) -> QueryResult<Option<String>> {
    epr_supersedence::table
        .filter(epr_supersedence::predecessor.eq(predecessor))
        .select(epr_supersedence::successor)
        .first::<String>(conn)
        .optional()
}

#[derive(Debug, Clone, Default)]
pub struct EprListQuery {
    pub kind: Option<String>,
    pub reach: Option<String>,
    pub schema_ref: Option<String>,
    pub after_cid: Option<String>,
    pub limit: i64,
}

pub fn list_atoms(conn: &mut SqliteConnection, q: &EprListQuery) -> QueryResult<Vec<EprAtom>> {
    let mut query = epr_atoms::table.into_boxed();
    if let Some(k) = &q.kind {
        query = query.filter(epr_atoms::kind.eq(k));
    }
    if let Some(r) = &q.reach {
        query = query.filter(epr_atoms::reach.eq(r));
    }
    if let Some(s) = &q.schema_ref {
        query = query.filter(epr_atoms::schema_ref.eq(s));
    }
    if let Some(a) = &q.after_cid {
        query = query.filter(epr_atoms::cid.gt(a));
    }
    query
        .order(epr_atoms::cid.asc())
        .limit(q.limit)
        .load::<EprAtom>(conn)
}

/// Fetch all coupling rows that point TO the given cid as target —
/// i.e. atoms that couple this cid from their outbound legs. Used by the
/// nav-context projection to populate `partOf`.
pub fn fetch_reverse_coupling(
    conn: &mut SqliteConnection,
    target_cid: &str,
) -> QueryResult<Vec<EprCouplingRow>> {
    epr_coupling::table
        .filter(epr_coupling::target_cid.eq(target_cid))
        .load::<EprCouplingRow>(conn)
}
