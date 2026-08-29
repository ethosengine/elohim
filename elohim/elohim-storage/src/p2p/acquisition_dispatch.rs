//! Acquisition (pull-leg) dispatch planning across BOTH transport planes.
//!
//! `drain_acquisition_queue` used to gate on `swarm.connected_peers()` and dispatch
//! every `GetContent` over libp2p only — so on a pure-iroh mesh the pull queue never
//! moved (`homo-iroh` failed P3 `pull.caughtUp` on the 2026-08-24 matrix), and on a
//! dual mesh iroh carried none of the bulk bytes (fleet 24h: ~700 iroh sync rounds per
//! pod, 8 iroh blob fetches fleet-wide). This module is the pure planning half of the
//! cure (spec 2026-08-24 transport self-awareness, row 13): take the libp2p-connected
//! set and the iroh peer book, and produce ONE target per peer.
//!
//! Selection prior: when a peer is reachable on both planes, iroh is chosen — the
//! complementarity canon's rule 2 ("prefer iroh when both support the plane") stands in
//! until `PathObservation`/`select_path` (rows 1–4) can pick by measured RTT. It can be
//! turned off (`prefer_iroh = false`, env `ELOHIM_ACQUISITION_IROH=off`) so a first fleet
//! landing has a one-line rollback that does not touch the plan.
//!
//! Concern-canon answers (C4 honest absence): an iroh-only peer is a real target, never
//! dropped for lacking a libp2p id; a libp2p-only peer stays a target when no book entry
//! joins it. Order is deterministic (libp2p order, then iroh-only in book order) so the
//! rotation index that walks retries to a different peer keeps its meaning.

use libp2p::PeerId;

/// One dispatch target for a queued content id.
#[derive(Debug, Clone)]
pub enum AcquisitionTarget {
    /// Send `ShardRequest::GetContent` over the libp2p shard protocol.
    Libp2p(PeerId),
    /// Send the same request over the iroh shard ALPN.
    #[cfg(feature = "p2p-iroh")]
    Iroh {
        /// Human/metric label for the peer: its libp2p id when known, else its agent
        /// cid, else the iroh node id — bounded to what the book already holds.
        label: String,
        addr: iroh::NodeAddr,
    },
}

impl AcquisitionTarget {
    pub fn transport(&self) -> &'static str {
        match self {
            AcquisitionTarget::Libp2p(_) => "libp2p",
            #[cfg(feature = "p2p-iroh")]
            AcquisitionTarget::Iroh { .. } => "iroh",
        }
    }
}

/// Is iroh preferred for acquisition dispatch? Read once per process:
/// `ELOHIM_ACQUISITION_IROH=off|0|false|no` turns the iroh plane off for the pull leg
/// (the libp2p behaviour is then byte-identical to before this module existed).
pub fn prefer_iroh_from_env() -> bool {
    match std::env::var("ELOHIM_ACQUISITION_IROH") {
        Ok(v) => !matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "off" | "0" | "false" | "no"
        ),
        Err(_) => true,
    }
}

/// Union the libp2p-connected set with the iroh peer book into one target per peer.
///
/// `iroh_entries` is the book snapshot EXCLUDING self. When `prefer_iroh` is false the
/// iroh plane is used only for peers that have no libp2p connection at all — iroh-only
/// peers are never left unreachable, whatever the preference.
/// Static planner — `prefer_iroh` decides every dual peer the same way.
#[cfg(feature = "p2p-iroh")]
pub fn plan_acquisition_targets(
    libp2p_peers: &[PeerId],
    iroh_entries: &[crate::p2p_iroh::IrohPeerEntry],
    prefer_iroh: bool,
) -> Vec<AcquisitionTarget> {
    plan_acquisition_targets_routed(libp2p_peers, iroh_entries, prefer_iroh, &|_| None)
}

/// Evidence-routed planner (spec 2026-08-24 §3.1, Bulk class): a dual peer's
/// plane comes from `transport_paths::select_path` over its observed RTT /
/// success per plane — best plane by evidence, exploration floor so the other
/// plane keeps earning samples. Falls back to the static `prefer_iroh` prior
/// when selection is off (`ELOHIM_TRANSPORT_SELECTION=off`) or iroh is
/// disabled for the pull leg (`ELOHIM_ACQUISITION_IROH=off`).
#[cfg(feature = "p2p-iroh")]
pub fn plan_acquisition_targets_selected(
    libp2p_peers: &[PeerId],
    iroh_entries: &[crate::p2p_iroh::IrohPeerEntry],
    prefer_iroh: bool,
) -> Vec<AcquisitionTarget> {
    use crate::p2p::transport_paths::{global, OpClass, Route, Transport};
    if !prefer_iroh {
        return plan_acquisition_targets(libp2p_peers, iroh_entries, false);
    }
    let choose = |label: &str| -> Option<Transport> {
        match global().route(label, &[Transport::Libp2p, Transport::Iroh], OpClass::Bulk) {
            Route::Single(t) => Some(t),
            Route::Race(ts) => ts.first().copied(),
            Route::None => None,
        }
    };
    plan_acquisition_targets_routed(libp2p_peers, iroh_entries, prefer_iroh, &choose)
}

