//! Graph-grounded reach authorization — author-side earning + receiver-side
//! pre-authorization classification.
//!
//! ## Two faces of the same principle
//!
//! Reach is coupled to embodied responsibilities at every node on the network.
//! That coupling has two faces:
//!
//!   1. **Author-side earning** — the signer of an EPR must have *earned* the
//!      declared reach (relationship / membership / delegation in the graph).
//!      `classify_reach_authorization` decides; `signer_is_known_agent`
//!      resolves Stage 1's structural check.
//!
//!   2. **Receiver-side pre-authorization** — a peer takes on standing in a
//!      scope by having embodied responsibilities for it (membership /
//!      stewardship / relationship). This decides which topics this node
//!      subscribes to, which Kad provider records it advertises, which
//!      validations it participates in. It is NOT a per-message filter.
//!      `classify_pre_authorization` decides; `node_has_embodied_responsibility`
//!      resolves Stage 1's structural check.
//!
//! ## Why author-side earning, not receive-side filtering
//!
//! Email collapsed because anyone could publish to anyone, putting the cost
//! of filtering on receivers. The asymmetry produced spam at scale; only
//! megacorps can afford to filter, so the protocol stopped working for
//! humans. The Elohim Protocol places the burden where it belongs: on the
//! author + the peers that steward what they author. Receivers trust what
//! arrives because authors had to earn — and pre-authorized peers carry the
//! responsibility of distributing/discovering/validating in their scopes.
//!
//! See memory pins `project_reach_earned_at_authoring` and
//! `project_first_class_graph_pattern`.
//!
//! ## Stage 1 (current — Bootstrap Social)
//!
//! Structural earning + structural pre-authorization, sufficient as a floor.
//!
//!   - **Author-side:** Public/Commons open; Private/SelfScope trivially earned
//!     by signing; Intimate/Trusted/Familiar/Community require signer to be a
//!     "known agent" (peer_identity_bindings row OR local conductor agent).
//!
//!   - **Receiver-side:** integrity topics + Public/Commons of any pillar are
//!     always pre-authorized (every node carries integrity); per-pillar
//!     Intimate/Trusted/Familiar/Community require this node to have a local
//!     agent with a peer_identity_bindings row attesting standing in the scope.
//!
//! ## Stage 2/3 — open question O2
//!
//! The graph walks land in Stage 2/3:
//!   - Author-side: signer presents relationship/membership/delegation proof
//!     (DHT-anchored, walked from the envelope claims)
//!   - Receiver-side: node walks its embodied-responsibility edges (qahal
//!     household memberships, imagodei attestations) to determine standing
//!
//! Both are graph traversals — see `project_first_class_graph_pattern`.

use elohim_epr::Reach;

/// The outcome of a Stage 1 reach earning classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReachAuthDecision {
    /// Signer has earned the declared reach. Proceed with put + publish.
    Authorized,
    /// Signer has NOT earned the declared reach. Refuse the put.
    Refused { reason: String },
}

/// Stage 1 reach earning decision.
///
/// Inputs:
///   - `reach`: the declared reach on the EPR envelope
///   - `signer_cid`: the signer CID from `epr.envelope.proof.signer`
///   - `signer_is_known_agent`: true if (a) the signer has a row in
///     `peer_identity_bindings` OR (b) the signer is the local conductor
///     agent. Caller resolves this from the diesel pool + local agent
///     pubkey before calling. Pure function so it stays unit-testable.
pub fn classify_reach_authorization(
    reach: Reach,
    signer_cid: &str,
    signer_is_known_agent: bool,
) -> ReachAuthDecision {
    match reach {
        Reach::Public | Reach::Commons => ReachAuthDecision::Authorized,
        Reach::Private | Reach::SelfScope => {
            // Reach is the signer's own scope; trivially earned by signing.
            // Proper signature verification is upstream of this function.
            ReachAuthDecision::Authorized
        }
        Reach::Intimate | Reach::Trusted | Reach::Familiar | Reach::Community => {
            if signer_is_known_agent {
                ReachAuthDecision::Authorized
            } else {
                ReachAuthDecision::Refused {
                    reason: format!(
                        "signer {} is not a known agent — Stage 1 requires \
                         peer_identity_bindings row or local-conductor signer \
                         to author at reach {:?}. Stage 2/3 will require \
                         relationship/membership attestation in the envelope.",
                        signer_cid, reach
                    ),
                }
            }
        }
    }
}

