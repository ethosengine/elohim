//! PeerStatus coordinator functions.
//!
//! Peers publish `PeerStatus` snapshots (self-authored, validated by the
//! integrity zome — see `peer_status.rs` in the integrity crate). This
//! module provides the coordinator surface:
//!
//! - `record_peer_status`: commit a snapshot and link it from the peer's
//!   AgentPubKey anchor for latest-status lookup.
//! - `get_latest_peer_status_for_agent`: read the most-recent snapshot for
//!   a given agent by scanning the `AgentToPeerStatus` links.
//!
//! Downstream (Task 6+) a post-commit signal projects these into
//! elohim-storage for fast cross-agent queries.

use hdk::prelude::*;
use infrastructure_integrity::{EntryTypes, LinkTypes, PeerStatus};

/// Record a PeerStatus snapshot.
///
/// Validation (integrity zome) enforces:
/// - `peer_id == action.author` (self-authored only)
/// - `timestamp` within ±5m of `action.timestamp`
#[hdk_extern]
pub fn record_peer_status(ps: PeerStatus) -> ExternResult<ActionHash> {
    let action_hash = create_entry(&EntryTypes::PeerStatus(ps.clone()))?;

    // Link from the peer's AgentPubKey to this status entry so we can
    // look up the latest snapshot by agent without a full DHT scan.
    create_link(
        AnyLinkableHash::from(ps.peer_id.clone()),
        AnyLinkableHash::from(action_hash.clone()),
        LinkTypes::AgentToPeerStatus,
        (),
    )?;

    Ok(action_hash)
}

/// Get the most-recent PeerStatus for an agent.
///
/// Returns `None` if the agent has never published a PeerStatus. When
/// multiple snapshots exist, the link with the highest create-link
/// timestamp wins. Downstream projection (Task 8+) uses a keyed upsert
/// against the same ordering, so storage and DHT agree on "latest".
#[hdk_extern]
pub fn get_latest_peer_status_for_agent(
    agent: AgentPubKey,
) -> ExternResult<Option<PeerStatus>> {
    let query = LinkQuery::try_new(
        AnyLinkableHash::from(agent),
        LinkTypes::AgentToPeerStatus,
    )?;
    let mut links = get_links(query, GetStrategy::default())?;
    links.sort_by_key(|l| l.timestamp);

    let Some(link) = links.last() else {
        return Ok(None);
    };
    let Some(action_hash) = link.target.clone().into_action_hash() else {
        return Ok(None);
    };
    let Some(record) = get(action_hash, GetOptions::default())? else {
        return Ok(None);
    };
    let ps = record
        .entry()
        .to_app_option::<PeerStatus>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;
    Ok(ps)
}

/// Debug convenience: returns the calling agent's latest PeerStatus (if any),
/// wrapped in a Vec for API compatibility with a future cross-agent query.
///
/// Cross-agent aggregation is deferred to doorway's PeerStatus subscription
/// layer (Phase 2). This fn exists so operator debug tools can poll a single
/// zome endpoint regardless of future scope expansion.
#[hdk_extern]
pub fn get_all_current_peer_statuses(_: ()) -> ExternResult<Vec<PeerStatus>> {
    let me = agent_info()?.agent_initial_pubkey;
    match get_latest_peer_status_for_agent(me)? {
        Some(ps) => Ok(vec![ps]),
        None => Ok(vec![]),
    }
}
