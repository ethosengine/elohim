//! ## Source of Truth
//!
//! Operational handler (Category C) that mirrors EconomicEvent signals
//! with action='ack-projection' into the projection_events log.
//! The DHT EconomicEvent entry remains authoritative.

use thiserror::Error;

use crate::db::projection_events::{append_projection_event, NewProjectionEvent};
use crate::db::DbPool;

#[derive(Debug, Error)]
pub enum ProjectionAckError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
}

const ACK_PROJECTION_ACTION: &str = "ack-projection";

/// Synchronous variant: handle a potential ack-projection event signal.
///
/// If `action != "ack-projection"` this is a no-op (returns Ok immediately).
/// Otherwise, mirrors the event into the `projection_events` append-only log.
/// Called from the synchronous `rea_projection::handle_rea_signal` dispatch.
pub fn handle_projection_ack_sync(
    pool: &DbPool,
    action: &str,
    provider: &str,
    resource_classified_as: &str,
    action_hash: &str,
    emitted_at: &str,
) -> Result<(), ProjectionAckError> {
    if action != ACK_PROJECTION_ACTION {
        return Ok(());
    }

    let mut conn = pool
        .get()
        .map_err(|e| ProjectionAckError::Pool(e.to_string()))?;
    append_projection_event(
        &mut conn,
        &NewProjectionEvent {
            blob_hash: resource_classified_as.to_string(),
            projector_agent_cid: provider.to_string(),
            emitted_at: emitted_at.to_string(),
            source_action_hash: action_hash.to_string(),
        },
    )?;

    tracing::debug!(
        target = "phase4::projection_ack",
        blob_hash = %resource_classified_as,
        projector = %provider,
        "recorded projection ack"
    );

    Ok(())
}

/// Async variant: handle a potential ack-projection event signal.
///
/// Thin async wrapper around [`handle_projection_ack_sync`]. Used in
/// integration tests that need an async call site.
pub async fn handle_projection_ack(
    pool: &DbPool,
    action: &str,
    provider: &str,
    resource_classified_as: &str,
    action_hash: &str,
    emitted_at: &str,
) -> Result<(), ProjectionAckError> {
    handle_projection_ack_sync(
        pool,
        action,
        provider,
        resource_classified_as,
        action_hash,
        emitted_at,
    )
}
