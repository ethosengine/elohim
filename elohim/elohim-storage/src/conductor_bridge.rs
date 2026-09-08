//! The conductor leg — a LIVE handle source, never a captured one.
//!
//! # Why this module exists at all
//!
//! `holochain_client` 0.8 has no reconnect, and a conductor restart both closes
//! the sockets AND invalidates the in-memory app-auth token, so recovery is a
//! full re-mint rather than a socket-level reconnect. That re-mint already
//! exists exactly once in this crate:
//! [`crate::hc_client_registry::HcClientRegistry::spawn_bridge_supervisor`].
//!
//! **There is exactly one reconnect strategy in this crate and it is the
//! supervisor's.** Everything here re-READS a handle; nothing here mints one.
//!
//! Two long-lived consumers hold a conductor leg across a possible roll — the
//! release-adoption controller (`services::release_adoption::watch`) and the
//! soak-attestation rail (`services::release_attestation`) — and both shipped
//! holding a boot-time `Arc<HcClient>` clone. That is why this is a shared
//! module rather than a field on either of them: the same defect landed twice,
//! and the type is now the thing that prevents a third.

use std::sync::Arc;

use crate::conductor_admission::AdmissionClass;
use crate::error::StorageError;
use crate::hc_client::HcClient;

/// Where a long-lived consumer gets a conductor handle, **re-read on every
/// use**.
///
/// # Why this is a trait and not an `Arc<HcClient>` field
///
/// `holochain_client` 0.8 has no reconnect, and a conductor restart both closes
/// the sockets AND invalidates the in-memory app-auth token, so recovery is a
/// full re-mint rather than a socket-level reconnect. That re-mint already
/// exists exactly once in this crate:
/// [`crate::hc_client_registry::HcClientRegistry::spawn_bridge_supervisor`]
/// probes each supervised role every [`crate::hc_client_registry::BRIDGE_PROBE_INTERVAL`],
/// and on a failed ping clears the registry slot and re-runs
/// `connect_role_forever`, storing a FRESH `Arc<HcClient>` in the slot.
///
/// That supervisor's own doc names this defect: *"Anything holding an Arc clone
/// keeps failing until it re-reads."* The adoption controller held an
/// `Arc<HcClient>` cloned out of the registry at boot
/// (`main.rs`: `hc_registry_for_http…lamad_client()` → `with_conductor`) and
/// never re-read it, so every sweep after the first conductor roll called a
/// corpse and reported `conductor_unavailable` forever — while the projector
/// path, which re-reads the same slot, kept working. This trait makes
/// "re-read" the shape of the type rather than a remembered step.
///
/// **There is exactly one reconnect strategy in this crate and it is the
/// supervisor's.** An implementation here re-READS a handle; it must never
/// mint one.
#[async_trait::async_trait]
pub trait ConductorBridge: Send + Sync {
    /// The handle to use RIGHT NOW. `None` while the bridge is being re-minted
    /// — honest backpressure, and the next sweep asks again.
    fn handle(&self) -> Option<Arc<HcClient>>;

    /// One zome call at the CALLER's admission class, through [`Self::handle`].
    ///
    /// The class is a parameter rather than a constant because the two
    /// consumers legitimately differ: a controller sweep is `Background` (it
    /// must never occupy the lane a person is standing in), while authoring a
    /// soak attestation is the `Interactive` write it has always been. Moving
    /// to a bridge must not silently re-class either.
    ///
    /// Defaulted rather than duplicated so every implementation gets the same
    /// "no handle is `Connection`, not a decode failure" mapping; a test fake
    /// overrides this one method.
    async fn call_zome_classed(
        &self,
        zome: &str,
        fn_name: &str,
        payload: Vec<u8>,
        class: AdmissionClass,
    ) -> Result<Vec<u8>, StorageError> {
        let Some(hc) = self.handle() else {
            return Err(StorageError::Connection(
                "no conductor bridge (being re-minted)".to_string(),
            ));
        };
        hc.call_zome_timed(zome, fn_name, payload, class)
            .await
            .map(|(bytes, _timing)| bytes)
    }

    /// [`Self::call_zome_classed`] at `Background` — the controller-sweep
    /// default.
    async fn call_zome(
        &self,
        zome: &str,
        fn_name: &str,
        payload: Vec<u8>,
    ) -> Result<Vec<u8>, StorageError> {
        self.call_zome_classed(zome, fn_name, payload, AdmissionClass::Background)
            .await
    }
}

/// What a long-lived sweep's conductor leg looks like on ONE tick.
///
/// The `generation` is the identity of the handle currently behind the bridge.
/// It changes exactly when the supervisor re-mints, which is what makes
/// "did this sweep follow the roll?" a testable question rather than a hope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepLeg {
    /// No handle right now — the bridge is mid-re-mint. The sweep skips its
    /// conductor leg and asks again next tick. Honest, and never a corpse.
    Unavailable,
    /// A handle is available, identified by `generation`.
    Live { generation: u64 },
}

