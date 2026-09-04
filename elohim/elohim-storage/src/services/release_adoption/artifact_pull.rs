//! Peer-pull for release artifacts — the integrator's wiring, made.
//!
//! [`watch::BlobStoreArtifactSource`](super::watch::BlobStoreArtifactSource) is
//! local-only **by design**: a blob that has not replicated yet reports
//! `artifact_unavailable`, which is transient, so the next sweep asks again once
//! ordinary replication has done its job. Its own doc calls peer-pull "the
//! integrator's wiring decision, deliberately not a side effect of an
//! observe-mode sweep". This module is that decision.
//!
//! # Why it had to be made
//!
//! Measured live on the household mesh, 2026-09-04: **both** epic deliveries
//! that night refused on `artifact_unavailable`, climbed the finite backoff
//! ladder (30 s → 120 s → 600 s → 1800 s), and were rescued only by an operator
//! hand-`PUT`ting the blob to each peer. The controller was correct at every
//! step and still could not converge, because the one thing it could not do was
//! *ask a peer for the bytes* — while the rest of this binary asks peers for
//! bytes constantly (`GET /blob/{hash}` heals a local miss that way, and the
//! custody reconciliation controller kicks fetches that way). The release
//! controller was the only consumer of the blob plane that could not.
//!
//! # What it does NOT change
//!
//! - **The floor.** A pulled artifact is verified by exactly the same
//!   [`verify::verify_artifacts`](super::verify::verify_artifacts) checks as a
//!   locally-held one: this source reports what arrived and judges nothing.
//! - **The refusal.** A pull that finds no peer, or no peer with the bytes,
//!   leaves today's transient `artifact_unavailable` in place — including its
//!   detail, so an operator can still tell "not replicated yet" from a
//!   substantive failure. The ladder still exists; a successful pull simply
//!   stops climbing it (`Verdict::Applied` resets `consecutive_refusals`).
//! - **Observe mode.** This is an artifact SOURCE, so it is exercised by
//!   whatever modes stage artifacts. It never applies anything and never moves
//!   a head.
//!
//! # Bounds (C6a — work is sized before it is started)
//!
//! - **One pull attempt per sweep per artifact.** `fetch` is called once per
//!   artifact per sweep, and it attempts at most one pull, so the bound is
//!   structural rather than a counter someone has to remember to reset.
//! - **The sweep's byte budget already applies**: `watch::check_channel`
//!   refuses to stage a release whose DECLARED bytes exceed the remaining
//!   per-sweep budget before this source is ever asked, so a pull can only ever
//!   move bytes the budget already admitted.
//! - **A size-aware per-peer deadline** ([`size_aware_timeout`]) floored by
//!   `Config::fetch_blob_timeout_seconds` and capped, plus an overall deadline
//!   across all batches, so one sweep can never park on a slow pull for longer
//!   than the sweep interval — and a large artifact is never mistaken for a dead
//!   peer.
//! - **A candidate cap.** Inventory-named hosts first (evidence-ordered, the
//!   same rows `GET /blob/{hash}` heals from), then currently-connected peers as
//!   a fallback — capped, because a release artifact is frequently NEWER than
//!   the ~60 s inventory-gossip cadence that would have named its hosts.
//!
//! # Following the manifest, not just naming it
//!
//! A peer that holds a blob SHARDED answers the composite-level race with a
//! durable `ShardManifest` instead of bytes. The first version of this source
//! read that as a miss — which is how a 10 MB `.happ` that every mesh peer was
//! serving happily over `GET /blob/{hash}` produced `none served it` on every
//! sweep (measured 2026-09-04, station 7).
//!
//! The pull therefore goes through [`race_fetch_with_swarm`], not
//! `race_fetch`: the manifest-aware superset that already existed for exactly
//! this, persists the manifest, and follows it into independent per-shard races.
//! Nothing here re-implements reassembly — the swarm module owns it, the same
//! way the HTTP local-miss heal consumes it.
//!
//! [`race_fetch_with_swarm`]: crate::p2p::blob_swarm::race_fetch_with_swarm

use std::path::Path;
use std::sync::Arc;

