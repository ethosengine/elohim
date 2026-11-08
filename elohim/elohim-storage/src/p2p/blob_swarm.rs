//! Torrent-style shard-swarm fetch (Q4, minutes-quiesce W1.3).
//!
//! Q3 (`blob_fetch::race_fetch` + `blob_protocol::BlobFetchResponse`) closes
//! the "no peer has bytes but a peer has the manifest" gap by letting a
//! responder answer with a durable `ShardManifest` instead of a dead-end
//! `NotFound`. This module closes the NEXT gap: once a manifest is in hand,
//! fetching its shards **one composite-level race at a time** would still
//! serialize on whichever single peer answers the whole-blob race first —
//! exactly the broken curve `chunked-blob-over-16mb-not-durable-mesh-repro.md`
//! describes. Instead, each shard races INDEPENDENTLY (its own
//! `race_fetch` call), spread across the known holder set, running
//! concurrently up to a bounded total-inflight fan-out — so replicas
//! accumulate torrent-style: every peer that lands a shard becomes a new
//! source for the NEXT requester's race, and aggregate bandwidth compounds
//! with holder count instead of bottlenecking on one composite-level race.
//!
//! Persistence reuses existing paths throughout — no new store shape:
//! - the manifest persists via `db::shard_manifests::record_generated_manifest`
//!   under the same `blob:{blob_cid}` / `"lamad"` projection key
//!   `put_blob_bytes` already uses for bare blob ingress (`http.rs`).
//! - each landed shard persists via `blob_fetch::finalize_fetch_success` —
//!   the SAME store-then-record-then-serve-blob-event path a direct blob hit
//!   uses, so per-shard REA delivery attribution stays correct (the peer that
//!   actually served shard N is who gets credited, not a single "composite
//!   source").
//! - the reassembled payload is verified against the manifest's byte CID before it
//!   is returned (every unit is also checked against its own CID as it lands).

use crate::blob_store::BlobStore;
use crate::db::peer_blob_inventory::lookup_hosts;
use crate::p2p::blob_fetch::{finalize_fetch_success, race_fetch, FetchOutcome};
use crate::sharding::{ShardEncoder, ShardManifest};
use diesel::SqliteConnection;
use elohim_epr::address::{compute_bytes_cid, ByteCid};
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Assign each shard hash its own candidate host list — the "holder
/// rotation" that turns one composite-level race into N independent shard
/// races spread across the known holders.
///
/// Per-shard inventory (`per_shard_hosts`) is preferred when present (a peer
/// that gossiped hosting a specific shard hash). When absent — the common
/// case, since shard-level inventory gossip is not universally populated —
/// the composite's own holder set is the fallback, but the list is ROTATED
/// per shard index: shard 0 tries `composite_holders[0]` first, shard 1
/// tries `composite_holders[1]` first, and so on (wrapping). Without this,
/// every shard's `race_fetch` batch would hit the SAME first-`parallelism`
/// peers in the SAME order, concentrating load on one subset of holders
/// instead of spreading it — defeating the swarm effect even though the
/// races run concurrently.
///
/// Pure and synchronous by design — this is the scheduling decision, kept
/// separate from I/O so it is directly unit-testable without a swarm.
pub fn plan_shard_holders(
    shard_cids: &[ByteCid],
    composite_holders: &[String],
    per_shard_hosts: &HashMap<ByteCid, Vec<String>>,
) -> Vec<(ByteCid, Vec<String>)> {
    let n = composite_holders.len();
    shard_cids
        .iter()
        .enumerate()
        .map(|(index, shard_cid)| {
            let mut candidates = per_shard_hosts.get(shard_cid).cloned().unwrap_or_default();
            if candidates.is_empty() && n > 0 {
                let offset = index % n;
                candidates = composite_holders[offset..]
                    .iter()
                    .chain(composite_holders[..offset].iter())
                    .cloned()
                    .collect();
            }
            (*shard_cid, candidates)
        })
        .collect()
}

/// Parameters shared by every shard race in a swarm-spread fetch. Grouped so
/// call sites (each already assembling most of these for their own
/// `race_fetch` call) can build one struct instead of threading eight
/// positional args.
/// Ceiling on a size-scaled per-shard deadline, so one oversized shard can
/// never own a whole heal round. Mirrors the release-pull cap.
const MAX_PER_SHARD_TIMEOUT: Duration = Duration::from_secs(600);

pub struct SwarmFetchParams<'a> {
    pub cmd_tx: &'a mpsc::Sender<crate::p2p::P2PCommand>,
    /// Snapshot of currently-connected peer ids — mirrors the `is_connected`
    /// closure every existing `race_fetch` call site already builds from a
    /// `HashSet`.
    pub connected: &'a HashSet<String>,
    /// Per-shard `race_fetch` batch size (reuses `fetch_blob_parallelism`,
    /// same knob the composite-level race already uses — Q4 does not
    /// introduce a new concurrency dial, only a new axis it applies to).
    pub per_shard_parallelism: usize,
    pub per_peer_timeout: Duration,
    /// Bound on CONCURRENT shard races in flight at once. Modest by design
    /// (see the doc on `fetch_shards_via_swarm`) — this is the "add a modest
    /// total-inflight bound" the design calls for, distinct from
    /// `per_shard_parallelism` (which bounds peers raced WITHIN one shard).
    pub total_inflight: usize,
    pub self_cid: &'a str,
    pub blob_store: &'a BlobStore,
}

/// Outcome of a full shard-swarm fetch for one composite manifest.
#[derive(Debug)]
pub struct SwarmFetchOutcome {
    /// Reassembled composite bytes — `Some` when every required chunk landed,
    /// or when an RS manifest reached its `data_shards` reconstruction floor.
    pub bytes: Option<Vec<u8>>,
    pub shards_fetched: usize,
    pub shards_failed: usize,
    /// Distinct peers that actually served a shard over the network this
    /// round (excludes shards that were already local) — the number the
    /// exponential-curve measure (plan W4.3) reads.
    pub distinct_source_peers: usize,
    pub elapsed: Duration,
}

