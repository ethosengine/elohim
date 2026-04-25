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
//!             ├─► on_key_rotation         (Task A.6 — pubkey timeline cache)
//!             ├─► on_key_revocation       (Task A.8 — sweep + cache invalidate)
//!             ├─► on_agent_peer_binding   (Task A.5 — peer_identity_bindings table)
//!             └─► on_revocation_attestation (Task A.10 — revocation_votes projection)
//! ```
//!
//! ## Current state
//!
//! This is the A.4 skeleton. All four handlers are **no-op stubs**; they record
//! the signal kind in `observed_kinds` for test introspection and return `Ok(())`.
//! Handler implementations are filled in by Tasks A.5, A.6, A.8, and A.10.

use thiserror::Error;
use tracing::{debug, warn};

use crate::reconcile::signal_stream::{
    AgentPeerBindingSignal, DnaSignal, DnaSignalStream, KeyRevocationSignal, KeyRotationSignal,
    RevocationAttestationSignal,
};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur in the reconcile controller loop.
///
/// Currently only wraps stream-level errors. Later tasks will add
/// handler-specific error variants (e.g. `DbWrite`, `CacheInsert`).
#[derive(Debug, Error)]
pub enum ReconcileError {
    #[error("signal stream error: {0}")]
    Stream(String),
    // Future variants added by A.5 / A.6 / A.8 / A.10:
    //   DbWrite(#[from] diesel::result::Error),
    //   CacheInsert(String),
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
/// The only state carried today is `observed_kinds` — a test-introspection
/// accumulator. Later tasks will add real per-handler state:
///
/// | Task | State added |
/// |------|-------------|
/// | A.5  | `binding_cache: Arc<RwLock<PeerBindingCache>>` |
/// | A.6  | `pubkey_cache: PubkeyTimelineCache` |
/// | A.8  | Uses `db_pool` for sweep queries |
/// | A.10 | Uses `db_pool` for revocation_votes projection |
/// | A.11 | `db_pool: Arc<Pool<ConnectionManager<SqliteConnection>>>` |
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
}

impl<S: DnaSignalStream> ReconcileController<S> {
    /// Construct a controller subscribing to `stream`.
    ///
    /// No caches or DB pools are accepted yet — those are added by later tasks.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            observed_kinds: Vec::new(),
        }
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

    /// STUB — Task A.8: invalidate pubkey cache + sweep epr_atoms.
    ///
    /// Will mark any EPR atom signed by `revoked_pubkey` after `compromise_at`
    /// as tainted and trigger re-verification or invalidation.
    async fn on_key_revocation(
        &mut self,
        signal: KeyRevocationSignal,
    ) -> Result<(), ReconcileError> {
        // Task A.8 replaces this stub.
        warn!(
            revoked_pubkey = %signal.revoked_pubkey,
            compromise_at = %signal.compromise_at,
            "KeyRevocation received — sweep not yet implemented (Task A.8)"
        );
        Ok(())
    }

    /// STUB — Task A.5: insert/update peer_identity_bindings row.
    ///
    /// Will upsert the libp2p PeerId → agent CID mapping into the
    /// `peer_identity_bindings` SQLite projection table.
    async fn on_agent_peer_binding(
        &mut self,
        _signal: AgentPeerBindingSignal,
    ) -> Result<(), ReconcileError> {
        // Task A.5 replaces this stub.
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
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::signal_stream::{
        AgentPeerBindingSignal, AttestationKind, ChannelSignalStream, DeviceArchetype, DnaSignal,
        InMemoryDnaSignalStream, KeyRevocationSignal, KeyRotationSignal,
        RevocationAttestationSignal,
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
}
