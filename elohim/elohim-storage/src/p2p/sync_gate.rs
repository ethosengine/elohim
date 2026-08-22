//! SyncGate — the ONE reason-carrying verdict surface for "may sync run
//! right now?".
//!
//! Design: `genesis/data/timeline/backlog/p2p-sync-mode-unified-gate-design.md`
//! (p2p-design-gate run 2026-08-22; both entities Ephemeral/C, no DHT touch).
//!
//! elohim-storage already suppressed sync internally (account-import pause,
//! bulk-write pause, drain-backlog auto-suppression) through a single shared
//! `AtomicBool`. This module unifies those EXISTING sources with the two new
//! operator-facing ones (operator mode, wifi-only x network class) behind one
//! pure verdict function — there is deliberately NO second pause plane, and
//! every surface (`/p2p/status`, `/p2p/sync-mode`, doorway `/health` p2p
//! block, the event-loop tick guards) reads exactly what this gate decides.
//!
//! Source of truth: SQLite `sync_mode_state` (operational, Category C — see
//! `db::sync_mode`). On loss the gate reconstructs to `mode=sync`,
//! `network=unknown`, which is exactly the pre-feature behavior.
//!
//! CounterEvidence floor note (Network Stakes): sync pause defers BULK content
//! sync only. It must NOT be wired to block correction-reach delivery paths
//! if/when those ride the same swarm — a paused peer defers convenience, never
//! correction.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

/// Operator/device sync mode. Persisted (sticky across restart); default `Sync`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    /// Sync continuously regardless of network.
    Sync,
    /// Operator has put their peer to sleep.
    Paused,
    /// Sync runs on unmetered networks, pauses on cellular.
    WifiOnly,
}

impl SyncMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncMode::Sync => "sync",
            SyncMode::Paused => "paused",
            SyncMode::WifiOnly => "wifi-only",
        }
    }

    /// Parse a wire/db string. Unrecognized values are a parse rejection
    /// (`None`), never a guess.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "sync" => Some(SyncMode::Sync),
            "paused" => Some(SyncMode::Paused),
            "wifi-only" => Some(SyncMode::WifiOnly),
            _ => None,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => SyncMode::Paused,
            2 => SyncMode::WifiOnly,
            _ => SyncMode::Sync,
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            SyncMode::Sync => 0,
            SyncMode::Paused => 1,
            SyncMode::WifiOnly => 2,
        }
    }
}

/// Device-declared network class. Detection is NOT the storage node's job —
/// the device/steward layer declares it via `POST /p2p/network-class` (or the
/// sync-mode POST body). `Unknown` is the honest default: it is surfaced,
/// never guessed, and does NOT suppress under wifi-only (the cellular guard is
/// an opt-in protection — permissive default = pre-feature behavior).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkClass {
    Wifi,
    Cellular,
    Unknown,
}

impl NetworkClass {
    pub fn as_str(self) -> &'static str {
        match self {
            NetworkClass::Wifi => "wifi",
            NetworkClass::Cellular => "cellular",
            NetworkClass::Unknown => "unknown",
        }
    }

    /// Parse a wire/db string. Unrecognized values are a parse rejection
    /// (`None`), never a guess.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "wifi" => Some(NetworkClass::Wifi),
            "cellular" => Some(NetworkClass::Cellular),
            "unknown" => Some(NetworkClass::Unknown),
            _ => None,
        }
    }

    fn from_u8(v: u8) -> Self {
        match v {
            1 => NetworkClass::Wifi,
            2 => NetworkClass::Cellular,
            _ => NetworkClass::Unknown,
        }
    }

    fn to_u8(self) -> u8 {
        match self {
            NetworkClass::Unknown => 0,
            NetworkClass::Wifi => 1,
            NetworkClass::Cellular => 2,
        }
    }
}