/// The number of landed units below which a striped manifest certainly cannot be
/// reconstructed: `data` shards for every stripe. Each stripe still needs its own
/// `data` (checked by reconstruction); an unstriped manifest needs every chunk.
fn reconstructible_threshold(manifest: &ShardManifest) -> Option<usize> {
    let data = usize::from(manifest.data_shards());
    (!manifest.stripes.is_empty() && data > 0).then(|| data * manifest.stripes.len())
}

/// Reassemble a payload from landed units keyed by position in
/// `manifest.custody_units()`, and verify it against the manifest's byte CID.
fn reassemble(manifest: &ShardManifest, fetched: &mut HashMap<usize, Vec<u8>>) -> Option<Vec<u8>> {
    let bytes = if manifest.stripes.is_empty() {
        let mut out = Vec::with_capacity(usize::try_from(manifest.byte_len).unwrap_or(0));
        for index in 0..manifest.chunks.len() {
            out.extend_from_slice(fetched.get(&index)?);
        }
        out
    } else {
        let mut position = 0usize;
        let stripes: Vec<Vec<Option<Vec<u8>>>> = manifest
            .stripes
            .iter()
            .map(|stripe| {
                stripe
                    .shards
                    .iter()
                    .map(|_| {
                        let shard = fetched.remove(&position);
                        position += 1;
                        shard
                    })
                    .collect()
            })
            .collect();
        match ShardEncoder::reconstruct(manifest, &stripes) {
            Ok(bytes) => bytes,
            Err(error) => {
                warn!(
                    target: "elohim_storage::blob_swarm",
                    blob_cid = %manifest.blob_cid,
                    error = %error,
                    "Q4: landed shards failed to reconstruct — refusing payload"
                );
                return None;
            }
        }
    };
    (compute_bytes_cid(&bytes) == manifest.blob_cid).then_some(bytes)
}

fn should_start_shard_race(landed: &AtomicUsize, threshold: Option<usize>) -> bool {
    threshold.is_none_or(|needed| landed.load(Ordering::Acquire) < needed)
}

fn desired_network_inflight(
    landed: usize,
    threshold: Option<usize>,
    total_inflight: usize,
) -> usize {
    let ceiling = total_inflight.max(1);
    threshold
        .map(|needed| needed.saturating_sub(landed).min(ceiling))
        .unwrap_or(ceiling)
}

