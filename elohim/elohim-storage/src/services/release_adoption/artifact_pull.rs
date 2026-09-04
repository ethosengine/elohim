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
//! - **The fetch's own timeout**, per peer, from `Config::fetch_blob_timeout_seconds`
//!   — plus an overall deadline across all batches, so one sweep can never park
//!   on a slow pull for longer than the sweep interval.
//! - **A candidate cap.** Inventory-named hosts first (evidence-ordered, the
//!   same rows `GET /blob/{hash}` heals from), then currently-connected peers as
//!   a fallback — capped, because a release artifact is frequently NEWER than
//!   the ~60 s inventory-gossip cadence that would have named its hosts.

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
    Miss,
    /// The pull could not be attempted or errored out (no DB pool, a malformed
    /// address, a deadline). Carries the operator-readable reason.
    Failed(String),
}

impl PullOutcome {
    fn detail(&self) -> String {
        match self {
            PullOutcome::Pulled { source_peer, bytes } => {
                format!("pulled {bytes} bytes from {source_peer}")
            }
            PullOutcome::NoCandidates => {
                "no peer was askable for it (none in inventory, none connected)".to_string()
            }
            PullOutcome::Miss => "asked the peers we could reach; none served it".to_string(),
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
    async fn pull(&self, blob_cid: &str) -> PullOutcome;
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

        let outcome = self.puller.pull(&artifact.blob_cid).await;
        if let PullOutcome::Pulled { source_peer, bytes } = &outcome {
            tracing::info!(
                blob_cid = %artifact.blob_cid,
                filename = %artifact.filename,
                source_peer = %source_peer,
                bytes,
                "release-adoption: pulled a release artifact from a peer on local miss"
            );
            // Re-ask the LOCAL source rather than staging the pulled bytes
            // here: the local source is what re-checks the digest and stages
            // atomically, and a second implementation of that would be a second
            // place for the "verify then stage" order to drift.
            return self.local.fetch(artifact, staging_dir).await;
        }

        tracing::debug!(
            blob_cid = %artifact.blob_cid,
            outcome = ?outcome,
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
    fn candidates(&self, blob_cid: &str, connected: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Ok(mut conn) = self.pool.get() {
            let fresh_after = chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::seconds(
                    self.inventory_freshness_secs as i64,
                ))
                .unwrap_or_else(chrono::Utc::now)
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string();
            if let Ok(rows) =
                crate::db::peer_blob_inventory::lookup_hosts(&mut conn, blob_cid, &fresh_after)
            {
                out.extend(rows.into_iter().map(|r| r.peer_id));
            }
        }
        for peer in connected {
            if !out.iter().any(|p| p == peer) {
                out.push(peer.clone());
            }
        }
        out.truncate(self.parallelism * Self::MAX_BATCHES);
        out
    }
}

#[cfg(feature = "p2p")]
#[async_trait::async_trait]
impl BlobPuller for RaceFetchPuller {
    async fn pull(&self, blob_cid: &str) -> PullOutcome {
        use crate::p2p::blob_fetch::{finalize_fetch_success, race_fetch, FetchOutcome};

        let connected: Vec<String> = self
            .peers
            .list_peers()
            .await
            .into_iter()
            .map(|p| p.peer_id)
            .collect();
        let candidates = self.candidates(blob_cid, &connected);
        if candidates.is_empty() {
            return PullOutcome::NoCandidates;
        }
        let connected_set: std::collections::HashSet<String> = connected.into_iter().collect();
        let is_connected = move |peer: &str| connected_set.contains(peer);

        // Overall deadline across every batch. `race_fetch` bounds each PEER,
        // not the whole walk; a controller sweep must not be able to park on
        // one artifact for longer than the sweep it belongs to.
        let deadline = self.per_peer_timeout * (Self::MAX_BATCHES as u32 + 1);
        let outcome = match tokio::time::timeout(
            deadline,
            race_fetch(
                blob_cid,
                candidates,
                &self.command_tx,
                is_connected,
                self.parallelism,
                self.per_peer_timeout,
            ),
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(_) => {
                return PullOutcome::Failed(format!(
                    "no peer answered within {}s",
                    deadline.as_secs()
                ))
            }
        };

        match outcome {
            FetchOutcome::Hit { bytes, source_peer } => {
                let len = bytes.len() as u64;
                let mut conn = match self.pool.get() {
                    Ok(c) => c,
                    Err(e) => return PullOutcome::Failed(format!("db pool exhausted: {e}")),
                };
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
            // A manifest answer means the bytes are sharded elsewhere; the
            // ordinary replication path owns reassembly. Honest miss for us.
            FetchOutcome::Manifest { .. } | FetchOutcome::Miss => PullOutcome::Miss,
            FetchOutcome::NoCandidates => PullOutcome::NoCandidates,
            FetchOutcome::InvalidAddress => PullOutcome::Failed(
                "the manifest's blobCid is not a content address any peer can accept".to_string(),
            ),
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
        async fn pull(&self, _blob_cid: &str) -> PullOutcome {
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
            PullOutcome::Miss,
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

    /// A digest mismatch is not a replication problem — asking a peer again
    /// cannot fix substituted bytes, so no pull is attempted and the typed
    /// refusal passes through unchanged.
    #[tokio::test]
    async fn a_non_replication_refusal_never_triggers_a_pull() {
        let puller = Arc::new(FakePuller {
            outcome: PullOutcome::Miss,
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
