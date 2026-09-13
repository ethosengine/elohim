//! Recover this peer's original signed binding through the live signal controller.
//! No publication, authority elevation, or source-chain scan happens here.
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use holochain_types::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::db::{models::PeerIdentityBindingRow, DbPool};
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::reconcile::signal_stream::{AgentPeerBindingSignal, DnaSignal};
use crate::services::authenticated_record::{verify_record, RecordRequirements};

use super::binding_cross_signature::TRANSPORT_KIND_LIBP2P;
use super::binding_proof_wire::{classify_binding_signature, PROOF_ENVELOPE_PREFIX};

const MAX_RECORDS: usize = 64;
const MAX_RECORD_BYTES: usize = 256 * 1024;

#[cfg(test)]
#[path = "binding_recovery_tests.rs"]
mod tests;

/// Boot-time producer capability. A weak sender does not keep a disconnected
/// conductor stream alive and prevent its existing reconnect loop from running.
#[derive(Clone, Debug)]
pub struct OwnBindingProjection {
    sender: Arc<RwLock<mpsc::WeakSender<DnaSignal>>>,
}

impl OwnBindingProjection {
    pub(crate) fn new(sender: &mpsc::Sender<DnaSignal>) -> Self {
        Self {
            sender: Arc::new(RwLock::new(sender.downgrade())),
        }
    }

    /// The existing reconnect loop refreshes the producer capability after
    /// registering its new live callback. This does not schedule another mint
    /// or retain either stream's strong sender.
    pub fn use_connected_stream(&self, connected: &Self) -> Result<(), StorageError> {
        let sender = connected
            .sender
            .read()
            .map_err(|_| unavailable("connected stream lock poisoned"))?
            .clone();
        *self
            .sender
            .write()
            .map_err(|_| unavailable("stream lock poisoned"))? = sender;
        Ok(())
    }

    fn enqueue(&self, signal: AgentPeerBindingSignal) -> Result<(), StorageError> {
        let sender = self
            .sender
            .read()
            .map_err(|_| unavailable("stream lock poisoned"))?
            .clone();
        sender
            .upgrade()
            .ok_or_else(|| unavailable("signal stream closed"))?
            .try_send(DnaSignal::AgentPeerBindingRecovery(signal))
            .map_err(|_| unavailable("signal queue full or closed"))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OwnQuery<'a> {
    peer_id: &'a str,
    agent_key: &'a str,
}

/// Only the record is projected; query metadata supplies the expected action.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OriginalBinding {
    action_hash: String,
    record: Option<Vec<u8>>,
}

fn unavailable(reason: &str) -> StorageError {
    StorageError::Internal(format!("own binding recovery deferred: {reason}"))
}

fn existing_row(
    pool: &DbPool,
    peer: &str,
    anchor: &str,
) -> Result<Option<PeerIdentityBindingRow>, StorageError> {
    use crate::db::diesel_schema::peer_identity_bindings::dsl;
    let mut conn = pool.get().map_err(|e| unavailable(&e.to_string()))?;
    dsl::peer_identity_bindings
        .filter(dsl::peer_id.eq(peer))
        .filter(dsl::dht_anchor_hash.eq(anchor))
        .first::<PeerIdentityBindingRow>(&mut conn)
        .optional()
        .map_err(|e| unavailable(&e.to_string()))
}

#[cfg(test)]
pub(crate) fn exact_acknowledged(
    pool: &DbPool,
    peer: &str,
    agent: &str,
    anchor: &str,
) -> Result<bool, StorageError> {
    Ok(existing_row(pool, peer, anchor)?.is_some_and(|row| {
        row.agent_cid == agent && row.superseded_by.is_none() && row.is_cross_signed()
    }))
}

