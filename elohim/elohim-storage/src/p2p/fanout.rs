//! Reach-tier fanout routing policy — Decision §3.7.
//!
//! This module is a **pure function** layer: it maps a `(Reach, kind)` pair to
//! the set of [`FanoutChannel`]s that should carry the EPR atom.  No swarm
//! interaction, no I/O.  Callers in D.2 (Kad publish), D.3 (gossip publish),
//! and D.5 (direct-notify) consult this function to decide which channels to
//! activate.
//!
//! ## Integrity exception (§3.7 — Decision #7)
//!
//! `KeyRotation`, `KeyRevocation`, `RevocationAttestation`, and
//! `AgentPeerBinding` bypass the reach tier entirely and are always broadcast on
//! all three channels simultaneously.  This ensures that key-state changes
//! propagate to every peer without delay, regardless of the EPR envelope's
//! declared reach.  The single source of truth for what constitutes an
//! "integrity kind" is [`crate::write_through::is_integrity_kind`] — do **not**
//! redefine the list here.

use elohim_epr::Reach;

use crate::write_through::is_integrity_kind;

// ---------------------------------------------------------------------------
// Channel enum
// ---------------------------------------------------------------------------

/// Which transport channels an EPR atom should be published on.
///
/// Variants are non-exclusive: a call to [`channels_for_reach`] may return
/// multiple channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FanoutChannel {
    /// Send directly to named/known peers only (no broadcast).
    DirectOnly,
    /// Publish to a gossipsub topic scoped to the atom's reach.
    Gossip,
    /// Provider record only (`start_providing`); no value stored on DHT.
    ///
    /// Use for opt-in cold discovery where the peer announces it can serve the
    /// content but does not push the value into the DHT.  D.2 currently only
    /// calls `start_providing`, so `KadLight` and `Kad` behave identically until
    /// the `put_record` path is wired in a later task.
    KadLight,
    /// Provider record + value stored on DHT via `put_record` for direct lookup.
    ///
    /// Maximally discoverable: any peer can retrieve the value without connecting
    /// to the origin.  D.2 will wire the `put_record` path; until then this
    /// behaves the same as `KadLight` in practice.
    Kad,
}

// ---------------------------------------------------------------------------
// Routing policy
// ---------------------------------------------------------------------------