use super::verify::FetchedArtifact;
use super::watch::ArtifactSource;
use super::{AdoptionRefusal, Artifact, RefusalReason};

/// What one pull attempt did. Reported into the refusal detail on failure, so a
/// stuck channel says WHY it is stuck rather than repeating "not held locally".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullOutcome {
    /// Bytes arrived, verified against the address, and are now local.
    Pulled { source_peer: String, bytes: u64 },
    /// No peer was askable — none in inventory, none connected.
    NoCandidates,
    /// Peers were asked; none served the bytes.
    Miss { asked: usize },
    /// The object was FOUND as a shard manifest, the manifest is now durable
    /// locally, and the swarm shard-fetch still could not complete it this
    /// round — at least one shard reached no connected holder.
    ///
    /// **Kept out of [`PullOutcome::Miss`] deliberately** (2026-09-04): the
    /// first live run of this source reported `none served it` for a 10 MB
    /// release artifact that every peer was serving happily over
    /// `GET /blob/{hash}` as a *reassembled* blob, and the collapsed variant
    /// made the two stories indistinguishable. A pull that found the object but
    /// not all of its bytes is a different fact from a pull that found nothing —
    /// and it is *progress*: the next sweep resumes from the persisted manifest
    /// without repeating the round-trip.
    ManifestOnly { source_peer: String },
    /// The pull could not be attempted or errored out (no DB pool, a malformed
    /// address, a deadline). Carries the operator-readable reason.
    Failed(String),
}

impl PullOutcome {
    /// A stable label for the log line and the metric-shaped read.
    pub fn label(&self) -> &'static str {
        match self {
            PullOutcome::Pulled { .. } => "pulled",
            PullOutcome::NoCandidates => "no_candidates",
            PullOutcome::Miss { .. } => "miss",
            PullOutcome::ManifestOnly { .. } => "manifest_only",
            PullOutcome::Failed(_) => "failed",
        }
    }

    fn detail(&self) -> String {
        match self {
            PullOutcome::Pulled { source_peer, bytes } => {
                format!("pulled {bytes} bytes from {source_peer}")
            }
            PullOutcome::NoCandidates => {
                "no peer was askable for it (none in inventory, none connected)".to_string()
            }
            PullOutcome::Miss { asked } => {
                format!("asked {asked} peer(s) we could reach; none served the bytes")
            }
            PullOutcome::ManifestOnly { source_peer } => format!(
                "found it sharded — {source_peer}; the manifest is now durable here, so the next \
                 sweep resumes the shard fetch without repeating the round-trip"
            ),
            PullOutcome::Failed(why) => format!("the pull could not be completed: {why}"),
        }
    }
}

/// One bounded attempt to bring a blob to this node from a peer.
///
/// A trait so [`PeerPullArtifactSource`] can be exercised without a swarm, and
/// so the release controller never grows a direct dependency on the swarm
/// command channel — exactly the seam `BlobStoreArtifactSource`'s doc names.
#[async_trait::async_trait]
pub trait BlobPuller: Send + Sync {
    /// Attempt to fetch `blob_cid` from peers and persist it locally.
    ///
    /// Implementations MUST verify the bytes against the address before
    /// persisting (the shared `blob_fetch` helper does), and MUST be bounded:
    /// this runs inside a controller sweep.
    ///
    /// `declared_bytes` is the manifest's own byte count for the artifact. It is
    /// a SIZING input, never a trust input — the digest check that decides
    /// whether these bytes are the right ones lives downstream in the local
    /// source and `verify_artifacts`, exactly where it did before.
    async fn pull(&self, blob_cid: &str, declared_bytes: u64) -> PullOutcome;
}

/// Bytes-per-second a peer transfer is assumed to sustain when sizing a
/// per-peer deadline. Deliberately pessimistic: the point is to stop timing out
/// a transfer that is *working*, not to predict throughput.
#[cfg(feature = "p2p")]
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
#[cfg(feature = "p2p")]
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

