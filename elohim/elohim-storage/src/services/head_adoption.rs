//! Adopt-before-author: consult the substrate for an existing canonical head
//! BEFORE minting a local root for a content id.
//!
//! ## The defect this closes
//!
//! Two peers can each hold a root for the same content id (`elohim-host-landing`
//! is the live case). Peer B's restart sweeps — the boot re-anchor pass
//! (`services::reanchor_backfill`) and the ghost-anchor witness
//! (`p2p::projection_reconcile::witness_ghost_anchors`) — selected their
//! candidates from LOCAL anchor state only, authored a fresh root through the
//! own conductor, and the projection immediately crowned that root as the
//! declared head. B therefore re-elected itself on every restart and could
//! never converge on A's declaration, no matter how many heal sweeps ran.
//!
//! The cure has two halves. The substrate half is
//! [`crate::db::content_diesel::HeadElection`]: authoring no longer implies
//! declaring. This module is the other half — a pre-flight that runs at the top
//! of each per-id loop body in both sweeps and asks, in order:
//!
//! 1. **LOCAL-DHT ARM.** Does my OWN conductor already resolve a CANONICAL head
//!    for this id? Then adopt it and do not author. This is the only arm that
//!    can act on notarized truth without any network round-trip.
//! 2. **PEER-HINT ARM.** Does a peer ADVERTISE a declaration (via the additive
//!    `ProjectionInventoryEntry::declared_head_action_hash`) while I hold none?
//!    Then fetch the head `Record` from that peer and declare it through MY
//!    conductor (declare-carries-Record). The advertised hash is a HINT that
//!    triggers a verified fetch — it is NEVER stamped into a row directly.
//! 3. **AUTHOR-THEN-ADOPT.** The zome refuses to declare a head for an id it has
//!    no local chain for (`declare_canonical_head: no content found for id`), so
//!    a genuinely fresh peer MUST author before it can adopt. Fix A makes that
//!    author non-declaring, so the sequence is: author the root, then
//!    immediately declare the PEER's head over it, in-process.
//!
//! ## What this module deliberately does NOT do
//!
//! **No cross-peer timestamp ordering.** `declared_head_at` is not globally
//! comparable — the zome substitutes the RECEIVING conductor's `sys_time` on the
//! carried-record branch, so "whose declaration is newer" cannot be decided from
//! two peers' projections. The Rust rule is PRESENCE-based
//! ([`decide_head_action`]); ordering is arbitrated in-zome by
//! `select_canonical_winner`. Inventing a newest-wins election here would
//! reintroduce the head-flapping this exists to stop.
//!
//! **No admission or budget deny.** Every arm degrades to the author path and
//! retries on the next sweep. Pacing and yielding are the callers' existing
//! bounds; nothing here sheds work.
//!
//! ## In-process, never over HTTP
//!
//! The declare goes through `conductor_writes::call_declare_canonical_content_head`
//! directly. The HTTP declare route is auth-gated and admission-shed-eligible;
//! routing a heal through it would make convergence depend on the edge's load
//! shedding (the heal-exemption rule — see `p2p::sync_round`).

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::content_diesel::{self, StampMode, StampOutcome};
use crate::db::{AppContext, DbPool};
use crate::hc_client::HcClient;
use crate::services::conductor_writes::{self, ContentHeadWire};

/// Zome guard substring meaning "I cannot declare a head for this id because I
/// have no local content chain for it" (`content_store::declare_canonical_head_inner`).
///
/// This is the AUTHOR-THEN-ADOPT trigger, not a failure: a peer that has never
/// held the content genuinely has nothing to hang a declaration on yet.
const ERR_NO_LOCAL_CHAIN: &str = "no content found for id";

/// Zome guard substring meaning "I cannot retrieve the target action". On a
/// full-arc fleet this is TERMINAL, not slow (the `get` cascade short-circuits
/// at authority), so it means the carried record was absent or rejected. Degrade
/// to the author path; the next sweep may have a peer that can serve the record.
const ERR_NOT_RETRIEVABLE: &str = "is not retrievable";