/// The planner proper. `choose(label)` answers the plane for a peer known on
/// BOTH planes (`None` → the static `prefer_iroh` prior); single-plane peers
/// are never consulted — there is nothing to select.
#[cfg(feature = "p2p-iroh")]
pub fn plan_acquisition_targets_routed(
    libp2p_peers: &[PeerId],
    iroh_entries: &[crate::p2p_iroh::IrohPeerEntry],
    prefer_iroh: bool,
    choose: &dyn Fn(&str) -> Option<crate::p2p::transport_paths::Transport>,
) -> Vec<AcquisitionTarget> {
    let mut targets: Vec<AcquisitionTarget> =
        Vec::with_capacity(libp2p_peers.len() + iroh_entries.len());
    let mut claimed: Vec<usize> = Vec::new(); // indexes into iroh_entries joined to a libp2p peer
    for peer in libp2p_peers {
        let peer_str = peer.to_string();
        let joined = iroh_entries
            .iter()
            .position(|e| e.libp2p_peer_id.as_deref() == Some(peer_str.as_str()));
        match joined {
            Some(idx) => {
                claimed.push(idx);
                let use_iroh = match choose(&peer_str) {
                    Some(crate::p2p::transport_paths::Transport::Iroh) => true,
                    Some(crate::p2p::transport_paths::Transport::Libp2p) => false,
                    None => prefer_iroh,
                };
                if use_iroh {
                    targets.push(AcquisitionTarget::Iroh {
                        label: peer_str,
                        addr: iroh_entries[idx].addr.clone(),
                    });
                } else {
                    targets.push(AcquisitionTarget::Libp2p(*peer));
                }
            }
            None => targets.push(AcquisitionTarget::Libp2p(*peer)),
        }
    }
    for (idx, entry) in iroh_entries.iter().enumerate() {
        if claimed.contains(&idx) {
            continue;
        }
        // An iroh-only peer (no libp2p connection right now) is dispatchable on iroh
        // regardless of the preference — otherwise a pure-iroh mesh has no pull path.
        let label = entry
            .libp2p_peer_id
            .clone()
            .or_else(|| entry.agent_cid.clone())
            .unwrap_or_else(|| entry.addr.node_id.to_string());
        targets.push(AcquisitionTarget::Iroh {
            label,
            addr: entry.addr.clone(),
        });
    }
    targets
}

/// libp2p-only build: the plan is the connected set, unchanged.
#[cfg(not(feature = "p2p-iroh"))]
pub fn plan_acquisition_targets(
    libp2p_peers: &[PeerId],
    _prefer_iroh: bool,
) -> Vec<AcquisitionTarget> {
    libp2p_peers
        .iter()
        .map(|p| AcquisitionTarget::Libp2p(*p))
        .collect()
}

#[cfg(all(test, feature = "p2p-iroh"))]
mod tests {
    use super::*;
    use crate::p2p_iroh::IrohPeerEntry;
    use iroh::{NodeAddr, SecretKey};

    fn entry(
        key: &SecretKey,
        agent_cid: Option<&str>,
        libp2p_peer_id: Option<&str>,
    ) -> IrohPeerEntry {
        IrohPeerEntry {
            addr: NodeAddr::new(key.public())
                .with_direct_addresses([([127, 0, 0, 1], 4433u16).into()]),
            agent_cid: agent_cid.map(|s| s.to_string()),
            libp2p_peer_id: libp2p_peer_id.map(|s| s.to_string()),
            announced_at_ms: 1,
        }
    }

    fn peer() -> PeerId {
        PeerId::random()
    }

    #[test]
    fn iroh_only_peer_is_dispatchable_with_no_libp2p_connections() {
        // The homo-iroh P3 defect: a pure-iroh mesh must still have a pull path.
        let key = SecretKey::generate(rand::rngs::OsRng);
        let book = vec![entry(&key, Some("agent-a"), None)];
        let plan = plan_acquisition_targets(&[], &book, true);
        assert_eq!(plan.len(), 1);
        assert!(matches!(&plan[0], AcquisitionTarget::Iroh { label, .. } if label == "agent-a"));
    }

    #[test]
    fn dual_peer_yields_one_target_on_iroh_when_preferred() {
        let p = peer();
        let key = SecretKey::generate(rand::rngs::OsRng);
        let book = vec![entry(&key, None, Some(&p.to_string()))];
        let plan = plan_acquisition_targets(&[p], &book, true);
        assert_eq!(plan.len(), 1, "a dual peer must not be dispatched twice");
        assert!(
            matches!(&plan[0], AcquisitionTarget::Iroh { label, .. } if *label == p.to_string())
        );
    }

    #[test]
    fn dual_peer_stays_on_libp2p_when_iroh_not_preferred() {
        let p = peer();
        let key = SecretKey::generate(rand::rngs::OsRng);
        let book = vec![entry(&key, None, Some(&p.to_string()))];
        let plan = plan_acquisition_targets(&[p], &book, false);
        assert_eq!(plan.len(), 1);
        assert!(matches!(&plan[0], AcquisitionTarget::Libp2p(id) if *id == p));
    }

    #[test]
    fn libp2p_only_peers_are_preserved_and_ordered_first() {
        let a = peer();
        let b = peer();
        let key = SecretKey::generate(rand::rngs::OsRng);
        let book = vec![entry(&key, Some("iroh-only"), None)];
        let plan = plan_acquisition_targets(&[a, b], &book, true);
        assert_eq!(plan.len(), 3);
        assert!(matches!(&plan[0], AcquisitionTarget::Libp2p(id) if *id == a));
        assert!(matches!(&plan[1], AcquisitionTarget::Libp2p(id) if *id == b));
        assert!(matches!(&plan[2], AcquisitionTarget::Iroh { label, .. } if label == "iroh-only"));
    }

    #[test]
    fn empty_inputs_plan_nothing() {
        assert!(plan_acquisition_targets(&[], &[], true).is_empty());
    }

    #[test]
    fn env_switch_defaults_on_and_honours_off() {
        std::env::remove_var("ELOHIM_ACQUISITION_IROH");
        assert!(prefer_iroh_from_env());
        std::env::set_var("ELOHIM_ACQUISITION_IROH", "off");
        assert!(!prefer_iroh_from_env());
        std::env::remove_var("ELOHIM_ACQUISITION_IROH");
    }
}