/// An [`ArtifactSource`] that falls back to a peer pull on a local miss.
///
/// Wraps the local source rather than replacing it: the local read is still the
/// fast path and still the idempotent one (a sweep that already staged these
/// exact bytes re-uses them). Only `artifact_unavailable` — the "the bytes are
/// simply not here" refusal — triggers a pull. Every other refusal is passed
/// through untouched, because a length or digest problem is not something
/// asking a peer again can fix.
pub struct PeerPullArtifactSource {
    local: Arc<dyn ArtifactSource>,
    puller: Arc<dyn BlobPuller>,
}

impl PeerPullArtifactSource {
    pub fn new(local: Arc<dyn ArtifactSource>, puller: Arc<dyn BlobPuller>) -> Self {
        Self { local, puller }
    }
}

#[async_trait::async_trait]
impl ArtifactSource for PeerPullArtifactSource {
    async fn fetch(
        &self,
        artifact: &Artifact,
        staging_dir: &Path,
    ) -> Result<FetchedArtifact, AdoptionRefusal> {
        let local_refusal = match self.local.fetch(artifact, staging_dir).await {
            Ok(fetched) => return Ok(fetched),
            Err(refusal) if refusal.reason_code() == RefusalReason::ArtifactUnavailable => refusal,
            // A length/digest/staging failure is not a replication problem.
            Err(other) => return Err(other),
        };

        // INFO, not debug. The first live run (2026-09-04, station 7) produced a
        // refusal nobody could diagnose because every non-landing branch here
        // logged at `debug!` while the mesh runs at INFO: a pull that ran, asked
        // real peers and missed was indistinguishable in the log from a pull
        // that never happened. A sweep-rate event that decides whether a node
        // converges is not debug-tier.
        tracing::info!(
            blob_cid = %artifact.blob_cid,
            filename = %artifact.filename,
            declared_bytes = artifact.bytes,
            staging_dir = %staging_dir.display(),
            "release-adoption: artifact not held locally — attempting one bounded peer pull"
        );
        let started = std::time::Instant::now();
        let outcome = self.puller.pull(&artifact.blob_cid, artifact.bytes).await;
        let elapsed_ms = started.elapsed().as_millis();

        if let PullOutcome::Pulled { source_peer, bytes } = &outcome {
            tracing::info!(
                blob_cid = %artifact.blob_cid,
                filename = %artifact.filename,
                source_peer = %source_peer,
                bytes,
                elapsed_ms,
                "release-adoption: pulled a release artifact from a peer on local miss"
            );
            // Re-ask the LOCAL source rather than staging the pulled bytes
            // here: the local source is what re-checks the digest and stages
            // atomically, and a second implementation of that would be a second
            // place for the "verify then stage" order to drift.
            return self.local.fetch(artifact, staging_dir).await;
        }

        tracing::warn!(
            blob_cid = %artifact.blob_cid,
            filename = %artifact.filename,
            outcome = outcome.label(),
            detail = %outcome.detail(),
            elapsed_ms,
            "release-adoption: peer pull did not land — the channel stays on the transient \
             artifact_unavailable ladder"
        );
        Err(AdoptionRefusal::new(
            RefusalReason::ArtifactUnavailable,
            format!("{} — peer pull: {}", local_refusal.detail, outcome.detail()),
        ))
    }
}

// ---------------------------------------------------------------------------
// The real puller — the EXISTING fetch machinery, nothing new
// ---------------------------------------------------------------------------

/// Pull through `p2p::blob_fetch::race_fetch` — the same helper the HTTP blob
/// handler's local-miss heal and the custody reconciliation controller already
/// use. No new protocol, no new wire message: this is the release controller
/// finally being allowed to ask.
#[cfg(feature = "p2p")]
pub struct RaceFetchPuller {
    command_tx: tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>,
    /// Peers currently askable on this plane — the `is_connected` filter
    /// `race_fetch` applies to its candidate list.
    peers: Arc<dyn crate::p2p::reconcile_peers::ReconcilePeers>,
    pool: crate::db::DbPool,
    blob_store: Arc<crate::blob_store::BlobStore>,
    /// This peer, as the `serve-blob` REA event's receiver.
    self_cid: String,
    parallelism: usize,
    per_peer_timeout: std::time::Duration,
    inventory_freshness_secs: u64,
}