/// A peer's ADVERTISED declaration for a content id, harvested from the
/// projection-inventory federation response.
///
/// Emphatically a hint, not a value: nothing in this module writes
/// `head_action_hash` into a local row. It selects WHICH peer to ask for a
/// verifiable `Record` and WHICH action to name in a conductor-side declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerHeadHint {
    /// The action the advertising peer says its canonical channel elected.
    pub head_action_hash: String,
    /// The ordering the peer carries. Recorded for logs only — see the
    /// module-level note on why this is never used to elect.
    pub declared_at: Option<i64>,
    /// Which peer advertised it (the one we ask for the `Record`).
    pub peer_id: String,
}

/// `content_id → ` the first peer-advertised declaration seen this sweep.
pub type PeerHeadHints = HashMap<String, PeerHeadHint>;

/// A head `Record` fetched from a peer — the SOURCE half of
/// declare-carries-Record, obtained over the P2P view-federation plane.
#[derive(Debug, Clone)]
pub struct CarriedHeadRecord {
    /// The action the serving peer says the record is for. May differ from the
    /// hint if the peer's head moved between the inventory sweep and this fetch;
    /// the SERVED hash wins, because it is the one the bytes actually prove.
    pub head_action_hash: String,
    /// Serialized `Record` bytes, or `None` when the peer holds a head but its
    /// conductor cannot retrieve the record (honest absence — we then declare
    /// without a carried record, which is the classic behaviour).
    pub record: Option<Vec<u8>>,
}

/// Fetches a peer's head `Record` over whichever transport the caller owns.
///
/// A trait rather than a `P2PHandle` parameter so this module — and the
/// `services` layer generally — stays transport-neutral: the libp2p and iroh
/// adapters both delegate to the same pre-flight.
#[async_trait::async_trait]
pub trait HeadRecordFetcher: Send + Sync {
    /// `None` for EVERY failure mode, including "this peer is too old to decode
    /// the request". Implementations MUST log the degradation explicitly and
    /// MUST NOT retry-loop — the caller falls through to the author path, and
    /// the next sweep tries again.
    async fn fetch(&self, peer_id: &str, content_id: &str) -> Option<CarriedHeadRecord>;
}

/// What the pre-flight resolved from the OWN conductor.
///
/// The ghost sweep has ALREADY paid for this answer (`heal_content` collects its
/// candidates precisely from `resolve_content_head` returning `Ok(None)`), so it
/// passes [`LocalResolve::Known`] rather than burning a second round-trip on an
/// answer it holds.
pub enum LocalResolve<'a> {
    /// Not yet asked — the pre-flight calls `resolve_content_head` itself.
    Probe,
    /// Already known. `None` means the conductor could not resolve the id at all.
    ///
    /// CONTRACT DEVIATION (2026-07-29) — `Known(None)` now carries TWO
    /// provenances, and the second is weaker than this doc implies:
    ///
    /// 1. OBSERVED absence — the ghost sweep saw `resolve_content_head` return
    ///    `Ok(None)`. This is the original, literal meaning.
    /// 2. UNKNOWN — the heal loop's resolve TIMED OUT (see
    ///    `projection_reconcile::timeout_should_route_to_adopt`). We never got an
    ///    answer, so absence was not observed; it is merely not established.
    ///
    /// This is safe TODAY only because both provenances lead to the same place:
    /// `None` forecloses the `AdoptLocal` arm and leaves `AdoptPeer` / `Hold`,
    /// and the timeout route is gated on a peer hint existing — so a timed-out
    /// candidate always has a peer to adopt FROM, and can only `AdoptPeer` or
    /// `Hold`. Neither outcome asserts absence.
    ///
    /// ASSUMPTION A FUTURE EDITOR MUST PRESERVE: do not make `Known(None)` mean
    /// anything AUTHORITATIVE — do not let it author, delete, tombstone, or
    /// otherwise treat the id as proven-absent. If a new arm needs to
    /// distinguish "observed absent" from "unknown", split this variant
    /// (e.g. `Known(None)` vs `Unresolved`) rather than adding behaviour that
    /// silently reads the timeout case as observed absence.
    Known(Option<&'a ContentHeadWire>),
}

