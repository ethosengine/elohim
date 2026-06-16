//! Identity-namespace coherence — enforcement-by-observation (rungs 1-2).
//!
//! A node/agent has three distinct identity namespaces that must NEVER be joined
//! or matched as raw strings (see `elohim/elohim-storage/CLAUDE.md` → "Identity &
//! Transport-Identity Coherence"):
//!
//! | Namespace        | Example      | Canonical home columns                          |
//! |------------------|--------------|-------------------------------------------------|
//! | Holochain agent  | `uhCAk…`     | `humans.agent_pub_key`, `shard_locations.peer_id` |
//! | key (`agent_cid`)|              | (⚠ misnamed — holds agent_cid), `peer_statuses.peer_id`, |
//! |                  |              | `rea_commitments.provider`                      |
//! | libp2p transport | `12D3Koo…`   | `peer_identity_bindings.peer_id`, the swarm     |
//! | iroh `NodeId`    | 64-char hex  | `peer_transport_manifest.iroh_node_id`          |
//!
//! **`agent_cid` (`uhCAk…`) is the canonical identity join key.** Raw-string
//! equality between an `agent_cid` and a transport id silently empties every
//! join — the all-zeros resilience card root
//! (`services/household_resilience.rs`, `matthew-edge-resiliency-rca-fanout`).
//!
//! ## What this module is (and is NOT)
//!
//! This is the **non-breaking first increment** of the enforcement ladder:
//! *enforcement-by-observation*. At each write site to a column the joins treat
//! as `agent_cid`, [`observe_agent_cid_write`] classifies the value's namespace
//! and, when it is NOT `uhCAk`-shaped, emits a WARN + increments a process
//! counter. It **never rejects or drops the write** — the runtime provide-loop
//! writes a transport id today (`provider = self_cid`,
//! `services/conductor_commitment_author.rs`), and rejecting it would break a
//! live data-plane path. The counter makes every wrong write *visible* so the
//! drift "shakes out" and the cleanup (rung 3 — populating `humans.agent_pub_key`
//! from the DHT human projection + fixing the provide-loop to write `agent_cid`)
//! is forced by evidence, not archaeology.
//!
//! This module is pure: no DB, no transport, no I/O beyond a `tracing` WARN and a
//! relaxed atomic increment. It is unit-testable with no harness, mirroring the
//! `signals::*_decode_miss_count` counter idiom already in this crate.
//!
//! ## What this module does NOT instrument
//!
//! `peer_identity_bindings.peer_id` legitimately holds a libp2p `12D3Koo…`
//! (the handshake-`source` home) — observing it would WARN on every CORRECT
//! write. Only the four columns the joins treat as `agent_cid` are observed:
//! `humans.agent_pub_key`, `rea_commitments.provider`, `shard_locations.peer_id`,
//! `peer_statuses.peer_id`.

use std::sync::atomic::{AtomicU64, Ordering};

/// The Holochain `AgentPubKey` CID textual prefix. An `agent_cid` always begins
/// with this (base64 of the multihash header for an agent key).
const AGENT_CID_PREFIX: &str = "uhCAk";

/// The libp2p Ed25519 `PeerId` textual prefix (base58btc, multihash-wrapped).
const LIBP2P_PEER_ID_PREFIX: &str = "12D3Koo";

/// Process-wide counter of observed identity-namespace violations: a value that
/// SHOULD be an `agent_cid` (`uhCAk…`) landed in a join-key column while carrying
/// some other namespace. Surfaced both as a `tracing` WARN with
/// `counter = "elohim_identity_namespace_violation_total"` (the operator's
/// log-scraped metric surface) and via [`namespace_violation_count`] (unit tests
/// + `/p2p/status`-style introspection).
static IDENTITY_NAMESPACE_VIOLATIONS: AtomicU64 = AtomicU64::new(0);

/// Read the running total of observed identity-namespace violations. Monotonic
/// for process lifetime; reset only by restart.
pub fn namespace_violation_count() -> u64 {
    IDENTITY_NAMESPACE_VIOLATIONS.load(Ordering::Relaxed)
}

