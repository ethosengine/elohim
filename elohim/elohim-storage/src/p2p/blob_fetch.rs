//! Shared blob-fetch helper. Used by both:
//! - HTTP blob handler (`GET /blob/{cid}`) on local miss
//! - Custody reconciliation controller (T16) for own-commitment kicks
//!
//! Strategy:
//! 1. Look up candidate peers in `peer_blob_inventory`, ordered by evidence
//!    strength (fetch-success first, then by recency).
//! 2. Filter to currently-connected peers.
//! 3. Race candidates in parallel batches of `fetch_blob_parallelism` (default 3).
//!    The first reply whose bytes hash to the requested byte CID wins; the losing
//!    racers in the batch are **cancelled** (their tasks aborted), and a failed batch
//!    advances to the next.
//! 4. On verified success: persist locally, record fetch-success in
//!    `peer_blob_inventory`, emit `serve-blob` REA event.
//!
//! Every fetch runs under a [`FetchBudget`]: a byte ceiling checked before a reply or
//! a manifest is acted on, a total deadline after which a trickling holder is
//! abandoned, and a bound on how many batches are tried.
//!
//! Verification: the bytes recompute to the requested byte CID (raw + BLAKE3-256),
//! the multihash the CID itself declares. There is no other accepted spelling.

use crate::blob_store::BlobStore;
use crate::config::Config;
use crate::db::diesel_schema::economic_events;
use crate::db::models::NewEconomicEvent;
use crate::db::peer_blob_inventory::record_fetch_success;
use crate::error::StorageError;
use crate::sharding::ShardManifest;
use chrono::Utc;
use diesel::Connection;
use diesel::RunQueryDsl;
use diesel::SqliteConnection;
use elohim_epr::address::{compute_bytes_cid, ByteCid};
use futures::stream::{FuturesUnordered, StreamExt};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Internal reply payload carried by `P2PCommand::FetchBlob`'s oneshot channel.
///
/// **Not a wire type** — the wire type is `blob_protocol::BlobFetchResponse`.
/// A peer with no direct bytes for a striped payload can still answer usefully with
/// the durable manifest it resolved, so `race_fetch` treats that as a distinct
/// outcome from a plain miss.
#[derive(Debug, Clone)]
pub enum BlobFetchReply {
    /// Bytes for the requested CID, not yet verified.
    Bytes(Vec<u8>),
    /// No direct bytes, but the peer resolved a durable manifest for the CID.
    Manifest(Box<ShardManifest>),
}

/// The bounds one artifact's fetch runs under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchBudget {
    /// Largest payload this fetch will accept: a reply or a manifest declaring more
    /// is refused before anything is allocated for it.
    pub max_bytes: u64,
    /// Deadline for one peer's reply (a stalled holder is dropped after this).
    pub stall_deadline: Duration,
    /// Deadline for the whole race: a holder still trickling when it passes is
    /// abandoned, and the fetch reports `BudgetExhausted`.
    pub total_deadline: Duration,
    /// How many batches of candidates are tried before giving up.
    pub retries: usize,
}

impl FetchBudget {
    /// The largest payload a fetch accepts by default (4 GiB): replies are further
    /// bounded by the protocol frame, so this binds manifests (the pivot for payloads
    /// too large for one frame).
    pub const DEFAULT_MAX_BYTES: u64 = 4 * 1024 * 1024 * 1024;

    /// A manifest naming more custody units than this is refused unread: at the
    /// epoch-0 profile it would describe tens of gigabytes.
    pub const MAX_MANIFEST_UNITS: usize = 1 << 16;

    /// The default budget around a per-peer stall deadline: [`Self::DEFAULT_MAX_BYTES`],
    /// eight batches, and a total deadline of four stall deadlines per batch.
    pub fn for_stall(stall_deadline: Duration) -> Self {
        const DEFAULT_RETRIES: u32 = 8;
        Self {
            max_bytes: Self::DEFAULT_MAX_BYTES,
            stall_deadline,
            total_deadline: stall_deadline.saturating_mul(4 * DEFAULT_RETRIES),
            retries: DEFAULT_RETRIES as usize,
        }
    }

    /// Whether a manifest received from a peer is within this budget: its declared
    /// length and unit counts are checked BEFORE any reconstruction buffer exists.
    pub fn admits_manifest(&self, manifest: &ShardManifest) -> bool {
        manifest.byte_len <= self.max_bytes
            && manifest.chunks.len() <= Self::MAX_MANIFEST_UNITS
            && manifest.stripes.len() <= Self::MAX_MANIFEST_UNITS
    }
}