/// Everything the peer-hint arm needs. Both fields are absent at boot (the
/// one-shot re-anchor pass runs before P2P discovery), which correctly degrades
/// the pre-flight to the local-DHT arm alone.
pub struct AdoptContext<'a> {
    pub hints: &'a PeerHeadHints,
    pub fetcher: Option<&'a dyn HeadRecordFetcher>,
}

impl AdoptContext<'_> {
    /// The boot-time shape: no peer inventory yet, no transport to fetch with.
    pub fn none() -> AdoptContext<'static> {
        static EMPTY: std::sync::OnceLock<PeerHeadHints> = std::sync::OnceLock::new();
        AdoptContext {
            hints: EMPTY.get_or_init(PeerHeadHints::new),
            fetcher: None,
        }
    }
}

/// The decision rule, as a pure total function.
///
/// Pure on purpose: this is the part that got the live behaviour wrong, and it
/// is the part that must be readable in isolation and exhaustively tested
/// without a conductor, a pool, or a peer.
///
/// | own conductor canonical | peer declares | local declared | ⇒ |
/// |---|---|---|---|
/// | yes | – | – | [`HeadDecision::AdoptLocal`] |
/// | no | yes | no | [`HeadDecision::AdoptPeer`] |
/// | no | – | yes | [`HeadDecision::Hold`] |
/// | no | no | no | [`HeadDecision::Author`] |
///
/// Two rulings worth naming:
///
/// - **Peer-declares + local-declared ⇒ Hold, not adopt.** Two declarations
///   cannot be ordered from Rust (see the module note), so the substrate holds
///   and lets the canonical channels — and the zome's own arbitration —
///   resolve it. Racing to overwrite is what made the head flap.
/// - **Nothing declared anywhere ⇒ Author.** Today's behaviour, unchanged, and
///   now safe: under [`content_diesel::HeadElection::PreserveExistingDeclaration`]
///   a self-authored root is NOT a declaration, so any later declaration — from
///   any peer, at any time — wins outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadDecision {
    /// The own conductor holds a canonical head — adopt it, never author.
    AdoptLocal,
    /// A peer advertises a declaration and this row has none — verified-fetch
    /// then declare through the own conductor.
    AdoptPeer,
    /// This row already carries a declaration — neither adopt nor author.
    Hold,
    /// Nothing is declared anywhere — mint a (non-declaring) root.
    Author,
}

/// DECLARE-STORM GATE, as a pure predicate: would declaring `incoming` actually
/// MOVE this row's declaration?
///
/// The pre-flight runs over every candidate of every sweep, and a zome declare
/// is a source-chain write plus a DHT link — orders of magnitude more expensive
/// than the SQL read that avoids it. Without this gate a CONVERGED corpus would
/// re-declare its already-correct head on every sweep forever: a cure that
/// installs a permanent write storm is not a cure. With it, a converged corpus
/// issues ZERO declares per sweep.
///
/// `None` (undeclared) always moves — that is the fill case the peer arm exists
/// for. Whitespace is trimmed on both sides so a stray-padding round-trip cannot
/// read as a difference and re-declare on every sweep.
pub fn declaration_would_move(current: Option<&str>, incoming: &str) -> bool {
    match current {
        None => true,
        Some(c) => c.trim() != incoming.trim(),
    }
}

