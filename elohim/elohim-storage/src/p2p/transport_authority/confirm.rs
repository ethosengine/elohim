//! DHT confirmation of gossip-arrived cross-signed bindings.
//!
//! A cross-signed `AgentPeerBinding` that arrives by gossip proves only that its sender
//! holds both keys; anyone can mint any number of agent↔PeerId pairs and gossip them.
//! It becomes AUTHORITY (`p2p::transport_authority`) only when this node finds, on the
//! DHT, a current `AgentPeerBinding` entry whose action hash is the row's
//! `dht_anchor_hash` and whose peer, agent and validity window are the row's. Then the
//! confirmation is recorded (`db::binding_confirmations`) and the claims held for that
//! author are replayed (`p2p::gossip_dispatch::schedule_replay`).
//!
//! The read is the imagodei `get_bindings_for_peer` (legacy mode: the bindings linked
//! from the peer's anchor, superseded ones filtered out). Bindings from this node's
//! OWN conductor arrive as DHT signals and are confirmed by the projection itself
//! (`reconcile::controller::on_agent_peer_binding`), not here.
//!
//! What the legacy view does not carry: the entry's author and signature. A
//! confirmation therefore proves that a current entry with these fields exists at this
//! anchor on the DHT (so each binding costs a DHT entry on a source chain the network
//! validates), not that the named agent authored it; author-level confirmation needs a
//! zome query that returns the record.
//!
//! Retries: a remote entry needs publish time before a `get` finds it, so each request
//! is retried on [`CONFIRM_BACKOFF_SECS`] (about 33 minutes in total; the hold window
//! in `db::held_unbound` covers it). Requests are deduplicated while in flight and run
//! at most [`CONFIRM_CONCURRENCY`] at a time.

use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};

use diesel::prelude::*;
use tracing::{debug, info, warn};

use crate::db::DbPool;
use crate::hc_client::HcClient;

/// Backoff between DHT reads for one binding, in seconds.
pub const CONFIRM_BACKOFF_SECS: [u64; 7] = [5, 15, 45, 120, 300, 600, 900];

/// Confirmations read at once.
pub const CONFIRM_CONCURRENCY: usize = 8;

/// One binding to confirm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfirmRequest {
    pub peer_id: String,
    pub dht_anchor_hash: String,
}

static CONFIRMER: OnceLock<tokio::sync::mpsc::UnboundedSender<ConfirmRequest>> = OnceLock::new();

/// Ask the confirmer to look for this binding on the DHT. A no-op when no confirmer is
/// running (no conductor bridge): the binding then never becomes authority.
pub fn request_confirmation(peer_id: &str, dht_anchor_hash: &str) {
    if let Some(tx) = CONFIRMER.get() {
        let _ = tx.send(ConfirmRequest {
            peer_id: peer_id.to_string(),
            dht_anchor_hash: dht_anchor_hash.to_string(),
        });
    }
}

/// Start the confirmer (once per process) with the imagodei conductor bridge.
pub fn spawn_binding_confirmer(hc: Arc<HcClient>, pool: DbPool) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ConfirmRequest>();
    if CONFIRMER.set(tx).is_err() {
        warn!("binding confirmer already running — second start ignored");
        return;
    }
    let in_flight: Arc<Mutex<HashSet<ConfirmRequest>>> = Arc::new(Mutex::new(HashSet::new()));
    let permits = Arc::new(tokio::sync::Semaphore::new(CONFIRM_CONCURRENCY));
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            if !in_flight
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .insert(request.clone())
            {
                continue;
            }
            let (hc, pool, in_flight, permits) =
                (hc.clone(), pool.clone(), in_flight.clone(), permits.clone());
            tokio::spawn(async move {
                let Ok(_permit) = permits.acquire_owned().await else {
                    return;
                };
                for (attempt, backoff) in CONFIRM_BACKOFF_SECS.iter().enumerate() {
                    match confirm_once(&hc, &pool, &request).await {
                        Ok(true) => break,
                        Ok(false) => debug!(
                            peer_id = %request.peer_id, attempt = attempt + 1,
                            "binding not on the DHT yet — retrying"
                        ),
                        Err(e) => {
                            debug!(peer_id = %request.peer_id, error = %e, "binding confirmation read failed — retrying")
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_secs(*backoff)).await;
                }
                in_flight
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .remove(&request);
            });
        }
    });
}

/// One entry of the legacy `get_bindings_for_peer` view.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DhtView {
    action_hash: crate::signals::HoloHashB64,
    peer_id: String,
    agent_cid: String,
    valid_from_micros: i64,
    valid_until_micros: Option<i64>,
    #[serde(default)]
    superseded_by: Option<crate::signals::HoloHashB64>,
}

fn iso_of_micros(micros: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(micros)
        .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string())
}