/// Outcome of a race-fetch.
#[derive(Debug)]
pub enum FetchOutcome {
    /// Bytes fetched and verified against the requested CID.
    Hit { bytes: Vec<u8>, source_peer: String },
    /// No peer served bytes, but a peer answered with a durable `ShardManifest` for
    /// the requested CID (verified to name it, and within the budget) — the caller
    /// can pivot to a swarm shard-fetch (`p2p::blob_swarm`). Boxed: `ShardManifest`
    /// would otherwise bloat every `FetchOutcome` (clippy::large_enum_variant).
    Manifest {
        manifest: Box<ShardManifest>,
        source_peer: String,
    },
    /// All candidates exhausted; no peer served verified bytes or a manifest.
    Miss,
    /// Inventory had no candidates to try (either empty or none connected).
    NoCandidates,
    /// The budget's total deadline passed before any peer delivered; every racer
    /// still in flight was cancelled.
    BudgetExhausted,
}

/// Race a fetch across the candidates known to host the blob, under the default
/// budget for `per_peer_timeout`. See [`race_fetch_budgeted`].
pub async fn race_fetch(
    cid: &ByteCid,
    candidates: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    is_connected: impl Fn(&str) -> bool,
    parallelism: usize,
    per_peer_timeout: Duration,
) -> FetchOutcome {
    race_fetch_budgeted(
        cid,
        candidates,
        cmd_tx,
        is_connected,
        parallelism,
        &FetchBudget::for_stall(per_peer_timeout),
    )
    .await
}

/// Race a fetch across the candidates known to host the blob.
///
/// Candidates are filtered to peers connected at call time and processed in batches
/// of `parallelism`; all peers in a batch are attempted concurrently. The first reply
/// whose bytes hash to `cid` wins and every other racer is aborted (no request keeps
/// running for a race already won). A manifest that names `cid` and fits the budget
/// ends the race as `Manifest`. Replies larger than `budget.max_bytes`, bytes that do
/// not hash to `cid`, and manifests naming another payload are refused. The whole race
/// is bounded by `budget.total_deadline` and `budget.retries` batches.
pub async fn race_fetch_budgeted(
    cid: &ByteCid,
    candidates: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    is_connected: impl Fn(&str) -> bool,
    parallelism: usize,
    budget: &FetchBudget,
) -> FetchOutcome {
    let connected: Vec<String> = candidates.into_iter().filter(|p| is_connected(p)).collect();
    if connected.is_empty() {
        return FetchOutcome::NoCandidates;
    }
    match tokio::time::timeout(
        budget.total_deadline,
        race_batches(cid, connected, cmd_tx, parallelism.max(1), budget),
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(_) => {
            tracing::debug!(
                target: "elohim_storage::blob_fetch",
                cid = %cid,
                total_deadline_ms = budget.total_deadline.as_millis() as u64,
                "fetch budget exhausted; in-flight racers cancelled"
            );
            FetchOutcome::BudgetExhausted
        }
    }
}

/// Aborts every task it holds when dropped, so a race that returns (won, lost or
/// timed out) leaves no racer running.
struct AbortOnDrop(Vec<tokio::task::AbortHandle>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        for handle in &self.0 {
            handle.abort();
        }
    }
}

/// A race was won: every other racer whose complete reply is already in hand is a loser
/// whose bytes were moved for nothing. Racers still in flight are aborted when the
/// caller's guard drops, and what they had not finished receiving is unknowable here.
fn count_losing_replies(in_flight: &mut FuturesUnordered<tokio::task::JoinHandle<RacerResult>>) {
    use futures::FutureExt;
    while let Some(Some(joined)) = in_flight.next().now_or_never() {
        if let Ok((_, _, Ok(BlobFetchReply::Bytes(bytes)))) = joined {
            crate::metrics::add_blob_fetch_wasted_bytes("losing_racer", bytes.len());
        }
    }
}

type RacerResult = (String, std::time::Instant, Result<BlobFetchReply, String>);