/// Fetch every shard named by `manifest`, spread across `composite_holders`
/// (rotated per shard via `plan_shard_holders`) and any per-shard hosts
/// already known in `peer_blob_inventory`, bounded by
/// `params.total_inflight` concurrent shard races. Each landed shard
/// persists through the SAME path a direct blob hit uses
/// (`finalize_fetch_success`) — shard bytes are ordinary content-addressed
/// blobs, so no new store shape is introduced.
///
/// A shard already present in the local blob store is used as-is (no
/// network race, no `finalize_fetch_success` — nothing was "served" to us
/// this round, so no new REA event is booked for it).
pub async fn fetch_shards_via_swarm(
    manifest: &ShardManifest,
    composite_holders: &[String],
    conn: &mut SqliteConnection,
    fresh_after: &str,
    params: &SwarmFetchParams<'_>,
) -> SwarmFetchOutcome {
    let started = Instant::now();

    // Per-shard inventory: batched, one query per shard (small N — the 1MB
    // banding caps a realistic manifest at low hundreds of shards). Absent
    // rows fall through to the composite-holder rotation in
    // `plan_shard_holders`.
    // bounded-work: units.len() — a fixed, already-persisted
    // manifest; no retry, no growth, one DB read per shard, once.
    let units = manifest.custody_units();
    let mut per_shard_hosts: HashMap<ByteCid, Vec<String>> = HashMap::new();
    for shard_cid in &units {
        if let Ok(rows) = lookup_hosts(conn, &shard_cid.to_string(), fresh_after) {
            if !rows.is_empty() {
                per_shard_hosts.insert(*shard_cid, rows.into_iter().map(|r| r.peer_id).collect());
            }
        }
    }

    let plan = plan_shard_holders(&units, composite_holders, &per_shard_hosts);

    // Local-first: a shard already on disk needs no race at all. Splitting
    // this from the network plan keeps `distinct_source_peers` honest (only
    // network-served shards count) and avoids wasting a swarm slot on a
    // shard we already have.
    let mut fetched: HashMap<usize, Vec<u8>> = HashMap::new();
    let mut to_race: Vec<(usize, ByteCid, Vec<String>)> = Vec::new();
    for (index, (shard_cid, candidates)) in plan.into_iter().enumerate() {
        match params.blob_store.get(&shard_cid).await {
            Ok(bytes) => {
                fetched.insert(index, bytes.to_vec());
            }
            Err(_) => to_race.push((index, shard_cid, candidates)),
        }
    }

    let mut completed_network_races = 0usize;
    let mut source_peers: HashSet<String> = HashSet::new();
    let reconstructible_at = reconstructible_threshold(manifest);
    let landed = Arc::new(AtomicUsize::new(fetched.len()));
    let network_race_count = to_race.len();

    let mut pending = to_race.into_iter();
    let mut results = FuturesUnordered::new();
    let cmd_tx = params.cmd_tx;
    let connected = params.connected;
    let per_shard_parallelism = params.per_shard_parallelism;
    // A deadline sized to the shard, not to the small-blob heal
    // `fetch_blob_timeout_seconds` (default 5 s) was tuned for. matthew's
    // rs-4-7 object bands into 57,948,918-byte shards: under a flat 5 s a
    // peer that is *transferring* is indistinguishable from a peer that is
    // *dead*, and the round records a miss either way. The configured value
    // stays the FLOOR — raising it raises it for every object.
    let largest_unit = manifest
        .shards()
        .map(|(_, s)| u64::from(s.len))
        .chain(manifest.chunks.iter().map(|c| u64::from(c.len)))
        .max()
        .unwrap_or(0);
    let per_peer_timeout = crate::p2p::blob_fetch::size_aware_timeout(
        largest_unit,
        params.per_peer_timeout,
        MAX_PER_SHARD_TIMEOUT,
    );
    let make_race = |(index, shard_cid, candidates): (usize, ByteCid, Vec<String>)| {
        let landed = Arc::clone(&landed);
        async move {
            if !should_start_shard_race(&landed, reconstructible_at) {
                return None;
            }
            // Q11 atomic timing budget: ONE shard's race, not the surrounding
            // swarm fan-out.
            let shard_fetch_started = Instant::now();
            let outcome = race_fetch_dual(
                &shard_cid,
                candidates,
                cmd_tx,
                |p: &str| connected.contains(p),
                per_shard_parallelism,
                per_peer_timeout,
            )
            .await;
            crate::metrics::observe_atom_duration(
                crate::metrics::ConvergenceAtom::ShardFetch,
                shard_fetch_started.elapsed(),
            );
            Some((index, shard_cid, outcome))
        }
    };

    let initial_target = desired_network_inflight(
        landed.load(Ordering::Acquire),
        reconstructible_at,
        params.total_inflight,
    );
    while results.len() < initial_target {
        let Some(task) = pending.next() else {
            break;
        };
        results.push(make_race(task));
    }

    // Process each completion before polling the fan-out again. A successful
    // persistence advances `landed`, so the next pending future observes the
    // RS threshold and declines to start. Breaking drops/cancels races already
    // in flight; no new parity work is admitted after reconstructibility.
    while let Some(result) = results.next().await {
        let Some((index, shard_cid, outcome)) = result else {
            continue;
        };
        completed_network_races += 1;
        match outcome {
            FetchOutcome::Hit { bytes, source_peer } => {
                if let Err(e) = finalize_fetch_success(
                    conn,
                    &shard_cid,
                    &source_peer,
                    &bytes,
                    params.self_cid,
                    params.blob_store,
                )
                .await
                {
                    warn!(
                        target: "elohim_storage::blob_swarm",
                        shard_cid = %shard_cid,
                        error = %e,
                        "Q4: swarm-fetched shard failed to persist"
                    );
                    crate::metrics::inc_blob_swarm_shard_fetched("miss");
                    crate::metrics::add_blob_fetch_wasted_bytes("unpersisted_shard", bytes.len());
                    continue;
                }
                source_peers.insert(source_peer);
                fetched.insert(index, bytes);
                landed.fetch_add(1, Ordering::Release);
                crate::metrics::inc_blob_swarm_shard_fetched("hit");
            }
            // A unit is never itself a composite — a Manifest reply here means a
            // misbehaving or confused peer; treat as a miss.
            FetchOutcome::Manifest { .. } | FetchOutcome::Miss | FetchOutcome::BudgetExhausted => {
                crate::metrics::inc_blob_swarm_shard_fetched("miss");
            }
            FetchOutcome::NoCandidates => {
                crate::metrics::inc_blob_swarm_shard_fetched("no_candidates");
            }
        }

        if reconstructible_at.is_some_and(|needed| fetched.len() >= needed) {
            break;
        }

        // Refill only after incorporating this completion. For RS manifests,
        // successful landings lower the desired in-flight count, so a slot
        // vacated by a hit does not admit an unnecessary parity race. A miss
        // leaves the target unchanged and admits one bounded replacement.
        let refill_target = desired_network_inflight(
            landed.load(Ordering::Acquire),
            reconstructible_at,
            params.total_inflight,
        );
        while results.len() < refill_target {
            let Some(task) = pending.next() else {
                break;
            };
            results.push(make_race(task));
        }
    }
    drop(results);

    let shards_fetched = fetched.len();
    let shards_failed = units.len().saturating_sub(shards_fetched);
    let parity_skipped = network_race_count.saturating_sub(completed_network_races);
    for _ in 0..parity_skipped {
        crate::metrics::inc_blob_swarm_shard_fetched("parity_skipped");
    }
    // bounded-work: units.len() — reassembly walks the already-completed
    // `fetched` map exactly once; no I/O, no retry.
    let required = reconstructible_threshold(manifest).unwrap_or(units.len());
    let bytes = if shards_fetched >= required {
        reassemble(manifest, &mut fetched)
    } else {
        None
    };

    if bytes.is_some() {
        crate::metrics::inc_blob_swarm_composite_completed();
        info!(
            target: "elohim_storage::blob_swarm",
            blob_cid = %manifest.blob_cid,
            units = units.len(),
            distinct_source_peers = source_peers.len(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "Q4: composite reassembled from swarm-fetched shards"
        );
    }

    SwarmFetchOutcome {
        bytes,
        shards_fetched,
        shards_failed,
        distinct_source_peers: source_peers.len(),
        elapsed: started.elapsed(),
    }
}

/// Outcome of a manifest-aware race — a drop-in superset of `FetchOutcome`
/// for callers that want Q3+Q4 wired end-to-end: a direct hit still resolves
/// immediately; a manifest reply is persisted and becomes either `Hit` when
/// every shard lands or `Reconstructible` when an RS data-shard floor lands.
#[derive(Debug)]
pub enum SwarmRaceOutcome {
    Hit {
        bytes: Vec<u8>,
        source_peer: String,
    },
    /// An erasure-coded manifest reached its data-shard floor before every
    /// parity shard landed. The bytes are servable now; ordinary background
    /// salvage may fill the missing parity later.
    Reconstructible {
        bytes: Vec<u8>,
        landed: usize,
        missing: usize,
    },
    /// A manifest was received and persisted, but at least one shard could
    /// not be fetched from any connected holder this round. The manifest is
    /// now durable regardless — a later round (this call again, or Q2's
    /// local reassembly once the missing shard arrives via ordinary
    /// replication) can complete it without repeating the manifest
    /// round-trip. Boxed — see the size-note on `blob_fetch::FetchOutcome::Manifest`.
    ManifestPersistedIncomplete {
        manifest: Box<ShardManifest>,
        missing_shards: usize,
    },
    Miss,
    NoCandidates,
}

/// `race_fetch` plus the Q3/Q4 manifest pivot: on a manifest reply, persist
/// it durably (the same write path `HttpServer::resolve_manifest`/Q2 reads —
/// `blob:{blob_cid}` under `"lamad"`, mirroring `put_blob_bytes`'s bare blob
/// ingress convention) and attempt the full swarm-spread shard fetch before
/// giving up.
pub async fn race_fetch_with_swarm(
    blob_hash: &ByteCid,
    candidates: Vec<String>,
    conn: &mut SqliteConnection,
    fresh_after: &str,
    params: &SwarmFetchParams<'_>,
) -> SwarmRaceOutcome {
    let outcome = race_fetch_dual(
        blob_hash,
        candidates.clone(),
        params.cmd_tx,
        |p: &str| params.connected.contains(p),
        params.per_shard_parallelism,
        params.per_peer_timeout,
    )
    .await;

    match outcome {
        FetchOutcome::Hit { bytes, source_peer } => SwarmRaceOutcome::Hit { bytes, source_peer },
        FetchOutcome::Manifest { manifest, .. } => {
            let content_id = format!("blob:{}", manifest.blob_cid);
            if let Err(e) = crate::db::shard_manifests::record_generated_manifest(
                conn,
                &content_id,
                "lamad",
                &manifest,
            ) {
                warn!(
                    target: "elohim_storage::blob_swarm",
                    blob_cid = %manifest.blob_cid,
                    error = %e,
                    "Q3: failed to persist manifest received from a peer"
                );
            }

            let swarm_outcome =
                fetch_shards_via_swarm(&manifest, &candidates, conn, fresh_after, params).await;
            match swarm_outcome.bytes {
                Some(bytes) if swarm_outcome.shards_failed > 0 => {
                    SwarmRaceOutcome::Reconstructible {
                        bytes,
                        landed: swarm_outcome.shards_fetched,
                        missing: swarm_outcome.shards_failed,
                    }
                }
                Some(bytes) => SwarmRaceOutcome::Hit {
                    bytes,
                    source_peer: "swarm".to_string(),
                },
                None => SwarmRaceOutcome::ManifestPersistedIncomplete {
                    manifest,
                    missing_shards: swarm_outcome.shards_failed,
                },
            }
        }
        FetchOutcome::Miss | FetchOutcome::BudgetExhausted => SwarmRaceOutcome::Miss,
        FetchOutcome::NoCandidates => SwarmRaceOutcome::NoCandidates,
    }
}

// ────────────────────────────────────────────────────────────────
// T2 — dual-plane heal-on-read
//
// `race_fetch` (libp2p) owns the libp2p wire and the verify-then-serve rule.
// This section adds a SECOND leg over the iroh shard ALPN (served from the same
// `BlobStore`, never an iroh-blobs copy) for the peers the iroh peer book can
// dial, races the two, and returns the first VERIFIED bytes: both legs terminate
// in `blob_fetch::verify_fetched_bytes` on the requested byte CID.
// ────────────────────────────────────────────────────────────────

/// Which transport legs a heal-on-read race should construct.
///
/// Derived from live facts rather than a config string, so the mapping is
/// pure and testable:
/// - **`libp2p` backend** — no iroh node exists, so nothing ever registers an
///   `IrohFetchLeg`; `iroh_targets` is empty and this returns `Libp2pOnly`.
///   No iroh future is constructed. Behavior byte-identical to before T2.
/// - **`dual`** — the book knows iroh addresses for at least one candidate AND
///   at least one candidate is libp2p-connected → `Both`.
/// - **`iroh`** — there is no libp2p swarm, so no candidate passes the
///   connected filter → `IrohOnly`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceLegs {
    Libp2pOnly,
    Both,
    IrohOnly,
}

