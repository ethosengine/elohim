//! Source-chain API controller — M-AGGR-2.
//!
//! Routes:
//!   GET /api/v1/source-chain/{agentId}/entries → Vec<SourceChainEntryView>
//!   GET /api/v1/source-chain/{agentId}/links   → Vec<EntryLinkView>
//!
//! Both routes proxy to the `imagodei` coordinator zome on the agent's
//! Holochain cell. No projection table: these entries live on the agent's
//! private source chain and are never gossiped to the DHT.
//!
//! ## Design classification (P2P Design Gate — §1.5 M-AGGR-2)
//!
//! - **Entity:** Source-chain entries/links — Category B (Agent-Scoped,
//!   private source chain). Never DHT-gossiped. Source of truth is the
//!   Holochain conductor; this route is a thin read proxy.
//! - **No new DHT entry type.** No projection table. The existing imagodei
//!   DNA source chain IS the substrate.
//! - **agentId path param:** used for routing context / authorization; the
//!   coordinator derives the agent from `agent_info()` on the calling cell —
//!   an agent can only read its own chain via this path. Cross-agent chain
//!   reads are not supported (and not meaningful for private entries).
//!
//! ## Substrate relationship
//!
//! The coordinator functions `query_my_source_chain` and
//! `query_my_source_chain_links` were added in Phase C. They return
//! `Vec<SourceChainEntryRecord>` / `Vec<SourceChainLinkRecord>` via
//! msgpack, which this handler decodes and maps to `SourceChainEntryView`
//! / `EntryLinkView` for the HTTP client.
//!
//! ## Fallback when no conductor
//!
//! Returns an empty array with a 200 status rather than 503, because
//! the substrate may be legitimately absent (storage-only test mode).
//! The client can detect "no conductor" via the empty list + a separate
//! /health signal. This differs from AttentionTending (which returns 503)
//! because source-chain reads are idempotent and non-critical in test mode.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::Deserialize;
use tracing;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;
use crate::services::response;
use elohim_views::{EntryLinkView, SourceChainEntryView};

// ---------------------------------------------------------------------------
// Zome wire types — must match coordinator output in imagodei/src/lib.rs
// ---------------------------------------------------------------------------

/// Mirror of `imagodei::SourceChainEntryRecord` (msgpack-decoded from conductor).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ZomeSourceChainEntry {
    pub action_hash: String,
    pub entry_hash: String,
    pub author_agent: String,
    pub entry_type: String,
    pub content_json: Option<String>,
    pub prev_action_hash: Option<String>,
    pub sequence: u32,
    pub timestamp: String,
}

/// Mirror of `imagodei::SourceChainLinkRecord` (msgpack-decoded from conductor).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ZomeSourceChainLink {
    pub link_hash: String,
    pub base_hash: String,
    pub target_hash: String,
    pub link_type: String,
    pub tag: Option<String>,
    pub author_agent: String,
    pub timestamp: String,
    pub deleted: bool,
    pub deleted_at: Option<String>,
}

// ---------------------------------------------------------------------------
// View conversions
// ---------------------------------------------------------------------------

impl From<ZomeSourceChainEntry> for SourceChainEntryView {
    fn from(z: ZomeSourceChainEntry) -> Self {
        Self {
            action_hash: z.action_hash,
            entry_hash: z.entry_hash,
            author_agent: z.author_agent,
            entry_type: z.entry_type,
            content_json: z.content_json,
            prev_action_hash: z.prev_action_hash,
            sequence: z.sequence,
            timestamp: z.timestamp,
        }
    }
}