async fn race_batches(
    cid: &ByteCid,
    connected: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    parallelism: usize,
    budget: &FetchBudget,
) -> FetchOutcome {
    let mut iter = connected.into_iter();
    // bounded-work: at most `budget.retries` batches of `parallelism` racers, each
    // racer bounded by `budget.stall_deadline`, the whole under `total_deadline`.
    for _ in 0..budget.retries.max(1) {
        let batch: Vec<String> = iter.by_ref().take(parallelism).collect();
        if batch.is_empty() {
            return FetchOutcome::Miss;
        }
        let mut in_flight: FuturesUnordered<tokio::task::JoinHandle<RacerResult>> =
            FuturesUnordered::new();
        let mut aborts = AbortOnDrop(Vec::with_capacity(batch.len()));
        for peer_label in batch {
            let Ok(peer_id) = peer_label.parse::<libp2p::PeerId>() else {
                continue;
            };
            let cmd_tx = cmd_tx.clone();
            let cid = *cid;
            let stall = budget.stall_deadline;
            let started = std::time::Instant::now();
            let handle = tokio::spawn(async move {
                let (reply_tx, reply_rx) = oneshot::channel();
                if cmd_tx
                    .send(crate::p2p::P2PCommand::FetchBlob {
                        peer_id,
                        cid,
                        reply: reply_tx,
                    })
                    .await
                    .is_err()
                {
                    return (peer_label, started, Err("swarm channel closed".to_string()));
                }
                match tokio::time::timeout(stall, reply_rx).await {
                    Ok(Ok(Ok(reply))) => (peer_label, started, Ok(reply)),
                    Ok(Ok(Err(e))) => (peer_label, started, Err(e)),
                    Ok(Err(_)) => (peer_label, started, Err("oneshot dropped".to_string())),
                    Err(_) => (peer_label, started, Err("stalled".to_string())),
                }
            });
            aborts.0.push(handle.abort_handle());
            in_flight.push(handle);
        }

        // Drain in completion order; the first verified reply wins and `aborts`
        // cancels the rest when this scope ends.
        while let Some(joined) = in_flight.next().await {
            let Ok((peer, started, reply)) = joined else {
                continue;
            };
            let ok = reply.is_ok();
            crate::p2p::transport_paths::global().record(
                &peer,
                crate::p2p::transport_paths::Transport::Libp2p,
                crate::p2p::transport_paths::OpClass::Small,
                ok.then(|| started.elapsed()),
                ok,
                false,
            );
            let Ok(reply) = reply else { continue };
            match reply {
                BlobFetchReply::Bytes(bytes) => {
                    if bytes.len() as u64 > budget.max_bytes {
                        tracing::warn!(
                            target: "elohim_storage::blob_fetch",
                            cid = %cid, source_peer = %peer, size = bytes.len(),
                            "reply exceeds the fetch budget; refused"
                        );
                        crate::metrics::add_blob_fetch_wasted_bytes("oversized_reply", bytes.len());
                        continue;
                    }
                    if verify_fetched_bytes(&bytes, cid) {
                        tracing::debug!(
                            target: "recovery::transport",
                            blob_cid = %cid,
                            source_peer = %peer,
                            transport = "libp2p",
                            "share-blob received"
                        );
                        count_losing_replies(&mut in_flight);
                        return FetchOutcome::Hit {
                            bytes,
                            source_peer: peer,
                        };
                    }
                    // Bytes that do not hash to the CID: never trusted.
                    crate::metrics::add_blob_fetch_wasted_bytes("hash_failure", bytes.len());
                }
                BlobFetchReply::Manifest(manifest) => {
                    if manifest.blob_cid != *cid {
                        tracing::warn!(
                            target: "elohim_storage::blob_swarm",
                            requested = %cid,
                            manifest_cid = %manifest.blob_cid,
                            source_peer = %peer,
                            "manifest names another payload — rejecting, not substituting"
                        );
                        continue;
                    }
                    if !budget.admits_manifest(&manifest) || manifest.validate().is_err() {
                        tracing::warn!(
                            target: "elohim_storage::blob_swarm",
                            cid = %cid,
                            source_peer = %peer,
                            byte_len = manifest.byte_len,
                            "manifest outside the fetch budget or inconsistent — refused before allocation"
                        );
                        continue;
                    }
                    crate::metrics::inc_blob_swarm_manifest_received();
                    return FetchOutcome::Manifest {
                        manifest,
                        source_peer: peer,
                    };
                }
            }
        }
        // All in batch failed (stall, error, or a refused reply); try the next batch.
    }
    FetchOutcome::Miss
}