/// The detected namespace of an identity string. Best-effort: only `AgentCid`
/// and `Libp2p` are positively shaped; everything else falls to `Iroh` (a
/// hex-looking blob) or `Unknown`. The classifier is conservative by design — it
/// exists to LABEL a violation, never to authorize a join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityNamespace {
    /// Holochain agent key — `uhCAk…` (the canonical join key).
    AgentCid,
    /// libp2p transport peer id — `12D3Koo…`.
    Libp2p,
    /// iroh `NodeId` — hex-encoded ed25519 public key (best-effort).
    Iroh,
    /// Empty/NULL or an unrecognized shape.
    Unknown,
}

impl IdentityNamespace {
    /// Stable lowercase label for metric/log dimensions.
    pub fn label(self) -> &'static str {
        match self {
            IdentityNamespace::AgentCid => "agent_cid",
            IdentityNamespace::Libp2p => "libp2p",
            IdentityNamespace::Iroh => "iroh",
            IdentityNamespace::Unknown => "unknown",
        }
    }
}

/// Strip the `agent:` vocabulary prefix some surfaces carry (the imagodei
/// `encode_agent_cid` form `agent:{pubkey}`), matching the strip the reconcile
/// controller already does before joining (`reconcile/controller.rs`).
fn strip_agent_prefix(s: &str) -> &str {
    s.strip_prefix("agent:").unwrap_or(s)
}

/// Is `s` an `agent_cid` (`uhCAk…`)? Tolerates the `agent:` prefix. This is the
/// one shape the convention enforces as the canonical join key.
pub fn is_agent_cid(s: &str) -> bool {
    strip_agent_prefix(s.trim()).starts_with(AGENT_CID_PREFIX)
}

/// Is `s` a libp2p transport peer id (`12D3Koo…`)?
pub fn is_libp2p_peer_id(s: &str) -> bool {
    s.trim().starts_with(LIBP2P_PEER_ID_PREFIX)
}