/// See [`HeadDecision`] for the table this implements.
pub fn decide_head_action(
    conductor_canonical: bool,
    peer_declared: bool,
    local_declared: bool,
) -> HeadDecision {
    if conductor_canonical {
        HeadDecision::AdoptLocal
    } else if peer_declared && !local_declared {
        HeadDecision::AdoptPeer
    } else if local_declared {
        HeadDecision::Hold
    } else {
        HeadDecision::Author
    }
}

/// What the caller should do with this id after the pre-flight ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptOutcome {
    /// A canonical head is now stamped on the row. SKIP the author path.
    Adopted,
    /// The row already carries a declaration (or a declaration this pre-flight
    /// tried to move was refused as not-provably-forward). SKIP the author path
    /// and leave it to the canonical channels.
    Held,
    /// Nothing adoptable. Proceed with the caller's existing author path.
    Author,
    /// A peer's head is adoptable but this conductor has no local chain to hang
    /// the declaration on. Run the author path, then call
    /// [`finish_author_then_adopt`] with these values.
    AuthorThenAdopt {
        head_action_hash: String,
        carried_record: Option<Vec<u8>>,
        peer_id: String,
    },
}

/// Pre-flight for ONE content id. Never errors: every failure degrades to
/// [`AdoptOutcome::Author`] (or `Held`) and is retried on the next sweep.
pub async fn try_adopt_canonical_head(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    local_resolve: LocalResolve<'_>,
    adopt: &AdoptContext<'_>,
) -> AdoptOutcome {
    // (0) What does this row already claim? A pool failure is not a licence to
    // author — treat an unreadable row as declared (Hold) so a transient DB
    // problem can never mint a competing root.
    let local_declared = match pool.get() {
        Ok(mut conn) => match content_diesel::declared_head_for(&mut conn, ctx, id) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    content_id = %id, error = %e,
                    "adopt-before-author: could not read the local declaration — holding \
                     (a transient DB error must never authorize minting a competing root)"
                );
                return AdoptOutcome::Held;
            }
        },
        Err(e) => {
            tracing::warn!(
                content_id = %id, error = %e,
                "adopt-before-author: db conn unavailable — holding"
            );
            return AdoptOutcome::Held;
        }
    };

    // (1) LOCAL-DHT ARM. Reuse a resolve the caller already paid for.
    let probed: Option<ContentHeadWire> = match local_resolve {
        LocalResolve::Known(_) => None,
        LocalResolve::Probe => match conductor_writes::call_resolve_content_head(hc, id).await {
            Ok(head) => head,
            Err(e) => {
                // A conductor that will not answer cannot be shown to hold a
                // canonical head; fall through with "not canonical" rather than
                // aborting the whole sweep.
                tracing::debug!(
                    content_id = %id, error = %e,
                    "adopt-before-author: own-conductor resolve failed; continuing to the peer arm"
                );
                None
            }
        },
    };
    let head: Option<&ContentHeadWire> = match local_resolve {
        LocalResolve::Known(h) => h,
        LocalResolve::Probe => probed.as_ref(),
    };
    let canonical_head = head.filter(|h| h.canonical);

    let hint = adopt.hints.get(id);
    let decision = decide_head_action(
        canonical_head.is_some(),
        hint.is_some(),
        local_declared.is_some(),
    );

    match decision {
        HeadDecision::AdoptLocal => {
            // `canonical_head.is_some()` is what produced this arm.
            let head = canonical_head.expect("AdoptLocal implies a canonical head");
            adopt_local(pool, ctx, id, head)
        }
        HeadDecision::AdoptPeer => {
            let hint = hint.expect("AdoptPeer implies a hint");
            adopt_peer(hc, pool, ctx, id, hint, adopt.fetcher).await
        }
        HeadDecision::Hold => {
            tracing::debug!(
                content_id = %id,
                declared = ?local_declared,
                "adopt-before-author: row already declared — neither adopting nor authoring"
            );
            AdoptOutcome::Held
        }
        HeadDecision::Author => AdoptOutcome::Author,
    }
}