/// Why sync is currently held. `reasons[]` names what holds it — every source
/// has a release path (operator POST, network change, operation end, backlog
/// drain), so nothing suppresses silently or forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSuppressReason {
    /// Operator set mode=paused.
    OperatorPaused,
    /// mode=wifi-only AND declared network class is cellular.
    WifiOnlyOnCellular,
    /// An account-package import is in flight (existing backpressure source).
    ImportInProgress,
    /// A bulk content write is in flight (existing backpressure source).
    BulkWriteInProgress,
    /// Drain publish queue backlog exceeded threshold (existing
    /// auto-suppression source — the externally-imposed backpressure arm).
    DrainBacklog,
}

impl SyncSuppressReason {
    pub fn as_str(self) -> &'static str {
        match self {
            SyncSuppressReason::OperatorPaused => "operator-paused",
            SyncSuppressReason::WifiOnlyOnCellular => "wifi-only-on-cellular",
            SyncSuppressReason::ImportInProgress => "import-in-progress",
            SyncSuppressReason::BulkWriteInProgress => "bulk-write-in-progress",
            SyncSuppressReason::DrainBacklog => "drain-backlog",
        }
    }
}

/// The gate's answer: may sync run, and if not, exactly what holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncVerdict {
    pub syncing: bool,
    pub reasons: Vec<SyncSuppressReason>,
}

impl SyncVerdict {
    /// Wire-shaped reason strings (kebab-case), for `/p2p/status.syncReasons`.
    pub fn reason_strings(&self) -> Vec<String> {
        self.reasons
            .iter()
            .map(|r| r.as_str().to_string())
            .collect()
    }
}

/// The pure verdict function (Step-4 concern canon: suppression is a monotonic
/// OR — no source can un-suppress another; evaluation is O(sources)).
///
/// `network=Unknown` does NOT suppress under wifi-only: the honest permissive
/// default. Only a positively declared `cellular` engages the guard.
pub fn effective_sync(
    mode: SyncMode,
    network: NetworkClass,
    import_in_progress: bool,
    bulk_write_in_progress: bool,
    drain_backlog: bool,
) -> SyncVerdict {
    let mut reasons = Vec::new();
    if mode == SyncMode::Paused {
        reasons.push(SyncSuppressReason::OperatorPaused);
    }
    if mode == SyncMode::WifiOnly && network == NetworkClass::Cellular {
        reasons.push(SyncSuppressReason::WifiOnlyOnCellular);
    }
    if import_in_progress {
        reasons.push(SyncSuppressReason::ImportInProgress);
    }
    if bulk_write_in_progress {
        reasons.push(SyncSuppressReason::BulkWriteInProgress);
    }
    if drain_backlog {
        reasons.push(SyncSuppressReason::DrainBacklog);
    }
    SyncVerdict {
        syncing: reasons.is_empty(),
        reasons,
    }
}

/// Shared, lock-free gate state. One instance per node, shared (Arc) between
/// the P2P event loop, the HTTP handlers, and the bulk-write pause guards.
///
/// Internal suppression sources are COUNTERS (imports and bulk writes can
/// overlap; the source releases only when the last in-flight operation ends).
#[derive(Debug)]
pub struct SyncGate {
    mode: AtomicU8,
    network: AtomicU8,
    import_ops: AtomicUsize,
    bulk_ops: AtomicUsize,
    drain_backlog: AtomicBool,
}

impl Default for SyncGate {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncGate {
    /// Reconstruction default: `mode=sync`, `network=unknown` (pre-feature
    /// behavior). Persisted state is re-applied at boot via [`Self::set_mode`]
    /// / [`Self::set_network`].
    pub fn new() -> Self {
        SyncGate {
            mode: AtomicU8::new(SyncMode::Sync.to_u8()),
            network: AtomicU8::new(NetworkClass::Unknown.to_u8()),
            import_ops: AtomicUsize::new(0),
            bulk_ops: AtomicUsize::new(0),
            drain_backlog: AtomicBool::new(false),
        }
    }

    pub fn mode(&self) -> SyncMode {
        SyncMode::from_u8(self.mode.load(Ordering::Acquire))
    }

    pub fn network(&self) -> NetworkClass {
        NetworkClass::from_u8(self.network.load(Ordering::Acquire))
    }

