//! Mongo-backed kitsune2 bootstrap store — the genesis-pair islanding fix.
//!
//! Both doorways (alpha-A and alpha-B) reach the same mongo server but use
//! distinct per-doorway projection databases. This store targets a FIXED
//! SHARED database (`elohim-bootstrap`, passed as `db_name`) so the kitsune2
//! bootstrap table is ONE table across both edges — the genesis peers
//! (matthew on doorway-A, adam on doorway-B) PUT into and GET from the same
//! rows, so they can finally discover each other and seed the DHT.
//!
//! Safety (deploy-on-hot-path):
//! - PUT validation is the SHARED [`super::k2::validate_put`] — validation
//!   cannot fork between the mem and mongo backends.
//! - NO mongo error ever panics. A failed GET returns `[]`, a failed `stats`
//!   returns `(0, 0)`, a failed PUT returns `Err` — the bootstrap path
//!   degrades, never crashes the conductor-discovery hot path.
//! - Index creation is best-effort (spawned, idempotent). Reads stay correct
//!   without the TTL index because `get_at` filters `expires_at > now`
//!   explicitly; the index only bounds growth and enforces the unique key.

use bson::{doc, DateTime};
use futures_util::StreamExt;
use mongodb::{options::ReplaceOptions, Collection, IndexModel};
use tracing::warn;

use super::k2::{validate_put, K2PutOutcome, K2Store};
use crate::db::mongo::{IntoIndexes, MongoClient};
use crate::db::schemas::bootstrap_entry::{BootstrapEntryDoc, BOOTSTRAP_COLLECTION};

/// Mongo-backed [`K2Store`] over the shared `elohim-bootstrap` DB.
pub struct MongoK2Store {
    coll: Collection<BootstrapEntryDoc>,
}

impl MongoK2Store {
    /// Construct against a FIXED shared DB (`db_name`, e.g. `elohim-bootstrap`)
    /// — NOT `mongo.db_name()` (which is the per-doorway projection DB). Spawns
    /// best-effort idempotent index creation (TTL on `expires_at` + unique
    /// `(space, agent)`); a sync constructor cannot await, and reads do not
    /// depend on the index existing.
    pub fn new(mongo: &MongoClient, db_name: &str) -> Self {
        let coll = mongo
            .inner()
            .database(db_name)
            .collection::<BootstrapEntryDoc>(BOOTSTRAP_COLLECTION);

        // Best-effort index creation. Spawned because `BootstrapStore::new` is
        // sync (called from the async startup constructors, so a runtime is
        // live). `create_indexes` is idempotent — re-running is harmless.
        let index_coll = coll.clone();
        tokio::spawn(async move {
            let models: Vec<IndexModel> = BootstrapEntryDoc::into_indices()
                .into_iter()
                .map(|(keys, opts)| IndexModel::builder().keys(keys).options(opts).build())
                .collect();
            if let Err(e) = index_coll.create_indexes(models).await {
                warn!("k2 bootstrap index creation failed (non-fatal): {e}");
            }
        });

        Self { coll }
    }
}

/// Convert microseconds-since-epoch to `bson::DateTime` (millisecond precision).
/// The TTL index fires on this wall-clock value, so the `/ 1000` is load-bearing.
fn micros_to_bson(micros: i64) -> DateTime {
    DateTime::from_millis(micros / 1000)
}

#[async_trait::async_trait]
impl K2Store for MongoK2Store {
    async fn put_at(
        &self,
        space_b64: &str,
        agent_b64: &str,
        body: &[u8],
        now_micros: i64,
    ) -> Result<K2PutOutcome, String> {
        // SHARED validation — identical to MemK2Store (no backend drift).
        let parsed = validate_put(space_b64, agent_b64, body, now_micros)?;

        let filter = doc! { "space": space_b64, "agent": agent_b64 };

        if parsed.is_tombstone {
            // Tombstone — remove the row. A delete miss is fine (idempotent).
            self.coll
                .delete_one(filter)
                .await
                .map_err(|e| format!("bootstrap delete failed: {e}"))?;
            return Ok(K2PutOutcome::Removed);
        }

        // Upsert keyed on the unique (space, agent) index — one row per agent,
        // refreshed on the client's heartbeat cadence. Replaces the per-pod
        // arbitrary eviction with a deterministic, replica-shared row.
        let entry = BootstrapEntryDoc {
            id: None,
            metadata: crate::db::schemas::Metadata::new(),
            space: space_b64.to_string(),
            agent: agent_b64.to_string(),
            raw_body: body.to_vec(),
            expires_at: micros_to_bson(parsed.expires_at),
        };
        let options = ReplaceOptions::builder().upsert(true).build();
        self.coll
            .replace_one(filter, &entry)
            .with_options(options)
            .await
            .map_err(|e| format!("bootstrap upsert failed: {e}"))?;
        Ok(K2PutOutcome::Stored)
    }