/// Stage 1 resolver: is the signer of this EPR a "known agent" to this node?
///
/// Returns true if either:
///   1. signer_cid matches the local conductor's agent CID (this node's
///      own user authored the EPR), OR
///   2. there is at least one row in `peer_identity_bindings` whose
///      `agent_cid` equals the signer_cid.
///
/// Returns true on db pool errors (fail-open at Stage 1 to avoid refusing
/// legitimate puts during projection-rebuild windows). Stage 2 tightens
/// to fail-closed.
pub fn signer_is_known_agent(
    pool: &crate::db::DbPool,
    local_agent_cid: Option<&str>,
    signer_cid: &str,
) -> bool {
    if local_agent_cid == Some(signer_cid) {
        return true;
    }
    use crate::db::diesel_schema::peer_identity_bindings::dsl as p;
    use diesel::prelude::*;
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "reach_authorization: db pool get failed; failing open at Stage 1"
            );
            return true;
        }
    };
    p::peer_identity_bindings
        .filter(p::agent_cid.eq(signer_cid))
        .select(p::peer_id)
        .first::<String>(&mut conn)
        .map(|_| true)
        .unwrap_or_else(|e| match e {
            diesel::result::Error::NotFound => false,
            _ => {
                tracing::warn!(
                    error = %e,
                    signer = %signer_cid,
                    "reach_authorization: db query failed; failing open at Stage 1"
                );
                true
            }
        })
}

// ---------------------------------------------------------------------------
// Receiver-side: classify pre-authorization for a scope (topic-as-scope-proxy)
// ---------------------------------------------------------------------------

/// Outcome of a Stage 1 receiver-side pre-authorization classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreAuthorizationDecision {
    /// This node has standing in the scope — subscribe / provide / validate
    /// for it.
    Standing,
    /// This node has no standing in the scope — do not subscribe, do not
    /// advertise as a provider, do not participate in validation. (This is
    /// the topology decision, not a filter on inbound traffic.)
    NoStanding,
}

/// Stage 1 pre-authorization decision for the RECEIVER side.
///
/// "Does this node have standing to subscribe / provide / validate in the
/// scope identified by this topic?" Pure function — caller resolves the
/// `node_has_embodied_responsibility` boolean before calling.
///
/// This is NOT a per-message filter. It is the topology decision: which
/// scopes does this node participate in? Pre-authorization, not filtering.
///
/// Inputs:
///   - `topic`: the gossipsub topic name (`elohim/<pillar>/<reach>[/<collective>]`
///     or one of the integrity topic constants)
///   - `node_has_embodied_responsibility`: true if this node has at least one
///     local agent with a peer_identity_bindings row standing in the topic's
///     scope. Stage 2/3 will tighten with graph walks.
///
/// ## Subscription wiring note
///
/// `classify_pre_authorization` will be called when storage decides which
/// topics to subscribe to and which collectives to act as Kad provider for.
/// That wiring is downstream of D.4 (Phase 3+ work) — D.4 ships the pure
/// policy module + tests so Stage 2/3 graph walks land at a clear seam
/// without rewriting call sites. The actual subscription / Kad-provider /
/// validation circuit lives in the swarm event loop; D.4 does NOT add any
/// subscription logic.
pub fn classify_pre_authorization(
    topic: &str,
    node_has_embodied_responsibility: bool,
) -> PreAuthorizationDecision {
    // Integrity topics: every node carries integrity (revocation, identity
    // binding). Always pre-authorized.
    if topic == crate::p2p::topics::TOPIC_IDENTITY_BINDING
        || topic == crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION
        || topic == "recovery.invitation"
        || topic == "recovery.revocation"
    {
        return PreAuthorizationDecision::Standing;
    }

    // Per-pillar topic: parse `elohim/<pillar>/<reach>[/<collective>]`
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() >= 3 && parts[0] == "elohim" {
        match parts[2] {
            // Public/Commons broadcast — every node may stand in (Stage 1).
            // Stage 2 may still require minimum capacity / serving capability.
            "public" | "commons" => PreAuthorizationDecision::Standing,

            // Private/SelfScope — should never appear in gossipsub. If it
            // does, refuse standing (defensive).
            "private" | "self" => PreAuthorizationDecision::NoStanding,

            // Bound tiers — node must have embodied responsibility.
            // Stage 1: structural check (any peer_identity_bindings row);
            // Stage 2/3: graph walk (qahal household membership, imagodei
            // relationship attestation).
            "intimate" | "trusted" | "familiar" | "community" => {
                if node_has_embodied_responsibility {
                    PreAuthorizationDecision::Standing
                } else {
                    PreAuthorizationDecision::NoStanding
                }
            }
            _ => PreAuthorizationDecision::NoStanding,
        }
    } else {
        PreAuthorizationDecision::NoStanding
    }
}

