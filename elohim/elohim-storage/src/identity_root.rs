//! Identity chain-root — the durable, rotation-stable identifier for an
//! identity's key lineage (the degenerate single-node slice).
//!
//! ## What this is
//!
//! An identity is a lineage DAG of key-rotation nodes (the `version_parent`
//! shape the lens-version-DAG spec sealed, instantiated for identity by
//! `genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md`).
//! Its durable name is the **chain-root cid** — the `version_parent=[]` genesis
//! node — which stays *stable across every rotation and recovery*. Other
//! subsystems point at the root instead of a rotation-fragile raw key, so a key
//! change never orphans attribution, economic standing, or claims.
//!
//! ## Wave A — the degenerate 1-node case only
//!
//! Today no rotation path exists: every identity is a single-node chain. So the
//! root is a **stable, deterministic derivation from the current agent key** —
//! there is nothing yet to walk back to. [`identity_root_cid`] IS that
//! derivation, and it is the indirection point every re-pointing targets.
//!
//! The moment real multi-node rotation lands (Wave B, `version_parent` DAG over
//! `KeyRotation` edges), this function's body generalizes to resolve the current
//! key back to the genesis root — but its **signature and every call site stay
//! put**. Installing the indirection now is the whole point of Wave A: the
//! re-pointings (REA party, contributor claim) already flow through the root, so
//! when two keys come to share a root, they resolve to the same identity for
//! free.
//!
//! ## The stability contract (property, not incident)
//!
//! The root cid must NEVER change for a fixed identity — if it did, every
//! re-pointing would silently break (plan watch-out: "Chain-root stability is
//! the contract — property-test it"). In the degenerate case that reduces to:
//! the derivation is a pure, deterministic, idempotent function of the key, and
//! trivially stable across the incidental surface variation a key string can
//! carry (surrounding whitespace).
//!
//! ## Why the derivation is *trim-only* (deliberately conservative)
//!
//! The degenerate root is `key.trim()` — byte-identical to the stored key modulo
//! surrounding whitespace. It is intentionally NOT a re-encoding (no `agent:`
//! prefix strip, no hashing): in Wave A only the WRITE paths route through the
//! root, while the many raw `.eq()` read filters across the crate do not yet.
//! Any derivation that *changed* the stored value would desynchronize a written
//! root from an un-routed reader and break a live join. Trim-only guarantees no
//! existing exact-match join can regress, so the seam is installable today with
//! zero behavioral risk. Wave B strengthens the derivation and routes the read
//! paths in the same move.
//!
//! This module is pure: no DB, no transport, no I/O. It mirrors the
//! feature-flag-free, unit-testable shape of its sibling `identity_namespace`.

/// The chain-root cid for an identity, given its current agent key.
///
/// Degenerate single-node slice (Wave A): the root is the normalized current
/// key. Same key always yields the same root (determinism); the root of a root
/// is the root (idempotence); incidental surrounding whitespace does not move it
/// (stability). See module docs for why the normalization is trim-only.
///
/// An empty / whitespace-only input yields an empty root (absence in → absence
/// out) — the caller decides whether an empty party is meaningful; this function
/// never invents an identity.
///
/// `key` may be any identity-string surface the crate carries (a bare `uhCAk…`
/// agent cid, an `agent:uhCAk…` prefixed form, a collective's content-cid). The
/// re-pointings pass whatever they hold; the root is the stable anchor they key
/// on.
pub fn identity_root_cid(key: &str) -> String {
    key.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENT_CID: &str = "uhCAkMATTHEWexampleAgentPubKeyCidForLineageRoot";
    const AGENT_CID_PREFIXED: &str = "agent:uhCAkADAMexampleAgentPubKeyPrefixed";
    const COLLECTIVE_CID: &str = "collective:uhCkkChurchCollectiveContentCid0001";

    #[test]
    fn root_is_deterministic_for_a_fixed_key() {
        // Determinism: repeated derivation of the same key yields the same root.
        assert_eq!(identity_root_cid(AGENT_CID), identity_root_cid(AGENT_CID));
        assert_eq!(
            identity_root_cid(AGENT_CID_PREFIXED),
            identity_root_cid(AGENT_CID_PREFIXED)
        );
        assert_eq!(
            identity_root_cid(COLLECTIVE_CID),
            identity_root_cid(COLLECTIVE_CID)
        );
    }

    #[test]
    fn root_is_idempotent() {
        // The root of a root is the root — the re-pointings can re-derive freely.
        let root = identity_root_cid(AGENT_CID);
        assert_eq!(identity_root_cid(&root), root);
    }

    #[test]
    fn root_is_stable_across_incidental_whitespace() {
        // The stability contract: surface whitespace must not move the root, or
        // a written root would fail to match a reader that trimmed differently.
        let bare = identity_root_cid(AGENT_CID);
        assert_eq!(identity_root_cid(&format!("  {AGENT_CID}")), bare);
        assert_eq!(identity_root_cid(&format!("{AGENT_CID}\n")), bare);
        assert_eq!(identity_root_cid(&format!("  {AGENT_CID}  ")), bare);
    }

    #[test]
    fn degenerate_root_preserves_the_key_value() {
        // Wave A's honest 1:1 property: today root == f(key) with f = trim, so a
        // re-pointing through the root is value-preserving and cannot regress an
        // existing exact-match join. (Wave B generalizes f; this assertion is the
        // pin that flags if the degenerate derivation is ever made lossy.)
        assert_eq!(identity_root_cid(AGENT_CID), AGENT_CID);
        assert_eq!(identity_root_cid(AGENT_CID_PREFIXED), AGENT_CID_PREFIXED);
    }

    #[test]
    fn empty_or_whitespace_yields_empty_root() {
        // Absence in → absence out: never invent an identity for an empty party
        // (e.g. the empty `receiver` on a one-sided provide commitment).
        assert_eq!(identity_root_cid(""), "");
        assert_eq!(identity_root_cid("   "), "");
        assert_eq!(identity_root_cid("\n\t"), "");
    }
}