/// Resolve the conductor leg for ONE sweep.
///
/// # The defect this replaces
///
/// A long-lived loop that stores its handle in a `OnceLock` (or any set-once
/// cell) reports "bridge up" forever after the first success — including for
/// every tick after the conductor rolled, when the handle behind it is a
/// corpse. Measured on the household mesh 2026-09-08: jessica's conductor was
/// SIGKILLed at 04:27:18, the supervisor logged a re-mint at 04:27:23, and the
/// projection-reconcile / election-obey / reanchor sweeps kept failing
/// `Zome call failed: Websocket` until 04:33:15 — 812 lines against a handle
/// they had latched six minutes earlier. The death witness authored inside that
/// window never anchored, which is how a conductor restart turned into a failed
/// death-witness replication receipt on a peer whose substrate was healthy.
///
/// Calling this per tick is the whole cure: a sweep that cannot ask says so and
/// retries, and the tick after the re-mint runs against the NEW generation.
pub fn sweep_leg(bridge: &dyn ConductorBridge) -> SweepLeg {
    match bridge.handle() {
        None => SweepLeg::Unavailable,
        Some(hc) => SweepLeg::Live {
            generation: generation_of(&hc),
        },
    }
}

/// Handle identity as a comparable number. Only ever compared, never
/// dereferenced.
pub fn generation_of(hc: &Arc<HcClient>) -> u64 {
    Arc::as_ptr(hc) as u64
}

/// The production bridge: the registry slot the bridge supervisor owns.
///
/// Holds the registry, never a client — so a conductor roll is ordinary churn
/// and the next use (a controller sweep, a soak attestation) resolves through
/// the re-minted handle.
pub struct RegistryConductorBridge {
    registry: Arc<crate::hc_client_registry::HcClientRegistry>,
    role: &'static str,
    /// Identity of the last handle this bridge handed out, as a raw pointer
    /// value. Only ever compared, never dereferenced.
    last_seen: std::sync::atomic::AtomicUsize,
}

impl RegistryConductorBridge {
    /// The `lamad` role — the registry slot that carries the `content_store`
    /// zome this controller resolves heads through.
    pub fn lamad(registry: Arc<crate::hc_client_registry::HcClientRegistry>) -> Self {
        Self {
            registry,
            role: "lamad",
            last_seen: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// The `imagodei` role — the registry slot the membership-truth reconcile
    /// resolves identity/qahal reads through. Same re-read discipline as
    /// [`Self::lamad`]; a separate constructor because the role string is the
    /// registry KEY (`HcClientRegistry::client(role)`), never a parameter a
    /// caller could get wrong.
    pub fn imagodei(registry: Arc<crate::hc_client_registry::HcClientRegistry>) -> Self {
        Self {
            registry,
            role: "imagodei",
            last_seen: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl ConductorBridge for RegistryConductorBridge {
    fn handle(&self) -> Option<Arc<HcClient>> {
        let hc = self.registry.client(self.role)?;
        // Count the moment this controller picks up a DIFFERENT handle than the
        // one it last used — i.e. the supervisor re-minted the bridge under us
        // and we followed it. Counted here rather than in the supervisor
        // because the supervisor's own reconnect counter cannot say whether the
        // adoption controller ever noticed, which is precisely what was wrong.
        let ptr = Arc::as_ptr(&hc) as usize;
        let previous = self
            .last_seen
            .swap(ptr, std::sync::atomic::Ordering::Relaxed);
        if previous != 0 && previous != ptr {
            crate::metrics::RELEASE_CHANNEL_CONDUCTOR_RECONNECT.inc();
            tracing::info!(
                role = self.role,
                "conductor-bridge: picked up a RE-MINTED handle — a conductor roll is ordinary \
                 churn, and this use resolves through the fresh handle"
            );
        }
        Some(hc)
    }
}

/// A bridge with no conductor at all. Every sweep reports `Unavailable` —
/// honest for a node wired with neither a registry nor a boot client, and
/// distinct from a bridge whose handle merely happens to be re-minting.
pub struct NoConductorBridge;

#[async_trait::async_trait]
impl ConductorBridge for NoConductorBridge {
    fn handle(&self) -> Option<Arc<HcClient>> {
        None
    }
}

/// A bridge over one fixed handle. Correct only for a process that outlives no
/// conductor restart — kept for callers (and tests) that hold a client
/// directly. Production wires [`RegistryConductorBridge`].
pub struct StaticConductorBridge(pub Arc<HcClient>);

#[async_trait::async_trait]
impl ConductorBridge for StaticConductorBridge {
    fn handle(&self) -> Option<Arc<HcClient>> {
        Some(self.0.clone())
    }
}