    /// Set the operator mode. Returns the previous mode (equal to `mode` when
    /// the POST was an idempotent no-op).
    pub fn set_mode(&self, mode: SyncMode) -> SyncMode {
        SyncMode::from_u8(self.mode.swap(mode.to_u8(), Ordering::AcqRel))
    }

    /// Set the declared network class. Returns the previous class.
    pub fn set_network(&self, network: NetworkClass) -> NetworkClass {
        NetworkClass::from_u8(self.network.swap(network.to_u8(), Ordering::AcqRel))
    }

    /// Mark an internal suppression source as begun. Paired with
    /// [`Self::end_source`] (RAII guard `SyncPauseGuard` in `p2p::mod`).
    /// Only `ImportInProgress` / `BulkWriteInProgress` are counter-backed;
    /// other reasons are ignored (mode/network/drain have their own setters).
    pub fn begin_source(&self, source: SyncSuppressReason) {
        match source {
            SyncSuppressReason::ImportInProgress => {
                self.import_ops.fetch_add(1, Ordering::AcqRel);
            }
            SyncSuppressReason::BulkWriteInProgress => {
                self.bulk_ops.fetch_add(1, Ordering::AcqRel);
            }
            _ => {}
        }
    }

    /// Release an internal suppression source (saturating — never underflows).
    pub fn end_source(&self, source: SyncSuppressReason) {
        let counter = match source {
            SyncSuppressReason::ImportInProgress => &self.import_ops,
            SyncSuppressReason::BulkWriteInProgress => &self.bulk_ops,
            _ => return,
        };
        let _ = counter.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| {
            Some(v.saturating_sub(1))
        });
    }

    /// Drain-backlog auto-suppression (existing behavior, now reason-named).
    /// Returns `true` when the flag actually changed (callers log transitions).
    pub fn set_drain_backlog(&self, suppressed: bool) -> bool {
        self.drain_backlog.swap(suppressed, Ordering::AcqRel) != suppressed
    }

    /// Snapshot every source and compose the verdict via [`effective_sync`].
    pub fn verdict(&self) -> SyncVerdict {
        effective_sync(
            self.mode(),
            self.network(),
            self.import_ops.load(Ordering::Acquire) > 0,
            self.bulk_ops.load(Ordering::Acquire) > 0,
            self.drain_backlog.load(Ordering::Acquire),
        )
    }