/// Pure leg selection. `iroh_targets` is the number of candidates the peer
/// book could resolve to a dialable iroh address; `libp2p_candidates` is the
/// number that survived the connected filter.
pub fn plan_race_legs(iroh_targets: usize, libp2p_candidates: usize) -> RaceLegs {
    match (iroh_targets > 0, libp2p_candidates > 0) {
        (false, _) => RaceLegs::Libp2pOnly,
        (true, true) => RaceLegs::Both,
        (true, false) => RaceLegs::IrohOnly,
    }
}

/// One fetch candidate, enriched with the iroh dial target when the peer
/// book knows one for it.
///
/// `peer_id` stays the identifier the rest of the blob plane speaks — the
/// `peer_blob_inventory.peer_id` string the candidate list was built from —
/// so a hit over either transport books the SAME provider in
/// `finalize_fetch_success`. The transport is a log field, never a different
/// identity.
#[cfg(feature = "p2p-iroh")]
#[derive(Debug, Clone, PartialEq)]
pub struct FetchCandidate {
    pub peer_id: String,
    pub iroh_addr: Option<iroh::NodeAddr>,
}

/// Attach an iroh dial target to each candidate the book can resolve.
///
/// Matching is by `libp2p_peer_id` OR `agent_cid` — both are ROUTING HINTS
/// carried on a signature-verified transport manifest, never attribution
/// (storage CLAUDE.md, the attribution cut). A candidate the book does not
/// know keeps `iroh_addr: None` and is raced over libp2p alone.
///
/// Pure and synchronous — the book snapshot is taken by the caller so this
/// is directly unit-testable without a live endpoint.
#[cfg(feature = "p2p-iroh")]
pub fn enrich_candidates(
    candidates: &[String],
    book: &[crate::p2p_iroh::IrohPeerEntry],
) -> Vec<FetchCandidate> {
    candidates
        .iter()
        .map(|peer_id| {
            let iroh_addr = book
                .iter()
                .find(|entry| {
                    entry.libp2p_peer_id.as_deref() == Some(peer_id.as_str())
                        || entry.agent_cid.as_deref() == Some(peer_id.as_str())
                        // pure-iroh fallback candidates are labelled by node id
                        || entry.addr.node_id.to_string() == *peer_id
                })
                .map(|entry| entry.addr.clone());
            FetchCandidate {
                peer_id: peer_id.clone(),
                iroh_addr,
            }
        })
        .collect()
}

