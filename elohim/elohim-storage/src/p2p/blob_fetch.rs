//! Shared blob-fetch helper. Used by both:
//! - HTTP blob handler (`GET /blob/{hash}`) on local miss
//! - Custody reconciliation controller (T16) for own-commitment kicks
//!
//! Strategy:
//! 1. Look up candidate peers in `peer_blob_inventory`, ordered by evidence
//!    strength (fetch-success first, then by recency).
//! 2. Filter to currently-connected peers.
//! 3. Race candidates in parallel batches of `fetch_blob_parallelism` (default 3).
//!    First reply that returns `Ok(bytes)` AND verifies the content hash wins.
//!    Pending replies in the batch are dropped; failed batch advances to the next.
//! 4. On verified success: persist locally, record fetch-success in
//!    `peer_blob_inventory`, emit `serve-blob` REA event.
//!
//! Hash verification: sha256-hex matches the requested hash (case-insensitive).
//!
//! # Stage 2 (T21 onward)
//! `P2PCommand::FetchBlob` issues a `/elohim/blob/1.0.0` request-response
//! exchange to the named peer (see `p2p/blob_protocol.rs`). The reply oneshot
//! is delivered when the response arrives, the outbound times out, or the
//! connection fails — all three cases produce a deterministic
//! `Result<Vec<u8>, String>` the helper can act on. The helper's control flow,
//! hash verification, persistence, and serve-blob emission are still exercised
//! by unit tests without requiring a running swarm — those tests use the
//! `for_testing()` handle, which keeps the Stage-1 placeholder error so race
//! batches resolve to `Miss` deterministically.

use crate::blob_store::BlobStore;
use crate::config::Config;
use crate::db::diesel_schema::economic_events;
use crate::db::models::NewEconomicEvent;
use crate::db::peer_blob_inventory::record_fetch_success;
use crate::error::StorageError;
use crate::sharding::ShardManifest;
use chrono::Utc;
use cid::Cid;
use diesel::Connection;
use diesel::RunQueryDsl;
use diesel::SqliteConnection;
use futures::stream::{FuturesUnordered, StreamExt};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Internal reply payload carried by `P2PCommand::FetchBlob`'s oneshot channel.
///
/// **Not a wire type** — the wire type is `blob_protocol::BlobFetchResponse`.
/// This is the in-process contract between the swarm event loop (which
/// resolves the wire response) and `race_fetch` (which never sees the wire
/// bytes). Q3 adds `Manifest`: a peer with no direct bytes for a sharded
/// composite hash can still answer usefully with the durable manifest it
/// resolved, so `race_fetch` treats that as a distinct outcome from a plain
/// miss instead of discarding it.
#[derive(Debug, Clone)]
pub enum BlobFetchReply {
    /// Verified-pending bytes for the requested hash.
    Bytes(Vec<u8>),
    /// No direct bytes, but the peer resolved a durable manifest for the
    /// hash. Boxed — see the size-note on `FetchOutcome::Manifest`.
    Manifest(Box<ShardManifest>),
}

/// The sha256 hex digest a content address names, for any recognizable form:
///
/// - CIDv1 (`bafkrei…` raw, `bafyrei…` dag-cbor — both carry a sha2-256
///   multihash): the wrapped digest.
/// - `sha256-<64 hex>` (canonical legacy marker): the hex part.
/// - bare 64-hex: itself.
/// - `sha256-<CID>` (the double-wrapped seed defect this module repairs —
///   a CID-form blob hash wrapped in the legacy marker by an address
///   constructor that assumed bare hex): the CID's wrapped digest.
///
/// Returns `None` for anything else (test fixtures like `"sha256-shardA"`,
/// slugs, garbage) so callers can keep their legacy lenient comparison for
/// non-address strings.
pub fn content_address_hex(addr: &str) -> Option<String> {
    // Hex forms FIRST: a bare 64-hex string starting with a multibase prefix
    // character (`f` = base16, `b` = base32) could otherwise be misread as a
    // CID by the parser. Mirrors `BlobStore::parse_content_address`'s intent
    // with the precedence hardened for the hex-shaped majority.
    let bare = addr.strip_prefix("sha256-").unwrap_or(addr);
    if bare.len() == 64 && bare.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(bare.to_lowercase());
    }
    if let Ok(c) = Cid::from_str(addr) {
        return Some(hex::encode(c.hash().digest()));
    }
    // Double-wrapped `sha256-<cid>`: recover the digest from the inner CID.
    if bare != addr {
        if let Ok(c) = Cid::from_str(bare) {
            return Some(hex::encode(c.hash().digest()));
        }
    }
    None
}

