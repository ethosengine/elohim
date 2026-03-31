//! Custodian metrics CRUD operations using Diesel
//!
//! Stores periodic metrics snapshots reported by custodian nodes.
//! The custodian_metrics table is keyed by custodian_id (one row per node,
//! INSERT OR REPLACE semantics via UpsertCustodianMetrics).

use diesel::prelude::*;
use serde::Deserialize;

use super::context::AppContext;
use super::diesel_schema::custodian_metrics;
use super::models::{CustodianMetrics, UpsertCustodianMetrics};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Rank ordering for the list endpoint.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MetricsRank {
    #[default]
    Health,
    Speed,
    Reputation,
}

/// Query parameters for listing custodian metrics.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustodianMetricsQuery {
    /// Sort by health | speed | reputation (default: health)
    pub rank: Option<MetricsRank>,
    /// If true, only include nodes whose health_json indicates availability
    pub available: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// List all custodian metrics rows scoped to the given app.
pub fn list_metrics(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &CustodianMetricsQuery,
) -> Result<Vec<CustodianMetrics>, StorageError> {
    let mut q = custodian_metrics::table
        .filter(custodian_metrics::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    q = q.limit(limit).offset(offset);

    // Ordering is applied after fetch (see service layer) because sort keys
    // live inside JSON blobs that SQLite cannot index without generated columns.
    // Here we just return in insertion order; the handler re-sorts in Rust.
    q.load::<CustodianMetrics>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list custodian_metrics: {e}")))
}

/// Fetch a single custodian metrics row by custodian_id.
pub fn get_metrics_by_id(
    conn: &mut SqliteConnection,
    custodian_id: &str,
    ctx: &AppContext,
) -> Result<Option<CustodianMetrics>, StorageError> {
    custodian_metrics::table
        .filter(custodian_metrics::custodian_id.eq(custodian_id))
        .filter(custodian_metrics::h_app_id.eq(&ctx.h_app_id))
        .first::<CustodianMetrics>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to get custodian_metrics: {e}")))
}

/// List metrics rows where health signals indicate a problem.
///
/// "Problem" here means the health_json blob contains `"availability":false`
/// or `"uptime_percent"` below 95. Because the field lives in JSON we load
/// all rows and filter in Rust; the table is expected to be small (O(10s) of
/// custodians per community node).
pub fn list_unhealthy_metrics(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<CustodianMetrics>, StorageError> {
    custodian_metrics::table
        .filter(custodian_metrics::h_app_id.eq(&ctx.h_app_id))
        .load::<CustodianMetrics>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list custodian_metrics: {e}")))
}

/// Upsert (INSERT OR REPLACE) a full metrics snapshot.
pub fn upsert_metrics(
    conn: &mut SqliteConnection,
    input: UpsertCustodianMetrics,
) -> Result<CustodianMetrics, StorageError> {
    diesel::replace_into(custodian_metrics::table)
        .values(&input)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to upsert custodian_metrics: {e}")))?;

    get_metrics_by_id(conn, &input.custodian_id, &AppContext::new(&input.h_app_id))?
        .ok_or_else(|| StorageError::Internal("Upserted row not found".into()))
}