/// Stamp a canonical head this node's OWN conductor resolved.
///
/// Stamp mode is [`StampMode::HealCanonical`], NOT `Declare`. This runs inside
/// the heal sweeps, and a conductor that has not yet integrated a newer
/// cross-root link answers with the OLD canonical record — canonical, yet stale.
/// An unconditional `Declare` here moved the head BACKWARDS in the 2026-07-12
/// regression (`elohim-host-landing` converged at edge #1187's seam-smoke and
/// healed back to the superseded head by #1188). `HealCanonical` FILLS an
/// undeclared row unconditionally — which is the adopt case this arm exists for
/// — and only MOVES a declared one with proof of forward ordering.
fn adopt_local(pool: &DbPool, ctx: &AppContext, id: &str, head: &ContentHeadWire) -> AdoptOutcome {
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "adopt-before-author: db conn for local stamp");
            return AdoptOutcome::Held;
        }
    };
    match content_diesel::stamp_declared_head_mode(
        &mut conn,
        ctx,
        id,
        head.head_action_hash.as_str(),
        Some(head.declared_at),
        None,
        StampMode::HealCanonical,
    ) {
        Ok(StampOutcome::Stamped) => {
            crate::metrics::inc_content_head_adopted();
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                head = %head.head_action_hash,
                "adopt-before-author: ADOPTED a canonical head from the own conductor \
                 (no local root minted)"
            );
            AdoptOutcome::Adopted
        }
        // Already holding this head, or holding one this answer cannot be shown
        // to supersede: in every case the row HAS a canonical head and must not
        // be re-rooted.
        Ok(_) => AdoptOutcome::Held,
        Err(e) => {
            tracing::warn!(content_id = %id, error = %e, "adopt-before-author: local stamp failed");
            AdoptOutcome::Held
        }
    }
}

/// Verified-fetch a peer's head record, then declare it through the OWN
/// conductor. Returns [`AdoptOutcome::AuthorThenAdopt`] when the conductor
/// refuses for lack of a local chain.
async fn adopt_peer(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    hint: &PeerHeadHint,
    fetcher: Option<&dyn HeadRecordFetcher>,
) -> AdoptOutcome {
    // Verified fetch: ask the ADVERTISING peer for the serialized Record behind
    // its head. Absent fetcher (boot pass) or an old peer that cannot serve one
    // both degrade to a record-less declare, which succeeds iff this conductor
    // can already retrieve the target itself.
    let carried = match fetcher {
        Some(f) => f.fetch(&hint.peer_id, id).await,
        None => None,
    };
    // The SERVED hash wins over the advertised one: it is the action the bytes
    // actually prove. Falling back to the hint keeps the record-less path working.
    let (head_action_hash, carried_record) = match carried {
        Some(c) => (c.head_action_hash, c.record),
        None => (hint.head_action_hash.clone(), None),
    };

    declare_peer_head(hc, pool, ctx, id, &head_action_hash, carried_record, hint).await
}

/// Second half of AUTHOR-THEN-ADOPT: call this after the caller's author path
/// has given this conductor a local chain for `id`.
///
/// A transient failure here is NOT fatal — the row is left "anchored,
/// undeclared", which is a legal, self-healing state: the next sweep finds it
/// undeclared and re-runs the pre-flight.
pub async fn finish_author_then_adopt(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    head_action_hash: &str,
    carried_record: Option<Vec<u8>>,
    peer_id: &str,
) -> AdoptOutcome {
    let hint = PeerHeadHint {
        head_action_hash: head_action_hash.to_string(),
        declared_at: None,
        peer_id: peer_id.to_string(),
    };
    match declare_peer_head(hc, pool, ctx, id, head_action_hash, carried_record, &hint).await {
        // A second "no local chain" refusal means the author did not actually
        // land; do not recurse — the next sweep retries from the top.
        AdoptOutcome::AuthorThenAdopt { .. } => {
            tracing::warn!(
                content_id = %id,
                "adopt-before-author: conductor still reports no local chain after authoring — \
                 left anchored-and-undeclared, retried next sweep"
            );
            AdoptOutcome::Author
        }
        other => other,
    }
}

