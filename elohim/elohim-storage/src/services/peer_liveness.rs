//! Live connected-peer view — the liveness signal the FELT badge reads.
//!
//! ## Why this exists (the incident it cures)
//!
//! The household-resilience card answered liveness from `peer_statuses`, a
//! heartbeat projection read behind a 900 s staleness window. A peer that is
//! SIGKILLed writes no offline row — it writes nothing, by definition — so its
//! last heartbeat stays inside the window for a quarter of an hour. The chaos
//! drills made the consequence plain: after two of three peers were killed, the
//! card still read **3 live copies** of "manifesto"
//! (blob-durability DELTA 2026-09-12c, cause 2). A liveness signal that cannot
//! go down is not a liveness signal.
//!
//! The local p2p layer already knows better, and knows it within the ping
//! timeout: libp2p ≥0.43 does not close a connection on ping failure, so the
//! swarm arm closes it explicitly precisely so that "the connected set reflects
//! liveness". That set is what this registry publishes.
//!
//! ## The contract
//!
//! - **`live` is intersection, not a window.** A household peer counts live iff
//!   the local transport currently sees it connected. `known` keeps its existing
//!   meaning — the heartbeat/`stewarded_nodes` junction — so the card still says
//!   "1 of 3", never a bare zero.
//! - **Two independent ways down.** The explicit `ConnectionClosed` publish is
//!   the fast path; the armed staleness bound (`ttl`) is the backstop for a
//!   wedged event loop, sized to the ping interval + timeout so a killed peer
//!   leaves `live` inside the same budget either way.
//! - **Unarmed means unmeasured, and unmeasured falls back.** A process with no
//!   p2p plane running (unit tests, a doorway-less read, a build without the
//!   transport feature) never arms the registry, and every reader then keeps the
//!   pre-existing heartbeat behaviour. This is the difference between "nobody is
//!   connected" and "nobody asked" — collapsing them would flip every such
//!   deployment's card to `live: 0`, which is the same class of false
//!   measurement this module exists to remove.
//! - **Both label namespaces, because both junctions use both.** A peer is
//!   published under its transport peer id AND (when resolvable) its
//!   `agent_cid`. The household peer set is assembled from `humans.agent_pub_key`
//!   and `stewarded_nodes.id`, which carry EITHER namespace — the same
//!   either-namespace matching `reconcile/custody.rs` documents. Publishing one
//!   label would silently match nothing on half the fleet.
//!
//! The iroh plane does not arm this registry today. That is deliberate rather
//! than overlooked: a node running iroh-only keeps the heartbeat behaviour it
//! has always had (honest, if slow) instead of reading `live: 0` from an
//! unpublished set. A dual-stack node is armed by its libp2p side.

use std::collections::{HashMap, HashSet};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant};

/// Staleness bound used when a caller arms the registry without naming one.
/// Sized as libp2p's default ping interval (15 s) + timeout (20 s): the budget
/// inside which a silent peer's connection is closed by the ping-failure arm.
pub const DEFAULT_LIVENESS_TTL: Duration = Duration::from_secs(35);

/// One connected peer, under every label a household junction might name it by.
#[derive(Debug, Clone)]
struct PeerEntry {
    /// Transport peer id and, when it could be resolved, the `agent_cid`.
    labels: Vec<String>,
    last_seen: Instant,
}

#[derive(Debug, Default)]
struct Registry {
    /// `None` until a transport plane arms it — see the module doc on why
    /// "unarmed" must not read as "nobody connected".
    armed: Option<Armed>,
}

#[derive(Debug)]
struct Armed {
    /// This node's own labels. It is never in its own connected set, yet it is
    /// plainly live, and a household of one would otherwise read `live: 0`.
    own_labels: Vec<String>,
    ttl: Duration,
    peers: HashMap<String, PeerEntry>,
}

fn registry() -> &'static RwLock<Registry> {
    static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Registry::default()))
}

/// Arm the registry: from now on an empty connected set means "nobody is
/// connected", not "nobody asked". Called once by the transport plane at
/// startup, with the labels naming THIS node and the staleness bound derived
/// from its ping configuration.
///
/// Idempotent in effect: re-arming replaces the own-labels/ttl and KEEPS the
/// observed peers, so a reconfiguration never blanks a live set.
pub fn arm(own_labels: Vec<String>, ttl: Duration) {
    let mut reg = match registry().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let peers = reg.armed.take().map(|a| a.peers).unwrap_or_default();
    reg.armed = Some(Armed {
        own_labels,
        ttl: if ttl.is_zero() {
            DEFAULT_LIVENESS_TTL
        } else {
            ttl
        },
        peers,
    });
}