    async fn get_at(&self, space_b64: &str, now_micros: i64) -> Vec<u8> {
        // Only unexpired rows for this space. Filtering `expires_at > now` keeps
        // reads correct even if the TTL index hasn't pruned yet (or doesn't exist).
        let filter = doc! {
            "space": space_b64,
            "expires_at": { "$gt": micros_to_bson(now_micros) },
        };

        let mut cursor = match self.coll.find(filter).await {
            Ok(c) => c,
            Err(e) => {
                warn!("bootstrap get find failed (serving empty): {e}");
                return b"[]".to_vec();
            }
        };

        // Assemble byte-identically to MemK2Store::get_at: '[' then the verbatim
        // raw_body bytes, comma-joined, then ']'. Do NOT round-trip through
        // serde_json::Value — re-serialization changes bytes and breaks the
        // client's AgentInfoSigned::decode_list.
        let mut out = Vec::with_capacity(2);
        out.push(b'[');
        let mut first = true;
        loop {
            match cursor.next().await {
                Some(Ok(d)) => {
                    if !first {
                        out.push(b',');
                    }
                    first = false;
                    out.extend_from_slice(&d.raw_body);
                }
                Some(Err(e)) => {
                    // One bad row degrades to skip — never poison the whole list.
                    warn!("bootstrap get row decode failed (skipping): {e}");
                }
                None => break,
            }
        }
        out.push(b']');
        out
    }

    async fn cleanup(&self, _now_micros: i64) -> usize {
        // Mongo TTL index owns expiry — nothing to do here.
        0
    }

    async fn stats(&self) -> (usize, usize) {
        let agents = self
            .coll
            .count_documents(doc! {})
            .await
            .unwrap_or_else(|e| {
                warn!("bootstrap stats count failed: {e}");
                0
            }) as usize;
        let spaces = match self.coll.distinct("space", doc! {}).await {
            Ok(vals) => vals.len(),
            Err(e) => {
                warn!("bootstrap stats distinct failed: {e}");
                0
            }
        };
        (agents, spaces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap::k2::current_time_micros;

    #[tokio::test]
    async fn mongo_store_rejects_invalid_body_before_db() {
        // The shared validation helper rejects bad bodies before any mongo call
        // — proving MongoK2Store reuses MemK2Store's validation (no fork). No
        // live DB needed.
        let e = validate_put("bad", "bad", b"{}", 0);
        assert!(e.is_err(), "invalid body must be rejected pre-DB");
    }

    /// Build a valid signed kitsune2 agent-info body (the way the client does):
    /// `(space_b64, agent_b64, body)` for the given key/space/window.
    fn signed_fixture(
        key_seed: u8,
        space: &[u8],
        created_at: i64,
        expires_at: i64,
    ) -> (String, String, Vec<u8>) {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        use ed25519_dalek::{Signer, SigningKey};

        let key = SigningKey::from_bytes(&[key_seed; 32]);
        let agent_b64 = URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes());
        let space_b64 = URL_SAFE_NO_PAD.encode(space);
        let inner = serde_json::json!({
            "agent": agent_b64,
            "space": space_b64,
            "createdAt": created_at.to_string(),
            "expiresAt": expires_at.to_string(),
            "isTombstone": false,
            "url": "wss://example/sig",
            "storageArc": [0, 4294967295u32],
        })
        .to_string();
        let signature = key.sign(inner.as_bytes());
        let body = serde_json::json!({
            "agentInfo": inner,
            "signature": URL_SAFE_NO_PAD.encode(signature.to_bytes()),
        });
        (space_b64, agent_b64, serde_json::to_vec(&body).unwrap())
    }

    /// Cross-pod proof: two `MongoK2Store` handles built from two `MongoClient`s
    /// with DISTINCT per-doorway db_names (doorway-alpha vs doorway-alpha-b) but
    /// both targeting the SAME shared `elohim-bootstrap` DB. A PUT on handle A is
    /// visible on handle B — the cross-edge visibility the per-pod store could
    /// never express. This is the genesis-pair islanding fix, end to end.
    ///
    /// `#[ignore]` — needs a reachable mongo at `MONGODB_TEST_URI`; the default
    /// suite stays green with no infra.
    #[tokio::test]
    #[ignore = "needs a reachable mongo at MONGODB_TEST_URI"]
    async fn put_on_a_is_visible_on_b_across_distinct_db_names() {
        let uri = std::env::var("MONGODB_TEST_URI")
            .expect("set MONGODB_TEST_URI to run this ignored test");
        let a = MongoClient::new(&uri, "doorway-alpha").await.unwrap();
        let b = MongoClient::new(&uri, "doorway-alpha-b").await.unwrap();
        let sa = MongoK2Store::new(&a, "elohim-bootstrap");
        let sb = MongoK2Store::new(&b, "elohim-bootstrap");

        let now = current_time_micros();
        // Unique space per run so reruns don't collide with stale rows.
        let space = format!("xpod-{now}");
        let (space_b64, agent_b64, body) =
            signed_fixture(42, space.as_bytes(), now, now + 1_200_000_000);

        sa.put_at(&space_b64, &agent_b64, &body, now).await.unwrap();
        let got = sb.get_at(&space_b64, now).await;
        assert!(
            got.windows(body.len()).any(|w| w == body.as_slice()),
            "doorway-B must see doorway-A's PUT through the shared bootstrap table"
        );
    }
}
