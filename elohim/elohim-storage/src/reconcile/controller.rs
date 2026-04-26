//! `ReconcileController` — k8s-style reconciliation loop for DHT signal dispatch.
//!
//! ## Principle P1 — elohim-storage as reconciliation controller
//!
//! The controller subscribes to the [`DnaSignalStream`] and eagerly projects
//! each signal into local state. It does **not** poll; the stream drives it.
//! Reconciliation is eager: each signal is handled as soon as it arrives.
//!
//! ## Lifecycle
//!
//! ```text
//! imagodei DNA (post-commit)
//!     └─► DnaSignal (via DnaSignalStream)
//!             │
//!             │  ReconcileController::dispatch
//!             ├─► on_key_rotation           (Task A.6 — pubkey timeline cache)
//!             ├─► on_key_revocation         (Task A.8 — sweep + cache invalidate)
//!             ├─► on_agent_peer_binding     (Task A.5 — peer_identity_bindings table)
//!             ├─► on_revocation_attestation (Task A.10 — revocation_votes projection)
//!             ├─► on_portal_host_created    (M5 — portal_hosts upsert)
//!             └─► on_portal_host_removed    (M5 — portal_hosts delete)
//! ```
//!
//! ## Current state
//!
//! This is the A.4 skeleton. All four handlers are **no-op stubs**; they record
//! the signal kind in `observed_kinds` for test introspection and return `Ok(())`.
//! Handler implementations are filled in by Tasks A.5, A.6, A.8, and A.10.

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

use crate::db::DbPool;
use crate::p2p::P2PCommand;
use crate::projector::sweep::sweep_projections_on_revocation;
use crate::projector::ManifestRegistry;
use crate::reconcile::portal_host_handlers;
use crate::reconcile::pubkey_timeline::PubkeyTimelineCache;
use crate::reconcile::signal_stream::{
    AgentPeerBindingSignal, DnaSignal, DnaSignalStream, KeyRevocationSignal, KeyRotationSignal,
    PortalHostCreatedSignal, PortalHostRemovedSignal, RevocationAttestationSignal,
};
use crate::reconcile::sweep::sweep_on_revocation;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur in the reconcile controller loop.
#[derive(Debug, Error)]
pub enum ReconcileError {
    #[error("signal stream error: {0}")]
    Stream(String),

    /// A.8: revocation sweep DB write failed.
    #[error("revocation sweep failed: {0}")]
    Sweep(String),

    /// A.8: r2d2 pool checkout failed.
    #[error("db pool error: {0}")]
    Pool(String),
}

// ---------------------------------------------------------------------------
// ReconcileController
// ---------------------------------------------------------------------------

/// K8s-style controller that consumes a [`DnaSignalStream`] and routes each
/// signal to a kind-specific handler.
///
/// ## Generics
///
/// `S` is the stream implementation. In production (Task A.11) this will be
/// `HolochainAppSignalStream`. In unit tests it is [`InMemoryDnaSignalStream`]
/// or [`ChannelSignalStream`].
///
/// ## State evolution
///
/// | Task | State added |
/// |------|-------------|
/// | A.5  | `binding_cache: Arc<RwLock<PeerBindingCache>>` |
/// | A.6  | `pubkey_cache: PubkeyTimelineCache` |
/// | A.8  | `db_pool` + `pubkey_cache` (sweep + cache invalidate) |
/// | A.10 | Uses `db_pool` for revocation_votes projection |
/// | A.11 | `db_pool` wired from `HttpServer` / `Services` startup |
/// | M5   | Uses `db_pool` for portal_hosts upsert/delete |
pub struct ReconcileController<S: DnaSignalStream> {
    stream: S,

    /// Test-introspection accumulator — records the signal kinds dispatched
    /// in order. Grows unbounded; not suitable for long-running production use
    /// without replacement. Later tasks will augment or supersede this with
    /// per-handler state (caches, counters, last-cursor).
    ///
    /// Expected to evolve: as Tasks A.5/A.6/A.8/A.10 add real handler state,
    /// this field may be moved to a `#[cfg(test)]`-only extension or removed.
    observed_kinds: Vec<String>,

    /// A.8: optional r2d2 pool for DB sweep operations.
    ///
    /// `None` in test stubs that don't need persistence (the no-storage
    /// `new()` constructor). `Some(...)` when wired via `new_with_storage`.
    db_pool: Option<Arc<DbPool>>,

    /// A.8: optional pubkey timeline cache for per-agent invalidation.
    ///
    /// Wrapped in `Arc<Mutex<...>>` so the cache can be shared between the
    /// controller task and the verify path (Task A.7). The `Mutex` is Tokio's
    /// async variant; cache ops are short so there is no contention concern.
    pubkey_cache: Option<Arc<Mutex<PubkeyTimelineCache>>>,