/// Best-effort: does `s` look like an iroh `NodeId` (a 64-char lowercase-hex
/// ed25519 public key)? Conservative — only used to LABEL a non-`agent_cid`
/// value, never to authorize a join.
pub fn is_iroh_node_id(s: &str) -> bool {
    let s = s.trim();
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Classify an identity string into its detected namespace. NULL/empty →
/// [`IdentityNamespace::Unknown`].
pub fn classify(s: &str) -> IdentityNamespace {
    let t = s.trim();
    if t.is_empty() {
        IdentityNamespace::Unknown
    } else if is_agent_cid(t) {
        IdentityNamespace::AgentCid
    } else if is_libp2p_peer_id(t) {
        IdentityNamespace::Libp2p
    } else if is_iroh_node_id(t) {
        IdentityNamespace::Iroh
    } else {
        IdentityNamespace::Unknown
    }
}

/// Observe a write to a column the joins treat as `agent_cid`. If `value` is
/// present and NOT `uhCAk`-shaped, increments the violation counter and emits a
/// WARN tagged for log-scraping. **Never rejects** — observation only.
///
/// NULL/empty is skipped: absence is the known design gap
/// (`humans.agent_pub_key` is NULL until healed), not a namespace violation, and
/// counting it would drown the real signal (wrong-shape non-null values).
///
/// `column` is the fully-qualified column (e.g. `"rea_commitments.provider"`) so
/// the operator can attribute the violation to a write path.
pub fn observe_agent_cid_write(column: &'static str, value: Option<&str>) {
    let Some(raw) = value else { return };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return; // NULL/empty is design-gap, not a namespace violation.
    }
    if is_agent_cid(trimmed) {
        return; // Correct namespace — the common, silent path.
    }

    let detected = classify(trimmed);
    IDENTITY_NAMESPACE_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
    tracing::warn!(
        target: "identity_coherence",
        counter = "elohim_identity_namespace_violation_total",
        column = column,
        expected = "agent_cid",
        got = detected.label(),
        "identity-namespace violation: a non-agent_cid value landed in an \
         agent_cid join-key column (LOGGED, not rejected — see \
         identity_namespace module docs)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Representative fixtures, matching the shapes used elsewhere in-crate
    // (node_transport.rs tests, humans.rs heal tests).
    const AGENT_CID: &str = "uhCAkMATTHEWexampleAgentPubKeyCidForCoherenceTest";
    const AGENT_CID_PREFIXED: &str = "agent:uhCAkADAMexampleAgentPubKeyCidPrefixed";
    const LIBP2P_ID: &str = "12D3KooWExampleLibp2pPeerIdForTransportSeamTest";
    // A real iroh NodeId is 64 lowercase-hex chars (ed25519 pubkey).
    const IROH_ID: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    #[test]
    fn is_agent_cid_accepts_uhcak_and_strips_agent_prefix() {
        assert!(is_agent_cid(AGENT_CID));
        assert!(is_agent_cid(AGENT_CID_PREFIXED));
        assert!(
            is_agent_cid(&format!("  {AGENT_CID}  ")),
            "tolerates whitespace"
        );
    }

    #[test]
    fn is_agent_cid_rejects_transport_ids() {
        assert!(!is_agent_cid(LIBP2P_ID));
        assert!(!is_agent_cid(IROH_ID));
        assert!(!is_agent_cid(""));
    }

    #[test]
    fn classify_each_namespace() {
        assert_eq!(classify(AGENT_CID), IdentityNamespace::AgentCid);
        assert_eq!(classify(AGENT_CID_PREFIXED), IdentityNamespace::AgentCid);
        assert_eq!(classify(LIBP2P_ID), IdentityNamespace::Libp2p);
        assert_eq!(classify(IROH_ID), IdentityNamespace::Iroh);
        assert_eq!(classify(""), IdentityNamespace::Unknown);
        assert_eq!(classify("   "), IdentityNamespace::Unknown);
        assert_eq!(classify("some-random-slug"), IdentityNamespace::Unknown);
    }

    #[test]
    fn iroh_classifier_is_strict_on_length_and_hex() {
        assert!(is_iroh_node_id(IROH_ID));
        assert!(!is_iroh_node_id("0123")); // too short
        assert!(!is_iroh_node_id(&"z".repeat(64))); // 64 chars but not hex
    }

    #[test]
    fn observe_correct_agent_cid_does_not_increment() {
        let before = namespace_violation_count();
        observe_agent_cid_write("rea_commitments.provider", Some(AGENT_CID));
        observe_agent_cid_write("rea_commitments.provider", Some(AGENT_CID_PREFIXED));
        assert_eq!(
            namespace_violation_count(),
            before,
            "a correct agent_cid (with or without agent: prefix) must NOT count"
        );
    }

    #[test]
    fn observe_null_or_empty_does_not_increment() {
        let before = namespace_violation_count();
        observe_agent_cid_write("humans.agent_pub_key", None);
        observe_agent_cid_write("humans.agent_pub_key", Some(""));
        observe_agent_cid_write("humans.agent_pub_key", Some("   "));
        assert_eq!(
            namespace_violation_count(),
            before,
            "NULL/empty is design-gap, not a namespace violation"
        );
    }

    #[test]
    fn observe_transport_id_in_agent_cid_column_increments() {
        let before = namespace_violation_count();
        // The live provide-loop violation: a libp2p self_cid written as provider.
        observe_agent_cid_write("rea_commitments.provider", Some(LIBP2P_ID));
        assert_eq!(
            namespace_violation_count(),
            before + 1,
            "a 12D3Koo value in an agent_cid column must increment the counter"
        );
        // And an iroh NodeId in shard_locations.peer_id.
        observe_agent_cid_write("shard_locations.peer_id", Some(IROH_ID));
        assert_eq!(namespace_violation_count(), before + 2);
    }
}