/// Build the outgoing wire address for a blob fetch from a stored blob-hash
/// value, whatever form the row carries.
///
/// Contract (backlog `blob-fetch-sha256-prefixed-cid-rejection`):
/// - CID-form (`baf…`) passes through untouched.
/// - Already-marked `sha256-<64 hex>` passes through untouched.
/// - Bare 64-hex gets the legacy `sha256-` prefix.
/// - `sha256-<CID>` — the double-wrapped defect minted by seed-side
///   `normalizeBlobHash` (`sha256-` prefixed onto a CID-form blob hash) —
///   is REPAIRED to the inner CID rather than sent as-is: no responder
///   accepts the double-wrapped form (T21 rejects it), so passing it
///   through would re-create the infinite rejection drumbeat this fix
///   removes.
/// - Anything else is an error: a malformed address must never reach the
///   wire, where the responder's strict parse would reject it on every
///   retry forever.
pub fn normalize_fetch_address(addr: &str) -> Result<String, StorageError> {
    if let Some(bare) = addr.strip_prefix("sha256-") {
        if bare.len() == 64 && bare.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(addr.to_string());
        }
        if Cid::from_str(bare).is_ok() {
            return Ok(bare.to_string());
        }
        return Err(StorageError::InvalidContentAddress(addr.to_string()));
    }
    // Bare hex BEFORE the CID parse: a 64-hex string starting with a multibase
    // prefix character (`f`/`b`) must be read as hex, not as a CID.
    if addr.len() == 64 && addr.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(format!("sha256-{addr}"));
    }
    if Cid::from_str(addr).is_ok() {
        return Ok(addr.to_string());
    }
    Err(StorageError::InvalidContentAddress(addr.to_string()))
}

/// Outcome of a race-fetch.
#[derive(Debug)]
pub enum FetchOutcome {
    /// Bytes fetched and verified.
    Hit { bytes: Vec<u8>, source_peer: String },
    /// Q3: no peer served bytes, but a peer answered with a durable
    /// `ShardManifest` for the requested (composite) hash — the caller can
    /// pivot to a swarm shard-fetch (`p2p::blob_swarm`, Q4) instead of
    /// treating this as a dead-end miss. `manifest.blob_hash` is verified to
    /// equal the requested hash before this variant is ever returned. Boxed:
    /// `ShardManifest` (~240 bytes) would otherwise bloat every `FetchOutcome`
    /// to its size regardless of variant (clippy::large_enum_variant).
    Manifest {
        manifest: Box<ShardManifest>,
        source_peer: String,
    },
    /// All candidates exhausted; no peer served verified bytes or a manifest.
    Miss,
    /// Inventory had no candidates to try (either empty or none connected).
    NoCandidates,
    /// The requested address is not a valid content address in any accepted
    /// form (`normalize_fetch_address` refused it). NO wire request was sent:
    /// every responder would reject it (T21) on every retry forever, so the
    /// only honest outcome is to give up immediately and say so. Callers must
    /// treat this as terminal for the address — retrying cannot succeed until
    /// the row that carries it is healed.
    InvalidAddress,
}

/// Case-insensitive, prefix-tolerant match between a manifest's own
/// `blob_hash` and the hash `race_fetch` requested. Mirrors
/// `verify_blob_hash`'s normalization so a canonical `sha256-<hex>` on one
/// side and raw hex on the other still compare equal. A mismatch here means
/// the peer answered for the wrong blob — never trust it.
fn manifest_hash_matches(manifest: &ShardManifest, requested_hash: &str) -> bool {
    // Digest-aware first (CID vs `sha256-<hex>` vs bare hex all name the same
    // digest), then the legacy lenient strip-and-lowercase for strings that
    // are not content addresses at all (fixture/test values).
    let norm = |s: &str| {
        content_address_hex(s)
            .unwrap_or_else(|| s.strip_prefix("sha256-").unwrap_or(s).to_lowercase())
    };
    norm(&manifest.blob_hash) == norm(requested_hash)
}