/// Stage 1 resolver for the receiver-side `node_has_embodied_responsibility`
/// boolean. Returns true if this node has ANY peer_identity_bindings row
/// (i.e., the node is a real participant in the network, not a stranger).
///
/// Stage 2/3 will tighten by parsing the topic's scope (collective ID) and
/// walking the qahal/imagodei graph for membership/relationship attestations
/// matching that scope. The function signature is intentionally placeholder-
/// shaped so Stage 2/3 can extend it without rewriting call sites.
///
/// Returns true on db pool errors (fail-open at Stage 1).
pub fn node_has_embodied_responsibility(
    pool: &crate::db::DbPool,
    _topic: &str, // unused at Stage 1; threaded for Stage 2/3 scope-walk
) -> bool {
    use crate::db::diesel_schema::peer_identity_bindings::dsl as p;
    use diesel::prelude::*;
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "reach_authorization: db pool get failed; failing open at Stage 1"
            );
            return true;
        }
    };
    p::peer_identity_bindings
        .select(p::peer_id)
        .first::<String>(&mut conn)
        .map(|_| true)
        .unwrap_or_else(|e| match e {
            diesel::result::Error::NotFound => false,
            _ => {
                tracing::warn!(
                    error = %e,
                    "reach_authorization: db query failed; failing open at Stage 1"
                );
                true
            }
        })
}