/// Shared declare + stamp for both peer-adopt entrypoints.
///
/// DECLARE-STORM GATE: the declare is skipped outright when it would not MOVE
/// anything (this row already declares exactly this head). Combined with the
/// once-per-id-per-sweep loop structure, a converged corpus issues ZERO zome
/// declares per sweep — the difference between a cure and a new write storm.
async fn declare_peer_head(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    head_action_hash: &str,
    carried_record: Option<Vec<u8>>,
    hint: &PeerHeadHint,
) -> AdoptOutcome {
    if let Ok(mut conn) = pool.get() {
        let current = content_diesel::declared_head_for(&mut conn, ctx, id).unwrap_or(None);
        if !declaration_would_move(current.as_deref(), head_action_hash) {
            tracing::debug!(
                content_id = %id,
                "adopt-before-author: declare-storm gate — already declaring this head, skipping"
            );
            return AdoptOutcome::Held;
        }
    }

    let carried_bytes = carried_record.as_ref().map(|b| b.len()).unwrap_or(0);
    match conductor_writes::call_declare_canonical_content_head(
        hc,
        id,
        head_action_hash.to_string(),
        carried_record.clone(),
    )
    .await
    {
        Ok(declared) => {
            // This declaration is a DELIBERATE own-conductor canonical act that
            // this process just caused — the same class as the declare route's
            // eager stamp — so `Declare` is the correct mode here (unlike the
            // local-resolve arm above, which is a heal-class read).
            let stamped = pool.get().map_err(|e| e.to_string()).and_then(|mut conn| {
                content_diesel::stamp_declared_head_mode(
                    &mut conn,
                    ctx,
                    id,
                    declared.head_action_hash.as_str(),
                    Some(declared.declared_at),
                    None,
                    StampMode::Declare,
                )
                .map_err(|e| e.to_string())
            });
            match stamped {
                Ok(StampOutcome::Stamped) | Ok(StampOutcome::Refreshed) => {
                    crate::metrics::inc_content_head_adopted();
                    tracing::warn!(
                        target: "elohim_storage::head_adoption",
                        content_id = %id,
                        head = %declared.head_action_hash,
                        from_peer = %hint.peer_id,
                        carried_bytes,
                        "adopt-before-author: ADOPTED a peer-advertised canonical head via \
                         conductor declare (no competing root minted)"
                    );
                    AdoptOutcome::Adopted
                }
                Ok(other) => {
                    tracing::info!(
                        content_id = %id, outcome = ?other,
                        "adopt-before-author: conductor declared but the local stamp was a no-op"
                    );
                    AdoptOutcome::Held
                }
                Err(e) => {
                    tracing::warn!(
                        content_id = %id, error = %e,
                        "adopt-before-author: conductor declared but the local stamp failed \
                         (the DHT declaration stands; the projection heals next sweep)"
                    );
                    AdoptOutcome::Held
                }
            }
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains(ERR_NO_LOCAL_CHAIN) {
                // Expected on a genuinely fresh peer — not an error. Author
                // first (non-declaring), then come back.
                tracing::info!(
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    "adopt-before-author: conductor has no local chain for this id — \
                     authoring a non-declaring root first, then adopting the peer's head"
                );
                return AdoptOutcome::AuthorThenAdopt {
                    head_action_hash: head_action_hash.to_string(),
                    carried_record,
                    peer_id: hint.peer_id.clone(),
                };
            }
            if msg.contains(ERR_NOT_RETRIEVABLE) {
                tracing::info!(
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    carried_bytes,
                    "adopt-before-author: peer head not retrievable and no usable carried record \
                     — falling back to the author path, retried next sweep"
                );
            } else {
                tracing::warn!(
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    error = %msg,
                    "adopt-before-author: conductor declare failed — falling back to the \
                     author path, retried next sweep"
                );
            }
            AdoptOutcome::Author
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All four branches of the decision rule, named by the situation each one
    /// describes rather than by its inputs.
    #[test]
    fn own_conductor_canonical_always_adopts_never_authors() {
        // Regardless of what peers say or what this row already claims.
        for peer_declared in [false, true] {
            for local_declared in [false, true] {
                assert_eq!(
                    decide_head_action(true, peer_declared, local_declared),
                    HeadDecision::AdoptLocal,
                    "canonical own-conductor answer wins over every other input \
                     (peer_declared={peer_declared}, local_declared={local_declared})"
                );
            }
        }
    }

    #[test]
    fn peer_declaration_on_an_undeclared_row_adopts() {
        assert_eq!(
            decide_head_action(false, true, false),
            HeadDecision::AdoptPeer,
            "this is the live cure: peer B holds no crown, alpha-A advertises one"
        );
    }

    #[test]
    fn an_existing_local_declaration_holds() {
        assert_eq!(
            decide_head_action(false, true, true),
            HeadDecision::Hold,
            "two declarations cannot be ordered from Rust — hold, do not race"
        );
        assert_eq!(
            decide_head_action(false, false, true),
            HeadDecision::Hold,
            "nobody is advertising anything better; keep what we have"
        );
    }

    #[test]
    fn nothing_declared_anywhere_authors() {
        assert_eq!(
            decide_head_action(false, false, false),
            HeadDecision::Author,
            "today's behaviour, now safe: the authored root is not a declaration"
        );
    }

    /// The rule is total: every input triple maps to exactly one decision, and
    /// authoring happens ONLY in the all-false corner.
    #[test]
    fn author_is_reachable_only_when_no_head_exists_anywhere() {
        let mut authored = 0;
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    if decide_head_action(c, p, l) == HeadDecision::Author {
                        authored += 1;
                        assert!(
                            !c && !p && !l,
                            "Author must be unreachable when any head exists \
                             (canonical={c}, peer={p}, local={l})"
                        );
                    }
                }
            }
        }
        assert_eq!(
            authored, 1,
            "exactly one of the eight input triples authors"
        );
    }

    /// DECLARE-STORM GATE. The cure must not install a write storm: a converged
    /// corpus has to issue ZERO zome declares per sweep.
    #[test]
    fn declare_storm_gate_refuses_a_declare_that_moves_nothing() {
        assert!(
            !declaration_would_move(Some("uhCkk-ALPHA-A"), "uhCkk-ALPHA-A"),
            "already declaring this exact head — re-declaring is a source-chain \
             write plus a DHT link for no change, every sweep, forever"
        );
        assert!(
            !declaration_would_move(Some("  uhCkk-ALPHA-A "), "uhCkk-ALPHA-A"),
            "stray padding must not read as a difference and re-declare each sweep"
        );
    }

    #[test]
    fn declare_storm_gate_allows_a_fill_and_a_real_move() {
        assert!(
            declaration_would_move(None, "uhCkk-ALPHA-A"),
            "an UNDECLARED row is the fill case the peer arm exists for"
        );
        assert!(
            declaration_would_move(Some("uhCkk-LOCAL-ROOT"), "uhCkk-ALPHA-A"),
            "a genuinely different head is a real move and must be allowed"
        );
    }

    #[test]
    fn boot_context_has_no_hints_and_no_fetcher() {
        let ctx = AdoptContext::none();
        assert!(ctx.hints.is_empty());
        assert!(
            ctx.fetcher.is_none(),
            "the one-shot boot pass runs before P2P discovery — the peer arm must \
             degrade to the local-DHT arm alone, not error"
        );
    }
}