#[cfg(feature = "p2p")]
impl RaceFetchPuller {
    /// Cap on candidates asked in one pull: `parallelism` × this many batches.
    /// Three batches at the default (3 peers, 5 s each) is a ~45 s worst case,
    /// which the overall deadline below then bounds properly.
    const MAX_BATCHES: usize = 3;

    /// Ceiling on the size-scaled per-peer deadline. A controller sweep runs
    /// every 60 s; one artifact may not own more than a sweep's worth of it.
    const MAX_PER_PEER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(45);

    /// The inventory freshness cutoff, in the wire format the rows carry.
    fn fresh_after(&self) -> String {
        chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(
                self.inventory_freshness_secs as i64,
            ))
            .unwrap_or_else(chrono::Utc::now)
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    }

    /// Persist composite bytes the swarm path reassembled, under their own
    /// content address, so this source's local re-read can find them.
    ///
    /// No REA event: `fetch_shards_via_swarm` already booked one per shard to
    /// the peer that actually served it. Attribution belongs to those peers, not
    /// to a synthetic composite source.
    async fn store_reassembled(&self, bytes: Vec<u8>, source_peer: &str) -> PullOutcome {
        let len = bytes.len() as u64;
        match self.blob_store.store(&bytes).await {
            Ok(_) => PullOutcome::Pulled {
                source_peer: source_peer.to_string(),
                bytes: len,
            },
            Err(e) => PullOutcome::Failed(format!("could not store reassembled bytes: {e}")),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_tx: tokio::sync::mpsc::Sender<crate::p2p::P2PCommand>,
        peers: Arc<dyn crate::p2p::reconcile_peers::ReconcilePeers>,
        pool: crate::db::DbPool,
        blob_store: Arc<crate::blob_store::BlobStore>,
        self_cid: String,
        config: &crate::config::Config,
    ) -> Self {
        let (parallelism, per_peer_timeout) =
            crate::p2p::blob_fetch::fetch_params_from_config(config);
        Self {
            command_tx,
            peers,
            pool,
            blob_store,
            self_cid,
            parallelism: parallelism.max(1),
            per_peer_timeout,
            inventory_freshness_secs: config.inventory_freshness_seconds,
        }
    }

    /// Inventory-named hosts first (evidence-ordered), then connected peers as
    /// a fallback, deduped and capped.
    ///
    /// The fallback is what makes this work for a RELEASE artifact specifically:
    /// a blob published minutes ago is routinely newer than the inventory-gossip
    /// cadence that would have named its hosts, so an inventory-only candidate
    /// list is empty precisely when the controller needs it most.
    fn candidates(&self, blob_cid: &str, connected: &[String]) -> (Vec<String>, usize) {
        let mut inventory: Vec<String> = Vec::new();
        if let Ok(mut conn) = self.pool.get() {
            let fresh_after = self.fresh_after();
            // Both spellings of the SAME digest. The manifest addresses the
            // artifact as a CIDv1 (`bafkrei…`) while a gossiped inventory row
            // may carry the legacy `sha256-<hex>` marker for the identical
            // bytes; `lookup_hosts` matches the column literally, so asking for
            // only one form silently drops the hosts recorded under the other.
            let mut forms = vec![blob_cid.to_string()];
            if let Some(hex) = crate::p2p::blob_fetch::content_address_hex(blob_cid) {
                let legacy = format!("sha256-{hex}");
                if legacy != blob_cid {
                    forms.push(legacy);
                }
                if hex != blob_cid {
                    forms.push(hex);
                }
            }
            for form in forms {
                if let Ok(rows) =
                    crate::db::peer_blob_inventory::lookup_hosts(&mut conn, &form, &fresh_after)
                {
                    inventory.extend(rows.into_iter().map(|r| r.peer_id));
                }
            }
        }
        let inventory_named = inventory.len();
        (
            merge_candidates(inventory, connected, self.parallelism * Self::MAX_BATCHES),
            inventory_named,
        )
    }
}