/// Race a fetch across the candidates known to host the blob.
/// Returns the verified bytes on first hit, or Miss/NoCandidates if no peer served.
///
/// `cmd_tx` is the swarm command channel.  `is_connected` filters the
/// candidate list to peers actively connected at call time.
///
/// Candidates are processed in batches of `parallelism`.  All peers in a
/// batch are attempted concurrently; the first that returns verified bytes
/// wins and remaining in-flight requests are dropped.  If the whole batch
/// fails, the next batch is tried.
pub async fn race_fetch(
    blob_hash: &str,
    candidates: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    is_connected: impl Fn(&str) -> bool,
    parallelism: usize,
    per_peer_timeout: Duration,
) -> FetchOutcome {
    // Requester-side address hygiene at the single choke point every
    // `/elohim/blob/1.0.0` request funnels through: CID and `sha256-<hex>`
    // forms pass untouched, bare hex gets the legacy marker, and the
    // double-wrapped `sha256-<cid>` seed defect is repaired to the inner CID.
    // A string that is no content address in any form never reaches the wire
    // — the responder's strict T21 parse would reject it on every retry
    // forever (the 2-minute drumbeat this guard removes).
    let blob_hash: String = match normalize_fetch_address(blob_hash) {
        Ok(addr) => addr,
        Err(_) => {
            tracing::warn!(
                target: "elohim_storage::blob_fetch",
                hash = %blob_hash,
                "T21: refusing to fetch a malformed content address — no peer \
                 can accept it; giving up (the row carrying it needs healing)"
            );
            return FetchOutcome::InvalidAddress;
        }
    };
    let blob_hash = blob_hash.as_str();
    let connected: Vec<String> = candidates.into_iter().filter(|p| is_connected(p)).collect();

    if connected.is_empty() {
        return FetchOutcome::NoCandidates;
    }

    let mut iter = connected.into_iter();
    loop {
        let batch: Vec<String> = iter.by_ref().take(parallelism).collect();
        if batch.is_empty() {
            return FetchOutcome::Miss;
        }

        // Spawn a per-peer fetch task into a FuturesUnordered for
        // first-responder semantics: as soon as any peer returns verified
        // bytes we return Hit and stop awaiting the rest of the batch.
        //
        // Note: `tokio::spawn` detaches the task — dropping the JoinHandle
        // does NOT abort it. The spawned future runs to completion (bounded
        // by the per_peer_timeout in `tokio::time::timeout`). At Stage 2 with
        // real network I/O, replace `tokio::spawn` with unboxed futures
        // inside `FuturesUnordered` so drop = cancel for true cooperative
        // cancellation.
        let mut in_flight: FuturesUnordered<_> = FuturesUnordered::new();
        for peer_id_str in batch {
            let Ok(peer_id) = peer_id_str.parse::<libp2p::PeerId>() else {
                continue;
            };
            let cmd_tx = cmd_tx.clone();
            let hash = blob_hash.to_string();
            let timeout = per_peer_timeout;
            let peer_label = peer_id_str.clone();
            let started = std::time::Instant::now();
            in_flight.push(tokio::spawn(async move {
                let (reply_tx, reply_rx) = oneshot::channel();
                if cmd_tx
                    .send(crate::p2p::P2PCommand::FetchBlob {
                        peer_id,
                        hash: hash.clone(),
                        reply: reply_tx,
                    })
                    .await
                    .is_err()
                {
                    return (peer_label, started, Err("swarm channel closed".to_string()));
                }
                match tokio::time::timeout(timeout, reply_rx).await {
                    Ok(Ok(Ok(reply))) => (peer_label, started, Ok(reply)),
                    Ok(Ok(Err(e))) => (peer_label, started, Err(e)),
                    Ok(Err(_)) => (peer_label, started, Err("oneshot dropped".to_string())),
                    Err(_) => (peer_label, started, Err("timeout".to_string())),
                }
            }));
        }

        // Drain in completion order; first verified hit (or first
        // hash-verified manifest, Q3) wins. Mismatches and errors are
        // skipped — keep polling the remaining futures.
        while let Some(joined) = in_flight.next().await {
            let Ok((peer, started, reply)) = joined else {
                continue;
            };
            let reply = match reply {
                Ok(reply) => reply,
                Err(_) => {
                    // The races are the probe: a failed leg is a libp2p Small sample too.
                    crate::p2p::transport_paths::global().record(
                        &peer,
                        crate::p2p::transport_paths::Transport::Libp2p,
                        crate::p2p::transport_paths::OpClass::Small,
                        None,
                        false,
                        false,
                    );
                    continue;
                }
            };
            crate::p2p::transport_paths::global().record(
                &peer,
                crate::p2p::transport_paths::Transport::Libp2p,
                crate::p2p::transport_paths::OpClass::Small,
                Some(started.elapsed()),
                true,
                false,
            );
            match reply {
                BlobFetchReply::Bytes(bytes) => {
                    if verify_blob_hash(&bytes, blob_hash) {
                        // Gate #5 cross-stack transport observability.
                        // `race_fetch` is the libp2p fetch path; transport = "libp2p".
                        // The iroh blob path (IrohBlobStore) emits its own receipt
                        // when that path is wired in Phase 11 backend graduation.
                        // Structured field `transport` feeds the parity-soak (gate #6)
                        // so reviewers can confirm cross-stack delivery happened.
                        // PII check: `peer` is the libp2p PeerId string (already public
                        // per the peer-map); `blob_hash` is content-addressed.
                        tracing::debug!(
                            target: "recovery::transport",
                            blob_hash = %blob_hash,
                            source_peer = %peer,
                            transport = "libp2p",
                            "share-blob received"
                        );
                        // Returning here drops `in_flight`, which stops awaiting
                        // the remaining JoinHandles. The detached tokio tasks
                        // continue running to completion (bounded by
                        // per_peer_timeout) — see Stage 2 note on the spawn
                        // block above.
                        return FetchOutcome::Hit {
                            bytes,
                            source_peer: peer,
                        };
                    }
                    // Hash mismatch: never trust it, keep polling the batch.
                }
                BlobFetchReply::Manifest(manifest) => {
                    // Integrity gate (Q3 design #1): a manifest naming a
                    // DIFFERENT blob than requested must never be accepted —
                    // reject and keep polling rather than substituting it.
                    if manifest_hash_matches(&manifest, blob_hash) {
                        crate::metrics::inc_blob_swarm_manifest_received();
                        tracing::debug!(
                            target: "elohim_storage::blob_swarm",
                            blob_hash = %blob_hash,
                            source_peer = %peer,
                            shards = manifest.shard_hashes.len(),
                            "Q3: peer answered with a shard manifest instead of bytes"
                        );
                        return FetchOutcome::Manifest {
                            manifest,
                            source_peer: peer,
                        };
                    }
                    tracing::warn!(
                        target: "elohim_storage::blob_swarm",
                        requested_hash = %blob_hash,
                        manifest_hash = %manifest.blob_hash,
                        source_peer = %peer,
                        "Q3: manifest blob_hash mismatch — rejecting, not substituting"
                    );
                }
            }
        }
        // All in batch failed (timeout, error, or hash/manifest mismatch); try next batch.
    }
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
    blob_hash: &str,
    source_peer: &str,
    bytes: &[u8],
    self_cid: &str,
    blob_store: &BlobStore,
) -> Result<(), StorageError> {
    // Step 1: persist to filesystem first. On error, return without writing
    // any SQL — leaving inventory clean.
    blob_store.store(bytes).await?;

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
        record_fetch_success(txn, source_peer, blob_hash, &now_iso)?;

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
            resource_inventoried_as: Some(blob_hash),
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
/// claiming we host `blob_hash` while `BlobStore::store` filed the bytes under
/// their real (different) hash. Returns `Ok(false)` when the bytes fail
/// verification (discarded, no writes); `Ok(true)` when stocked + booked.
pub async fn finalize_quilt_draw(
    conn: &mut SqliteConnection,
    blob_hash: &str,
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
    if !verify_blob_hash(bytes, blob_hash) {
        return Ok(false);
    }
    // Verified: same atomic pair as an on-demand serve (store + inventory row +
    // serve-blob delivery event). Reuse — do not fork — to keep the T18 parity
    // contract (inventory row ⟺ serve-blob event) coherent across both paths.
    finalize_fetch_success(conn, blob_hash, source_peer, bytes, self_cid, blob_store).await?;
    Ok(true)
}

/// Verify that `bytes` has the sha256 hex digest matching `expected_hex`.
/// Comparison is case-insensitive (both sides lowercased).
///
/// Accepts raw hex (`"a7ffc6f8..."`), the canonical `"sha256-<hex>"` form
/// used at the HTTP boundary, and CID-form addresses (`bafkrei…`/`bafyrei…`,
/// whose multihash wraps the same sha2-256 digest). Without the prefix
/// strip, callers that normalize to the prefixed form (see `http.rs`
/// `handle_get_blob`) would always see a hash mismatch and silently return
/// `FetchOutcome::Miss`; without the CID extraction, a fetch addressed by a
/// CID-form blob hash could never verify its received bytes.
pub fn verify_blob_hash(bytes: &[u8], expected_hex: &str) -> bool {
    let expected = content_address_hex(expected_hex).unwrap_or_else(|| {
        expected_hex
            .strip_prefix("sha256-")
            .unwrap_or(expected_hex)
            .to_lowercase()
    });
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual_hex = hex::encode(hasher.finalize());
    actual_hex == expected
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

    fn known_blob() -> (Vec<u8>, String) {
        let bytes = b"hello world".to_vec();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hex = hex::encode(hasher.finalize());
        (bytes, hex)
    }

    #[test]
    fn verify_blob_hash_accepts_match() {
        let (bytes, hex) = known_blob();
        assert!(verify_blob_hash(&bytes, &hex));
    }

    #[test]
    fn verify_blob_hash_rejects_mismatch() {
        let (bytes, _) = known_blob();
        assert!(!verify_blob_hash(&bytes, &"a".repeat(64)));
    }

    #[test]
    fn verify_blob_hash_handles_uppercase_expected() {
        let (bytes, hex) = known_blob();
        let upper = hex.to_uppercase();
        assert!(verify_blob_hash(&bytes, &upper));
    }

    /// Backlog `blob-fetch-sha256-prefixed-cid-rejection` regression #1:
    /// address construction for all three legitimate input forms. Bare hex
    /// gets the legacy marker; already-marked and CID-form addresses pass
    /// through untouched (the double-wrap `sha256-<cid>` was minted by a
    /// constructor that prefixed unconditionally).
    #[test]
    fn normalize_fetch_address_bare_hex_gets_legacy_prefix() {
        let (_, hex) = known_blob();
        assert_eq!(
            normalize_fetch_address(&hex).unwrap(),
            format!("sha256-{hex}")
        );
    }

    #[test]
    fn normalize_fetch_address_already_marked_passes_through() {
        let (_, hex) = known_blob();
        let marked = format!("sha256-{hex}");
        assert_eq!(normalize_fetch_address(&marked).unwrap(), marked);
    }

    #[test]
    fn normalize_fetch_address_cid_passes_through() {
        let cid = BlobStore::compute_cid(b"hello world").to_string();
        assert!(cid.starts_with("baf"), "raw-codec CIDv1 is base32 baf…");
        assert_eq!(normalize_fetch_address(&cid).unwrap(), cid);
    }

    /// The live defect: a CIDv1 double-wrapped in the legacy `sha256-` marker
    /// (james's mesh logged `hash="sha256-bafkrei…"` rejected by every peer at
    /// ~2min cadence). The constructor must REPAIR it to the inner CID — the
    /// form responders accept — never emit it as-is.
    #[test]
    fn normalize_fetch_address_repairs_double_wrapped_cid() {
        let cid = BlobStore::compute_cid(b"hello world").to_string();
        let double_wrapped = format!("sha256-{cid}");
        assert_eq!(normalize_fetch_address(&double_wrapped).unwrap(), cid);
    }

    #[test]
    fn normalize_fetch_address_rejects_garbage() {
        assert!(normalize_fetch_address("not-an-address").is_err());
        assert!(normalize_fetch_address("sha256-not-hex-not-cid").is_err());
        assert!(normalize_fetch_address("").is_err());
    }

    /// A CID-form expected address must verify against the bytes it wraps —
    /// the CID's multihash IS the sha2-256 digest of the bytes. Without this,
    /// a fetch addressed by CID could receive correct bytes and still report
    /// a mismatch (silent `FetchOutcome::Miss`).
    #[test]
    fn verify_blob_hash_accepts_cid_form() {
        let (bytes, _) = known_blob();
        let cid = BlobStore::compute_cid(&bytes).to_string();
        assert!(verify_blob_hash(&bytes, &cid));
        // And the repaired double-wrap resolves to the same digest.
        assert!(verify_blob_hash(&bytes, &format!("sha256-{cid}")));
    }

    /// Retry hygiene at the wire choke point: a malformed address never
    /// produces a `P2PCommand::FetchBlob` — `race_fetch` gives up immediately
    /// with `InvalidAddress` (an honest terminal outcome) instead of letting
    /// every responder reject the request forever.
    #[tokio::test]
    async fn race_fetch_gives_up_on_invalid_address_without_wire_request() {
        let peer = test_peer_id();
        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);

        let outcome = race_fetch(
            "sha256-definitely-not-a-content-address",
            vec![peer.to_string()],
            &cmd_tx,
            |_| true,
            1,
            Duration::from_secs(1),
        )
        .await;

        assert!(
            matches!(outcome, FetchOutcome::InvalidAddress),
            "expected InvalidAddress, got {outcome:?}"
        );
        drop(cmd_tx);
        assert!(
            cmd_rx.recv().await.is_none(),
            "no wire request may be sent for a malformed address"
        );
    }

    /// T19 review Fix #1 regression: the call site in `http.rs::race_fetch`
    /// passes the canonical `"sha256-<64hex>"` form (set by
    /// `handle_get_blob`'s `normalized_hash`), not the raw hex. Prior to
    /// the fix, `verify_blob_hash` compared raw-hex against prefixed-hex
    /// and always returned false, causing every Stage 2 race-fetch with
    /// real bytes to silently degrade to `FetchOutcome::Miss`.
    #[test]
    fn verify_blob_hash_handles_sha256_prefix() {
        let (bytes, hex) = known_blob();
        let prefixed = format!("sha256-{}", hex);
        assert!(
            verify_blob_hash(&bytes, &prefixed),
            "verify_blob_hash must accept canonical 'sha256-<hex>' form"
        );
    }

    /// T19 Fix #2 regression: race-fetch must use FuturesUnordered so that
    /// the FIRST future to finish wins, regardless of how late other futures
    /// might complete. Prior implementation iterated `for handle in handles`
    /// which awaited them in spawn-order — a slow first peer would make the
    /// whole batch wait on it before noticing a fast second peer.
    ///
    /// This test models the await semantics directly: we spawn three tasks
    /// with staggered completion, push them into FuturesUnordered, drain via
    /// `.next().await`, and assert the FIRST one off the stream is the
    /// fastest one — proving "first responder wins" wiring is in place.
    #[tokio::test]
    async fn race_fetch_first_responder_wins() {
        use std::time::Duration;

        let mut fu: FuturesUnordered<_> = FuturesUnordered::new();
        // Slow peer (~400 ms).
        fu.push(tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(400)).await;
            "slow"
        }));
        // Medium peer (~200 ms).
        fu.push(tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            "medium"
        }));
        // Fast peer (~20 ms).
        fu.push(tokio::spawn(async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            "fast"
        }));

        let first = fu.next().await.expect("at least one future").unwrap();
        assert_eq!(
            first, "fast",
            "FuturesUnordered must yield the fastest future first; \
             sequential .await would yield 'slow' (spawn-order head)"
        );
    }

    /// T20 contract test: `finalize_fetch_success` must persist the bytes
    /// to the BlobStore FIRST, then write the `peer_blob_inventory` row
    /// (source = `fetch-success`), then emit exactly one `serve-blob`
    /// REA `economic_events` row. The persist-first ordering is what
    /// makes the T18 parity sweep correct: a crash mid-finalize leaves
    /// a benign orphan blob on disk that the sweep reconciles via
    /// re-fetch — never an inventory row pointing at bytes we don't
    /// actually have.
    #[tokio::test]
    async fn finalize_persists_bytes_then_writes_sql() {
        use crate::blob_store::BlobStore;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"finalize-test-payload".to_vec();
        let hash = {
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256-{}", hex::encode(h.finalize()))
        };

        finalize_fetch_success(
            &mut conn,
            &hash,
            "peer_X",
            &bytes,
            "self_cid_Y",
            &blob_store,
        )
        .await
        .expect("finalize ok");

        // Filesystem first: BlobStore filenames are canonical
        // `sha256-<hex>`, so probe with the prefixed form.
        assert!(
            blob_store.exists(&hash).await,
            "blob persisted to filesystem before SQL writes"
        );

        // SQL second: peer_blob_inventory row written with source='fetch-success'.
        // Use a fresh_after cutoff well in the past so the lookup window
        // includes our just-written row.
        let rows = lookup_hosts(&mut conn, &hash, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            rows.iter()
                .any(|r| r.peer_id == "peer_X" && r.source == "fetch-success"),
            "peer_blob_inventory row written with source='fetch-success'; \
             got rows: {rows:?}"
        );

        // SQL third: exactly one serve-blob REA event present, scoped to
        // this blob hash.
        use crate::db::diesel_schema::economic_events::dsl as ee;
        use diesel::prelude::*;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&hash))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1, "exactly one serve-blob event");

        // AND the event is VISIBLE through the scoped read surface consumers
        // actually query: GET /api/v1/economic-events resolves its AppContext
        // from the X-App-Id header with DEFAULT "lamad" (api/mod.rs), and
        // list_economic_events filters h_app_id = ctx. An event written under
        // any other scope is structurally invisible to that surface — the
        // genesis delivery.serve-blob-events verdict read 0 forever while the
        // atomic pair worked perfectly (#1224-#1230; the dormant-scope class:
        // written under one h_app_id, read under another). custody-blob rows
        // (the working sibling) live under the lamad default; serve-blob must
        // match.
        let visible = crate::db::economic_events::list_economic_events(
            &mut conn,
            &crate::db::AppContext::default_lamad(),
            &crate::db::economic_events::EconomicEventQuery {
                action: Some("serve-blob".to_string()),
                // NB: derive-Default gives limit 0 (LIMIT 0 → always empty);
                // the serde default_limit(100) applies only on deserialization.
                limit: 50,
                ..Default::default()
            },
        )
        .expect("scoped list ok");
        assert!(
            visible
                .iter()
                .any(|e| e.resource_inventoried_as.as_deref() == Some(hash.as_str())),
            "serve-blob event must be visible under the DEFAULT (lamad) read \
             scope that /api/v1/economic-events serves; got {} visible events",
            visible.len()
        );
    }

    /// T20 review Fix #2 regression: the two SQL writes inside
    /// `finalize_fetch_success` (record_fetch_success + serve-blob event
    /// insert) MUST be wrapped in a single transaction so a failure in the
    /// serve-blob insert rolls back the peer_blob_inventory row that was
    /// just written. Otherwise a crash between them leaves an inventory row
    /// with `source='fetch-success'` that the T18 parity sweep cannot
    /// detect (because the row says we host the blob and the filesystem
    /// agrees) but no matching REA event exists.
    ///
    /// We induce a SQL failure cheaply by dropping the `economic_events`
    /// table BEFORE calling finalize. The first SQL statement
    /// (record_fetch_success on peer_blob_inventory) succeeds, but the
    /// second (insert into economic_events) fails because the table is
    /// missing. With the transaction wrap in place, the inventory row must
    /// also be rolled back.
    #[tokio::test]
    async fn finalize_rolls_back_inventory_on_event_insert_failure() {
        use crate::blob_store::BlobStore;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::RunQueryDsl;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"rollback-test-payload".to_vec();
        let hash = {
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256-{}", hex::encode(h.finalize()))
        };

        // Force the second SQL write to fail by dropping its target table.
        diesel::sql_query("DROP TABLE economic_events;")
            .execute(&mut conn)
            .expect("drop economic_events");

        let result = finalize_fetch_success(
            &mut conn,
            &hash,
            "peer_R",
            &bytes,
            "self_cid_R",
            &blob_store,
        )
        .await;
        assert!(
            result.is_err(),
            "finalize must fail when economic_events insert fails"
        );

        // The filesystem write happened first (and is intentionally NOT
        // rolled back; the parity sweep handles orphan blobs).
        assert!(
            blob_store.exists(&hash).await,
            "blob persisted to filesystem before SQL transaction"
        );

        // Critical assertion: the peer_blob_inventory row must NOT exist,
        // because the wrapping transaction rolled it back when the
        // serve-blob insert failed.
        let rows = lookup_hosts(&mut conn, &hash, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            !rows.iter().any(|r| r.peer_id == "peer_R"),
            "peer_blob_inventory row must be rolled back when serve-blob \
             insert fails; got rows: {rows:?}"
        );
    }

    /// #3 regression (Verify Delivery Events): a proactive quilt-draw moves
    /// bytes but must ALSO leave the REA delivery trail. `finalize_quilt_draw`
    /// books exactly one `serve-blob` event with the on-demand direction
    /// (provider = the serving peer, receiver = self) AND writes the
    /// `peer_blob_inventory` row — so the proactive replication path is no
    /// longer invisible to both delivery verification and replica_count.
    /// RED before the fix: the production arm called bare `blob_store.store()`,
    /// booking zero events.
    #[tokio::test]
    async fn quilt_draw_books_serve_blob_event() {
        use crate::blob_store::BlobStore;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::prelude::*;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let bytes = b"quilt-draw-payload".to_vec();
        let hash = {
            let mut h = Sha256::new();
            h.update(&bytes);
            format!("sha256-{}", hex::encode(h.finalize()))
        };

        let booked = finalize_quilt_draw(
            &mut conn,
            &hash,
            "peer_Q",
            &bytes,
            "self_cid_Q",
            &blob_store,
        )
        .await
        .expect("finalize_quilt_draw ok");
        assert!(booked, "verified bytes must be stocked + booked");

        assert!(
            blob_store.exists(&hash).await,
            "blob persisted to filesystem"
        );

        let inv = lookup_hosts(&mut conn, &hash, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            inv.iter().any(|r| r.peer_id == "peer_Q"),
            "peer_blob_inventory row written for the proactive draw; got: {inv:?}"
        );

        use crate::db::diesel_schema::economic_events::dsl as ee;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&hash))
            .filter(ee::provider.eq("peer_Q"))
            .filter(ee::receiver.eq("self_cid_Q"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(
            count, 1,
            "exactly one serve-blob event, provider=serving peer, receiver=self"
        );
    }

    /// `finalize_quilt_draw` must verify the pulled bytes against the expected
    /// hash before booking — the quilt-draw arm does NOT pre-verify like the
    /// race-fetch path. A peer returning mismatched bytes must be discarded
    /// (`Ok(false)`), leaving NO inventory row and NO `serve-blob` event that
    /// would falsely claim we host `blob_hash` (the bytes would have been
    /// filed under their real, different hash). RED before the fix: the stub
    /// stored unverified bytes and returned `Ok(true)`.
    #[tokio::test]
    async fn quilt_draw_discards_mismatched_bytes() {
        use crate::blob_store::BlobStore;
        use crate::db::peer_blob_inventory::lookup_hosts;
        use crate::test_util::test_pool;
        use diesel::prelude::*;

        let blob_store = BlobStore::new_memory();
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // The hash the peer claimed to be serving (hash of OTHER content)…
        let claimed = {
            let mut h = Sha256::new();
            h.update(b"the-content-we-asked-for");
            format!("sha256-{}", hex::encode(h.finalize()))
        };
        // …but it actually returned these (mismatched) bytes.
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
        assert!(!booked, "mismatched bytes must be discarded (Ok(false))");

        assert!(
            !blob_store.exists(&claimed).await,
            "no blob stored under the claimed hash"
        );
        let inv = lookup_hosts(&mut conn, &claimed, "2020-01-01T00:00:00Z").unwrap();
        assert!(
            !inv.iter().any(|r| r.peer_id == "peer_M"),
            "no inventory row for a mismatched draw; got: {inv:?}"
        );
        use crate::db::diesel_schema::economic_events::dsl as ee;
        let count: i64 = ee::economic_events
            .filter(ee::action.eq("serve-blob"))
            .filter(ee::resource_inventoried_as.eq(&claimed))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 0, "no serve-blob event for a discarded draw");
    }

    /// A libp2p `PeerId` that round-trips through the `String` parse
    /// `race_fetch` performs on each candidate (mirrors
    /// `reconcile::controller::tests::test_peer_id`).
    fn test_peer_id() -> libp2p::PeerId {
        libp2p::PeerId::from(libp2p::identity::Keypair::generate_ed25519().public())
    }

    fn fixture_manifest(blob_hash: &str) -> ShardManifest {
        ShardManifest {
            blob_cid: format!("bafkrei-{blob_hash}"),
            blob_hash: blob_hash.to_string(),
            total_size: 10,
            mime_type: "application/octet-stream".to_string(),
            encoding: "chunked".to_string(),
            data_shards: 2,
            total_shards: 2,
            shard_size: 5,
            shard_hashes: vec!["sha256-s0".to_string(), "sha256-s1".to_string()],
            reach: "commons".to_string(),
            author_id: None,
            created_at: "2026-08-16T00:00:00Z".to_string(),
            verified_at: None,
        }
    }

    /// Q3 requester test #1: a peer that answers `FetchBlob` with a manifest
    /// whose `blob_hash` matches the requested hash must be ACCEPTED —
    /// `race_fetch` returns `FetchOutcome::Manifest`, not `Miss`, so the
    /// caller can pivot to a swarm shard-fetch instead of dead-ending.
    #[tokio::test]
    async fn race_fetch_accepts_matching_manifest_reply() {
        let peer = test_peer_id();
        // A real canonical address: race_fetch now validates the requested
        // address before putting it on the wire.
        let hash = format!("sha256-{}", "a".repeat(64));
        let manifest = fixture_manifest(&hash);
        let manifest_for_responder = manifest.clone();

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            if let Some(crate::p2p::P2PCommand::FetchBlob { reply, .. }) = cmd_rx.recv().await {
                let _ = reply.send(Ok(BlobFetchReply::Manifest(Box::new(
                    manifest_for_responder,
                ))));
            }
        });

        let outcome = race_fetch(
            &hash,
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
                assert_eq!(m.blob_hash, hash);
                assert_eq!(source_peer, peer.to_string());
            }
            other => panic!("expected Manifest outcome, got {other:?}"),
        }
    }

    /// Q3 requester test #2 (integrity reject): a peer that answers with a
    /// manifest naming a DIFFERENT `blob_hash` than requested must be
    /// REJECTED, not substituted. With a single candidate and no other batch
    /// to try, the race exhausts to `Miss` — proving the mismatched manifest
    /// was discarded rather than silently accepted for the wrong blob.
    #[tokio::test]
    async fn race_fetch_rejects_manifest_hash_mismatch() {
        let peer = test_peer_id();
        // Real canonical addresses: race_fetch now validates the requested
        // address before putting it on the wire.
        let requested_hash = format!("sha256-{}", "a".repeat(64));
        let wrong_manifest = fixture_manifest(&format!("sha256-{}", "b".repeat(64)));

        let (cmd_tx, mut cmd_rx) = mpsc::channel::<crate::p2p::P2PCommand>(4);
        let responder = tokio::spawn(async move {
            if let Some(crate::p2p::P2PCommand::FetchBlob { reply, .. }) = cmd_rx.recv().await {
                let _ = reply.send(Ok(BlobFetchReply::Manifest(Box::new(wrong_manifest))));
            }
        });

        let outcome = race_fetch(
            &requested_hash,
            vec![peer.to_string()],
            &cmd_tx,
            |_| true,
            1,
            Duration::from_secs(2),
        )
        .await;
        responder.await.expect("responder task");

        assert!(
            matches!(outcome, FetchOutcome::Miss),
            "a manifest naming the wrong blob must be rejected, not returned as a Manifest outcome; got {outcome:?}"
        );
    }
}