/// Persist verified bytes to the local blob store, record fetch-success in
/// `peer_blob_inventory`, and emit a `serve-blob` REA event.
///
/// **Ordering contract — persist filesystem first, then SQL.**
/// A crash between filesystem and SQL leaves a benign orphan blob on disk;
/// the parity sweep (T18, `/api/v1/diagnostics/inventory-parity`) reconciles
/// by triggering a re-fetch on the next gossip round. The opposite order
/// (SQL first) would create the worse failure mode: an inventory row
/// claiming we host a blob whose bytes we never managed to write.
///
/// **Atomicity contract — the two SQL writes are wrapped in a single
/// transaction.** `record_fetch_success` and the `serve-blob` event insert
/// run inside one `conn.transaction(|txn| ...)` block so a crash or
/// constraint-violation between them leaves no half-state. (SQLite via
/// Diesel emits a SAVEPOINT for the inner `record_fetch_success` call,
/// which already opens its own transaction; nested transactions on
/// SqliteConnection are supported and roll back together on outer error.)
/// The T18 parity sweep can then assume: if a `peer_blob_inventory` row
/// with `source='fetch-success'` exists for `(self, hash)`, the matching
/// `serve-blob` REA event also exists.
///
/// Sequence inside this function:
/// 1. `blob_store.store(bytes).await` — write bytes to local filesystem.
/// 2. `conn.transaction(|txn| { record_fetch_success(txn, ...); insert serve-blob event })`.
pub async fn finalize_fetch_success(
    conn: &mut SqliteConnection,
    cid: &ByteCid,
    source_peer: &str,
    bytes: &[u8],
    self_cid: &str,
    blob_store: &BlobStore,
) -> Result<(), StorageError> {
    // Under the arrival gate from the store to the arrival record, so the
    // retention pass cannot let these bytes go in between.
    let _arriving = crate::db::blob_arrivals::arrival_gate().read().await;

    // Step 1: persist to filesystem first. On error, return without writing
    // any SQL — leaving inventory clean.
    let stored = blob_store.store(bytes).await?;
    if stored.cid != *cid {
        return Err(StorageError::HashMismatch {
            expected: cid.to_string(),
            actual: stored.cid.to_string(),
        });
    }
    let blob_cid = cid.to_string();

    let now_iso = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let event_id = uuid::Uuid::new_v4().to_string();
    // TODO(stewardship-precision): `economic_events.resource_quantity_value`
    // is f32 in the schema, which loses precision for blobs >16 MB.
    // Migrating to f64 (or fixed-point) requires a Diesel migration; tracked
    // separately from this T20 review pass.
    let qty_value = bytes.len() as f32;

    // Step 2 + 3 (atomic): record fetch-success AND emit the serve-blob REA
    // event in a single transaction so a crash between them cannot leave a
    // peer_blob_inventory row without its matching event (or vice versa).
    //
    // REA event direction: a remote peer SERVED the bytes to us, so the remote
    // peer is the provider (source_peer) and we are the receiver (self_cid).
    // Note: T16's custody-blob events use the opposite convention (custodian
    // is provider, content steward is receiver). Cross-event analytics joining
    // on these fields must be aware of the per-action semantics.
    conn.transaction::<(), StorageError, _>(|txn| {
        record_fetch_success(txn, source_peer, &blob_cid, &now_iso)?;
        // How these bytes got here: this peer fetched them.
        crate::db::blob_arrivals::record_arrival(
            txn,
            &blob_cid,
            crate::db::blob_arrivals::ArrivalVia::SelfFetch,
            None,
            None,
        )?;

        let new_event = NewEconomicEvent {
            id: &event_id,
            // "lamad" = the DEFAULT read scope of /api/v1/economic-events
            // (X-App-Id header default) and the scope custody-blob rows (the
            // sibling REA plane) live under. This event was born under
            // "elohim" — a leak of the SYNC-plane DocStore namespace
            // (PROJECTION_NAMESPACE) into REA scoping — which made every
            // serve-blob event structurally invisible to the scoped read
            // surface (genesis delivery.serve-blob-events read 0 across
            // #1224-#1230 while the atomic pair worked; the dormant-scope
            // class: written under one h_app_id, read under another).
            h_app_id: "lamad",
            action: "serve-blob",
            provider: source_peer,
            receiver: self_cid,
            resource_conforms_to: None,
            resource_inventoried_as: Some(&blob_cid),
            resource_classified_as_json: None,
            resource_quantity_value: Some(qty_value),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: &now_iso,
            has_duration: None,
            input_of: None,
            // The matching custody-blob commitment hash is not always known at this layer.
            output_of: None,
            lamad_event_type: None,
            content_id: None,
            contributor_presence_id: None,
            path_id: None,
            triggered_by: None,
            state: "completed",
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
            at_location: None,
            verified_at: None,
            scope_collab_cid: None,
            bounded_by: None,
            substrate_signal: None,
        };
        diesel::insert_into(economic_events::table)
            .values(&new_event)
            .execute(txn)
            .map_err(|e| StorageError::Database(format!("insert serve-blob event: {e}")))?;
        Ok(())
    })
}

/// Finalize a **proactive quilt-draw** (replication pull): a remote peer served
/// us the blob bytes in response to our `ShardRequest::Get`, so — exactly like
/// the on-demand race-fetch — this is a serve-transfer that belongs on the REA
/// delivery surface. p2p-design-gate (2026-06-18): book a `serve-blob` event
/// (provider = the serving peer, receiver = `self_cid`) via the SAME atomic
/// pair as `finalize_fetch_success`, NOT a distinct `blob-hosted` action — the
/// hosting/observation fact is the `peer_blob_inventory` row that the same
/// finalize writes; `blob-hosted` would duplicate the propagation surface and
/// has zero consumers. The verify script (`substrate-verify.sh` delivery) keeps
/// its `action=serve-blob` query unchanged, so emitter and verify speak one
/// vocabulary and cannot re-diverge.
///
/// Unlike the race-fetch path (which verifies bytes BEFORE `finalize_fetch_success`),
/// the quilt-draw arm has NOT verified the reply, so verify here first. Without
/// it a mismatched reply would write an inventory row + `serve-blob` event
/// claiming we host `cid` while `BlobStore::store` filed the bytes under
/// their real (different) CID. Returns `Ok(false)` when the bytes fail
/// verification (discarded, no writes); `Ok(true)` when stocked + booked.
pub async fn finalize_quilt_draw(
    conn: &mut SqliteConnection,
    cid: &ByteCid,
    source_peer: &str,
    bytes: &[u8],
    self_cid: &str,
    blob_store: &BlobStore,
) -> Result<bool, StorageError> {
    // Verify FIRST — the quilt-draw arm has not validated the reply (unlike the
    // race-fetch path). A mismatched reply must be discarded, never booked:
    // BlobStore::store files bytes under their REAL hash, so an inventory row +
    // serve-blob event keyed on `blob_hash` would otherwise claim we host
    // content we never stored.
    if !verify_fetched_bytes(bytes, cid) {
        return Ok(false);
    }
    // Verified: same atomic pair as an on-demand serve (store + inventory row +
    // serve-blob delivery event). Reuse — do not fork — to keep the T18 parity
    // contract (inventory row ⟺ serve-blob event) coherent across both paths.
    finalize_fetch_success(conn, cid, source_peer, bytes, self_cid, blob_store).await?;
    Ok(true)
}