// ---------------------------------------------------------------------------
// Unit tests — pure function coverage (8 minimum per spec)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::Reach;

    // -----------------------------------------------------------------------
    // Public / Commons — anyone authorized, regardless of known-agent status
    // -----------------------------------------------------------------------

    #[test]
    fn public_reach_anyone_authorized() {
        let decision = classify_reach_authorization(
            Reach::Public,
            "bafyreiunknown",
            false, // unknown agent
        );
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    #[test]
    fn commons_reach_anyone_authorized() {
        let decision = classify_reach_authorization(
            Reach::Commons,
            "bafyreiunknown",
            false, // unknown agent
        );
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    // -----------------------------------------------------------------------
    // Private / SelfScope — trivially earned by signing (no known-agent check)
    // -----------------------------------------------------------------------

    #[test]
    fn private_reach_signer_authorized() {
        // Reach::Private is trivially earned by signing; unknown agent still passes.
        let decision = classify_reach_authorization(
            Reach::Private,
            "bafyreiunknown",
            false, // not a known agent
        );
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    #[test]
    fn self_reach_signer_authorized() {
        // Reach::SelfScope is trivially earned by signing.
        let decision = classify_reach_authorization(
            Reach::SelfScope,
            "bafyreiunknown",
            false, // not a known agent
        );
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    // -----------------------------------------------------------------------
    // Familiar — Stage 1 requires known agent
    // -----------------------------------------------------------------------

    #[test]
    fn familiar_reach_known_agent_authorized() {
        let decision = classify_reach_authorization(
            Reach::Familiar,
            "bafyreiknown",
            true, // known agent (has peer_identity_bindings row or is local conductor)
        );
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    #[test]
    fn familiar_reach_unknown_agent_refused() {
        let signer = "bafyreiunknown";
        let decision = classify_reach_authorization(Reach::Familiar, signer, false);
        match decision {
            ReachAuthDecision::Refused { reason } => {
                assert!(
                    reason.contains(signer),
                    "reason must contain signer CID: {reason}"
                );
                assert!(
                    reason.contains("Stage 1"),
                    "reason must mention Stage 1: {reason}"
                );
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Community — Stage 1 same floor as Familiar (known agent required)
    // -----------------------------------------------------------------------

    #[test]
    fn community_reach_known_agent_authorized_stage1() {
        // Stage 1: Community is the same gate as Familiar.
        // Stage 2/3 will require collective-membership proof (O2).
        let decision = classify_reach_authorization(
            Reach::Community,
            "bafyreiknown",
            true, // known agent
        );
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    #[test]
    fn community_reach_unknown_agent_refused() {
        // Stage 1: unknown signers cannot author at Community reach.
        // NOTE: Stage 2/3 will tighten this further (collective-membership DHT proof).
        let signer = "bafyreinever";
        let decision = classify_reach_authorization(Reach::Community, signer, false);
        match decision {
            ReachAuthDecision::Refused { reason } => {
                assert!(
                    reason.contains(signer),
                    "reason must contain signer CID: {reason}"
                );
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Trusted / Intimate — same gate as Familiar at Stage 1
    // -----------------------------------------------------------------------

    #[test]
    fn trusted_reach_known_agent_authorized() {
        let decision = classify_reach_authorization(Reach::Trusted, "bafyreiknown", true);
        assert_eq!(decision, ReachAuthDecision::Authorized);
    }

    #[test]
    fn intimate_reach_unknown_agent_refused() {
        let decision = classify_reach_authorization(Reach::Intimate, "bafyreiunknown", false);
        assert!(matches!(decision, ReachAuthDecision::Refused { .. }));
    }

    // -----------------------------------------------------------------------
    // classify_pre_authorization — receiver-side topology decision
    // -----------------------------------------------------------------------

    #[test]
    fn integrity_identity_binding_topic_always_standing() {
        // Integrity topics are always pre-authorized — every node carries integrity.
        let decision = classify_pre_authorization(
            crate::p2p::topics::TOPIC_IDENTITY_BINDING,
            false, // even without embodied responsibility
        );
        assert_eq!(decision, PreAuthorizationDecision::Standing);
    }

    #[test]
    fn integrity_revocation_topic_always_standing() {
        let decision =
            classify_pre_authorization(crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION, false);
        assert_eq!(decision, PreAuthorizationDecision::Standing);
    }

    #[test]
    fn legacy_recovery_topics_always_standing() {
        // Legacy topics used by M3/M4 publishers — must remain pre-authorized
        // for backward compatibility even without embodied responsibility.
        let invitation = classify_pre_authorization("recovery.invitation", false);
        assert_eq!(invitation, PreAuthorizationDecision::Standing);

        let revocation = classify_pre_authorization("recovery.revocation", false);
        assert_eq!(revocation, PreAuthorizationDecision::Standing);
    }

    #[test]
    fn public_pillar_topic_always_standing() {
        // Public reach — any node has standing; Stage 1 open broadcast.
        let decision = classify_pre_authorization("elohim/lamad/public", false);
        assert_eq!(decision, PreAuthorizationDecision::Standing);
    }

    #[test]
    fn commons_pillar_topic_always_standing() {
        let decision = classify_pre_authorization("elohim/shefa/commons", false);
        assert_eq!(decision, PreAuthorizationDecision::Standing);
    }

    #[test]
    fn private_pillar_topic_no_standing() {
        // Private/SelfScope should never appear in gossipsub; defensive NoStanding.
        let decision = classify_pre_authorization("elohim/lamad/private", true);
        assert_eq!(decision, PreAuthorizationDecision::NoStanding);
    }

    #[test]
    fn self_scope_pillar_topic_no_standing() {
        let decision = classify_pre_authorization("elohim/lamad/self", true);
        assert_eq!(decision, PreAuthorizationDecision::NoStanding);
    }

    #[test]
    fn familiar_pillar_topic_requires_embodied() {
        // Bound tiers require embodied responsibility.
        let with_resp = classify_pre_authorization("elohim/lamad/familiar", true);
        assert_eq!(with_resp, PreAuthorizationDecision::Standing);

        let without_resp = classify_pre_authorization("elohim/lamad/familiar", false);
        assert_eq!(without_resp, PreAuthorizationDecision::NoStanding);
    }

    #[test]
    fn community_with_collective_requires_embodied() {
        // Collective-scoped community topic: `elohim/qahal/community/household-xyz`
        let with_resp = classify_pre_authorization("elohim/qahal/community/household-xyz", true);
        assert_eq!(with_resp, PreAuthorizationDecision::Standing);

        let without_resp =
            classify_pre_authorization("elohim/qahal/community/household-xyz", false);
        assert_eq!(without_resp, PreAuthorizationDecision::NoStanding);
    }

    #[test]
    fn unknown_topic_shape_no_standing() {
        // Random string — no standing.
        let a = classify_pre_authorization("random.string", true);
        assert_eq!(a, PreAuthorizationDecision::NoStanding);

        // Too short (only `elohim/<pillar>` — missing reach segment).
        let b = classify_pre_authorization("elohim/foo", true);
        assert_eq!(b, PreAuthorizationDecision::NoStanding);

        // Unknown reach label.
        let c = classify_pre_authorization("elohim/lamad/unknownreach", true);
        assert_eq!(c, PreAuthorizationDecision::NoStanding);
    }
}
