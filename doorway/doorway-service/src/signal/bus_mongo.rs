//! Mongo-backed signal bus — the shared cross-relay backing (D2).
//!
//! Mirrors [`crate::bootstrap::k2_mongo::MongoK2Store`]: a collection in a FIXED
//! SHARED database (independent of the per-doorway projection DB) so every doorway
//! relay in a domain publishes to and drains from ONE `signal_bus` collection. The
//! origin relay publishes a frame it could not deliver locally; a sibling relay
//! draining the bus delivers it to its own live connection. This is the signal
//! analog of the shared bootstrap table — but for a LIVE relay, so it forwards
//! frames rather than sharing presence rows.
//!
//! Safety (deploy-on-hot-path):
//! - NO mongo error ever panics. A failed publish logs and drops (the peer retries
//!   the handshake); a failed drain returns `([], cursor)` — the relay keeps serving
//!   its local connections, never crashes the signal path.
//! - Index creation is best-effort (spawned, idempotent). Drains stay correct
//!   without the TTL index because they filter `created_at > cursor` explicitly;
//!   the index only bounds growth.

use bson::{doc, DateTime};
use futures_util::StreamExt;
use mongodb::{options::FindOptions, Collection, IndexModel};
use tracing::warn;

use super::bus::{BusMessage, SignalBus};
use crate::db::mongo::{IntoIndexes, MongoClient};
use crate::db::schemas::signal_bus::{SignalBusDoc, SIGNAL_BUS_COLLECTION};

/// Mongo-backed [`SignalBus`] over the shared domain DB.
pub struct MongoSignalBus {
    coll: Collection<SignalBusDoc>,
    /// This relay's id — its own publishes are filtered out on drain (loop-free).
    relay_id: String,
}

impl MongoSignalBus {
    /// Construct against a FIXED shared DB (`db_name`) — NOT `mongo.db_name()`
    /// (the per-doorway projection DB). Spawns best-effort idempotent TTL index
    /// creation; a sync constructor cannot await, and drains do not depend on it.
    pub fn new(mongo: &MongoClient, db_name: &str, relay_id: impl Into<String>) -> Self {
        let coll = mongo
            .inner()
            .database(db_name)
            .collection::<SignalBusDoc>(SIGNAL_BUS_COLLECTION);

        let index_coll = coll.clone();
        tokio::spawn(async move {
            let models: Vec<IndexModel> = SignalBusDoc::into_indices()
                .into_iter()
                .map(|(keys, opts)| IndexModel::builder().keys(keys).options(opts).build())
                .collect();
            if let Err(e) = index_coll.create_indexes(models).await {
                warn!("signal-bus index creation failed (non-fatal): {e}");
            }
        });

        Self {
            coll,
            relay_id: relay_id.into(),
        }
    }
}

/// Microseconds-since-epoch → `bson::DateTime` (millisecond precision).
fn micros_to_bson(micros: i64) -> DateTime {
    DateTime::from_millis(micros / 1000)
}

#[async_trait::async_trait]
impl SignalBus for MongoSignalBus {
    async fn publish(&self, dest: &[u8; 32], payload: &[u8]) {
        let entry = SignalBusDoc {
            id: None,
            metadata: crate::db::schemas::Metadata::new(),
            dest: dest.to_vec(),
            payload: payload.to_vec(),
            origin_relay: self.relay_id.clone(),
            created_at: DateTime::now(),
        };
        if let Err(e) = self.coll.insert_one(&entry).await {
            // Drop-and-log: the peer retries the handshake. Never crash the relay.
            warn!("signal-bus publish failed (frame dropped, peer will retry): {e}");
        }
    }

    async fn drain(&self, cursor: i64) -> (Vec<BusMessage>, i64) {
        // All frames published AFTER the cursor, ascending. No origin filter in
        // the QUERY (so the cursor advances past our own frames too); filter own
        // frames in code. Millisecond cursor precision: a frame written in the
        // same ms as the advanced cursor could be missed on the next drain — an
        // accepted trade for a retry-tolerant, low-volume, bounded-domain plane
        // (a dropped handshake frame self-heals via WebRTC retry).
        let filter = doc! { "created_at": { "$gt": micros_to_bson(cursor) } };
        let opts = FindOptions::builder()
            .sort(doc! { "created_at": 1 })
            .build();

        let mut db_cursor = match self.coll.find(filter).with_options(opts).await {
            Ok(c) => c,
            Err(e) => {
                warn!("signal-bus drain find failed (serving none): {e}");
                return (Vec::new(), cursor);
            }
        };

        let mut out = Vec::new();
        let mut new_cursor = cursor;
        loop {
            match db_cursor.next().await {
                Some(Ok(d)) => {
                    // Advance the cursor past every frame we see (incl. our own).
                    let micros = d.created_at.timestamp_millis() * 1000;
                    new_cursor = new_cursor.max(micros);
                    if d.origin_relay == self.relay_id {
                        continue; // never redeliver our own publishes (loop-free)
                    }
                    if d.dest.len() != 32 {
                        continue; // malformed dest — skip, never poison the batch
                    }
                    let mut dest = [0u8; 32];
                    dest.copy_from_slice(&d.dest);
                    out.push(BusMessage {
                        dest,
                        payload: d.payload,
                    });
                }
                Some(Err(e)) => {
                    warn!("signal-bus drain row decode failed (skipping): {e}");
                }
                None => break,
            }
        }
        (out, new_cursor)
    }

    fn backend_name(&self) -> &'static str {
        "mongo"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cross-pod proof: two `MongoSignalBus` handles (distinct relay ids) built
    /// from two `MongoClient`s with DISTINCT per-doorway db_names but both targeting
    /// the SAME shared DB. A frame published on relay A is drained by relay B (and
    /// NOT by A itself) — the cross-relay forward the per-pod store could never do.
    ///
    /// `#[ignore]` — needs a reachable mongo at `MONGODB_TEST_URI`; the default
    /// suite stays green with no infra.
    #[tokio::test]
    #[ignore = "needs a reachable mongo at MONGODB_TEST_URI"]
    async fn frame_published_on_a_is_drained_by_b_not_a() {
        let uri = std::env::var("MONGODB_TEST_URI")
            .expect("set MONGODB_TEST_URI to run this ignored test");
        let a_client = MongoClient::new(&uri, "doorway-alpha").await.unwrap();
        let b_client = MongoClient::new(&uri, "doorway-alpha-b").await.unwrap();
        let db = "elohim-bootstrap";
        let a = MongoSignalBus::new(&a_client, db, "relay-a");
        let b = MongoSignalBus::new(&b_client, db, "relay-b");

        let dest = [9u8; 32];
        a.publish(&dest, b"handshake").await;

        let (b_msgs, _) = b.drain(0).await;
        assert!(
            b_msgs
                .iter()
                .any(|m| m.dest == dest && m.payload == b"handshake"),
            "sibling relay B must drain A's published frame through the shared bus"
        );

        let (a_msgs, _) = a.drain(0).await;
        assert!(
            !a_msgs.iter().any(|m| m.dest == dest),
            "origin relay A must NOT drain its own publish (loop-free)"
        );
    }
}