/// Returns the ordered set of fanout channels for the given `(reach, kind)` pair.
///
/// Integrity kinds short-circuit reach: `KeyRotation`, `KeyRevocation`,
/// `RevocationAttestation`, and `AgentPeerBinding` always return
/// `[Gossip, Kad, DirectOnly]` regardless of the declared reach.
///
/// For all other kinds the routing matrix from spec §3.7 (Decision #7) applies:
///
/// | Reach               | Channels                         |
/// |---------------------|----------------------------------|
/// | Private / SelfScope | DirectOnly                       |
/// | Intimate            | DirectOnly                       |
/// | Trusted / Familiar  | Gossip                           |
/// | Community           | Gossip + KadLight                |
/// | Public              | Gossip + Kad                     |
/// | Commons             | Kad + Gossip (Kad primary)       |
pub fn channels_for_reach(reach: Reach, kind: &str) -> Vec<FanoutChannel> {
    if is_integrity_kind(kind) {
        // §3.7 integrity exception: always all three channels so that
        // key-state changes reach every peer immediately.
        return vec![
            FanoutChannel::Gossip,
            FanoutChannel::Kad,
            FanoutChannel::DirectOnly,
        ];
    }

    match reach {
        Reach::Private | Reach::SelfScope => vec![FanoutChannel::DirectOnly],
        Reach::Intimate => vec![FanoutChannel::DirectOnly],
        Reach::Trusted | Reach::Familiar => vec![FanoutChannel::Gossip],
        Reach::Community => vec![FanoutChannel::Gossip, FanoutChannel::KadLight],
        Reach::Public => vec![FanoutChannel::Gossip, FanoutChannel::Kad],
        Reach::Commons => vec![FanoutChannel::Kad, FanoutChannel::Gossip],
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    const NON_INTEGRITY_KIND: &str = "EconomicEvent";

    fn assert_channels(reach: Reach, kind: &str, expected: &[FanoutChannel]) {
        let got = channels_for_reach(reach, kind);
        assert_eq!(
            got, expected,
            "channels_for_reach({reach:?}, {kind:?}) mismatch"
        );
    }

    // ------------------------------------------------------------------
    // Reach-tier matrix — non-integrity kind
    // ------------------------------------------------------------------

    #[test]
    fn private_yields_direct_only() {
        assert_channels(
            Reach::Private,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::DirectOnly],
        );
    }

    #[test]
    fn self_scope_yields_direct_only() {
        assert_channels(
            Reach::SelfScope,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::DirectOnly],
        );
    }

    #[test]
    fn intimate_yields_direct_only() {
        assert_channels(
            Reach::Intimate,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::DirectOnly],
        );
    }

    #[test]
    fn trusted_yields_gossip() {
        assert_channels(Reach::Trusted, NON_INTEGRITY_KIND, &[FanoutChannel::Gossip]);
    }

    #[test]
    fn familiar_yields_gossip() {
        assert_channels(
            Reach::Familiar,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::Gossip],
        );
    }

    #[test]
    fn community_yields_gossip_and_kad_light() {
        assert_channels(
            Reach::Community,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::Gossip, FanoutChannel::KadLight],
        );
    }

    #[test]
    fn public_yields_gossip_and_kad() {
        assert_channels(
            Reach::Public,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::Gossip, FanoutChannel::Kad],
        );
    }

    #[test]
    fn commons_yields_kad_then_gossip() {
        assert_channels(
            Reach::Commons,
            NON_INTEGRITY_KIND,
            &[FanoutChannel::Kad, FanoutChannel::Gossip],
        );
    }

    // ------------------------------------------------------------------
    // Integrity exception — each kind bypasses reach at reach=Private
    // ------------------------------------------------------------------

    #[test]
    fn key_rotation_bypasses_reach() {
        let got = channels_for_reach(Reach::Private, "KeyRotation");
        assert_eq!(
            got,
            vec![
                FanoutChannel::Gossip,
                FanoutChannel::Kad,
                FanoutChannel::DirectOnly
            ],
            "KeyRotation must short-circuit reach=Private"
        );
    }

    #[test]
    fn key_revocation_bypasses_reach() {
        let got = channels_for_reach(Reach::Private, "KeyRevocation");
        assert_eq!(
            got,
            vec![
                FanoutChannel::Gossip,
                FanoutChannel::Kad,
                FanoutChannel::DirectOnly
            ],
            "KeyRevocation must short-circuit reach=Private"
        );
    }

    #[test]
    fn revocation_attestation_bypasses_reach() {
        let got = channels_for_reach(Reach::Private, "RevocationAttestation");
        assert_eq!(
            got,
            vec![
                FanoutChannel::Gossip,
                FanoutChannel::Kad,
                FanoutChannel::DirectOnly
            ],
            "RevocationAttestation must short-circuit reach=Private"
        );
    }

    #[test]
    fn agent_peer_binding_bypasses_reach() {
        let got = channels_for_reach(Reach::Private, "AgentPeerBinding");
        assert_eq!(
            got,
            vec![
                FanoutChannel::Gossip,
                FanoutChannel::Kad,
                FanoutChannel::DirectOnly
            ],
            "AgentPeerBinding must short-circuit reach=Private"
        );
    }

    // ------------------------------------------------------------------
    // Integrity exception also overrides non-private reaches
    // ------------------------------------------------------------------

    #[test]
    fn key_rotation_bypasses_public_reach() {
        let got = channels_for_reach(Reach::Public, "KeyRotation");
        assert_eq!(
            got,
            vec![
                FanoutChannel::Gossip,
                FanoutChannel::Kad,
                FanoutChannel::DirectOnly
            ],
            "KeyRotation must short-circuit even at reach=Public"
        );
    }

    // ------------------------------------------------------------------
    // Sanity: non-integrity "Content" kind follows reach matrix
    // ------------------------------------------------------------------

    #[test]
    fn content_kind_follows_reach_matrix() {
        assert_channels(Reach::Trusted, "Content", &[FanoutChannel::Gossip]);
        assert_channels(
            Reach::Public,
            "Content",
            &[FanoutChannel::Gossip, FanoutChannel::Kad],
        );
    }
}