/// Record a peer as connected, under every label it is known by. Called from the
/// transport's connection-established arm, where resolving the `agent_cid` costs
/// one lookup per connection rather than one per ping.
///
/// A no-op while unarmed: a plane that never armed must not start publishing
/// half a set.
pub fn record_connected(peer_id: &str, labels: Vec<String>) {
    let mut reg = match registry().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(armed) = reg.armed.as_mut() {
        armed.peers.insert(
            peer_id.to_string(),
            PeerEntry {
                labels,
                last_seen: Instant::now(),
            },
        );
    }
}

/// Refresh a connected peer's last-seen stamp. Called from the transport's
/// ping-success arm — deliberately cheap (no lookup, no allocation beyond the
/// key compare) because it fires once per peer per ping interval.
pub fn touch(peer_id: &str) {
    let mut reg = match registry().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(armed) = reg.armed.as_mut() {
        if let Some(entry) = armed.peers.get_mut(peer_id) {
            entry.last_seen = Instant::now();
        }
    }
}

/// Drop a peer from the connected set. Called from the transport's
/// connection-closed arm — including the close the ping-failure arm performs,
/// which is what makes a SIGKILLed peer leave `live` within the ping budget.
pub fn record_disconnected(peer_id: &str) {
    let mut reg = match registry().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(armed) = reg.armed.as_mut() {
        armed.peers.remove(peer_id);
    }
}

/// Every label the local transport currently sees connected, plus this node's
/// own — or `None` when no plane has armed the registry, which readers MUST
/// treat as "unmeasured" and fall back on rather than as an empty set.
///
/// Entries older than the armed TTL are excluded: the explicit disconnect is the
/// fast path, and this is the backstop for an event loop that stopped turning.
pub fn connected_snapshot() -> Option<HashSet<String>> {
    let reg = match registry().read() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let armed = reg.armed.as_ref()?;
    let now = Instant::now();
    let mut out: HashSet<String> = armed.own_labels.iter().cloned().collect();
    for entry in armed.peers.values() {
        if now.duration_since(entry.last_seen) <= armed.ttl {
            out.extend(entry.labels.iter().cloned());
        }
    }
    Some(out)
}

/// This node's OWN labels, or `None` while the registry is unarmed.
///
/// Published separately from [`connected_snapshot`] because they answer a
/// different question. The connected set is evidence about OTHER peers — it is
/// assembled from `ConnectionEstablished`, and a node is never in its own. The
/// local labels are not evidence at all: a peer that is answering the request is
/// alive by definition, and no transport event can tell it so.
///
/// Readers that fold a household peer set (see
/// `services::household_resilience::count_household_peers`) must union the two,
/// or a three-peer household can never read three live from ANY member's fold —
/// each one silently omits itself and reports `2 of 3` with nothing wrong
/// (household mesh run 20260912T224420Z: all three peers up, connected 2/2/2,
/// card still read the "partial" rung before a single kill).
pub fn local_labels() -> Option<Vec<String>> {
    let reg = match registry().read() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    Some(reg.armed.as_ref()?.own_labels.clone())
}

/// Drop the registry back to UNARMED. Test-only: the registry is process-global,
/// so a test that arms it would otherwise leak its connected set into every
/// later test in the same binary.
#[cfg(test)]
pub(crate) fn reset_for_test() {
    let mut reg = match registry().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    reg.armed = None;
}