impl From<ZomeSourceChainLink> for EntryLinkView {
    fn from(z: ZomeSourceChainLink) -> Self {
        Self {
            link_hash: z.link_hash,
            base_hash: z.base_hash,
            target_hash: z.target_hash,
            link_type: z.link_type,
            tag: z.tag,
            author_agent: z.author_agent,
            timestamp: z.timestamp,
            deleted: z.deleted,
            deleted_at: z.deleted_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Route handler
// ---------------------------------------------------------------------------

/// Dispatch `/api/v1/source-chain/{agentId}/{entries|links}` requests.
///
/// `resource_path` arrives as `/{agentId}/entries` or `/{agentId}/links`
/// (the `source-chain` prefix is stripped by mod.rs before dispatch).
pub async fn handle(
    _req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    _pool: &DbPool,
    _ctx: &AppContext,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    // Extract agentId and sub-resource from "{agentId}/entries" or "{agentId}/links".
    let (agent_id, sub) = match path.split_once('/') {
        Some((id, s)) => (id, s),
        None => {
            return Ok(response::bad_request(
                "source-chain path must be /{agentId}/{entries|links}",
            ));
        }
    };

    if agent_id.is_empty() {
        return Ok(response::bad_request("agentId must not be empty"));
    }

    match (&method, sub) {
        (&Method::GET, "entries") => handle_get_entries(agent_id, hc_registry).await,
        (&Method::GET, "links") => handle_get_links(agent_id, hc_registry).await,
        _ => Ok(response::not_found(&format!(
            "Unknown source-chain route: {} /api/v1/source-chain/{}",
            method, path
        ))),
    }
}

/// Map a zome-call failure onto this handler's error.
///
/// A conductor-admission shed passes through UNCHANGED. It carries the marker
/// `conductor_admission::is_admission_shed` keys on, and collapsing it into the
/// generic "unavailable" message destroys the only signal saying the conductor
/// never saw the call — which is how a shed reached the wire as a bare 500.
fn zome_error(op: &str, e: StorageError) -> StorageError {
    if crate::conductor_admission::is_admission_shed(&e) {
        return e;
    }
    StorageError::Conductor(format!("{op}: conductor unavailable"))
}

// ---------------------------------------------------------------------------
// GET /api/v1/source-chain/{agentId}/entries
// ---------------------------------------------------------------------------

async fn handle_get_entries(
    agent_id: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let Some(hc) = hc_registry.and_then(|r| r.imagodei.clone()) else {
        tracing::debug!(
            agent_id,
            "no imagodei conductor configured — returning empty source-chain entries"
        );
        let empty: Vec<SourceChainEntryView> = vec![];
        return Ok(response::ok(&empty));
    };

    let payload = rmp_serde::to_vec_named(&())
        .map_err(|e| StorageError::Conductor(format!("encode source-chain entries input: {e}")))?;

    match hc
        .call_zome("imagodei", "query_my_source_chain", payload)
        .await
    {
        Ok(resp_bytes) => {
            let zome_entries: Vec<ZomeSourceChainEntry> = rmp_serde::from_slice(&resp_bytes)
                .map_err(|e| {
                    StorageError::Conductor(format!("decode query_my_source_chain response: {e}"))
                })?;
            let views: Vec<SourceChainEntryView> =
                zome_entries.into_iter().map(Into::into).collect();
            Ok(response::ok(&views))
        }
        Err(e) => {
            tracing::warn!(
                agent_id,
                error = %e,
                "imagodei zome unreachable for query_my_source_chain"
            );
            Err(zome_error("query_my_source_chain", e))
        }
    }
}

// ---------------------------------------------------------------------------
// GET /api/v1/source-chain/{agentId}/links
// ---------------------------------------------------------------------------

async fn handle_get_links(
    agent_id: &str,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let Some(hc) = hc_registry.and_then(|r| r.imagodei.clone()) else {
        tracing::debug!(
            agent_id,
            "no imagodei conductor configured — returning empty source-chain links"
        );
        let empty: Vec<EntryLinkView> = vec![];
        return Ok(response::ok(&empty));
    };

    let payload = rmp_serde::to_vec_named(&())
        .map_err(|e| StorageError::Conductor(format!("encode source-chain links input: {e}")))?;

    match hc
        .call_zome("imagodei", "query_my_source_chain_links", payload)
        .await
    {
        Ok(resp_bytes) => {
            let zome_links: Vec<ZomeSourceChainLink> =
                rmp_serde::from_slice(&resp_bytes).map_err(|e| {
                    StorageError::Conductor(format!(
                        "decode query_my_source_chain_links response: {e}"
                    ))
                })?;
            let views: Vec<EntryLinkView> = zome_links.into_iter().map(Into::into).collect();
            Ok(response::ok(&views))
        }
        Err(e) => {
            tracing::warn!(
                agent_id,
                error = %e,
                "imagodei zome unreachable for query_my_source_chain_links"
            );
            Err(zome_error("query_my_source_chain_links", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conductor_admission::{is_admission_shed, ADMISSION_SHED_MARKER};

    /// The shape `ConductorAdmission::shed_error` produces.
    fn shed() -> StorageError {
        StorageError::Timeout(format!(
            "{ADMISSION_SHED_MARKER}: no conductor permit for imagodei within 100ms \
             (class=interactive, capacity=4, in_flight=4) — nothing was dispatched"
        ))
    }

    /// A shed must survive the handler unchanged. Collapsing it into a generic
    /// "conductor unavailable" destroys the ADMISSION_SHED_MARKER, and with it
    /// every downstream `is_admission_shed` classification — which is how a
    /// shed reached the wire as a bare 500 (measured live 2026-08-18).
    #[test]
    fn a_shed_survives_the_handlers_error_wrap() {
        let mapped = zome_error("query_my_source_chain", shed());
        assert!(
            is_admission_shed(&mapped),
            "the handler laundered the shed marker: {mapped}"
        );
    }

    /// A genuine conductor failure still collapses to the readable message —
    /// the classification keys on the marker, not on every error.
    #[test]
    fn a_real_conductor_failure_still_reads_as_unavailable() {
        let mapped = zome_error(
            "query_my_source_chain",
            StorageError::Conductor("Zome call failed: ZomeNotFound".into()),
        );
        assert!(!is_admission_shed(&mapped));
        assert!(mapped.to_string().contains("conductor unavailable"));
    }
}