    /// A.10: optional P2P command sender for gossipsub publish.
    ///
    /// When `Some(tx)`, `on_agent_peer_binding` sends a
    /// `P2PCommand::PublishIdentityBinding` to propagate the new binding to
    /// subscribed peers. `None` in test stubs that don't need live P2P
    /// (e.g. the A.4 suite constructed via `new()` / `new_with_storage()`).
    ///
    /// Set via `with_swarm_tx(tx)`. In production the sender is extracted from
    /// the `P2PHandle` (Task A.11 wiring). The channel is unbounded to avoid
    /// blocking the controller loop — the P2P event loop drains it promptly.
    swarm_tx: Option<mpsc::Sender<P2PCommand>>,

    /// B.6: optional manifest registry for projection revocation sweep.
    ///
    /// When `Some(registry)`, `on_key_revocation` also calls
    /// `sweep_projections_on_revocation` after the `epr_atoms` sweep, clearing
    /// `verified_at` on any projection rows whose source EPR atoms were tainted
    /// by the revocation (Invariant I4).
    ///
    /// `None` in controllers constructed without a registry. In that case the
    /// projection sweep is silently skipped (logged at debug level). Set via
    /// `with_manifest_registry(registry)`. Existing constructors are unchanged.
    manifest_registry: Option<Arc<ManifestRegistry>>,
}