/// Does `view` carry exactly the row's anchor, peer, agent and window, and is it
/// current? Pure, so the matching rule is testable without a conductor.
fn view_matches(view: &DhtView, row: &crate::db::models::PeerIdentityBindingRow) -> bool {
    view.superseded_by.is_none()
        && view.action_hash.as_str() == row.dht_anchor_hash
        && view.peer_id == row.peer_id
        && view.agent_cid == row.agent_cid
        && iso_of_micros(view.valid_from_micros).as_deref() == Some(row.valid_from.as_str())
        && view.valid_until_micros.and_then(iso_of_micros) == row.valid_until
}

/// One DHT read. `Ok(true)` when the request is settled (confirmed, or nothing left to
/// confirm); `Ok(false)` when the entry is not found yet.
async fn confirm_once(
    hc: &HcClient,
    pool: &DbPool,
    request: &ConfirmRequest,
) -> Result<bool, crate::error::StorageError> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl;
    let row = {
        let mut conn = pool
            .get()
            .map_err(|e| crate::error::StorageError::Database(e.to_string()))?;
        dsl::peer_identity_bindings
            .filter(dsl::peer_id.eq(&request.peer_id))
            .filter(dsl::dht_anchor_hash.eq(&request.dht_anchor_hash))
            .first::<crate::db::models::PeerIdentityBindingRow>(&mut conn)
            .optional()
            .map_err(|e| crate::error::StorageError::Database(e.to_string()))?
    };
    let Some(row) = row.filter(|row| row.is_cross_signed()) else {
        return Ok(true);
    };
    let payload = rmp_serde::to_vec_named(&request.peer_id)
        .map_err(|e| crate::error::StorageError::Serialization(e.to_string()))?;
    let bytes = hc
        .call_zome_imagodei("imagodei", "get_bindings_for_peer", payload)
        .await?;
    let views: Vec<DhtView> = rmp_serde::from_slice(&bytes)
        .map_err(|e| crate::error::StorageError::Serialization(e.to_string()))?;
    if !views.iter().any(|view| view_matches(view, &row)) {
        return Ok(false);
    }
    {
        let mut conn = pool
            .get()
            .map_err(|e| crate::error::StorageError::Database(e.to_string()))?;
        crate::db::binding_confirmations::confirm(
            &mut conn,
            &crate::db::binding_confirmations::DhtBinding {
                peer_id: &row.peer_id,
                dht_anchor_hash: &row.dht_anchor_hash,
                agent_cid: &row.agent_cid,
                valid_from: &row.valid_from,
                valid_until: row.valid_until.as_deref(),
            },
        )?;
    }
    info!(
        peer_id = %row.peer_id, agent_cid = %row.agent_cid,
        "cross-signed binding confirmed on the DHT — now authority"
    );
    crate::p2p::gossip_dispatch::schedule_replay(
        pool,
        crate::p2p::gossip_dispatch::authors_bound_by_peer(pool, &row.peer_id),
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> crate::db::models::PeerIdentityBindingRow {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let peer =
            crate::test_util::bind_cross_signed_unconfirmed(&mut conn, "uhCAkDht", [91u8; 32]);
        use crate::db::diesel_schema::peer_identity_bindings::dsl;
        dsl::peer_identity_bindings
            .filter(dsl::peer_id.eq(&peer))
            .first(&mut conn)
            .unwrap()
    }

    fn view_of(row: &crate::db::models::PeerIdentityBindingRow) -> DhtView {
        let micros = |iso: &str| {
            chrono::DateTime::parse_from_rfc3339(iso)
                .unwrap()
                .timestamp_micros()
        };
        let bytes = rmp_serde::to_vec_named(&serde_json::json!({
            "actionHash": row.dht_anchor_hash,
            "peerId": row.peer_id,
            "agentCid": row.agent_cid,
            "validFromMicros": micros(&row.valid_from),
            "validUntilMicros": row.valid_until.as_deref().map(micros),
            "deviceArchetype": "node",
            "supersededBy": null,
        }))
        .unwrap();
        rmp_serde::from_slice(&bytes).unwrap()
    }

    #[test]
    fn only_an_exact_current_dht_entry_confirms() {
        let row = row();
        assert!(view_matches(&view_of(&row), &row));
        let mut other = row.clone();
        other.agent_cid = crate::test_util::test_agent("uhCAkSomeoneElse");
        assert!(!view_matches(&view_of(&other), &row), "another agent");
        let mut anchor = row.clone();
        anchor.dht_anchor_hash = "uhCkkAnotherAnchor".into();
        assert!(!view_matches(&view_of(&anchor), &row), "another anchor");
        let mut window = row.clone();
        window.valid_until = Some("2099-01-01T00:00:00Z".into());
        assert!(!view_matches(&view_of(&window), &row), "another window");
    }
}