/// `race_fetch`, raced against the iroh plane when one is registered.
///
/// Drop-in for `race_fetch` (identical signature and `FetchOutcome`), so the
/// heal-on-read call sites change by one identifier. On a libp2p-only node it
/// IS `race_fetch` — same call, no wrapper future, no allocation.
///
/// Bound: one shot. Each leg gets one dial per candidate, capped at the
/// existing `parallelism` knob; no new retry loop, no queue, no second
/// concurrency dial.
pub async fn race_fetch_dual(
    blob_hash: &ByteCid,
    candidates: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    // `Sync` (not just `Fn`) because the dual path boxes the libp2p leg into a
    // `Send` future alongside the iroh legs; every call site's closure borrows
    // an immutable `HashSet`, which already satisfies it.
    is_connected: impl Fn(&str) -> bool + Sync,
    parallelism: usize,
    per_peer_timeout: Duration,
) -> FetchOutcome {
    #[cfg(feature = "p2p-iroh")]
    if let Some(leg) = crate::p2p_iroh::iroh_fetch_leg() {
        return race_both_planes(
            leg,
            blob_hash,
            candidates,
            cmd_tx,
            &is_connected,
            parallelism,
            per_peer_timeout,
        )
        .await;
    }

    race_fetch(
        blob_hash,
        candidates,
        cmd_tx,
        &is_connected,
        parallelism,
        per_peer_timeout,
    )
    .await
}

