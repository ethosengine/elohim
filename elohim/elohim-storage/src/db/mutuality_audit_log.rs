//! Mutuality audit log CRUD. Per spec §6.2.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use super::diesel_schema::mutuality_audit_log::dsl;
use super::models::{MutualityAuditLogRow, NewMutualityAuditLogRow};
use crate::error::StorageError;

pub fn insert(conn: &mut SqliteConnection, row: &NewMutualityAuditLogRow) -> Result<(), StorageError> {
    diesel::insert_into(dsl::mutuality_audit_log)
        .values(row)
        .execute(conn)
        .map_err(|e| StorageError::Database(e.to_string()))?;
    Ok(())
}

pub fn list_recent_for_recipient(
    conn: &mut SqliteConnection,
    recipient: &str,
    limit: i64,
) -> Result<Vec<MutualityAuditLogRow>, StorageError> {
    dsl::mutuality_audit_log
        .filter(dsl::recipient_dwelling_hub_id.eq(recipient))
        .order(dsl::swept_at.desc())
        .limit(limit)
        .load::<MutualityAuditLogRow>(conn)
        .map_err(|e| StorageError::Database(e.to_string()))
}

pub fn latest_for_commitment(
    conn: &mut SqliteConnection,
    commitment_cid: &str,
) -> Result<Option<MutualityAuditLogRow>, StorageError> {
    dsl::mutuality_audit_log
        .filter(dsl::commitment_cid.eq(commitment_cid))
        .order(dsl::swept_at.desc())
        .first::<MutualityAuditLogRow>(conn)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))
}
