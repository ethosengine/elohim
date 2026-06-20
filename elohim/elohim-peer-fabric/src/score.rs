//! Capability-aware peer ranking — a pure composer over operational signals.

/// One candidate peer for serve-routing, composed from operational signals
/// (NodeRegistration capability, NodeHeartbeat load, HealthAttestation RTT, Mishpat bond, delivery history).
#[derive(Debug, Clone)]
pub struct Candidate {
    pub agent_cid: String,
    pub capability_level: u8,
    pub current_load: f64,          // 0.0..=1.0 (NodeHeartbeat.current_load)
    pub attested_rtt_ms: Option<u32>, // HealthAttestation.response_time_ms; None = not yet attested
    pub household_id: String,        // fault-domain key
    pub bonded: bool,                // backed by a replicates-* / delegates-compute commitment
    pub delivery_score: f64,         // 0.0..=1.0 decaying delivery-success (advertise-then-drop decays it)
}

/// A ranked peer. Higher `score` = preferred.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredPeer {
    pub agent_cid: String,
    pub score: f64,
}

/// Rank candidates that meet `min_capability` AND have headroom (`current_load < 1.0`), best-first.
/// Score = headroom*0.4 + rtt_factor*0.3 + delivery*0.2 + bond*0.1. Unknown RTT → neutral 0.5 (graceful
/// degradation: a not-yet-attested peer is rankable, not crashed and not unfairly penalized). An empty
/// result means "no peer has headroom" → the caller sheds (503), never fans out.
pub fn rank(candidates: &[Candidate], min_capability: u8) -> Vec<ScoredPeer> {
    let mut scored: Vec<ScoredPeer> = candidates
        .iter()
        .filter(|c| c.capability_level >= min_capability && c.current_load < 1.0)
        .map(|c| {
            let headroom = (1.0 - c.current_load).clamp(0.0, 1.0);
            let rtt_factor = match c.attested_rtt_ms {
                Some(ms) => 1.0 / (1.0 + ms as f64 / 100.0), // 0ms→1.0, 100ms→0.5, 300ms→0.25
                None => 0.5,
            };
            let bond_factor = if c.bonded { 1.0 } else { 0.5 };
            let delivery = c.delivery_score.clamp(0.0, 1.0);
            let score = headroom * 0.4 + rtt_factor * 0.3 + delivery * 0.2 + bond_factor * 0.1;
            ScoredPeer { agent_cid: c.agent_cid.clone(), score }
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(cid: &str, cap: u8, load: f64, rtt: Option<u32>, hh: &str, bonded: bool, delivery: f64) -> Candidate {
        Candidate { agent_cid: cid.into(), capability_level: cap, current_load: load, attested_rtt_ms: rtt, household_id: hh.into(), bonded, delivery_score: delivery }
    }

    #[test]
    fn capability_floor_filters_out_incapable_peers() {
        let cs = vec![cand("low", 1, 0.1, Some(10), "h1", true, 1.0), cand("ok", 5, 0.1, Some(10), "h2", true, 1.0)];
        let r = rank(&cs, 3);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].agent_cid, "ok");
    }

    #[test]
    fn more_headroom_ranks_higher() {
        let cs = vec![cand("busy", 5, 0.9, Some(10), "h1", true, 1.0), cand("idle", 5, 0.1, Some(10), "h2", true, 1.0)];
        let r = rank(&cs, 0);
        assert_eq!(r[0].agent_cid, "idle", "the less-loaded peer should rank first");
    }

    #[test]
    fn saturated_peers_are_excluded_so_caller_can_shed() {
        let cs = vec![cand("full", 5, 1.0, Some(10), "h1", true, 1.0)];
        assert!(rank(&cs, 0).is_empty(), "load>=1.0 means no headroom → not a candidate → caller sheds");
    }

    #[test]
    fn unknown_rtt_degrades_gracefully_not_crashes() {
        let cs = vec![cand("nort", 5, 0.1, None, "h1", true, 0.5)];
        let r = rank(&cs, 0);
        assert_eq!(r.len(), 1, "a peer with no attested RTT is still rankable (neutral rtt factor)");
    }

    #[test]
    fn lower_rtt_ranks_higher_when_all_else_equal() {
        let cs = vec![cand("far", 5, 0.1, Some(300), "h1", true, 1.0), cand("near", 5, 0.1, Some(10), "h2", true, 1.0)];
        let r = rank(&cs, 0);
        assert_eq!(r[0].agent_cid, "near");
    }
}