impl<S: DnaSignalStream> ReconcileController<S> {
    /// Construct a controller subscribing to `stream`, without DB or cache.
    ///
    /// Suitable for stub-only test controllers that don't need persistence.
    /// The A.4 test suite uses this constructor; it remains unchanged.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            observed_kinds: Vec::new(),
            db_pool: None,
            pubkey_cache: None,
            swarm_tx: None,
            manifest_registry: None,
        }
    }

    /// Construct a controller with DB pool and pubkey cache wired in (A.8).
    ///
    /// Use this constructor in production (Task A.11 startup) and in tests
    /// that exercise the full sweep + cache-invalidate path.
    pub fn new_with_storage(
        stream: S,
        db_pool: Arc<DbPool>,
        pubkey_cache: Arc<Mutex<PubkeyTimelineCache>>,
    ) -> Self {
        Self {
            stream,
            observed_kinds: Vec::new(),
            db_pool: Some(db_pool),
            pubkey_cache: Some(pubkey_cache),
            swarm_tx: None,
            manifest_registry: None,
        }
    }

    /// Wire a P2P command sender so the controller can publish gossip messages
    /// when DHT signals arrive (Task A.10).
    ///
    /// The sender is cloned from the `P2PHandle`'s command channel. Using
    /// `Option` preserves backward compatibility — the A.4 unit-test suite
    /// constructs controllers without a live swarm. When `None`, gossip publish
    /// is silently skipped (logged at debug level).
    pub fn with_swarm_tx(mut self, tx: mpsc::Sender<P2PCommand>) -> Self {
        self.swarm_tx = Some(tx);
        self
    }

    /// Wire a manifest registry so the controller clears projection rows on
    /// key revocation (Task B.6 / Invariant I4).
    ///
    /// When set, `on_key_revocation` calls `sweep_projections_on_revocation`
    /// after the `epr_atoms` sweep, atomically clearing `verified_at` on any
    /// projection rows whose source EPR atoms were tainted by the revocation.
    ///
    /// Preserves backward compatibility: without this call the projection sweep
    /// is silently skipped (logged at debug level). Existing constructors and
    /// test suites do not need modification.
    pub fn with_manifest_registry(mut self, registry: Arc<ManifestRegistry>) -> Self {
        self.manifest_registry = Some(registry);
        self
    }

    // -----------------------------------------------------------------------
    // Public loop entry points
    // -----------------------------------------------------------------------

    /// Drain all currently-available signals from the stream.
    ///
    /// Returns once `stream.next_signal()` yields `None` (stream closed or
    /// pre-loaded batch exhausted). This is the primary entry point for unit
    /// tests and for one-shot reconciliation passes on startup.
    pub async fn run_one_pass(&mut self) -> Result<(), ReconcileError> {
        while let Some(signal) = self.stream.next_signal().await {
            self.dispatch(signal).await?;
        }
        Ok(())
    }

    /// Block until the stream closes, dispatching every signal as it arrives.
    ///
    /// For production use: spawn as a `tokio::task` at storage startup.
    /// Task A.11 wires this into the `HttpServer` / `Services` startup sequence.
    ///
    /// Terminates cleanly when the stream returns `None` (conductor disconnects
    /// or channel sender is dropped). Callers that need reconnection/backoff
    /// should wrap this in an outer retry loop (not implemented here).
    pub async fn run_loop(&mut self) -> Result<(), ReconcileError> {
        // Same shape as run_one_pass for now.
        // Tasks may add backoff, reconnection, or shutdown signals later.
        self.run_one_pass().await
    }

    // -----------------------------------------------------------------------
    // Test introspection
    // -----------------------------------------------------------------------

    /// Returns the ordered list of signal kinds dispatched since construction.
    ///
    /// For test assertions only. In production this will either be removed or
    /// replaced by per-handler counters exposed through a health/metrics endpoint.
    ///
    /// Expected to evolve: later tasks add per-handler state; this accessor
    /// may be narrowed to `#[cfg(test)]` once real observability lands.
    pub fn observed_kinds(&self) -> &[String] {
        &self.observed_kinds
    }

    // -----------------------------------------------------------------------
    // Internal dispatch
    // -----------------------------------------------------------------------

    async fn dispatch(&mut self, signal: DnaSignal) -> Result<(), ReconcileError> {
        match signal {
            DnaSignal::KeyRotation(r) => {
                debug!(agent_cid = %r.agent_cid, "dispatching KeyRotation signal");
                self.observed_kinds.push("keyRotation".into());
                self.on_key_rotation(r).await
            }
            DnaSignal::KeyRevocation(r) => {
                debug!(agent_cid = %r.agent_cid, "dispatching KeyRevocation signal");
                self.observed_kinds.push("keyRevocation".into());
                self.on_key_revocation(r).await
            }
            DnaSignal::AgentPeerBinding(b) => {
                debug!(peer_id = %b.peer_id, agent_cid = %b.agent_cid, "dispatching AgentPeerBinding signal");
                self.observed_kinds.push("agentPeerBinding".into());
                self.on_agent_peer_binding(b).await
            }
            DnaSignal::RevocationAttestation(a) => {
                debug!(revocation_id = %a.revocation_id, "dispatching RevocationAttestation signal");
                self.observed_kinds.push("revocationAttestation".into());
                self.on_revocation_attestation(a).await
            }
            DnaSignal::PortalHostCreated(p) => {
                debug!(host_url = %p.host_url, "dispatching PortalHostCreated signal");
                self.observed_kinds.push("portalHostCreated".into());
                self.on_portal_host_created(p).await
            }
            DnaSignal::PortalHostRemoved(p) => {
                debug!(host_url = %p.host_url, "dispatching PortalHostRemoved signal");
                self.observed_kinds.push("portalHostRemoved".into());
                self.on_portal_host_removed(p).await
            }
        }
    }

    // -----------------------------------------------------------------------
    // Stub handlers — replaced by later batch tasks
    //
    // Each handler is intentionally a no-op. The controller's sole
    // responsibility at this stage is to demonstrate correct routing.
    // -----------------------------------------------------------------------

    /// STUB — Task A.6: update pubkey timeline cache.
    ///
    /// Will insert/update the agent's pubkey timeline so EPR verification
    /// can resolve which key was active at a given timestamp.
    async fn on_key_rotation(&mut self, _signal: KeyRotationSignal) -> Result<(), ReconcileError> {
        // Task A.6 replaces this stub.
        Ok(())
    }

    /// Task A.8: invalidate pubkey cache + sweep epr_atoms.
    ///
    /// ## Steps
    ///
    /// 1. Invalidate the per-agent pubkey timeline cache entry so the next
    ///    EPR verify reloads fresh key history from the DB (preventing stale
    ///    cache from approving tainted envelopes after revocation).
    ///
    /// 2. Run `sweep_on_revocation` against `epr_atoms`: clear `verified_at`
    ///    and set `verified_signer_fingerprint = "revoked_stale"` for any atom
    ///    whose `signer_cid` matches the revoked agent AND whose `issued_at >=
    ///    compromise_at`. Pre-compromise atoms are untouched.
    ///
    /// Steps 1 and 2 are intentionally ordered: invalidate cache before sweep
    /// so any concurrent verify request that races with the sweep sees a cache
    /// miss and must re-check the DB rather than returning a stale hit.
    ///
    /// ## No-storage path
    ///
    /// If the controller was constructed with `new()` (no pool/cache), the
    /// method logs a warning and returns `Ok(())` — preserving backward
    /// compatibility with the A.4 test suite that doesn't wire storage.
    async fn on_key_revocation(
        &mut self,
        signal: KeyRevocationSignal,
    ) -> Result<(), ReconcileError> {
        // Step 1: invalidate the pubkey timeline cache for the revoked agent.
        // Do this BEFORE the sweep so racing verify calls see a cache miss.
        if let Some(cache) = &self.pubkey_cache {
            cache.lock().await.invalidate(&signal.agent_cid);
            debug!(agent_cid = %signal.agent_cid, "pubkey timeline cache invalidated on revocation");
        }

        // Step 2: sweep epr_atoms if a DB pool is available.
        if let Some(pool) = &self.db_pool {
            let pool = Arc::clone(pool);
            let signal_clone = signal.clone();
            let registry_clone = self.manifest_registry.clone();

            // The r2d2 pool is sync. We use block_in_place to avoid blocking
            // the Tokio runtime's async thread while holding the connection.
            let (atom_report, projection_report) = tokio::task::block_in_place(move || {
                let mut conn = pool
                    .get()
                    .map_err(|e| ReconcileError::Pool(e.to_string()))?;

                // Step 2a: sweep epr_atoms.
                let atom_report = sweep_on_revocation(&mut conn, &signal_clone)
                    .map_err(|e| ReconcileError::Sweep(e.to_string()))?;

                // Step 2b: sweep projection rows if a registry is wired (B.6/I4).
                let effective_at_str = signal_clone
                    .effective_at
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string();

                let proj_report = if let Some(registry) = registry_clone {
                    sweep_projections_on_revocation(
                        &mut conn,
                        &registry,
                        &signal_clone.agent_cid,
                        &effective_at_str,
                    )
                    .map_err(|e| ReconcileError::Sweep(format!("projection sweep: {e}")))?
                } else {
                    debug!(
                        agent_cid = %signal_clone.agent_cid,
                        "KeyRevocation: no manifest_registry wired — projection sweep skipped \
                         (use with_manifest_registry for full I4 compliance)"
                    );
                    crate::projector::ProjectionSweepReport::default()
                };

                Ok::<_, ReconcileError>((atom_report, proj_report))
            })?;

            info!(
                agent_cid = %atom_report.agent_cid,
                affected_rows = atom_report.affected_rows,
                compromise_at = %atom_report.compromise_at,
                swept_at = %atom_report.at,
                projection_tables_updated = projection_report.tables_updated,
                projection_rows_cleared = projection_report.rows_cleared,
                "revocation sweep complete"
            );
        } else {
            warn!(
                agent_cid = %signal.agent_cid,
                revoked_pubkey = %signal.revoked_pubkey,
                compromise_at = %signal.compromise_at,
                "KeyRevocation received but no db_pool wired — sweep skipped (use new_with_storage)"
            );
        }

        Ok(())
    }

    /// Task A.10: publish AgentPeerBinding to the `elohim/identity/binding` gossipsub topic.
    ///
    /// When a local agent creates a new `AgentPeerBinding` DHT entry, the DNA
    /// emits an `AgentPeerBinding` signal. The controller propagates this to all
    /// subscribed peers by sending `P2PCommand::PublishIdentityBinding` to the
    /// P2P event loop.
    ///
    /// ## Stage 1 signature
    ///
    /// Per `project_bootstrap_to_elohim_security_gradient`, the signature field
    /// in the gossip payload is a non-empty sentinel for Stage 1. Stage 2 (A.13
    /// or later) replaces this with a real Ed25519 signature.
    ///
    /// ## No-swarm path
    ///
    /// If the controller was constructed without `with_swarm_tx(tx)`, the
    /// method logs at debug level and returns `Ok(())` — preserving backward
    /// compatibility with the A.4 test suite that doesn't wire a live swarm.
    async fn on_agent_peer_binding(
        &mut self,
        signal: AgentPeerBindingSignal,
    ) -> Result<(), ReconcileError> {
        // Build the gossip payload from the DHT signal fields.
        // Note: action_hash is intentionally excluded from the gossip wire payload —
        // binding_action_hash is the canonical DHT provenance field for P2P gossip.
        let payload = crate::p2p::identity_binding_gossip::IdentityBindingGossip {
            peer_id: signal.peer_id.clone(),
            agent_cid: signal.agent_cid.clone(),
            valid_from: signal.valid_from.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            valid_until: signal
                .valid_until
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
            device_archetype: signal.device_archetype.as_str().to_string(),
            binding_action_hash: signal.binding_action_hash.clone(),
            emitted_at: signal.emitted_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            // Stage 1: structural-only sentinel. Stage 2 replaces with real Ed25519
            // signature over canonical payload bytes (see identity_binding_gossip.rs).
            signature: crate::p2p::identity_binding_gossip::STAGE1_SIGNATURE_SENTINEL.to_string(),
        };

        match &self.swarm_tx {
            Some(tx) => {
                if let Err(e) = tx.send(P2PCommand::PublishIdentityBinding(payload)).await {
                    warn!(
                        peer_id = %signal.peer_id,
                        agent_cid = %signal.agent_cid,
                        error = %e,
                        "Failed to send PublishIdentityBinding command — swarm channel closed?"
                    );
                } else {
                    debug!(
                        peer_id = %signal.peer_id,
                        agent_cid = %signal.agent_cid,
                        "Sent PublishIdentityBinding command to P2P event loop"
                    );
                }
            }
            None => {
                debug!(
                    peer_id = %signal.peer_id,
                    agent_cid = %signal.agent_cid,
                    "on_agent_peer_binding: no swarm_tx wired — gossip publish skipped \
                     (use with_swarm_tx for production or gossip integration tests)"
                );
            }
        }

        Ok(())
    }

    /// STUB — Task A.10: track attestation state in revocation_votes projection.
    ///
    /// Will upsert a row in the `revocation_votes` table so the controller
    /// can monitor vote progress toward the quorum threshold. When
    /// `threshold_reached` is true a `DnaSignal::KeyRevocation` should follow.
    async fn on_revocation_attestation(
        &mut self,
        _signal: RevocationAttestationSignal,
    ) -> Result<(), ReconcileError> {
        // Task A.10 replaces this stub.
        Ok(())
    }

    /// M5: upsert the `portal_hosts` projection when a doorway registration is notarized.
    ///
    /// Delegates to [`portal_host_handlers::on_portal_host_created`].
    /// If no DB pool is wired (test stub constructed via `new()`), the handler
    /// logs a warning and returns `Ok(())`.
    async fn on_portal_host_created(
        &mut self,
        signal: PortalHostCreatedSignal,
    ) -> Result<(), ReconcileError> {
        portal_host_handlers::on_portal_host_created(&self.db_pool, signal)
    }

    /// M5: delete the `portal_hosts` projection row when a doorway registration is removed.
    ///
    /// Delegates to [`portal_host_handlers::on_portal_host_removed`].
    /// If no DB pool is wired (test stub constructed via `new()`), the handler
    /// logs a warning and returns `Ok(())`.
    async fn on_portal_host_removed(
        &mut self,
        signal: PortalHostRemovedSignal,
    ) -> Result<(), ReconcileError> {
        portal_host_handlers::on_portal_host_removed(&self.db_pool, signal)
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::signal_stream::{
        AgentPeerBindingSignal, AttestationKind, ChannelSignalStream, DeviceArchetype, DnaSignal,
        InMemoryDnaSignalStream, KeyRevocationSignal, KeyRotationSignal, PortalHostCreatedSignal,
        PortalHostRemovedSignal, RevocationAttestationSignal,
    };
    use chrono::Utc;
    use tokio::sync::mpsc;

    // -----------------------------------------------------------------------
    // Sample signal fixtures — hard-coded test values matching each struct.
    // -----------------------------------------------------------------------

    fn sample_rotation_signal() -> KeyRotationSignal {
        KeyRotationSignal {
            action_hash: "uhCkk-rotation-action-hash".to_string(),
            agent_cid: "bafybeicid-agent-rotation".to_string(),
            new_pubkey: "bmV3a2V5YmFzZTY0AAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            old_pubkey: "b2xka2V5YmFzZTY0AAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            rotated_at: Utc::now(),
            emitted_at: Utc::now(),
        }
    }

    fn sample_revocation_signal() -> KeyRevocationSignal {
        KeyRevocationSignal {
            action_hash: "uhCkk-revocation-action-hash".to_string(),
            agent_cid: "bafybeicid-agent-revocation".to_string(),
            revoked_pubkey: "cmV2b2tlZGtleWJhc2U2NAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            compromise_at: Utc::now(),
            effective_at: Utc::now(),
            triggering_revocation_id: Some("rev-001".to_string()),
            emitted_at: Utc::now(),
        }
    }

    fn sample_binding_signal() -> AgentPeerBindingSignal {
        AgentPeerBindingSignal {
            action_hash: "uhCkk-binding-action-hash".to_string(),
            peer_id: "12D3KooWTestPeerId".to_string(),
            agent_cid: "bafybeicid-agent-binding".to_string(),
            valid_from: Utc::now(),
            valid_until: None,
            device_archetype: DeviceArchetype::Node,
            binding_action_hash: "uhCkk-binding-action-hash".to_string(),
            emitted_at: Utc::now(),
        }
    }

    fn sample_attestation_signal() -> RevocationAttestationSignal {
        RevocationAttestationSignal {
            action_hash: "uhCkk-vote-action-hash".to_string(),
            revocation_id: "rev-001".to_string(),
            steward_id: "bafybeicid-steward-1".to_string(),
            approved: true,
            attestation_kind: AttestationKind::Vote,
            current_votes: 2,
            required_votes: 3,
            threshold_reached: false,
            attested_at: Utc::now(),
            emitted_at: Utc::now(),
        }
    }

    fn sample_portal_host_created_signal() -> PortalHostCreatedSignal {
        PortalHostCreatedSignal {
            action_hash: "uhCAk-portal-create-hash".to_string(),
            human_action_hash: "uhCAk-human-action-hash".to_string(),
            host_url: "https://doorway.example.com".to_string(),
            label: Some("Home doorway".to_string()),
            added_at: "2026-04-25T00:00:00Z".to_string(),
            reach: "Trusted".to_string(),
        }
    }

    fn sample_portal_host_removed_signal() -> PortalHostRemovedSignal {
        PortalHostRemovedSignal {
            action_hash: "uhCAk-portal-delete-hash".to_string(),
            original_action_hash: "uhCAk-portal-create-hash".to_string(),
            human_action_hash: "uhCAk-human-action-hash".to_string(),
            host_url: "https://doorway.example.com".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    /// A single KeyRotation signal is routed and recorded correctly.
    #[tokio::test]
    async fn controller_routes_key_rotation_to_handler() {
        let signals = vec![DnaSignal::KeyRotation(sample_rotation_signal())];
        let stream = InMemoryDnaSignalStream::with_signals(signals);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        assert_eq!(controller.observed_kinds(), &["keyRotation"]);
    }

    /// All four signal kinds are routed in order and all recorded.
    #[tokio::test]
    async fn controller_routes_all_four_signal_kinds() {
        let signals = vec![
            DnaSignal::KeyRotation(sample_rotation_signal()),
            DnaSignal::KeyRevocation(sample_revocation_signal()),
            DnaSignal::AgentPeerBinding(sample_binding_signal()),
            DnaSignal::RevocationAttestation(sample_attestation_signal()),
        ];
        let stream = InMemoryDnaSignalStream::with_signals(signals);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        assert_eq!(
            controller.observed_kinds(),
            &[
                "keyRotation",
                "keyRevocation",
                "agentPeerBinding",
                "revocationAttestation"
            ]
        );
    }

    /// `run_loop` terminates cleanly when the channel sender is dropped.
    #[tokio::test]
    async fn controller_run_loop_exits_on_stream_drop() {
        let (tx, rx) = mpsc::channel(2);
        let stream = ChannelSignalStream::new(rx);
        let mut controller = ReconcileController::new(stream);

        tx.send(DnaSignal::KeyRotation(sample_rotation_signal()))
            .await
            .unwrap();
        drop(tx); // close the channel — stream returns None after this

        controller.run_loop().await.unwrap(); // must terminate cleanly

        assert_eq!(controller.observed_kinds(), &["keyRotation"]);
    }

    /// Repeated runs accumulate kinds from both passes.
    #[tokio::test]
    async fn controller_accumulates_across_passes() {
        let signals = vec![
            DnaSignal::KeyRotation(sample_rotation_signal()),
            DnaSignal::KeyRevocation(sample_revocation_signal()),
        ];
        let stream = InMemoryDnaSignalStream::with_signals(signals);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        // Stream is exhausted; a second pass is a no-op but must not error.
        controller.run_one_pass().await.unwrap();

        assert_eq!(
            controller.observed_kinds(),
            &["keyRotation", "keyRevocation"]
        );
    }

    /// Empty stream: run_one_pass returns Ok immediately with no kinds observed.
    #[tokio::test]
    async fn controller_empty_stream_is_no_op() {
        let stream = InMemoryDnaSignalStream::with_signals(vec![]);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        assert!(controller.observed_kinds().is_empty());
    }

    // -----------------------------------------------------------------------
    // A.8: controller + sweep + cache integration
    // -----------------------------------------------------------------------

    /// End-to-end: a KeyRevocation signal through new_with_storage causes
    /// cache invalidation AND DB sweep in the correct order.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn controller_on_revocation_invalidates_cache_and_runs_sweep() {
        use crate::db::diesel_schema::epr_atoms::dsl as a_dsl;
        use crate::db::epr_atoms::EprAtom;
        use crate::reconcile::pubkey_timeline::{PubkeyTimeline, PubkeyTimelineCache};
        use crate::test_util::test_pool;
        use chrono::{TimeZone, Utc};
        use diesel::prelude::*;
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let pool = Arc::new(test_pool());
        let mut conn = pool.get().expect("connection");

        let t = |secs: i64| Utc.timestamp_opt(secs, 0).single().unwrap();
        let ts = |dt: chrono::DateTime<Utc>| dt.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        // Insert two atoms for the agent: pre-compromise (t50) and post (t90).
        for (issued, verified) in [(t(50), t(51)), (t(90), t(91))] {
            let atom = EprAtom {
                cid: format!("bafy-ctrl-{}", issued.timestamp()),
                kind: "test-kind".into(),
                schema_ref: "test-schema".into(),
                schema_key: "test-key".into(),
                reach: "commons".into(),
                issued_at: ts(issued),
                signer_cid: "ctrl-agent".into(),
                supersedes: None,
                canonical_bytes: vec![0u8; 4],
                payload_bytes: vec![0u8; 4],
                proof_bytes: vec![0u8; 64],
                proof_algorithm: "ed25519".into(),
                verified_at: Some(ts(verified)),
                verified_signer_fingerprint: Some("abcdef0123456789abcdef0123456789".into()),
            };
            diesel::insert_into(a_dsl::epr_atoms)
                .values(&atom)
                .execute(&mut conn)
                .expect("insert atom");
        }

        // Pre-warm the pubkey cache for the agent.
        let cache = Arc::new(Mutex::new(PubkeyTimelineCache::with_capacity(16)));
        {
            let mut c = cache.lock().await;
            let mut tl = PubkeyTimeline::new();
            tl.insert([0x01; 32], t(0), None, "ah-warmup".into());
            c.insert("ctrl-agent".into(), tl);
        }
        assert!(
            cache.lock().await.get(&"ctrl-agent".to_string()).is_some(),
            "cache should be warm before signal"
        );

        drop(conn); // release connection back to pool before block_in_place

        // Build controller with storage wired.
        let signal = KeyRevocationSignal {
            action_hash: "uhCkk-ctrl-rev-hash".into(),
            agent_cid: "ctrl-agent".into(),
            revoked_pubkey: "cmV2b2tlZGtleWJhc2U2NAAAAAAAAAAAAAAAAAAAAAA".into(),
            compromise_at: t(75),
            effective_at: t(76),
            triggering_revocation_id: Some("rev-ctrl-1".into()),
            emitted_at: Utc::now(),
        };

        let stream = InMemoryDnaSignalStream::with_signals(vec![DnaSignal::KeyRevocation(signal)]);
        let mut controller =
            ReconcileController::new_with_storage(stream, Arc::clone(&pool), Arc::clone(&cache));

        controller.run_one_pass().await.expect("run_one_pass");

        // Cache must be invalidated.
        assert!(
            cache.lock().await.get(&"ctrl-agent".to_string()).is_none(),
            "cache entry must be removed after revocation"
        );

        // DB: pre-compromise atom (t50) stays verified.
        let mut conn2 = pool.get().expect("connection2");
        let pre: EprAtom = a_dsl::epr_atoms
            .filter(a_dsl::signer_cid.eq("ctrl-agent"))
            .filter(a_dsl::issued_at.eq(ts(t(50))))
            .first(&mut conn2)
            .expect("pre-compromise atom");
        assert!(
            pre.verified_at.is_some(),
            "pre-compromise atom must remain verified"
        );

        // DB: post-compromise atom (t90) is cleared.
        let post: EprAtom = a_dsl::epr_atoms
            .filter(a_dsl::signer_cid.eq("ctrl-agent"))
            .filter(a_dsl::issued_at.eq(ts(t(90))))
            .first(&mut conn2)
            .expect("post-compromise atom");
        assert!(
            post.verified_at.is_none(),
            "post-compromise atom must have verified_at cleared"
        );
        assert_eq!(
            post.verified_signer_fingerprint,
            Some(crate::reconcile::sweep::REVOKED_STALE_FINGERPRINT.to_string())
        );

        assert_eq!(controller.observed_kinds(), &["keyRevocation"]);
    }

    // -----------------------------------------------------------------------
    // A.10: on_agent_peer_binding → PublishIdentityBinding
    // -----------------------------------------------------------------------

    /// `on_agent_peer_binding` without `with_swarm_tx` returns Ok and records
    /// the kind — preserving backward compatibility with the A.4 test suite.
    #[tokio::test]
    async fn controller_on_agent_peer_binding_no_swarm_tx_is_ok() {
        let signals = vec![DnaSignal::AgentPeerBinding(sample_binding_signal())];
        let stream = InMemoryDnaSignalStream::with_signals(signals);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        assert_eq!(controller.observed_kinds(), &["agentPeerBinding"]);
    }

    /// `on_agent_peer_binding` WITH `with_swarm_tx` sends a
    /// `P2PCommand::PublishIdentityBinding` containing the correct fields.
    #[tokio::test]
    async fn controller_on_agent_peer_binding_sends_publish_identity_binding() {
        use crate::p2p::identity_binding_gossip::STAGE1_SIGNATURE_SENTINEL;
        use crate::p2p::P2PCommand;

        let signal = sample_binding_signal();
        let stream = InMemoryDnaSignalStream::with_signals(vec![DnaSignal::AgentPeerBinding(
            signal.clone(),
        )]);

        let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);
        let mut controller = ReconcileController::new(stream).with_swarm_tx(tx);
        controller.run_one_pass().await.unwrap();

        // Must have recorded the kind.
        assert_eq!(controller.observed_kinds(), &["agentPeerBinding"]);

        // Must have sent a PublishIdentityBinding command.
        let cmd = rx.try_recv().expect("expected command on swarm channel");
        match cmd {
            P2PCommand::PublishIdentityBinding(payload) => {
                assert_eq!(payload.peer_id, signal.peer_id);
                assert_eq!(payload.agent_cid, signal.agent_cid);
                assert_eq!(payload.binding_action_hash, signal.binding_action_hash);
                assert_eq!(payload.signature, STAGE1_SIGNATURE_SENTINEL);
                assert!(
                    !payload.valid_from.is_empty(),
                    "valid_from must be non-empty"
                );
            }
            _other => panic!("expected PublishIdentityBinding, got unexpected command variant"),
        }
    }

    /// Multiple AgentPeerBinding signals in sequence each produce a command.
    #[tokio::test]
    async fn controller_on_agent_peer_binding_multiple_signals_produce_commands() {
        use crate::p2p::P2PCommand;

        let signals = vec![
            DnaSignal::AgentPeerBinding(sample_binding_signal()),
            DnaSignal::AgentPeerBinding(AgentPeerBindingSignal {
                peer_id: "12D3KooWSecondPeer".into(),
                agent_cid: "bafybeicid-second".into(),
                ..sample_binding_signal()
            }),
        ];
        let stream = InMemoryDnaSignalStream::with_signals(signals);

        let (tx, mut rx) = mpsc::channel::<P2PCommand>(8);
        let mut controller = ReconcileController::new(stream).with_swarm_tx(tx);
        controller.run_one_pass().await.unwrap();

        assert_eq!(
            controller.observed_kinds(),
            &["agentPeerBinding", "agentPeerBinding"]
        );

        let cmd1 = rx.try_recv().expect("first command");
        let cmd2 = rx.try_recv().expect("second command");

        assert!(
            matches!(cmd1, P2PCommand::PublishIdentityBinding(_)),
            "first command must be PublishIdentityBinding"
        );
        assert!(
            matches!(cmd2, P2PCommand::PublishIdentityBinding(_)),
            "second command must be PublishIdentityBinding"
        );

        assert!(rx.try_recv().is_err(), "no more commands after two signals");
    }

    // -----------------------------------------------------------------------
    // M5: PortalHostCreated / PortalHostRemoved dispatch
    // -----------------------------------------------------------------------

    /// `PortalHostCreated` signal is routed and recorded when no DB pool is wired.
    ///
    /// Without `new_with_storage` the handler logs a warning and returns `Ok(())`.
    /// This preserves backward compatibility with the A.4 stub-only test suite.
    #[tokio::test]
    async fn controller_routes_portal_host_created_no_pool_is_ok() {
        let signals = vec![DnaSignal::PortalHostCreated(
            sample_portal_host_created_signal(),
        )];
        let stream = InMemoryDnaSignalStream::with_signals(signals);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        assert_eq!(controller.observed_kinds(), &["portalHostCreated"]);
    }

    /// `PortalHostRemoved` signal is routed and recorded when no DB pool is wired.
    #[tokio::test]
    async fn controller_routes_portal_host_removed_no_pool_is_ok() {
        let signals = vec![DnaSignal::PortalHostRemoved(
            sample_portal_host_removed_signal(),
        )];
        let stream = InMemoryDnaSignalStream::with_signals(signals);
        let mut controller = ReconcileController::new(stream);

        controller.run_one_pass().await.unwrap();

        assert_eq!(controller.observed_kinds(), &["portalHostRemoved"]);
    }

    /// End-to-end: `PortalHostCreated` through `new_with_storage` upserts a
    /// projection row; `PortalHostRemoved` deletes it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn controller_portal_host_created_then_removed_roundtrip() {
        use crate::db::{portal_hosts, run_migrations, DbPool};
        use crate::reconcile::pubkey_timeline::PubkeyTimelineCache;
        use diesel::r2d2::{ConnectionManager, Pool};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        // Build an in-memory test DB.
        let url = format!(
            "file:ctrl_ph_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool: DbPool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        let pool = Arc::new(pool);
        let cache = Arc::new(Mutex::new(PubkeyTimelineCache::with_capacity(4)));

        let created = sample_portal_host_created_signal();
        let removed = sample_portal_host_removed_signal();

        // --- Create pass ---
        let stream = InMemoryDnaSignalStream::with_signals(vec![DnaSignal::PortalHostCreated(
            created.clone(),
        )]);
        let mut ctrl =
            ReconcileController::new_with_storage(stream, Arc::clone(&pool), Arc::clone(&cache));
        ctrl.run_one_pass().await.expect("create pass");

        // Row must now exist for the human.
        let rows = tokio::task::block_in_place(|| {
            let mut conn = pool.get().expect("conn");
            portal_hosts::list_for_human(&mut conn, &created.human_action_hash)
                .expect("list_for_human")
        });
        assert_eq!(
            rows.len(),
            1,
            "expected one portal_host row after create signal"
        );
        assert_eq!(rows[0].host_url, created.host_url);
        assert_eq!(rows[0].dht_anchor_hash, created.action_hash);

        // --- Remove pass ---
        let stream = InMemoryDnaSignalStream::with_signals(vec![DnaSignal::PortalHostRemoved(
            removed.clone(),
        )]);
        let mut ctrl =
            ReconcileController::new_with_storage(stream, Arc::clone(&pool), Arc::clone(&cache));
        ctrl.run_one_pass().await.expect("remove pass");

        let rows_after = tokio::task::block_in_place(|| {
            let mut conn = pool.get().expect("conn");
            portal_hosts::list_for_human(&mut conn, &created.human_action_hash)
                .expect("list_for_human after")
        });
        assert!(
            rows_after.is_empty(),
            "portal_host row must be gone after remove signal"
        );
    }
}