/// Failure is never absence. The existing mint driver owns retry/backoff; a
/// newly queued original is acknowledged on its next attempt, without another
/// authoring action or a second polling loop.
pub(crate) async fn recover_before_mint(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    peer: &str,
    agent: &str,
    projection: &OwnBindingProjection,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    let payload = rmp_serde::to_vec_named(&OwnQuery {
        peer_id: peer,
        agent_key: agent,
    })
    .map_err(|e| unavailable(&e.to_string()))?;
    let bytes = hc
        .call_zome_imagodei("imagodei", "get_bindings_for_peer", payload)
        .await?;
    if bytes.len() > MAX_RECORDS * MAX_RECORD_BYTES {
        return Err(unavailable("query response exceeds recovery budget"));
    }
    let rows: Vec<OriginalBinding> = rmp_serde::from_slice(&bytes)
        .map_err(|e| unavailable(&format!("decode original records: {e}")))?;
    recover_records(rows, pool, peer, agent, projection, now)
}

fn recover_records(
    rows: Vec<OriginalBinding>,
    pool: &DbPool,
    peer: &str,
    agent: &str,
    projection: &OwnBindingProjection,
    now: DateTime<Utc>,
) -> Result<(), StorageError> {
    if rows.len() > MAX_RECORDS {
        return Err(unavailable("record budget exceeded"));
    }
    let author = AgentPubKey::try_from(agent).map_err(|_| unavailable("invalid own agent key"))?;
    let mut pending = false;
    for row in rows {
        let hash = ActionHash::try_from(row.action_hash.as_str())
            .map_err(|_| unavailable("invalid original action hash"))?;
        let bytes = row
            .record
            .ok_or_else(|| unavailable("original record missing"))?;
        let record = verify_record(
            &bytes,
            RecordRequirements {
                action_hash: &hash,
                entry_hash: None,
                author: Some(&author),
                // imagodei_integrity::EntryTypes::AgentPeerBinding, current DNA.
                zome_index: 0,
                entry_index: 20,
                allow_update: false,
                max_bytes: MAX_RECORD_BYTES,
            },
        )?
        .ok_or_else(|| unavailable("original record absent"))?;
        let Some(Entry::App(entry)) = record.entry().as_option() else {
            return Err(unavailable("binding entry missing"));
        };
        let signal = crate::reconcile::holochain_app_signal::binding_signal_from_record(
            row.action_hash,
            entry.bytes(),
        )?;
        if signal.agent_cid != agent || signal.peer_id != peer {
            return Err(unavailable("original binding names another agent or peer"));
        }
        // Historical self-assertions are not cross-signed originals. They do not
        // block the existing mint which upgrades that posture.
        if !signal.signature.starts_with(PROOF_ENVELOPE_PREFIX) {
            continue;
        }
        let from = signal.valid_from.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let until = signal
            .valid_until
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string());
        if !classify_binding_signature(
            agent,
            peer,
            TRANSPORT_KIND_LIBP2P,
            &from,
            until.as_deref(),
            &signal.signature,
        )
        .is_cross_signed()
        {
            return Err(unavailable("claimed original proof does not verify"));
        }
        if signal.superseded_by.is_some() {
            return Err(unavailable("original binding superseded"));
        }
        if signal.valid_from > now {
            return Err(unavailable("original binding starts in the future"));
        }
        if signal.valid_until.is_some_and(|until| until <= now) {
            continue;
        }
        if let Some(existing) = existing_row(pool, peer, &signal.binding_action_hash)? {
            if existing.superseded_by.is_some() {
                return Err(unavailable(
                    "stored binding supersession must not be cleared",
                ));
            }
            if existing.agent_cid != agent {
                return Err(unavailable("stored original names another agent"));
            }
            if existing.is_cross_signed()
                && existing.signature == signal.signature
                && existing.valid_from == from
                && existing.valid_until == until
            {
                continue;
            }
        }
        projection.enqueue(signal)?;
        pending = true;
    }
    if pending {
        Err(unavailable(
            "waiting for exact original projection acknowledgement",
        ))
    } else {
        Ok(())
    }
}