/// The registry is process-global, so EVERY test that arms or reads it runs
/// under this one mutex rather than racing the others — including the household
/// -resilience folds in another module, which now read the local labels and
/// would otherwise see a neighbouring test's arm/reset (the parallel-test flake
/// class the env-var discipline names).
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        test_guard()
    }

    #[test]
    fn unarmed_is_unmeasured_not_empty() {
        let _g = guard();
        reset_for_test();
        assert!(
            connected_snapshot().is_none(),
            "a process with no transport plane must report UNMEASURED — an empty set \
             would read as 'nobody is connected' and dark every card in the fleet"
        );
    }

    #[test]
    fn an_armed_registry_reports_itself_live_with_no_peers() {
        let _g = guard();
        reset_for_test();
        arm(vec!["agent:self".into()], DEFAULT_LIVENESS_TTL);
        let snap = connected_snapshot().expect("armed");
        assert_eq!(snap.len(), 1);
        assert!(
            snap.contains("agent:self"),
            "a household of one is still live"
        );
        reset_for_test();
    }

    #[test]
    fn a_disconnected_peer_leaves_the_live_set() {
        let _g = guard();
        reset_for_test();
        arm(vec!["agent:self".into()], DEFAULT_LIVENESS_TTL);
        record_connected(
            "12D3KooWJessica",
            vec!["12D3KooWJessica".into(), "agent:jessica".into()],
        );
        record_connected(
            "12D3KooWJames",
            vec!["12D3KooWJames".into(), "agent:james".into()],
        );
        assert!(connected_snapshot().unwrap().contains("agent:jessica"));

        // The ping-failure arm closes the connection of a peer gone silent.
        record_disconnected("12D3KooWJessica");
        let snap = connected_snapshot().unwrap();
        assert!(
            !snap.contains("agent:jessica") && !snap.contains("12D3KooWJessica"),
            "a killed peer must leave `live` under BOTH its labels"
        );
        assert!(
            snap.contains("agent:james"),
            "its household-mate is untouched"
        );
        reset_for_test();
    }

    /// The backstop: an event loop that stops turning must not leave a dead peer
    /// live forever. A zero TTL expires everything but the node's own labels.
    #[test]
    fn a_stale_entry_expires_even_without_a_close_event() {
        let _g = guard();
        reset_for_test();
        arm(vec!["agent:self".into()], Duration::from_nanos(1));
        record_connected("12D3KooWJessica", vec!["agent:jessica".into()]);
        std::thread::sleep(Duration::from_millis(5));
        let snap = connected_snapshot().unwrap();
        assert!(
            !snap.contains("agent:jessica"),
            "past the TTL a peer is stale, close event or not"
        );
        assert!(snap.contains("agent:self"));
        reset_for_test();
    }

    #[test]
    fn a_touch_keeps_a_peer_live_across_the_ttl() {
        let _g = guard();
        reset_for_test();
        arm(vec!["agent:self".into()], Duration::from_millis(50));
        record_connected("12D3KooWJessica", vec!["agent:jessica".into()]);
        std::thread::sleep(Duration::from_millis(30));
        touch("12D3KooWJessica");
        std::thread::sleep(Duration::from_millis(30));
        assert!(
            connected_snapshot().unwrap().contains("agent:jessica"),
            "a ping-success refresh is what keeps a quiet-but-live peer in the set"
        );
        reset_for_test();
    }

    /// The local labels are the node's own, never a peer's — the union a
    /// household fold needs so the answering peer counts itself.
    #[test]
    fn local_labels_are_unmeasured_until_armed_then_name_this_node() {
        let _g = guard();
        reset_for_test();
        assert!(
            local_labels().is_none(),
            "unarmed is UNMEASURED here too — a reader must fall back, not assume"
        );
        arm(
            vec!["12D3KooWSelf".into(), "uhCAkSelf".into()],
            DEFAULT_LIVENESS_TTL,
        );
        record_connected("12D3KooWJessica", vec!["uhCAkJessica".into()]);
        let local = local_labels().expect("armed");
        assert_eq!(local.len(), 2);
        assert!(local.contains(&"uhCAkSelf".to_string()));
        assert!(
            !local.contains(&"uhCAkJessica".to_string()),
            "a connected peer is not this node"
        );
        reset_for_test();
    }

    #[test]
    fn re_arming_keeps_the_observed_peers() {
        let _g = guard();
        reset_for_test();
        arm(vec!["agent:self".into()], DEFAULT_LIVENESS_TTL);
        record_connected("12D3KooWJessica", vec!["agent:jessica".into()]);
        arm(vec!["agent:self".into()], DEFAULT_LIVENESS_TTL);
        assert!(
            connected_snapshot().unwrap().contains("agent:jessica"),
            "a re-arm must never blank a live set"
        );
        reset_for_test();
    }
}