/// Whether fetched `bytes` are the payload `cid` names: the BLAKE3 the byte CID
/// declares, recomputed over the bytes (C5 verify before use).
pub fn verify_fetched_bytes(bytes: &[u8], cid: &ByteCid) -> bool {
    compute_bytes_cid(bytes) == *cid
}

/// Bytes-per-second a peer transfer is assumed to sustain when sizing a
/// per-peer deadline. Deliberately pessimistic: the point is to stop timing out
/// a transfer that is *working*, not to predict throughput.
const ASSUMED_TRANSFER_BYTES_PER_SEC: u64 = 1024 * 1024;

/// **Pure.** A per-peer deadline that scales with the object being moved.
///
/// `Config::fetch_blob_timeout_seconds` defaults to **5 s**, which is sized for
/// the small-object blob heals it was written for. The first live run of this
/// source asked peers for a **10 MB** `.happ` under that same 5 s (measured
/// 2026-09-04, station 7): a transfer that is merely *large* is
/// indistinguishable from a peer that is *dead*, and the sweep records a miss
/// either way. Scaling the floor by declared size makes the deadline mean "this
/// peer has stopped answering" instead of "this object is big".
///
/// The configured timeout is the FLOOR, never a ceiling that shrinks — an
/// operator who raises it is raising it for every object — and the result is
/// capped so one artifact can never own a whole sweep.
pub fn size_aware_timeout(
    declared_bytes: u64,
    floor: std::time::Duration,
    cap: std::time::Duration,
) -> std::time::Duration {
    let scaled = std::time::Duration::from_secs(
        declared_bytes
            .div_ceil(ASSUMED_TRANSFER_BYTES_PER_SEC)
            .max(1),
    );
    scaled.max(floor).min(cap.max(floor))
}

