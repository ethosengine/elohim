//! Read-side queries over the `hosted_agent_bindings` projection.
//!
//! Source of truth: the imagodei DHT `hosted-at` link (Category A2). This module
//! only READS — projection writes land exclusively through
//! `ReconcileController::on_hosted_agent_binding`, per Principle P1.
//!
//! The consumer is a *sibling* doorway resolving "where is this agent hosted?"
//! for a verified foreign token, so it can issue an honest 307 to the home
//! doorway instead of a 502 or a silently-forked identity.
//!
//! ## Correct-but-dormant
//!
//! This reader is correct even before any doorway writes a binding: it returns
//! `None`, which the doorway degrades to today's honest 409. That is honest, not
//! a stub — and it unblocks the doorway consumer the moment bindings start
//! landing.

use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::diesel_schema::hosted_agent_bindings as hab;
use crate::error::StorageError;

/// A projected hosted-at binding, camelCase on the wire.
///
/// Deliberately NOT a `#[derive(TS)]` view: this is an operational federation
/// read consumed doorway-to-doorway (like the sibling `/api/v1/federation/*`
/// routes), not a client-facing view type, so it needs no ts-rs export and no
/// `cargo test export_bindings` run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostedAgentBindingRow {
    pub agent_pub_key: String,
    pub doorway_id: String,
    pub doorway_url: String,
    pub installed_app_id: String,
    /// CreateLink ActionHash — provenance back to the DHT link.
    pub dht_anchor_hash: String,
    pub bound_at: String,
}

/// Resolve the CURRENT hosted-at binding for an agent, or `None`.
///
/// A human who migrates between doorways accumulates rows; the newest
/// `bound_at` is the current home. Ties break on `dht_anchor_hash` so the
/// result is deterministic across peers reading the same projection.
pub fn current_binding_for_agent(
    conn: &mut SqliteConnection,
    agent_pub_key: &str,
) -> Result<Option<HostedAgentBindingRow>, StorageError> {
    let row = hab::table
        .filter(hab::agent_pub_key.eq(agent_pub_key))
        .order((hab::bound_at.desc(), hab::dht_anchor_hash.desc()))
        .select((
            hab::agent_pub_key,
            hab::doorway_id,
            hab::doorway_url,
            hab::installed_app_id,
            hab::dht_anchor_hash,
            hab::bound_at,
        ))
        .first::<(String, String, String, String, String, String)>(conn)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))?;

    Ok(row.map(
        |(agent_pub_key, doorway_id, doorway_url, installed_app_id, dht_anchor_hash, bound_at)| {
            HostedAgentBindingRow {
                agent_pub_key,
                doorway_id,
                doorway_url,
                installed_app_id,
                dht_anchor_hash,
                bound_at,
            }
        },
    ))
}