/// The dual-plane race itself. Both legs run inside ONE `FuturesUnordered`,
/// so the first leg to produce a terminal-good outcome (verified bytes, or a
/// hash-matched manifest from the libp2p leg) returns and the losers are
/// dropped — for the iroh futures, which are plain async blocks rather than
/// `tokio::spawn`, drop IS cancellation.
///
/// Returning on the first `Manifest` as well as the first `Hit` preserves
/// today's latency to the Q3/Q4 swarm pivot exactly: waiting for the iroh leg
/// to settle before honoring a manifest would add up to `per_peer_timeout` to
/// every sharded composite read.
#[cfg(feature = "p2p-iroh")]
#[allow(clippy::too_many_arguments)]
async fn race_both_planes(
    leg: &'static crate::p2p_iroh::IrohFetchLeg,
    blob_hash: &ByteCid,
    candidates: Vec<String>,
    cmd_tx: &mpsc::Sender<crate::p2p::P2PCommand>,
    is_connected: &(impl Fn(&str) -> bool + Sync),
    parallelism: usize,
    per_peer_timeout: Duration,
) -> FetchOutcome {
    use futures::future::FutureExt;

    let wire_addr = *blob_hash;

    // iroh reachability is INDEPENDENT of libp2p connectedness: a peer we are
    // not libp2p-connected to may still be dialable over QUIC. So enrich over
    // the full candidate list, not the connected subset.
    let iroh_targets: Vec<(String, iroh::NodeAddr)> =
        enrich_candidates(&candidates, &leg.book().snapshot(None))
            .into_iter()
            .filter_map(|c| c.iroh_addr.map(|addr| (c.peer_id, addr)))
            .take(parallelism)
            .collect();

    let libp2p_candidates = candidates.iter().filter(|p| is_connected(p)).count();
    let legs = plan_race_legs(iroh_targets.len(), libp2p_candidates);
    if legs == RaceLegs::Libp2pOnly {
        return race_fetch(
            blob_hash,
            candidates,
            cmd_tx,
            is_connected,
            parallelism,
            per_peer_timeout,
        )
        .await;
    }

    let mut in_flight: FuturesUnordered<futures::future::BoxFuture<'_, FetchOutcome>> =
        FuturesUnordered::new();

    if legs == RaceLegs::Both {
        in_flight.push(
            race_fetch(
                blob_hash,
                candidates.clone(),
                cmd_tx,
                is_connected,
                parallelism,
                per_peer_timeout,
            )
            .boxed(),
        );
    }

    for (peer_id, addr) in iroh_targets.iter().cloned() {
        let wire = wire_addr;
        in_flight.push(
            async move {
                // The iroh leg may ride a relay locally (no direct address in the
                // book entry) — a 1.8 MB landing bundle timed out at the 5 s
                // per-peer budget 171× on 2026-08-29 while the responder served it
                // 456×. Bytes in flight get a size-tolerant bound; the HTTP request
                // budget still returns `syncing` and the heal lands in the background.
                let iroh_timeout = per_peer_timeout.max(Duration::from_secs(60));
                let started = std::time::Instant::now();
                match tokio::time::timeout(iroh_timeout, leg.fetch(addr.clone(), &wire)).await {
                    Err(_) => {
                        crate::metrics::inc_iroh_blob_fetch("timeout");
                        crate::p2p::transport_paths::global().record(
                            &peer_id,
                            crate::p2p::transport_paths::Transport::Iroh,
                            crate::p2p::transport_paths::OpClass::Small,
                            Some(started.elapsed()),
                            false,
                            false,
                        );
                        tracing::warn!(
                            target: "elohim_storage::blob_fetch",
                            blob_hash = %wire,
                            source_peer = %peer_id,
                            transport = "iroh",
                            after_s = started.elapsed().as_secs(),
                            "iroh blob fetch timed out"
                        );
                        FetchOutcome::Miss
                    }
                    Ok(Err(crate::p2p_iroh::IrohBlobFetchError::NotFound(_))) => {
                        crate::metrics::inc_iroh_blob_fetch("not_found");
                        crate::p2p::transport_paths::global().record(
                            &peer_id,
                            crate::p2p::transport_paths::Transport::Iroh,
                            crate::p2p::transport_paths::OpClass::Small,
                            Some(started.elapsed()),
                            true,
                            false,
                        );
                        // Composite pivot (parity with the libp2p leg): an honest
                        // whole-bytes miss may still be a peer holding the blob as
                        // RS shards — ask for the manifest over the same ALPN.
                        let req =
                            crate::p2p::shard_protocol::ShardRequest::GetManifest { cid: wire };
                        let client = crate::p2p_iroh::IrohShardClient::new(leg.endpoint());
                        match tokio::time::timeout(per_peer_timeout, client.request(addr, &req))
                            .await
                        {
                            Ok(Ok(crate::p2p::shard_protocol::ShardResponse::Manifest(
                                manifest,
                            ))) if manifest.blob_cid == wire => {
                                tracing::debug!(
                                    target: "recovery::transport",
                                    blob_hash = %wire,
                                    source_peer = %peer_id,
                                    transport = "iroh",
                                    units = manifest.custody_units().len(),
                                    "peer answered with a shard manifest instead of bytes"
                                );
                                FetchOutcome::Manifest {
                                    manifest,
                                    source_peer: peer_id,
                                }
                            }
                            _ => FetchOutcome::Miss,
                        }
                    }
                    Ok(Err(e)) => {
                        crate::metrics::inc_iroh_blob_fetch(e.metric_result());
                        crate::p2p::transport_paths::global().record(
                            &peer_id,
                            crate::p2p::transport_paths::Transport::Iroh,
                            crate::p2p::transport_paths::OpClass::Small,
                            Some(started.elapsed()),
                            false,
                            false,
                        );
                        tracing::warn!(
                            target: "elohim_storage::blob_fetch",
                            blob_hash = %wire,
                            source_peer = %peer_id,
                            transport = "iroh",
                            after_ms = started.elapsed().as_millis() as u64,
                            error = %e,
                            "iroh blob fetch failed"
                        );
                        FetchOutcome::Miss
                    }
                    Ok(Ok(bytes)) => {
                        // Verify parity: the SAME predicate the libp2p leg
                        // applies to its reply (`blob_fetch::verify_fetched_bytes`
                        // on the requested address). Unverified iroh bytes are
                        // discarded here and never reach persistence.
                        if crate::p2p::blob_fetch::verify_fetched_bytes(&bytes, &wire) {
                            crate::metrics::inc_iroh_blob_fetch("ok");
                            crate::p2p::transport_paths::global().record(
                                &peer_id,
                                crate::p2p::transport_paths::Transport::Iroh,
                                crate::p2p::transport_paths::OpClass::Small,
                                Some(started.elapsed()),
                                true,
                                false,
                            );
                            tracing::debug!(
                                target: "recovery::transport",
                                blob_hash = %wire,
                                source_peer = %peer_id,
                                transport = "iroh",
                                "share-blob received"
                            );
                            FetchOutcome::Hit {
                                bytes,
                                source_peer: peer_id,
                            }
                        } else {
                            tracing::warn!(
                                target: "elohim_storage::blob_fetch",
                                blob_hash = %wire,
                                source_peer = %peer_id,
                                transport = "iroh",
                                bytes = bytes.len(),
                                "iroh blob fetch: bytes do not hash to the requested address"
                            );
                            crate::metrics::inc_iroh_blob_fetch("verify_failed");
                            crate::metrics::add_blob_fetch_wasted_bytes(
                                "hash_failure",
                                bytes.len(),
                            );
                            crate::p2p::transport_paths::global().record(
                                &peer_id,
                                crate::p2p::transport_paths::Transport::Iroh,
                                crate::p2p::transport_paths::OpClass::Small,
                                Some(started.elapsed()),
                                false,
                                false,
                            );
                            warn!(
                                target: "elohim_storage::blob_swarm",
                                blob_hash = %wire,
                                source_peer = %peer_id,
                                "T2: iroh-served bytes failed hash verification — discarded"
                            );
                            FetchOutcome::Miss
                        }
                    }
                }
            }
            .boxed(),
        );
    }

    // Bytes beat a manifest. A peer that only holds the manifest answers in one
    // round trip while another peer is still streaming the whole blob; returning
    // the manifest at once sent the heal down the shard path — which fails when
    // the blob exists only as an aliased whole object (measured 2026-08-28,
    // re-staged landing: `manifest persisted but swarm shard-fetch incomplete`).
    // Hold the manifest as the fallback until every leg has spoken.
    let mut manifest_fallback: Option<FetchOutcome> = None;
    while let Some(outcome) = in_flight.next().await {
        match outcome {
            hit @ FetchOutcome::Hit { .. } => return hit,
            manifest @ FetchOutcome::Manifest { .. } => {
                if manifest_fallback.is_none() {
                    manifest_fallback = Some(manifest);
                }
            }
            // A leg that gave up is just a leg that gave up; keep polling.
            FetchOutcome::Miss | FetchOutcome::NoCandidates | FetchOutcome::BudgetExhausted => {}
        }
    }
    if let Some(manifest) = manifest_fallback {
        return manifest;
    }

    // At least one iroh dial was attempted (legs != Libp2pOnly), so "no
    // candidates" would be a lie — this is an honest miss.
    FetchOutcome::Miss
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holders(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn unit(label: &str) -> ByteCid {
        elohim_epr::address::compute_bytes_cid(label.as_bytes())
    }

    /// Q4 scheduling test: with NO per-shard inventory at all, each shard
    /// must still get a candidate list, and consecutive shards must prefer a
    /// DIFFERENT first holder — the rotation that spreads load instead of
    /// every shard racing the identical peer-order (which would collapse the
    /// "spread across holders" back into one composite-level race in
    /// practice, since every batch's first pick would be the same peer).
    #[test]
    fn plan_shard_holders_rotates_composite_fallback_across_shards() {
        let shard_cids = vec![unit("shard0"), unit("shard1"), unit("shard2")];
        let composite_holders = holders(&["peer-A", "peer-B"]);
        let per_shard_hosts = HashMap::new();

        let plan = plan_shard_holders(&shard_cids, &composite_holders, &per_shard_hosts);
        assert_eq!(plan.len(), 3);

        // Shard 0 → offset 0 → [A, B]; shard 1 → offset 1 → [B, A]; shard 2
        // wraps back to offset 0 → [A, B]. The FIRST candidate differs
        // between adjacent shards, proving rotation (not identical order).
        assert_eq!(plan[0].1, vec!["peer-A", "peer-B"]);
        assert_eq!(plan[1].1, vec!["peer-B", "peer-A"]);
        assert_eq!(plan[2].1, vec!["peer-A", "peer-B"]);
        assert_ne!(
            plan[0].1.first(),
            plan[1].1.first(),
            "adjacent shards must prefer different first holders"
        );
    }

    /// Q4 scheduling test: per-shard inventory (a shard-specific host)
    /// overrides the composite-holder rotation for that shard — a more
    /// specific fact beats the generic fallback.
    #[test]
    fn plan_shard_holders_prefers_per_shard_inventory_when_present() {
        let shard_cids = vec![unit("shard0"), unit("shard1")];
        let composite_holders = holders(&["peer-A", "peer-B"]);
        let mut per_shard_hosts = HashMap::new();
        per_shard_hosts.insert(unit("shard1"), holders(&["peer-C"]));

        let plan = plan_shard_holders(&shard_cids, &composite_holders, &per_shard_hosts);
        assert_eq!(plan[0].1, vec!["peer-A", "peer-B"]); // no per-shard row → fallback
        assert_eq!(plan[1].1, vec!["peer-C"]); // per-shard row → used verbatim
    }

    /// No composite holders and no per-shard inventory: a shard simply gets
    /// an empty candidate list (the eventual `race_fetch` call resolves
    /// `NoCandidates`) rather than panicking on the `index % n` rotation.
    #[test]
    fn plan_shard_holders_handles_no_holders_at_all() {
        let shard_cids = vec![unit("shard0")];
        let plan = plan_shard_holders(&shard_cids, &[], &HashMap::new());
        assert_eq!(plan.len(), 1);
        assert!(plan[0].1.is_empty());
    }

    /// A one-stripe RS(4,3) manifest and its shards (erasure forced on).
    fn rs_4_7(data: &[u8]) -> (ShardManifest, Vec<crate::sharding::StripeShards>) {
        crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig {
            rs_threshold: 0,
            rs_data_shards: 4,
            rs_parity_shards: 3,
        })
        .encode(data, "application/octet-stream", "commons")
        .expect("rs manifest")
    }

    /// An unstriped (chunked) manifest.
    fn chunked(data: &[u8]) -> ShardManifest {
        crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default())
            .create_manifest(data, "application/octet-stream", "commons")
            .expect("chunked manifest")
    }

    #[test]
    fn rs_4_7_stops_admitting_races_at_four_landed_shards() {
        let (rs, _) = rs_4_7(b"seven bytes of payload");
        let threshold = reconstructible_threshold(&rs);
        let landed = AtomicUsize::new(3);
        assert!(should_start_shard_race(&landed, threshold));
        landed.fetch_add(1, Ordering::Release);
        for _ in 0..4 {
            assert!(!should_start_shard_race(&landed, threshold));
        }
    }

    #[test]
    fn rs_4_7_successes_reduce_inflight_before_refill() {
        let threshold = Some(4);
        let mut started = desired_network_inflight(0, threshold, 8);
        let mut inflight = started;

        for landed in 1..=4 {
            inflight -= 1;
            let target = desired_network_inflight(landed, threshold, 8);
            if inflight < target {
                started += target - inflight;
                inflight = target;
            }
        }

        assert_eq!(started, 4, "successful data races must not admit parity");
        assert_eq!(inflight, 0);
    }

    #[test]
    fn rs_4_7_miss_admits_one_bounded_replacement() {
        let threshold = Some(4);
        let mut started = desired_network_inflight(0, threshold, 8);
        let mut inflight = started - 1; // one of the first four races missed
        let target = desired_network_inflight(0, threshold, 8);
        if inflight < target {
            started += target - inflight;
            inflight = target;
        }

        assert_eq!(started, 5);
        assert_eq!(inflight, 4);
    }

    #[test]
    fn rs_4_7_with_three_landed_remains_incomplete() {
        let (rs, _) = rs_4_7(b"seven bytes of payload");
        let landed = AtomicUsize::new(3);
        assert!(should_start_shard_race(
            &landed,
            reconstructible_threshold(&rs)
        ));
    }

    #[test]
    fn chunked_manifest_never_skips_a_missing_shard() {
        let chunked = chunked(b"a small chunked payload");
        let landed = AtomicUsize::new(2);
        assert_eq!(reconstructible_threshold(&chunked), None);
        assert!(should_start_shard_race(&landed, None));
    }

    // ── T2: dual-plane heal-on-read ──────────────────────────────

    /// `libp2p` backend: no iroh leg is ever registered, so leg planning
    /// must never construct an iroh future no matter how many candidates
    /// exist. This is the regression guard on "libp2p mode unchanged".
    #[test]
    fn plan_race_legs_libp2p_mode_never_constructs_an_iroh_leg() {
        assert_eq!(plan_race_legs(0, 3), RaceLegs::Libp2pOnly);
        assert_eq!(plan_race_legs(0, 0), RaceLegs::Libp2pOnly);
    }

    /// `dual`: book knows an iroh address AND a candidate is libp2p-connected
    /// → both planes race.
    #[test]
    fn plan_race_legs_dual_races_both_planes() {
        assert_eq!(plan_race_legs(2, 3), RaceLegs::Both);
    }

    /// `iroh`: no libp2p swarm means no candidate survives the connected
    /// filter, so the iroh leg races alone rather than the whole read
    /// collapsing to `NoCandidates`.
    #[test]
    fn plan_race_legs_iroh_only_when_no_libp2p_candidate_is_connected() {
        assert_eq!(plan_race_legs(2, 0), RaceLegs::IrohOnly);
    }

    /// A chunked payload reassembles from its chunks in order and is verified
    /// against the manifest's byte CID; a missing or altered chunk yields nothing.
    #[test]
    fn reassemble_chunked_payload_verifies_against_the_blob_cid() {
        let data: Vec<u8> = (0..20_000u32).flat_map(|n| n.to_le_bytes()).collect();
        let m = chunked(&data);
        let mut fetched: HashMap<usize, Vec<u8>> = HashMap::new();
        let mut offset = 0usize;
        for (index, chunk) in m.chunks.iter().enumerate() {
            let len = chunk.len as usize;
            fetched.insert(index, data[offset..offset + len].to_vec());
            offset += len;
        }
        assert_eq!(reassemble(&m, &mut fetched.clone()), Some(data.clone()));

        let mut altered = fetched.clone();
        altered.get_mut(&0).expect("chunk 0")[0] ^= 1;
        assert_eq!(reassemble(&m, &mut altered), None);

        let mut missing = fetched;
        missing.remove(&0);
        assert_eq!(reassemble(&m, &mut missing), None);
    }

    /// A striped payload reassembles from any `data` shards of its stripe.
    #[test]
    fn reassemble_striped_payload_from_data_threshold_of_shards() {
        let data = b"payload reconstructed from four of seven shards".to_vec();
        let (m, stripes) = rs_4_7(&data);
        let shards = &stripes[0];
        // Keep shards 1, 2, 4 and 6 (positions in custody_units()).
        let mut fetched: HashMap<usize, Vec<u8>> = [1usize, 2, 4, 6]
            .into_iter()
            .map(|i| (i, shards[i].clone()))
            .collect();
        assert_eq!(reassemble(&m, &mut fetched), Some(data));
    }

    #[cfg(feature = "p2p-iroh")]
    mod iroh_leg {
        use super::*;
        use crate::p2p_iroh::IrohPeerEntry;
        use iroh::{NodeAddr, SecretKey};

        fn entry(
            key: &SecretKey,
            agent_cid: Option<&str>,
            libp2p_peer_id: Option<&str>,
        ) -> IrohPeerEntry {
            IrohPeerEntry {
                addr: NodeAddr::new(key.public()).with_direct_addresses([(
                    [127, 0, 0, 1],
                    4433u16,
                )
                    .into()]),
                agent_cid: agent_cid.map(|s| s.to_string()),
                libp2p_peer_id: libp2p_peer_id.map(|s| s.to_string()),
                user_agent: None,
                announced_at_ms: 1,
            }
        }

        /// Book HAS the peer (by libp2p PeerId) → the candidate carries a
        /// dialable iroh address. Book MISS → libp2p-only candidate.
        #[test]
        fn enrich_attaches_iroh_addr_only_for_peers_the_book_knows() {
            let mut rng = rand::rngs::OsRng;
            let key = SecretKey::generate(&mut rng);
            let book = vec![entry(&key, Some("agent-a"), Some("12D3KooWpeerA"))];

            let enriched = enrich_candidates(
                &["12D3KooWpeerA".to_string(), "12D3KooWpeerB".to_string()],
                &book,
            );

            assert_eq!(enriched.len(), 2);
            assert_eq!(
                enriched[0].iroh_addr.as_ref().map(|a| a.node_id),
                Some(key.public()),
                "book hit must carry the iroh dial target"
            );
            assert!(
                enriched[1].iroh_addr.is_none(),
                "book miss stays a libp2p-only candidate"
            );
            // The peer identity the blob plane books is unchanged by
            // enrichment — the transport is not a different provider.
            assert_eq!(enriched[0].peer_id, "12D3KooWpeerA");
        }

        /// The candidate list can carry an agent CID (inventory rows are not
        /// uniformly keyed by libp2p PeerId); the book resolves either hint.
        #[test]
        fn enrich_matches_by_agent_cid_when_no_libp2p_hint_is_bound() {
            let mut rng = rand::rngs::OsRng;
            let key = SecretKey::generate(&mut rng);
            let book = vec![entry(&key, Some("agent-jessica"), None)];

            let enriched = enrich_candidates(&["agent-jessica".to_string()], &book);
            assert_eq!(
                enriched[0].iroh_addr.as_ref().map(|a| a.node_id),
                Some(key.public())
            );
        }

        /// An empty book yields no iroh targets, so planning falls back to
        /// the libp2p leg alone even on a dual node.
        #[test]
        fn empty_book_degrades_to_libp2p_only() {
            let enriched = enrich_candidates(&["12D3KooWpeerA".to_string()], &[]);
            let targets = enriched.iter().filter(|c| c.iroh_addr.is_some()).count();
            assert_eq!(targets, 0);
            assert_eq!(plan_race_legs(targets, 1), RaceLegs::Libp2pOnly);
        }
    }
}