/// Extract race-fetch parameters from the runtime `Config`.
pub fn fetch_params_from_config(config: &Config) -> (usize, Duration) {
    (
        config.fetch_blob_parallelism,
        Duration::from_secs(config.fetch_blob_timeout_seconds),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharding::{ShardConfig, ShardEncoder};

    /// A libp2p `PeerId` that round-trips through the `String` parse `race_fetch`
    /// performs on each candidate.
    fn test_peer_id() -> libp2p::PeerId {
        libp2p::PeerId::from(libp2p::identity::Keypair::generate_ed25519().public())
    }

    fn manifest_for(data: &[u8]) -> ShardManifest {
        ShardEncoder::new(ShardConfig::default())
            .create_manifest(data, "application/octet-stream", "commons")
            .expect("manifest")
    }

    /// The fetched bytes are checked under the multihash the CID declares (raw +
    /// BLAKE3-256): the CID's digest IS blake3(bytes), and one flipped byte fails.
    #[test]
    fn fetched_bytes_verified_under_declared_multihash() {
        let bytes = b"hello world".to_vec();
        let cid = compute_bytes_cid(&bytes);
        assert_eq!(cid.digest(), blake3::hash(&bytes).as_bytes());
        assert!(verify_fetched_bytes(&bytes, &cid));

        let mut tampered = bytes.clone();
        tampered[0] ^= 0x01;
        assert!(!verify_fetched_bytes(&tampered, &cid));
        assert!(!verify_fetched_bytes(&bytes, &compute_bytes_cid(b"other")));
    }

    /// The first verified reply wins and every other racer in the batch is
    /// cancelled: the slow holder's reply channel closes because its racer task
    /// was aborted, not because it answered.
    #[tokio::test]
    async fn losing_racers_are_cancelled() {
        let bytes = b"raced payload".to_vec();
        let cid = compute_bytes_cid(&bytes);
        let (fast, slow) = (test_peer_id(), test_peer_id());
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);

        let served = bytes.clone();
        let responder = tokio::spawn(async move {
            let mut held = Vec::new();
            for _ in 0..2 {
                if let Some(crate::p2p::P2PCommand::FetchBlob { peer_id, reply, .. }) =
                    cmd_rx.recv().await
                {
                    if peer_id == fast {
                        let _ = reply.send(Ok(BlobFetchReply::Bytes(served.clone())));
                    } else {
                        held.push(reply);
                    }
                }
            }
            // The losing racer's receiver must be dropped (its task aborted).
            let mut slow_reply = held.pop().expect("the slow holder was asked");
            tokio::time::timeout(Duration::from_secs(2), slow_reply.closed())
                .await
                .is_ok()
        });

        let outcome = race_fetch(
            &cid,
            vec![slow.to_string(), fast.to_string()],
            &cmd_tx,
            |_| true,
            2,
            Duration::from_secs(30),
        )
        .await;
        match outcome {
            FetchOutcome::Hit {
                bytes: got,
                source_peer,
            } => {
                assert_eq!(got, bytes);
                assert_eq!(source_peer, fast.to_string());
            }
            other => panic!("expected Hit, got {other:?}"),
        }
        assert!(
            responder.await.expect("responder"),
            "the losing racer was not cancelled"
        );
    }

    /// A holder that never finishes is abandoned when the total deadline passes,
    /// even though its per-peer stall deadline is far away; its racer is cancelled.
    #[tokio::test]
    async fn trickling_holder_abandoned_on_total_deadline() {
        let cid = compute_bytes_cid(b"never arrives");
        let peer = test_peer_id();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            let Some(crate::p2p::P2PCommand::FetchBlob { mut reply, .. }) = cmd_rx.recv().await
            else {
                return false;
            };
            tokio::time::timeout(Duration::from_secs(2), reply.closed())
                .await
                .is_ok()
        });

        let budget = FetchBudget {
            max_bytes: FetchBudget::DEFAULT_MAX_BYTES,
            stall_deadline: Duration::from_secs(60),
            total_deadline: Duration::from_millis(200),
            retries: 4,
        };
        let started = std::time::Instant::now();
        let outcome =
            race_fetch_budgeted(&cid, vec![peer.to_string()], &cmd_tx, |_| true, 1, &budget).await;
        assert!(
            matches!(outcome, FetchOutcome::BudgetExhausted),
            "expected BudgetExhausted, got {outcome:?}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(
            responder.await.expect("responder"),
            "the trickling racer was not cancelled"
        );
    }

    /// A manifest declaring more bytes than the budget admits is refused before
    /// any reconstruction buffer exists: the race reports a miss, not a manifest.
    #[tokio::test]
    async fn oversized_manifest_refused_before_allocation() {
        let data = vec![7u8; 4096];
        let mut manifest = manifest_for(&data);
        let cid = manifest.blob_cid;
        manifest.byte_len = 1 << 40;
        let budget = FetchBudget::for_stall(Duration::from_secs(2));
        assert!(!budget.admits_manifest(&manifest));

        let peer = test_peer_id();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            if let Some(crate::p2p::P2PCommand::FetchBlob { reply, .. }) = cmd_rx.recv().await {
                let _ = reply.send(Ok(BlobFetchReply::Manifest(Box::new(manifest))));
            }
        });
        let outcome =
            race_fetch_budgeted(&cid, vec![peer.to_string()], &cmd_tx, |_| true, 1, &budget).await;
        responder.await.expect("responder");
        assert!(
            matches!(outcome, FetchOutcome::Miss),
            "an oversized manifest must be refused; got {outcome:?}"
        );
    }

    /// Bytes that do not hash to the requested CID are never a hit.
    #[tokio::test]
    async fn race_fetch_refuses_bytes_that_do_not_hash_to_the_cid() {
        let cid = compute_bytes_cid(b"the payload we asked for");
        let peer = test_peer_id();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            if let Some(crate::p2p::P2PCommand::FetchBlob { reply, .. }) = cmd_rx.recv().await {
                let _ = reply.send(Ok(BlobFetchReply::Bytes(b"something else".to_vec())));
            }
        });
        let outcome = race_fetch(
            &cid,
            vec![peer.to_string()],
            &cmd_tx,
            |_| true,
            1,
            Duration::from_secs(2),
        )
        .await;
        responder.await.expect("responder");
        assert!(matches!(outcome, FetchOutcome::Miss), "got {outcome:?}");
    }

    /// `finalize_fetch_success` persists the bytes FIRST, then writes the
    /// `peer_blob_inventory` row (source = `fetch-success`), then exactly one
    /// `serve-blob` REA event visible under the default (lamad) read scope.
    #[tokio::test]
    async fn finalize_persists_bytes_then_writes_sql() {
        use crate::blob_store::Exists;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"finalize-test-payload".to_vec();
        let cid = compute_bytes_cid(&bytes);
        let cid_text = cid.to_string();

        finalize_fetch_success(&mut conn, &cid, "peer_X", &bytes, "self_cid_Y", &blob_store)
            .await
            .expect("finalize ok");

        assert_eq!(blob_store.exists(&cid).await, Exists::Verified);

        let rows = lookup_hosts(&mut conn, &cid_text, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            rows.iter()
                .any(|r| r.peer_id == "peer_X" && r.source == "fetch-success"),
            "peer_blob_inventory row written with source='fetch-success'; got rows: {rows:?}"
        );

        use crate::db::diesel_schema::economic_events::dsl as ee;
        use diesel::prelude::*;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&cid_text))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1, "exactly one serve-blob event");

        let visible = crate::db::economic_events::list_economic_events(
            &mut conn,
            &crate::db::AppContext::default_lamad(),
            &crate::db::economic_events::EconomicEventQuery {
                action: Some("serve-blob".to_string()),
                limit: 50,
                ..Default::default()
            },
        )
        .expect("scoped list ok");
        assert!(visible
            .iter()
            .any(|e| e.resource_inventoried_as.as_deref() == Some(cid_text.as_str())));
    }

    /// The two SQL writes inside `finalize_fetch_success` share one transaction:
    /// a failed `serve-blob` insert rolls back the inventory row.
    #[tokio::test]
    async fn finalize_rolls_back_inventory_on_event_insert_failure() {
        use crate::blob_store::Exists;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::RunQueryDsl;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"rollback-test-payload".to_vec();
        let cid = compute_bytes_cid(&bytes);

        diesel::sql_query("DROP TABLE economic_events;")
            .execute(&mut conn)
            .expect("drop economic_events");

        let result =
            finalize_fetch_success(&mut conn, &cid, "peer_R", &bytes, "self_cid_R", &blob_store)
                .await;
        assert!(result.is_err());
        assert_eq!(blob_store.exists(&cid).await, Exists::Verified);

        let rows = lookup_hosts(&mut conn, &cid.to_string(), "2020-01-01T00:00:00Z").unwrap();
        assert!(
            !rows.iter().any(|r| r.peer_id == "peer_R"),
            "got rows: {rows:?}"
        );
    }

    /// A proactive quilt-draw books exactly one `serve-blob` event (provider = the
    /// serving peer, receiver = self) and writes the inventory row.
    #[tokio::test]
    async fn quilt_draw_books_serve_blob_event() {
        use crate::blob_store::Exists;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::prelude::*;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"quilt-draw-payload".to_vec();
        let cid = compute_bytes_cid(&bytes);
        let cid_text = cid.to_string();

        let booked =
            finalize_quilt_draw(&mut conn, &cid, "peer_Q", &bytes, "self_cid_Q", &blob_store)
                .await
                .expect("finalize_quilt_draw ok");
        assert!(booked);
        assert_eq!(blob_store.exists(&cid).await, Exists::Verified);

        let inv = lookup_hosts(&mut conn, &cid_text, "2020-01-01T00:00:00Z").unwrap();
        assert!(inv.iter().any(|r| r.peer_id == "peer_Q"), "got: {inv:?}");

        use crate::db::diesel_schema::economic_events::dsl as ee;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&cid_text))
            .filter(ee::provider.eq("peer_Q"))
            .filter(ee::receiver.eq("self_cid_Q"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1);
    }

    /// A quilt-draw whose bytes do not hash to the claimed CID is discarded:
    /// nothing stored under the claim, no inventory row, no `serve-blob` event.
    #[tokio::test]
    async fn quilt_draw_discards_mismatched_bytes() {
        use crate::blob_store::Exists;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::prelude::*;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let claimed = compute_bytes_cid(b"the-content-we-asked-for");
        let claimed_text = claimed.to_string();
        let bytes = b"a-different-payload-entirely".to_vec();

        let booked = finalize_quilt_draw(
            &mut conn,
            &claimed,
            "peer_M",
            &bytes,
            "self_cid_M",
            &blob_store,
        )
        .await
        .expect("finalize_quilt_draw ok");
        assert!(!booked);
        assert_ne!(blob_store.exists(&claimed).await, Exists::Verified);
        let inv = lookup_hosts(&mut conn, &claimed_text, "2020-01-01T00:00:00Z").unwrap();
        assert!(!inv.iter().any(|r| r.peer_id == "peer_M"), "got: {inv:?}");
        use crate::db::diesel_schema::economic_events::dsl as ee;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&claimed_text))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 0);
    }

    /// A manifest reply naming the requested CID (and within budget) ends the race
    /// as `Manifest`, so the caller can pivot to the shard swarm.
    #[tokio::test]
    async fn race_fetch_accepts_matching_manifest_reply() {
        let peer = test_peer_id();
        let manifest = manifest_for(b"a payload a peer only holds as a manifest");
        let cid = manifest.blob_cid;
        let for_responder = manifest.clone();

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            if let Some(crate::p2p::P2PCommand::FetchBlob { reply, .. }) = cmd_rx.recv().await {
                let _ = reply.send(Ok(BlobFetchReply::Manifest(Box::new(for_responder))));
            }
        });

        let outcome = race_fetch(
            &cid,
            vec![peer.to_string()],
            &cmd_tx,
            |_| true,
            1,
            Duration::from_secs(2),
        )
        .await;
        responder.await.expect("responder task");

        match outcome {
            FetchOutcome::Manifest {
                manifest: m,
                source_peer,
            } => {
                assert_eq!(m.blob_cid, cid);
                assert_eq!(source_peer, peer.to_string());
            }
            other => panic!("expected Manifest outcome, got {other:?}"),
        }
    }

    /// A manifest naming a DIFFERENT payload is rejected, not substituted.
    #[tokio::test]
    async fn race_fetch_rejects_manifest_naming_another_payload() {
        let peer = test_peer_id();
        let requested = compute_bytes_cid(b"requested payload");
        let wrong_manifest = manifest_for(b"some other payload");

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            if let Some(crate::p2p::P2PCommand::FetchBlob { reply, .. }) = cmd_rx.recv().await {
                let _ = reply.send(Ok(BlobFetchReply::Manifest(Box::new(wrong_manifest))));
            }
        });

        let outcome = race_fetch(
            &requested,
            vec![peer.to_string()],
            &cmd_tx,
            |_| true,
            1,
            Duration::from_secs(2),
        )
        .await;
        responder.await.expect("responder task");
        assert!(matches!(outcome, FetchOutcome::Miss), "got {outcome:?}");
    }

    #[tokio::test]
    async fn race_fetch_with_no_connected_candidate_reports_no_candidates() {
        let (cmd_tx, _cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(1);
        let outcome = race_fetch(
            &compute_bytes_cid(b"x"),
            vec![test_peer_id().to_string()],
            &cmd_tx,
            |_| false,
            1,
            Duration::from_secs(1),
        )
        .await;
        assert!(matches!(outcome, FetchOutcome::NoCandidates));
    }

    /// A racer that delivers a truncated body beside the winner moved bytes for nothing;
    /// the meter counts them whichever way the race drains them (a loser, or a body that
    /// fails the CID).
    #[tokio::test]
    async fn wasted_bytes_counts_a_losing_racers_partial_body() {
        let bytes: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();
        let cid = compute_bytes_cid(&bytes);
        let partial = bytes[..bytes.len() / 2].to_vec();
        let (fast, slow) = (test_peer_id(), test_peer_id());
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);

        let (served, truncated) = (bytes.clone(), partial.clone());
        let responder = tokio::spawn(async move {
            let mut replies = Vec::new();
            for _ in 0..2 {
                if let Some(crate::p2p::P2PCommand::FetchBlob { peer_id, reply, .. }) =
                    cmd_rx.recv().await
                {
                    replies.push((peer_id, reply));
                }
            }
            // The winner's reply first, then the loser's partial body.
            replies.sort_by_key(|(peer, _)| *peer != fast);
            for (peer, reply) in replies {
                let body = if peer == fast {
                    served.clone()
                } else {
                    truncated.clone()
                };
                let _ = reply.send(Ok(BlobFetchReply::Bytes(body)));
            }
        });

        let before = crate::metrics::blob_fetch_wasted_bytes_total();
        let outcome = race_fetch(
            &cid,
            vec![slow.to_string(), fast.to_string()],
            &cmd_tx,
            |_| true,
            2,
            Duration::from_secs(30),
        )
        .await;
        responder.await.expect("responder");
        assert!(matches!(outcome, FetchOutcome::Hit { .. }), "{outcome:?}");
        let counted = crate::metrics::blob_fetch_wasted_bytes_total() - before;
        assert!(
            counted >= partial.len() as u64,
            "the partial body's {} bytes are counted, got {counted}",
            partial.len()
        );
    }

    /// A holder that never answers is dropped at the per-peer stall deadline, long
    /// before the total deadline; the race reports a miss, not an exhausted budget.
    #[tokio::test]
    async fn stall_deadline_abandons_a_silent_holder_before_total_deadline() {
        let cid = compute_bytes_cid(b"a holder that never answers");
        let peer = test_peer_id();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            let Some(crate::p2p::P2PCommand::FetchBlob { mut reply, .. }) = cmd_rx.recv().await
            else {
                return false;
            };
            // Never answers; the racer must hang up on its own stall deadline.
            tokio::time::timeout(Duration::from_secs(5), reply.closed())
                .await
                .is_ok()
        });

        let budget = FetchBudget {
            max_bytes: FetchBudget::DEFAULT_MAX_BYTES,
            stall_deadline: Duration::from_millis(150),
            total_deadline: Duration::from_secs(60),
            retries: 1,
        };
        let started = std::time::Instant::now();
        let outcome =
            race_fetch_budgeted(&cid, vec![peer.to_string()], &cmd_tx, |_| true, 1, &budget).await;
        let elapsed = started.elapsed();
        assert!(
            matches!(outcome, FetchOutcome::Miss),
            "a stalled holder is a miss, not an exhausted budget; got {outcome:?}"
        );
        assert!(
            elapsed >= Duration::from_millis(100) && elapsed < Duration::from_secs(5),
            "abandoned at the stall deadline, not the total: {elapsed:?}"
        );
        assert!(
            responder.await.expect("responder"),
            "the racer hung up on the silent holder"
        );
    }
}
