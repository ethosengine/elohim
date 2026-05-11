//! Gossipsub topic naming for EPR atom federation.
//!
//! Naming scheme: `elohim/<pillar>/<reach>/[<collective-id>]`.
//! Plus two well-known integrity topics that ALWAYS receive integrity-kind
//! atoms regardless of reach: identity binding + integrity revocation.

use elohim_epr::Reach;

/// Build the gossipsub topic name for a (pillar, reach, optional collective).
///
/// # Examples
///
/// ```
/// use elohim_epr::Reach;
/// use elohim_storage::p2p::topics::topic_for;
///
/// assert_eq!(topic_for("lamad", Reach::Community, None), "elohim/lamad/community");
/// assert_eq!(topic_for("shefa", Reach::Trusted, Some("coll-42")), "elohim/shefa/trusted/coll-42");
/// ```
pub fn topic_for(pillar: &str, reach: Reach, collective: Option<&str>) -> String {
    let base = format!("elohim/{}/{}", pillar, reach_segment(reach));
    match collective {
        Some(c) => format!("{}/{}", base, c),
        None => base,
    }
}

fn reach_segment(reach: Reach) -> &'static str {
    match reach {
        Reach::Private => "private",
        Reach::SelfScope => "self",
        Reach::Intimate => "intimate",
        Reach::Trusted => "trusted",
        Reach::Familiar => "familiar",
        Reach::Community => "community",
        Reach::Public => "public",
        Reach::Commons => "commons",
    }
}

/// Identity binding topic — already exists as IDENTITY_BINDING_TOPIC in
/// p2p/identity_binding_gossip.rs (= "elohim/identity/binding"). Re-exported
/// here as an alias so D.3+ callers have a single import surface.
pub const TOPIC_IDENTITY_BINDING: &str =
    crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC;

/// Integrity-revocation topic — Phase 2B Batch D introduces this name as the
/// canonical integrity-kind revocation channel. The legacy `recovery.revocation`
/// topic remains subscribed for backward compatibility (M3/M4 publishers still
/// use it); this new name is what new publishers (D.5 direct-notify) and new
/// subscribers should use. See spec §3.7 integrity-exception.
pub const TOPIC_INTEGRITY_REVOCATION: &str = "elohim/integrity/revocation";

/// Well-known integrity topics — always subscribed at startup regardless of
/// any per-pillar or reach-scoped subscription gating.
///
/// D.4 owns subscription enforcement for per-pillar reach-scoped topics; this
/// slice covers only the two integrity channels that D.3 introduces.
pub fn integrity_topics() -> &'static [&'static str] {
    &[TOPIC_IDENTITY_BINDING, TOPIC_INTEGRITY_REVOCATION]
}

/// libp2p request-response protocol id for fetching observation-log segments
/// from an observer's iroh-blob log.
pub const OBSERVATION_LOG_PROTOCOL_ID: &str = "/elohim/observation/1.0.0";

/// Gossipsub topic prefix for observation cursor announcements. Per-kind
/// topics are formed by appending the namespace, e.g.
/// `elohim/observations/infrastructure`.
pub const OBSERVATION_GOSSIP_TOPIC_PREFIX: &str = "elohim/observations/";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // topic_for: all reach segments
    // -----------------------------------------------------------------------

    #[test]
    fn topic_for_private() {
        assert_eq!(
            topic_for("lamad", Reach::Private, None),
            "elohim/lamad/private"
        );
    }

    #[test]
    fn topic_for_self_scope() {
        assert_eq!(
            topic_for("lamad", Reach::SelfScope, None),
            "elohim/lamad/self"
        );
    }

    #[test]
    fn topic_for_intimate() {
        assert_eq!(
            topic_for("lamad", Reach::Intimate, None),
            "elohim/lamad/intimate"
        );
    }

    #[test]
    fn topic_for_trusted() {
        assert_eq!(
            topic_for("lamad", Reach::Trusted, None),
            "elohim/lamad/trusted"
        );
    }

    #[test]
    fn topic_for_familiar() {
        assert_eq!(
            topic_for("lamad", Reach::Familiar, None),
            "elohim/lamad/familiar"
        );
    }

    #[test]
    fn topic_for_community() {
        assert_eq!(
            topic_for("lamad", Reach::Community, None),
            "elohim/lamad/community"
        );
    }

    #[test]
    fn topic_for_public() {
        assert_eq!(
            topic_for("lamad", Reach::Public, None),
            "elohim/lamad/public"
        );
    }

    #[test]
    fn topic_for_commons() {
        assert_eq!(
            topic_for("lamad", Reach::Commons, None),
            "elohim/lamad/commons"
        );
    }

    // -----------------------------------------------------------------------
    // topic_for: collective suffix
    // -----------------------------------------------------------------------

    #[test]
    fn topic_for_with_collective_appends_suffix() {
        assert_eq!(
            topic_for("shefa", Reach::Trusted, Some("coll-abc")),
            "elohim/shefa/trusted/coll-abc"
        );
    }

    #[test]
    fn topic_for_with_collective_community() {
        assert_eq!(
            topic_for("qahal", Reach::Community, Some("household-42")),
            "elohim/qahal/community/household-42"
        );
    }

    // -----------------------------------------------------------------------
    // Integrity constants
    // -----------------------------------------------------------------------

    #[test]
    fn topic_identity_binding_is_non_empty_and_expected() {
        assert!(!TOPIC_IDENTITY_BINDING.is_empty());
        assert_eq!(TOPIC_IDENTITY_BINDING, "elohim/identity/binding");
    }

    #[test]
    fn topic_integrity_revocation_is_non_empty_and_expected() {
        assert!(!TOPIC_INTEGRITY_REVOCATION.is_empty());
        assert_eq!(TOPIC_INTEGRITY_REVOCATION, "elohim/integrity/revocation");
    }

    #[test]
    fn integrity_topics_contains_both_constants() {
        let topics = integrity_topics();
        assert!(
            topics.contains(&TOPIC_IDENTITY_BINDING),
            "integrity_topics must include TOPIC_IDENTITY_BINDING"
        );
        assert!(
            topics.contains(&TOPIC_INTEGRITY_REVOCATION),
            "integrity_topics must include TOPIC_INTEGRITY_REVOCATION"
        );
    }

    // -----------------------------------------------------------------------
    // topic_for: different pillars produce distinct topics
    // -----------------------------------------------------------------------

    #[test]
    fn different_pillars_produce_distinct_topics() {
        let a = topic_for("lamad", Reach::Public, None);
        let b = topic_for("shefa", Reach::Public, None);
        assert_ne!(a, b, "different pillars must produce distinct topics");
    }
}