/// **Pure.** Inventory-named hosts first (evidence-ordered), then connected
/// peers as a fallback, deduped in order and capped.
///
/// The connected fallback is what makes this work for a RELEASE artifact
/// specifically: a blob published minutes ago is routinely newer than the ~60 s
/// inventory-gossip cadence that would have named its hosts, so an
/// inventory-only candidate list is empty precisely when the controller needs it
/// most. Isolated from the DB read so both the empty case and the fallback case
/// are testable without a pool or a swarm.
#[cfg(feature = "p2p")]
fn merge_candidates(inventory: Vec<String>, connected: &[String], cap: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for peer in inventory.into_iter().chain(connected.iter().cloned()) {
        if !out.iter().any(|p| *p == peer) {
            out.push(peer);
        }
    }
    out.truncate(cap);
    out
}

#[cfg(feature = "p2p")]
#[async_trait::async_trait]
impl BlobPuller for RaceFetchPuller {
    async fn pull(&self, blob_cid: &str, declared_bytes: u64) -> PullOutcome {
        use crate::p2p::blob_fetch::finalize_fetch_success;
        use crate::p2p::blob_swarm::{race_fetch_with_swarm, SwarmFetchParams, SwarmRaceOutcome};

        let connected: Vec<String> = self
            .peers
            .list_peers()
            .await
            .into_iter()
            .map(|p| p.peer_id)
            .collect();
        let (candidates, inventory_named) = self.candidates(blob_cid, &connected);
        if candidates.is_empty() {
            // WARN, and it names both sources: an empty candidate list is a
            // discovery failure, not a replication failure, and reporting the
            // two the same way is how a silent no-op reads as "not replicated
            // yet" forever.
            tracing::warn!(
                blob_cid = %blob_cid,
                inventory_named,
                connected_peers = connected.len(),
                "release-adoption: peer pull has NO candidates — no inventory row names a host \
                 for this blob and no peer is connected on this plane"
            );
            return PullOutcome::NoCandidates;
        }
        // A per-peer deadline sized to the object, not to the small-blob heal
        // this timeout was originally tuned for. See `size_aware_timeout`.
        let per_peer = size_aware_timeout(
            declared_bytes,
            self.per_peer_timeout,
            Self::MAX_PER_PEER_TIMEOUT,
        );
        let asked = candidates.len();
        tracing::info!(
            blob_cid = %blob_cid,
            declared_bytes,
            candidates = asked,
            inventory_named,
            connected_peers = connected.len(),
            parallelism = self.parallelism,
            per_peer_timeout_s = per_peer.as_secs(),
            configured_timeout_s = self.per_peer_timeout.as_secs(),
            "release-adoption: peer pull asking peers for a release artifact"
        );
        let connected_set: std::collections::HashSet<String> = connected.into_iter().collect();

        let mut conn = match self.pool.get() {
            Ok(c) => c,
            Err(e) => return PullOutcome::Failed(format!("db pool exhausted: {e}")),
        };
        let fresh_after = self.fresh_after();
        let params = SwarmFetchParams {
            cmd_tx: &self.command_tx,
            connected: &connected_set,
            per_shard_parallelism: self.parallelism,
            per_peer_timeout: per_peer,
            // The same modest fan-out bound the HTTP GET-miss heal and the
            // bytes-heal path use (`parallelism * 4`) — this rung introduces no
            // new concurrency dial.
            total_inflight: self.parallelism.max(1) * 4,
            self_cid: &self.self_cid,
            blob_store: &self.blob_store,
        };

        // `race_fetch_with_swarm`, NOT `race_fetch`: it is the manifest-aware
        // superset — a direct hit still resolves immediately, and a manifest
        // reply is persisted and FOLLOWED into an independent per-shard swarm
        // fetch rather than reported as a dead end. That is the whole cure for
        // the live station-7 miss: every peer held the 10 MB artifact SHARDED
        // and answered with a manifest, which the composite-level race alone can
        // only read as "nobody served it".
        let deadline = per_peer * (Self::MAX_BATCHES as u32 + 1);
        let outcome = match tokio::time::timeout(
            deadline,
            race_fetch_with_swarm(blob_cid, candidates, &mut conn, &fresh_after, &params),
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(_) => {
                return PullOutcome::Failed(format!(
                    "no peer completed the transfer within {}s (per-peer {}s)",
                    deadline.as_secs(),
                    per_peer.as_secs()
                ))
            }
        };

        match outcome {
            // Reassembled from independently-fetched shards. Each shard was
            // already persisted and REA-booked to its REAL source peer inside
            // `fetch_shards_via_swarm`, so finalizing again under the synthetic
            // "swarm" peer would double-book a delivery that never happened from
            // one peer (the same rule the bytes-heal call site honours).
            //
            // The composite is deliberately never stored under its own name by
            // the swarm path — the HTTP reader reassembles on demand. This
            // source's local re-read goes through `BlobStore::get_by_address`,
            // which does NOT reassemble, so the composite is stored here. That
            // is safe by construction: `store` is CONTENT-ADDRESSED, so bytes
            // that are not what they claim land under a different address and
            // the re-read still misses.
            SwarmRaceOutcome::Hit { bytes, source_peer } if source_peer == "swarm" => {
                self.store_reassembled(bytes, "swarm").await
            }
            SwarmRaceOutcome::Reconstructible {
                bytes,
                landed,
                missing,
            } => {
                tracing::info!(
                    blob_cid = %blob_cid,
                    landed,
                    missing,
                    "release-adoption: artifact reassembled from an RS data-shard floor — \
                     servable now, parity fills by ordinary salvage"
                );
                self.store_reassembled(bytes, "swarm(reconstructed)").await
            }
            // A single peer served the whole composite: book the delivery to it,
            // exactly as before.
            SwarmRaceOutcome::Hit { bytes, source_peer } => {
                let len = bytes.len() as u64;
                match finalize_fetch_success(
                    &mut conn,
                    blob_cid,
                    &source_peer,
                    &bytes,
                    &self.self_cid,
                    &self.blob_store,
                )
                .await
                {
                    Ok(()) => PullOutcome::Pulled {
                        source_peer,
                        bytes: len,
                    },
                    Err(e) => PullOutcome::Failed(format!("could not persist pulled bytes: {e}")),
                }
            }
            // The manifest is now DURABLE locally even though a shard is still
            // missing, so the next sweep resumes without repeating the manifest
            // round-trip — and ordinary replication may land the gap meanwhile.
            // Transient, and it says which peer to look at.
            SwarmRaceOutcome::ManifestPersistedIncomplete {
                manifest,
                missing_shards,
            } => PullOutcome::ManifestOnly {
                source_peer: format!(
                    "a holder of {} ({missing_shards} shard(s) still missing; manifest persisted)",
                    manifest.blob_hash
                ),
            },
            SwarmRaceOutcome::Miss => PullOutcome::Miss { asked },
            SwarmRaceOutcome::NoCandidates => PullOutcome::NoCandidates,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn artifact() -> Artifact {
        Artifact {
            blob_cid: "bafkreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            bytes: 11,
            sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
            filename: "coordinators.happ".to_string(),
            mime_type: None,
            role: None,
        }
    }

    /// A local source that misses until the puller "lands" the bytes.
    struct FlippingLocal {
        present: std::sync::atomic::AtomicBool,
        reads: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl ArtifactSource for FlippingLocal {
        async fn fetch(
            &self,
            artifact: &Artifact,
            _staging_dir: &Path,
        ) -> Result<FetchedArtifact, AdoptionRefusal> {
            self.reads.fetch_add(1, Ordering::SeqCst);
            if self.present.load(Ordering::SeqCst) {
                Ok(FetchedArtifact {
                    blob_cid: artifact.blob_cid.clone(),
                    path: std::path::PathBuf::from("/tmp/staged"),
                    bytes: artifact.bytes,
                    sha256: artifact.sha256.clone(),
                })
            } else {
                Err(AdoptionRefusal::new(
                    RefusalReason::ArtifactUnavailable,
                    "blob is not held locally",
                ))
            }
        }
    }

    /// A local source whose refusal is NOT about replication.
    struct DigestMismatchLocal;

    #[async_trait::async_trait]
    impl ArtifactSource for DigestMismatchLocal {
        async fn fetch(
            &self,
            _artifact: &Artifact,
            _staging_dir: &Path,
        ) -> Result<FetchedArtifact, AdoptionRefusal> {
            Err(AdoptionRefusal::new(
                RefusalReason::ArtifactDigestMismatch,
                "substituted bytes",
            ))
        }
    }

    struct FakePuller {
        outcome: PullOutcome,
        lands: Option<Arc<FlippingLocal>>,
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl BlobPuller for FakePuller {
        async fn pull(&self, _blob_cid: &str, _declared_bytes: u64) -> PullOutcome {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if let Some(local) = self.lands.as_ref() {
                local.present.store(true, Ordering::SeqCst);
            }
            self.outcome.clone()
        }
    }

    /// **The cure.** Local miss → one pull → served. The bytes are re-read
    /// through the LOCAL source so the digest check and the atomic staging
    /// happen exactly once, in one place.
    #[tokio::test]
    async fn a_local_miss_is_pulled_from_a_peer_and_then_served() {
        let local = Arc::new(FlippingLocal {
            present: std::sync::atomic::AtomicBool::new(false),
            reads: AtomicUsize::new(0),
        });
        let puller = Arc::new(FakePuller {
            outcome: PullOutcome::Pulled {
                source_peer: "12D3KooWSomePeer".to_string(),
                bytes: 11,
            },
            lands: Some(local.clone()),
            calls: AtomicUsize::new(0),
        });
        let source = PeerPullArtifactSource::new(local.clone(), puller.clone());

        let fetched = source
            .fetch(&artifact(), Path::new("/tmp/staging"))
            .await
            .expect("the pulled bytes are served");
        assert_eq!(fetched.bytes, 11);
        assert_eq!(
            puller.calls.load(Ordering::SeqCst),
            1,
            "exactly one attempt"
        );
        assert_eq!(
            local.reads.load(Ordering::SeqCst),
            2,
            "miss, then the re-read that stages and digest-checks"
        );
    }

    /// A pull that does not land leaves TODAY's behaviour in place: the same
    /// transient `artifact_unavailable`, so the ladder still governs and the
    /// next sweep still asks.
    #[tokio::test]
    async fn a_failed_pull_keeps_the_transient_artifact_unavailable_refusal() {
        for outcome in [
            PullOutcome::NoCandidates,
            PullOutcome::Miss { asked: 2 },
            PullOutcome::ManifestOnly {
                source_peer: "12D3KooWShardedPeer".to_string(),
            },
            PullOutcome::Failed("db pool exhausted".to_string()),
        ] {
            let local = Arc::new(FlippingLocal {
                present: std::sync::atomic::AtomicBool::new(false),
                reads: AtomicUsize::new(0),
            });
            let puller = Arc::new(FakePuller {
                outcome: outcome.clone(),
                lands: None,
                calls: AtomicUsize::new(0),
            });
            let source = PeerPullArtifactSource::new(local.clone(), puller.clone());

            let refusal = source
                .fetch(&artifact(), Path::new("/tmp/staging"))
                .await
                .expect_err("a pull that did not land is still unavailable");
            assert_eq!(refusal.reason_code(), RefusalReason::ArtifactUnavailable);
            assert!(
                refusal.transient,
                "replication that has not happened yet may still happen"
            );
            assert!(
                refusal.detail.contains("peer pull:"),
                "the refusal says what the pull did: {}",
                refusal.detail
            );
            assert_eq!(puller.calls.load(Ordering::SeqCst), 1);
            assert_eq!(
                local.reads.load(Ordering::SeqCst),
                1,
                "no pointless second local read when nothing landed"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Candidate selection (2026-09-04, from the live station-7 diagnosis)
    // -----------------------------------------------------------------------

    /// **The fallback that matters.** A release blob published minutes ago is
    /// newer than the ~60 s inventory-gossip cadence, so no row names a host for
    /// it — and inventory-only selection would ask nobody at exactly the moment
    /// the controller needs the bytes. Connected peers carry the pull instead.
    #[cfg(feature = "p2p")]
    #[test]
    fn connected_peers_carry_the_pull_when_inventory_names_nobody() {
        let connected = vec!["12D3KooWMatthew".to_string(), "12D3KooWJessica".to_string()];
        let candidates = merge_candidates(Vec::new(), &connected, 9);
        assert_eq!(candidates, connected, "both connected peers get asked");
    }

    /// Inventory evidence outranks the fallback and is never duplicated by it.
    #[cfg(feature = "p2p")]
    #[test]
    fn inventory_named_hosts_come_first_and_are_not_duplicated() {
        let connected = vec!["12D3KooWMatthew".to_string(), "12D3KooWJessica".to_string()];
        let candidates = merge_candidates(vec!["12D3KooWJessica".to_string()], &connected, 9);
        assert_eq!(
            candidates,
            vec!["12D3KooWJessica".to_string(), "12D3KooWMatthew".to_string()],
            "the inventory-named host leads; the fallback appends only what is new"
        );
    }

    /// Nothing to ask is NOT the same fact as asked-and-missed — the empty case
    /// must stay reachable so the `no_candidates` warn can fire.
    #[cfg(feature = "p2p")]
    #[test]
    fn no_inventory_and_no_connected_peers_is_an_empty_candidate_list() {
        assert!(merge_candidates(Vec::new(), &[], 9).is_empty());
    }

    /// **The 5-second trap.** A 10 MB artifact under the small-blob default
    /// deadline is indistinguishable from a dead peer; the deadline must scale
    /// with the object. The configured value stays a FLOOR (an operator who
    /// raises it raises it for everything), and the result is capped so one
    /// artifact can never own a whole sweep.
    #[cfg(feature = "p2p")]
    #[test]
    fn the_per_peer_deadline_scales_with_the_object_but_stays_bounded() {
        use std::time::Duration;
        let floor = Duration::from_secs(5);
        let cap = Duration::from_secs(45);

        // A small artifact keeps the configured floor.
        assert_eq!(size_aware_timeout(1024, floor, cap), floor);
        // The live station-7 artifact: 10,104,227 bytes at 1 MiB/s ≈ 10 s.
        assert_eq!(
            size_aware_timeout(10_104_227, floor, cap),
            Duration::from_secs(10),
            "the 10 MB artifact that missed at 5s now gets a deadline it can finish in"
        );
        // Absurdly large stays bounded by the cap.
        assert_eq!(size_aware_timeout(u64::MAX, floor, cap), cap);
        // A raised floor is honoured even above the cap — the operator's knob
        // is never silently shrunk.
        assert_eq!(
            size_aware_timeout(1024, Duration::from_secs(90), cap),
            Duration::from_secs(90)
        );
    }

    /// The cap is a bound on work per sweep, applied after ordering so evidence
    /// is never dropped in favour of a fallback peer.
    #[cfg(feature = "p2p")]
    #[test]
    fn the_candidate_cap_bounds_one_pull() {
        let many: Vec<String> = (0..20).map(|i| format!("peer{i}")).collect();
        let candidates = merge_candidates(many.clone(), &[], 9);
        assert_eq!(candidates.len(), 9);
        assert_eq!(candidates[0], many[0], "ordering survives the cap");
    }

    /// A digest mismatch is not a replication problem — asking a peer again
    /// cannot fix substituted bytes, so no pull is attempted and the typed
    /// refusal passes through unchanged.
    #[tokio::test]
    async fn a_non_replication_refusal_never_triggers_a_pull() {
        let puller = Arc::new(FakePuller {
            outcome: PullOutcome::Miss { asked: 1 },
            lands: None,
            calls: AtomicUsize::new(0),
        });
        let source = PeerPullArtifactSource::new(Arc::new(DigestMismatchLocal), puller.clone());

        let refusal = source
            .fetch(&artifact(), Path::new("/tmp/staging"))
            .await
            .expect_err("a digest mismatch is still a digest mismatch");
        assert_eq!(refusal.reason_code(), RefusalReason::ArtifactDigestMismatch);
        assert_eq!(
            puller.calls.load(Ordering::SeqCst),
            0,
            "the pull is for missing bytes, never for wrong ones"
        );
    }
}