    /// Convenience for the event-loop tick guards: is sync currently held?
    pub fn suppressed(&self) -> bool {
        !self.verdict().syncing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_gate_syncs_with_no_reasons() {
        let v = effective_sync(SyncMode::Sync, NetworkClass::Unknown, false, false, false);
        assert!(v.syncing);
        assert!(v.reasons.is_empty());
    }

    #[test]
    fn operator_paused_suppresses_and_names_itself() {
        let v = effective_sync(SyncMode::Paused, NetworkClass::Wifi, false, false, false);
        assert!(!v.syncing);
        assert_eq!(v.reasons, vec![SyncSuppressReason::OperatorPaused]);
    }

    #[test]
    fn wifi_only_on_cellular_suppresses() {
        let v = effective_sync(
            SyncMode::WifiOnly,
            NetworkClass::Cellular,
            false,
            false,
            false,
        );
        assert!(!v.syncing);
        assert_eq!(v.reasons, vec![SyncSuppressReason::WifiOnlyOnCellular]);
    }

    #[test]
    fn wifi_only_on_wifi_syncs() {
        let v = effective_sync(SyncMode::WifiOnly, NetworkClass::Wifi, false, false, false);
        assert!(v.syncing);
    }

    #[test]
    fn wifi_only_on_unknown_network_does_not_suppress() {
        // C4: unknown is honest and permissive — the cellular guard is an
        // opt-in protection; unknown is surfaced, never guessed.
        let v = effective_sync(
            SyncMode::WifiOnly,
            NetworkClass::Unknown,
            false,
            false,
            false,
        );
        assert!(v.syncing);
        assert!(v.reasons.is_empty());
    }

    #[test]
    fn import_in_progress_suppresses() {
        let v = effective_sync(SyncMode::Sync, NetworkClass::Unknown, true, false, false);
        assert!(!v.syncing);
        assert_eq!(v.reasons, vec![SyncSuppressReason::ImportInProgress]);
    }

    #[test]
    fn bulk_write_in_progress_suppresses() {
        let v = effective_sync(SyncMode::Sync, NetworkClass::Unknown, false, true, false);
        assert!(!v.syncing);
        assert_eq!(v.reasons, vec![SyncSuppressReason::BulkWriteInProgress]);
    }

    #[test]
    fn drain_backlog_suppresses() {
        let v = effective_sync(SyncMode::Sync, NetworkClass::Unknown, false, false, true);
        assert!(!v.syncing);
        assert_eq!(v.reasons, vec![SyncSuppressReason::DrainBacklog]);
    }

    #[test]
    fn composition_is_monotonic_or_naming_every_active_source() {
        // C2: no source can un-suppress another — all five compose.
        let v = effective_sync(SyncMode::Paused, NetworkClass::Cellular, true, true, true);
        assert!(!v.syncing);
        // Paused (not wifi-only), so WifiOnlyOnCellular is NOT active here.
        assert_eq!(
            v.reasons,
            vec![
                SyncSuppressReason::OperatorPaused,
                SyncSuppressReason::ImportInProgress,
                SyncSuppressReason::BulkWriteInProgress,
                SyncSuppressReason::DrainBacklog,
            ]
        );
        let v = effective_sync(SyncMode::WifiOnly, NetworkClass::Cellular, true, true, true);
        assert_eq!(v.reasons.len(), 4);
        assert!(v.reasons.contains(&SyncSuppressReason::WifiOnlyOnCellular));
    }

    #[test]
    fn gate_counters_release_only_when_last_operation_ends() {
        let gate = SyncGate::new();
        gate.begin_source(SyncSuppressReason::ImportInProgress);
        gate.begin_source(SyncSuppressReason::ImportInProgress);
        gate.end_source(SyncSuppressReason::ImportInProgress);
        assert!(gate.suppressed(), "one import still in flight");
        gate.end_source(SyncSuppressReason::ImportInProgress);
        assert!(!gate.suppressed());
        // Saturating: an extra end never underflows into permanent suppression.
        gate.end_source(SyncSuppressReason::ImportInProgress);
        assert!(!gate.suppressed());
    }

    #[test]
    fn gate_set_mode_is_idempotent_and_reports_previous() {
        let gate = SyncGate::new();
        assert_eq!(gate.set_mode(SyncMode::Paused), SyncMode::Sync);
        assert_eq!(gate.set_mode(SyncMode::Paused), SyncMode::Paused);
        assert!(gate.suppressed());
        assert_eq!(gate.set_mode(SyncMode::Sync), SyncMode::Paused);
        assert!(!gate.suppressed());
    }

    #[test]
    fn gate_drain_flag_reports_change() {
        let gate = SyncGate::new();
        assert!(gate.set_drain_backlog(true), "false -> true is a change");
        assert!(!gate.set_drain_backlog(true), "true -> true is not");
        assert!(gate
            .verdict()
            .reasons
            .contains(&SyncSuppressReason::DrainBacklog));
        assert!(gate.set_drain_backlog(false));
        assert!(!gate.suppressed());
    }

    #[test]
    fn mode_and_network_parse_reject_unknown_strings() {
        assert_eq!(SyncMode::parse("sync"), Some(SyncMode::Sync));
        assert_eq!(SyncMode::parse("wifi-only"), Some(SyncMode::WifiOnly));
        assert_eq!(SyncMode::parse("WIFI"), None);
        assert_eq!(
            NetworkClass::parse("cellular"),
            Some(NetworkClass::Cellular)
        );
        assert_eq!(NetworkClass::parse("5g"), None);
    }

    #[test]
    fn verdict_reason_strings_are_kebab_case_wire_names() {
        let v = effective_sync(
            SyncMode::WifiOnly,
            NetworkClass::Cellular,
            false,
            false,
            false,
        );
        assert_eq!(
            v.reason_strings(),
            vec!["wifi-only-on-cellular".to_string()]
        );
    }
}
