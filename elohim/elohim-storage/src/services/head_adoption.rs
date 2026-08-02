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
//! reintroduce the head-flapping this exists to stop. That remains true after
//! the CONTEST arm below: contesting supplies a CANDIDATE to the in-zome
//! arbiter, it never picks a winner.
//!
//! ## 4. CONTEST-THEN-OBEY (the two-way-declared class, 2026-08-02)
//!
//! The three arms above all assume at most one side holds a crown. The live
//! fleet's dominant class violates that: ~11.4k ids where THIS row declares X
//! and a peer advertises Y. `AdoptPeer` requires an undeclared local row, so it
//! never fired; the rule answered `Hold`; and because the ONLY automated minter
//! of canonical-head links is `AdoptPeer`, the DHT's `canonical_head` anchor
//! stayed EMPTY for the whole class. `select_canonical_winner` was never called
//! with a non-empty set — the arbiter existed but had nothing to arbitrate, so
//! every conductor answered with its own root-author fallback and every heal
//! honestly refused ("row already declared — heal left it to the canonical
//! channels", ~3.5k/hr on matthew alone). A permanent stalemate that no amount
//! of healing could break.
//!
//! [`HeadDecision::ContestPeer`] breaks it WITHOUT deciding anything locally: it
//! mints a canonical-head link naming the peer's head, which is one candidate in
//! the election. Every peer that does the same adds its own. The set is then
//! arbitrated on the DHT — tier, then notarized link clock, then link-hash
//! tiebreak — and the winner comes back through `resolve_content_head` as a
//! `canonical == true` answer carrying its election ordering, which
//! `content_diesel::canonical_move_verdict` obeys. Contest supplies; the DHT
//! elects; the projection obeys.
//!
//! Quiescence is structural, not timed: once a row carries an election
//! (`canonical_declared_at IS NOT NULL`) the rule returns to `Hold`, so a
//! converged corpus mints ZERO links per sweep. The transient window is at most
//! one sweep — the minting conductor's own next `resolve_content_head_local`
//! sees the link it just committed and stamps the election onto the row.
//!
//! Expected drain: ~11.4k contested ids fleet-wide, `WITNESS_MAX_PER_TICK` (200)
//! per tick per pod on ~300s sweeps ⇒ contest supply completes in roughly 1-2h
//! across 7 pods, with election-and-obey draining behind it.
//!
//! ## Contest supply: what the first live window taught (2026-08-02)
//!
//! The first deploy of the contest arm ran attempts but minted ZERO, failing in
//! ~0.4s per attempt. Two hypotheses were raised and BOTH disconfirmed in code
//! before any fix was written — recorded here because each is the kind of theory
//! that looks obviously right and would have cost a deploy cycle:
//!
//! 1. **Reach starvation** (advertise-but-refuse). Disconfirmed: the inventory
//!    (`list_content_anchor_inventory`) and the head-record responder
//!    (`is_distribution_safe_reach`) read the SAME `DISTRIBUTION_SAFE_REACH`
//!    constant, and `community` is a member of it. Scoped rows are excluded from
//!    BOTH surfaces symmetrically, so no class is advertised yet refused. The
//!    observed `403 requiredReach:community` on `/db/content` is the *viewer
//!    standing* gate — a different plane — and actually confirms the contested
//!    rows are `community`, i.e. distribution-safe.
//! 2. **Fetcher never wired.** Disconfirmed: `run_heal` constructs
//!    `PeerHeadRecordFetcher` and passes `fetcher: Some(&…)` in the same
//!    `AdoptContext` the contest call receives.
//!
//! The mechanism is upstream of both. A two-way-declared row is DECLARED, so it
//! cannot enter `adopt_candidates` through the gapfill-refused route (that
//! predicate requires `local_declared.is_none()`). It enters only via
//! conductor-missing (`resolve_content_head_local` → `Ok(None)`) or timeout — and
//! for the conductor-missing majority the conductor has **no chain for the id**,
//! which `declare_canonical_head_inner` rejects at its FIRST, target-independent
//! gate. No candidate shape can pass that gate, and a carried record cannot help:
//! the gate runs before the carried path is consulted. The 0.4s fast-fail is the
//! tell — a `Network` `get_links` that truly left the box would block toward the
//! 60s conductor timeout, so it short-circuited at authority on genuine local
//! absence.
//!
//! Hence the split in `inc_contest_failed`: `no_local_chain` is NOT a storage
//! bug and self-candidacy cannot fix it. If it dominates the next window, the
//! remedy is upstream witnessing (or a deliberate coordinator-zome decision about
//! the no-chain gate), not another storage arm. `not_retrievable` / `fetch_none`
//! ARE storage-fixable, and self-candidacy is their answer.
//!
//! ## The second live window, and the composition that closes the loop
//!
//! Edge #1291 read: `no_local_chain` 887, `not_retrievable` 1, `declare_error` 3,
//! **`fetch_none` ZERO**, mints `adopt_peer=1` and no self-head. The zero is the
//! sharper signal: it means no CHAIN-HOLDING pod ever reached a fetch, i.e. the
//! holding side was not contesting at all.
//!
//! The reason is an admission gap, not a decision bug. A holding pod resolves its
//! declared rows fine, so they never take the conductor-missing or timeout
//! routes; and `gapfill_would_self_elect` requires `local_declared.is_none()`, so
//! a DECLARED row fails that guard too. Those rows landed in the general heal
//! arm, were honestly refused (`SkippedDeclared`), and were pushed to NO
//! candidate list — never reaching [`decide_head_action`]. So the election
//! received a candidate from neither side: the missing side *cannot* declare, and
//! the holding side was never *asked* to.
//! `declared_divergence_should_route_to_contest` admits exactly that class.
//!
//! **Expected composition once both sides participate:**
//!
//! 1. The HOLDING pod admits its declared+divergent rows and contests. Its fetch
//!    from the missing peer returns hash-only (`record: None`) — verified: a peer
//!    serves `head_action_hash` from its SQLite row while its conductor cannot
//!    produce the bytes, so the payload carries the hash and no record.
//! 2. Declaring the peer's head therefore fails `is not retrievable`, and
//!    SELF-CANDIDACY mints the holding pod's OWN head. That is the first real
//!    candidate the election has ever received. (`fetch_none` should now become
//!    non-zero where self-candidacy does not immediately succeed — its absence
//!    was the fingerprint of the admission gap.)
//! 3. An election now EXISTS for the id.
//! 4. The MISSING pod, which can see the election but not the content, obeys it
//!    via `try_obey_visible_election`: fetch the winner's bytes from the holding
//!    peer, have the zome prove them, stamp under the election's ordering.
//! 5. The HOLDING pod's own row converges through ordinary heal — its conductor
//!    now answers `canonical == true` with the election ordering, and the Fix-2
//!    guard authorizes the move.
//!
//! **Residual class, out of reach this wave:** ids where BOTH sides are
//! conductor-missing. Neither can declare (the no-chain gate is
//! target-independent) and neither can obey (no election can be minted). Closing
//! it needs the deliberate coordinator-zome decision about that gate, which
//! touches adopt-before-author semantics and deserves its own review.
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
    /// The conductor was ASKED and did not answer (timeout), or answered
    /// `Ok(None)` for an id whose absence we cannot act on.
    ///
    /// Split out from `Known(None)` on 2026-08-02, exactly as the note above
    /// instructed: absence was NOT observed here, only unestablished. It behaves
    /// identically today (forecloses `AdoptLocal`, leaves `AdoptPeer` /
    /// `ContestPeer` / `Hold`), but it no longer LIES about provenance — a
    /// future arm that needs "the conductor observed nothing" can now
    /// distinguish the two without re-reading this comment and guessing.
    Unresolved,
}

/// Everything the peer-hint arm needs. Both fields are absent at boot (the
/// one-shot re-anchor pass runs before P2P discovery), which correctly degrades
/// the pre-flight to the local-DHT arm alone.
pub struct AdoptContext<'a> {
    pub hints: &'a PeerHeadHints,
    pub fetcher: Option<&'a dyn HeadRecordFetcher>,
    /// `config.contest_two_way_declared` — the per-pod switch for the CONTEST
    /// arm. `false` restores the pre-2026-08-02 `Hold`.
    pub contest_enabled: bool,
}

impl AdoptContext<'_> {
    /// The boot-time shape: no peer inventory yet, no transport to fetch with.
    ///
    /// `contest_enabled: false` is not a policy choice — with no hints there is
    /// nothing to contest, and the boot pass runs before P2P discovery.
    pub fn none() -> AdoptContext<'static> {
        static EMPTY: std::sync::OnceLock<PeerHeadHints> = std::sync::OnceLock::new();
        AdoptContext {
            hints: EMPTY.get_or_init(PeerHeadHints::new),
            fetcher: None,
            contest_enabled: false,
        }
    }
}

/// The decision rule, as a pure total function.
///
/// Pure on purpose: this is the part that got the live behaviour wrong, and it
/// is the part that must be readable in isolation and exhaustively tested
/// without a conductor, a pool, or a peer.
///
/// | own conductor canonical | peer declares | local declared | local election | ⇒ |
/// |---|---|---|---|---|
/// | yes | – | – | – | [`HeadDecision::AdoptLocal`] |
/// | no | yes | no | – | [`HeadDecision::AdoptPeer`] |
/// | no | yes | yes | no | [`HeadDecision::ContestPeer`] |
/// | no | yes | yes | yes | [`HeadDecision::Hold`] |
/// | no | no | yes | – | [`HeadDecision::Hold`] |
/// | no | no | no | – | [`HeadDecision::Author`] |
///
/// Rulings worth naming:
///
/// - **Peer-declares + local-declared + NO local election ⇒ ContestPeer.** Still
///   not an adopt: Rust does not order the two declarations, and nothing is
///   stamped. It mints the DHT evidence so the zome's arbiter has a set to elect
///   on. See [`HeadDecision::ContestPeer`].
/// - **Peer-declares + local-declared + local election ⇒ Hold.** An election has
///   already run and this row is obeying it. Contesting again would re-mint a
///   link every sweep forever — this is the QUIESCENCE gate, and it is why a
///   converged corpus mints exactly zero.
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
    /// BOTH sides declare, and this row's declaration has no canonical election
    /// behind it — mint a canonical-head link naming the PEER's head so the DHT
    /// has an election to run.
    ///
    /// ## Why this is not "racing to overwrite"
    ///
    /// The old rule held here, on the reasoning that two declarations cannot be
    /// ordered from Rust. That reasoning is still exactly right — and it is why
    /// this arm does NOT order them. It writes no row. It adds one candidate to
    /// the DHT link set and lets `content_store::select_canonical_winner` decide,
    /// which is deterministic (tier, then notarized link clock, then link-hash
    /// tiebreak) and identical on every peer.
    ///
    /// Two peers contesting each other therefore CONVERGE rather than flap: A
    /// mints a link naming B's head, B mints one naming A's, and both then
    /// resolve the same winner from the same set. The old `Hold` was safe but
    /// terminal — with no minter, the arbiter had an empty set and never ran, so
    /// the class could not converge by any path.
    ContestPeer,
    /// This row already carries a declaration that an election stands behind (or
    /// no peer is advertising anything) — neither adopt, contest, nor author.
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
///
/// `local_has_canonical_election` is `content.canonical_declared_at IS NOT NULL`
/// — "an election has already run and this row is obeying its outcome". It is
/// the QUIESCENCE input: it alone separates a contestable stalemate from a
/// settled one, and it is what makes a converged corpus mint zero links.
///
/// `contest_enabled` is the per-pod config switch. `false` collapses
/// `ContestPeer` back into `Hold`, i.e. exactly the pre-2026-08-02 behaviour.
pub fn decide_head_action(
    conductor_canonical: bool,
    peer_declared: bool,
    local_declared: bool,
    local_has_canonical_election: bool,
    contest_enabled: bool,
) -> HeadDecision {
    if conductor_canonical {
        HeadDecision::AdoptLocal
    } else if peer_declared && !local_declared {
        HeadDecision::AdoptPeer
    } else if local_declared {
        // Both sides declare. Contest ONLY when no election stands behind this
        // row — once one does, the row is obeying the DHT and re-minting would
        // be a permanent write storm against an already-settled question.
        if peer_declared && !local_has_canonical_election && contest_enabled {
            HeadDecision::ContestPeer
        } else {
            HeadDecision::Hold
        }
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
    /// A canonical-head link was minted naming the PEER's head — one candidate
    /// added to the DHT election. The local row is deliberately UNCHANGED: the
    /// winner is decided on the DHT and arrives later through the ordinary
    /// canonical heal path. SKIP the author path.
    Contested,
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
    // Read the declaration AND whether an election stands behind it in one
    // borrow. A row obeying an election is settled (Hold); a row declaring on
    // its own authority alone is contestable.
    let (local_declared, local_election) = match pool.get() {
        Ok(mut conn) => match content_diesel::declared_head_with_election(&mut conn, ctx, id) {
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
        LocalResolve::Known(_) | LocalResolve::Unresolved => None,
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
        // Absence was never observed — the conductor did not answer. Forecloses
        // `AdoptLocal` and nothing else, exactly as documented.
        LocalResolve::Unresolved => None,
        LocalResolve::Probe => probed.as_ref(),
    };
    let canonical_head = head.filter(|h| h.canonical);
    let hint = adopt.hints.get(id);

    // (1b) ELECTION-OBEY ARM. Scoped to the CONDUCTOR-MISSING class — no head
    // answer of any kind. Those rows are the ones whose `Ok(None)` may be hiding
    // a visible election (see `try_obey_visible_election`); a row that DID get an
    // answer is already served by the arms below, so probing here would spend a
    // conductor call per row per sweep for nothing.
    //
    // Runs BEFORE the decision rule on purpose: obeying an election the DHT has
    // already settled is strictly better than contesting it again, and a row this
    // arm moves is left holding the elected head with its ordering recorded — so
    // the very next sweep sees `canonical_declared_at` set and quiesces.
    if should_probe_election(head.is_some()) {
        if let Some(outcome) =
            try_obey_visible_election(hc, pool, ctx, id, hint, adopt.fetcher).await
        {
            return outcome;
        }
    }

    let decision = decide_head_action(
        canonical_head.is_some(),
        hint.is_some(),
        local_declared.is_some(),
        local_election,
        adopt.contest_enabled,
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
        HeadDecision::ContestPeer => {
            let hint = hint.expect("ContestPeer implies a hint");
            contest_peer(hc, ctx, id, hint, adopt.fetcher, local_declared.as_deref()).await
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
        // The DHT election behind this answer — what the guard actually compares.
        head.canonical_ordering(),
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

/// ELECTION-OBEY: move a row to the head the DHT elected, when this conductor can
/// SEE the election but cannot resolve the winner's bytes itself.
///
/// Returns `None` to mean "not applicable — carry on with the normal decision",
/// and `Some(outcome)` when this arm took responsibility for the id.
///
/// ## The class this closes
///
/// `resolve_content_head_local` answers `Ok(None)` for two unrelated reasons, and
/// the caller cannot tell them apart: no election exists, OR an election exists
/// whose target has not gossiped in. A pod in the second state is holding the
/// answer and unable to act on it — the ~24.9k/2h conductor-missing class, which
/// no amount of contesting can fix because contest SUPPLIES elections and this
/// class needs to OBEY one.
///
/// Link ops and entry ops travel independently: canonical-head links gossip on
/// the `canonical_head` StringAnchor authority, while the target Content is a
/// separate entry the conductor may simply not hold. `resolve_canonical_election`
/// reports the election as soon as the LINKS land; the winner's bytes then come
/// from a peer.
///
/// ## Why this is not "trusting a peer"
///
/// Nothing is stamped on a peer's say-so. Two independent gates must BOTH pass:
///
/// 1. **The election is read from THIS node's own conductor.** The peer does not
///    get to say which head is canonical — the DHT already decided, and we read
///    that decision locally.
/// 2. **The bytes are proven in wasm.** `validate_carried_head_record` re-derives
///    the action hash, verifies the author's signature, checks the entry↔action
///    binding, and enforces the target-id gate. A peer that serves anything other
///    than the elected action fails.
///
/// The peer is a byte courier, never an authority. And the stamp is
/// `HealCanonical` carrying the ELECTION's ordering, so
/// [`content_diesel::canonical_move_verdict`] still arbitrates — this arm cannot
/// move a row that already obeys an equal-or-newer election, and cannot move one
/// backwards. It never authors, never self-elects, and never widens `Declare`.
/// Should the election-obey arm be probed for this row?
///
/// Pure + total, matching the sibling routing predicates
/// (`gapfill_would_self_elect`, `conductor_missing_should_route_to_adopt`) — the
/// scope rule is the part worth testing without a conductor.
///
/// YES exactly when the own conductor gave NO head answer at all. That `Ok(None)`
/// is the state that may be hiding a visible election (see
/// [`try_obey_visible_election`]). A row that DID get an answer — canonical or
/// fallback — is already served by the existing arms, so probing it would spend a
/// conductor round-trip per row per sweep for nothing.
fn should_probe_election(head_answer_present: bool) -> bool {
    !head_answer_present
}

async fn try_obey_visible_election(
    hc: &Arc<HcClient>,
    pool: &DbPool,
    ctx: &AppContext,
    id: &str,
    hint: Option<&PeerHeadHint>,
    fetcher: Option<&dyn HeadRecordFetcher>,
) -> Option<AdoptOutcome> {
    // (1) What did the DHT elect? Read from our OWN conductor.
    let election = match conductor_writes::call_resolve_canonical_election(hc, id).await {
        Ok(Some(e)) => e,
        // No election visible — nothing to obey. Behaviour is exactly unchanged
        // for every id in this state, which is the pre-wave-4 world.
        Ok(None) => return None,
        Err(e) => {
            // A conductor that will not answer is not evidence of anything.
            // Fall through to the normal decision rather than holding the id.
            tracing::debug!(
                content_id = %id, error = %e,
                "election-obey: could not read the canonical election; continuing with the \
                 normal decision"
            );
            return None;
        }
    };

    // (2) Someone has to hand us the bytes. With no peer or no transport this is
    // not a failure — there is simply nothing to try yet.
    let (hint, fetcher) = match (hint, fetcher) {
        (Some(h), Some(f)) => (h, f),
        _ => return None,
    };

    let winner = election.winner_target.to_string();

    // (3) Fetch. The peer answers with ITS head for this id; that is only useful
    // if it happens to BE the elected action. A mismatch is not a lie — the peer
    // may simply hold a different head — but it is not the bytes we need.
    let carried = fetcher.fetch(&hint.peer_id, id).await;
    let bytes = match &carried {
        Some(c) if c.head_action_hash.trim() == winner.trim() => c.record.clone(),
        Some(c) => {
            crate::metrics::inc_election_obey_failed("fetch");
            tracing::debug!(
                content_id = %id,
                from_peer = %hint.peer_id,
                elected = %winner,
                peer_served = %c.head_action_hash,
                "election-obey: this peer holds a different head than the DHT elected — \
                 no bytes to prove the winner with; holding"
            );
            return Some(AdoptOutcome::Held);
        }
        None => {
            crate::metrics::inc_election_obey_failed("fetch");
            tracing::debug!(
                content_id = %id,
                from_peer = %hint.peer_id,
                elected = %winner,
                "election-obey: no record served for the elected head; holding, retried next sweep"
            );
            return Some(AdoptOutcome::Held);
        }
    };
    if bytes.is_none() {
        crate::metrics::inc_election_obey_failed("fetch");
        tracing::debug!(
            content_id = %id,
            from_peer = %hint.peer_id,
            elected = %winner,
            "election-obey: peer served the elected hash but carried no record bytes; holding"
        );
        return Some(AdoptOutcome::Held);
    }

    // (4) PROVE the bytes in wasm before they can touch a row.
    let proven =
        match conductor_writes::call_validate_carried_head_record(hc, id, &winner, bytes).await {
            Ok(Some(head)) => head,
            Ok(None) => {
                crate::metrics::inc_election_obey_failed("validate");
                return Some(AdoptOutcome::Held);
            }
            Err(e) => {
                crate::metrics::inc_election_obey_failed("validate");
                tracing::warn!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    elected = %winner,
                    error = %e,
                    "election-obey: a peer served bytes that do NOT prove the elected head — \
                     refusing them and holding (this is a peer serving something it cannot back)"
                );
                return Some(AdoptOutcome::Held);
            }
        };

    // (5) Stamp under the ELECTION's ordering. `HealCanonical` + the election
    // clock means `canonical_move_verdict` still decides: fill, forward-move, or
    // refuse. Proof of the bytes never becomes a licence to move backwards.
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            crate::metrics::inc_election_obey_failed("stamp_refused");
            tracing::warn!(content_id = %id, error = %e, "election-obey: db conn for stamp");
            return Some(AdoptOutcome::Held);
        }
    };
    let c = &proven.content;
    let patch = content_diesel::ContentProjectionPatch {
        blob_cid: c.blob_cid.clone(),
        content_size_bytes: c
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX)),
        title: Some(c.title.clone()),
        description: Some(c.description.clone()),
        content_type: Some(c.content_type.clone()),
        content_format: Some(c.content_format.clone()),
        reach: Some(c.reach.clone()),
        metadata_json: Some(c.metadata_json.clone()),
    };
    match content_diesel::stamp_declared_head_mode(
        &mut conn,
        ctx,
        id,
        proven.head_action_hash.to_string().as_str(),
        Some(proven.declared_at),
        Some(patch),
        StampMode::HealCanonical,
        Some(election.ordering()),
    ) {
        Ok(StampOutcome::Stamped) => {
            crate::metrics::inc_election_obeyed("carried");
            crate::metrics::inc_content_head_adopted();
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                head = %winner,
                from_peer = %hint.peer_id,
                earned = election.canonical_earned,
                "election-obey: OBEYED the DHT-elected canonical head — this conductor could \
                 see the election but not the content, so a peer supplied the bytes and the \
                 zome proved them; the row now agrees with the fleet"
            );
            Some(AdoptOutcome::Adopted)
        }
        Ok(other) => {
            // Already obeying this or a newer/earned election — a correct refusal.
            crate::metrics::inc_election_obey_failed("stamp_refused");
            tracing::debug!(
                content_id = %id, outcome = ?other,
                "election-obey: the row already obeys an equal-or-newer election; no move"
            );
            Some(AdoptOutcome::Held)
        }
        Err(e) => {
            crate::metrics::inc_election_obey_failed("stamp_refused");
            tracing::warn!(content_id = %id, error = %e, "election-obey: stamp failed");
            Some(AdoptOutcome::Held)
        }
    }
}

/// Process-local record of ids this node has already SELF-CANDIDATED, keyed by
/// `(id, target)`.
///
/// Self-candidacy names THIS row's own declared head, so — unlike peer
/// candidacy, whose target changes as peers move — the target is stable across
/// sweeps. Without a ledger a node would re-mint the identical link every sweep
/// during the window between minting and the election projecting onto the row.
/// Keying on `(id, target)` (not `id` alone) keeps a genuinely NEW local head
/// candidatable: if our head moves, that is a new candidate and must be minted.
///
/// Process-local on purpose: it is a de-duplication cache, not a truth store.
/// The DURABLE quiescence gate is `canonical_declared_at` on the row (see
/// [`decide_head_action`]) — once the election projects, contest is never
/// reached at all. This only bounds the transient window, so losing it on
/// restart costs at most one redundant mint per id.
static SELF_CANDIDATE_MINTS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::OnceLock::new();

/// Hard cap on the de-dup ledger. The contested set is ~11.4k ids fleet-wide;
/// this leaves generous headroom while making unbounded growth impossible on a
/// pathological corpus. On overflow the ledger CLEARS rather than evicting
/// selectively — the cost of a clear is at most one redundant mint per id, and a
/// simple bound is worth more here than a perfect one.
const SELF_CANDIDATE_LEDGER_CAP: usize = 50_000;

/// Ledger key. Separate fn so the `(id, target)` shape is asserted in one place.
fn self_candidate_key(id: &str, target: &str) -> String {
    format!("{id}\u{1}{target}")
}

/// Claim the right to self-candidate `(id, target)`. Returns `true` on the FIRST
/// call for a pair and `false` afterwards — so a caller mints at most once per
/// (id, local head) per process.
fn claim_self_candidacy(id: &str, target: &str) -> bool {
    let ledger = SELF_CANDIDATE_MINTS.get_or_init(|| std::sync::Mutex::new(Default::default()));
    let mut guard = match ledger.lock() {
        Ok(g) => g,
        // A poisoned lock must not silently license a re-mint storm; refuse.
        Err(_) => return false,
    };
    if guard.len() >= SELF_CANDIDATE_LEDGER_CAP {
        guard.clear();
    }
    guard.insert(self_candidate_key(id, target))
}

/// Which candidate a contest attempt named — the label for
/// `elohim_content_canonical_links_minted_total{source}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContestShape {
    /// Named the PEER's head, carried over view-federation.
    PeerHead,
    /// Named THIS row's own declared head, because the peer's could not be
    /// resolved by our conductor.
    SelfHead,
}

impl ContestShape {
    fn source_label(self) -> &'static str {
        match self {
            ContestShape::PeerHead => "contest_peer_head",
            ContestShape::SelfHead => "contest_self_head",
        }
    }
}

/// CONTEST: add ONE candidate to the DHT election for a two-way declared id.
/// Never stamps the local row.
///
/// This is the supply half of contest-then-obey (see the module note). The
/// declare goes through the OWN conductor and the DHT still decides the winner —
/// evidence, not authority.
///
/// ## Two candidate shapes, and why the second exists
///
/// **PEER-HEAD (preferred).** Fetch the peer's `Record` over view-federation and
/// declare it with `carried_record`, which the zome's `validate_carried_record`
/// proves in wasm (action-hash binding, author signature, entry↔action binding)
/// before writing the link.
///
/// **SELF-HEAD (fallback).** When the conductor answers `is not retrievable` for
/// the peer's head, declare THIS row's own declared head instead. This is
/// symmetric candidacy, NOT self-election: nothing is stamped, the peer is hitting
/// the same wall from its side and will mint ITS head as the competing candidate,
/// and `select_canonical_winner` then picks deterministically from the resulting
/// set. Both projections obey the winner through the ordinary canonical heal path.
///
/// The fallback is gated on the id being LOCALLY PRESENT, and the gate is free:
/// `is not retrievable` is returned only AFTER `declare_canonical_head_inner`'s
/// first gate (`gather_content_chain(id)`) has already passed, so that error is
/// itself proof that a local chain exists. `no local chain` gets NO fallback —
/// that first gate is target-INDEPENDENT, so naming a different target would fail
/// identically. Attempting it would burn a conductor round-trip per id per sweep
/// to reproduce the same refusal.
///
/// ## Reach is not the constraint here (2026-08-02 disconfirmation)
///
/// A reach-starvation hypothesis was investigated and DISCONFIRMED in code: the
/// inventory (`list_content_anchor_inventory`) and the head-record responder
/// (`is_distribution_safe_reach`) read the SAME `DISTRIBUTION_SAFE_REACH`
/// constant, and `community` is in it. There is no advertise-but-refuse class.
/// Scoped rows are excluded symmetrically from BOTH surfaces, so they never
/// become contest candidates at all.
///
/// Self-candidacy is nonetheless the reach-SAFE shape by construction: it moves
/// no records between peers. It writes a DHT link and calls this node's own
/// conductor about this node's own head — so even if a scoped row ever reached
/// this path, no bytes would cross a peer boundary.
///
/// Every failure degrades to [`AdoptOutcome::Held`] — the pre-cure behaviour —
/// and rides the next sweep. Deliberately no retry loop: the contest set is
/// ~11.4k ids and a retry ladder inside a 200/tick budget would starve the tail.
async fn contest_peer(
    hc: &Arc<HcClient>,
    ctx: &AppContext,
    id: &str,
    hint: &PeerHeadHint,
    fetcher: Option<&dyn HeadRecordFetcher>,
    local_declared: Option<&str>,
) -> AdoptOutcome {
    let _ = ctx;
    // IDEMPOTENCE, first gate: if the peer is advertising the head this row
    // already declares, there is no contest — the two sides agree and the
    // divergence is elsewhere (an anchor, not a head). Minting here would add a
    // candidate identical to our own position, every sweep, forever.
    if !declaration_would_move(local_declared, &hint.head_action_hash) {
        tracing::debug!(
            content_id = %id,
            "adopt-before-author: contest gate — the peer advertises the head this row already \
             declares, nothing to contest"
        );
        return AdoptOutcome::Held;
    }

    let fetcher_present = fetcher.is_some();
    let carried = match fetcher {
        Some(f) => f.fetch(&hint.peer_id, id).await,
        None => None,
    };
    // "Did we get BYTES?" — not "did the peer answer?". A peer holding a row it
    // cannot resolve answers hash-only (`record: None`), which is the DOMINANT
    // shape when the advertising peer is itself conductor-missing. Counting that
    // as `carried` would label it `not_retrievable` when it is precisely
    // `fetch_none`, and those two point at different remedies.
    let carried_present = carried.as_ref().is_some_and(|c| c.record.is_some());
    // The SERVED hash wins over the advertised one (same rule as `adopt_peer`):
    // it is the action the bytes actually prove.
    let (peer_head, carried_record) = match carried {
        Some(c) => (c.head_action_hash, c.record),
        None => (hint.head_action_hash.clone(), None),
    };
    // When the declare fails for want of a resolvable target, the actionable
    // distinction is whether we HAD a record to carry. Split here so the two
    // read as different problems on the dashboard.
    let unresolvable_class = if carried_present {
        "not_retrievable"
    } else {
        "fetch_none"
    };

    // ARM 1 — PEER-HEAD CANDIDACY.
    match conductor_writes::call_declare_canonical_content_head(
        hc,
        id,
        peer_head.clone(),
        carried_record,
    )
    .await
    {
        Ok(declared) => {
            return contested(
                ContestShape::PeerHead,
                id,
                &declared.head_action_hash,
                local_declared,
                hint,
                carried_present,
                fetcher_present,
            );
        }
        Err(e) => {
            let msg = e.to_string();

            // NO LOCAL CHAIN — terminal for this sweep. The first gate in
            // `declare_canonical_head_inner` is target-independent, so no
            // fallback target can pass it. This arm must NOT author: the row
            // already carries a declaration, and authoring here is exactly the
            // self-election the module exists to stop.
            if msg.contains(ERR_NO_LOCAL_CHAIN) {
                crate::metrics::inc_contest_failed("no_local_chain");
                tracing::info!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    carried = carried_present,
                    fetcher = fetcher_present,
                    "adopt-before-author: contest could not be minted — this conductor holds no \
                     chain for the id at all, so no candidate of any shape can be declared; \
                     holding, retried next sweep"
                );
                return AdoptOutcome::Held;
            }

            // NOT RETRIEVABLE — the chain EXISTS (that gate passed) but the
            // peer's head action cannot be resolved here. This is exactly the
            // subclass self-candidacy answers.
            if !msg.contains(ERR_NOT_RETRIEVABLE) {
                crate::metrics::inc_contest_failed("declare_error");
                tracing::warn!(
                    target: "elohim_storage::head_adoption",
                    content_id = %id,
                    from_peer = %hint.peer_id,
                    carried = carried_present,
                    fetcher = fetcher_present,
                    error = %msg,
                    "adopt-before-author: contest declare failed — holding, retried next sweep"
                );
                return AdoptOutcome::Held;
            }
        }
    }

    // ARM 2 — SELF-HEAD CANDIDACY (the not-retrievable subclass).
    let Some(own_head) = local_declared.filter(|h| !h.trim().is_empty()) else {
        // Nothing of our own to nominate. (Unreachable via `decide_head_action`,
        // which only routes here when the row IS declared — kept explicit rather
        // than unwrapped so a future caller cannot turn it into a panic.)
        crate::metrics::inc_contest_failed(unresolvable_class);
        tracing::info!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            from_peer = %hint.peer_id,
            carried = carried_present,
            fetcher = fetcher_present,
            "adopt-before-author: contest could not be minted — the peer's head is not \
             retrievable here and this row names no head of its own to nominate; holding"
        );
        return AdoptOutcome::Held;
    };

    // QUIESCENCE, second shape: we already nominated this exact head. Re-minting
    // would add an identical candidate every sweep until the election projects.
    if !claim_self_candidacy(id, own_head) {
        tracing::debug!(
            content_id = %id,
            "adopt-before-author: contest gate — this node already nominated its own head for \
             this id; awaiting the election"
        );
        return AdoptOutcome::Held;
    }

    match conductor_writes::call_declare_canonical_content_head(hc, id, own_head.to_string(), None)
        .await
    {
        Ok(declared) => contested(
            ContestShape::SelfHead,
            id,
            &declared.head_action_hash,
            local_declared,
            hint,
            carried_present,
            fetcher_present,
        ),
        Err(e) => {
            crate::metrics::inc_contest_failed(unresolvable_class);
            tracing::warn!(
                target: "elohim_storage::head_adoption",
                content_id = %id,
                from_peer = %hint.peer_id,
                carried = carried_present,
                fetcher = fetcher_present,
                error = %e,
                "adopt-before-author: self-candidacy declare failed — this node could not even \
                 nominate its own declared head; holding, retried next sweep"
            );
            AdoptOutcome::Held
        }
    }
}

/// Shared success path for both contest shapes: count the mint and say, in one
/// human sentence, which candidate was nominated and why.
#[allow(clippy::too_many_arguments)]
fn contested(
    shape: ContestShape,
    id: &str,
    minted_head: &crate::signals::HoloHashB64,
    local_declared: Option<&str>,
    hint: &PeerHeadHint,
    carried: bool,
    fetcher: bool,
) -> AdoptOutcome {
    crate::metrics::inc_content_canonical_link_minted(shape.source_label());
    match shape {
        ContestShape::PeerHead => tracing::warn!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            contested_head = %minted_head,
            held_head = %local_declared.unwrap_or(""),
            from_peer = %hint.peer_id,
            carried,
            fetcher,
            "adopt-before-author: CONTESTED a two-way declared head — minted a canonical \
             declaration naming the peer's head; the DHT election decides, the row is \
             unchanged until it does"
        ),
        ContestShape::SelfHead => tracing::warn!(
            target: "elohim_storage::head_adoption",
            content_id = %id,
            contested_head = %minted_head,
            held_head = %local_declared.unwrap_or(""),
            from_peer = %hint.peer_id,
            carried,
            fetcher,
            "adopt-before-author: CONTESTED a two-way declared head by nominating THIS node's \
             own head — the peer's head is not retrievable here, so both sides nominate their \
             own and the DHT election picks between them; the row is unchanged until it does"
        ),
    }
    AdoptOutcome::Contested
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
            crate::metrics::inc_content_canonical_link_minted("adopt_peer");
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
                    // The declare has written a link but not read back its
                    // notarized timestamp, so it carries no election to record.
                    // The row's ordering backfills on the next canonical heal
                    // resolve — which answers with the election just created.
                    None,
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

    /// Contest ON — the rollout default. Keeps the older tests reading as
    /// situations rather than argument lists.
    fn decide(c: bool, p: bool, l: bool, election: bool) -> HeadDecision {
        decide_head_action(c, p, l, election, true)
    }

    /// All branches of the decision rule, named by the situation each one
    /// describes rather than by its inputs.
    #[test]
    fn own_conductor_canonical_always_adopts_never_authors() {
        // Regardless of what peers say, what this row already claims, or whether
        // an election stands behind it.
        for peer_declared in [false, true] {
            for local_declared in [false, true] {
                for election in [false, true] {
                    assert_eq!(
                        decide(true, peer_declared, local_declared, election),
                        HeadDecision::AdoptLocal,
                        "canonical own-conductor answer wins over every other input \
                         (peer={peer_declared}, local={local_declared}, election={election})"
                    );
                }
            }
        }
    }

    #[test]
    fn peer_declaration_on_an_undeclared_row_adopts() {
        assert_eq!(
            decide(false, true, false, false),
            HeadDecision::AdoptPeer,
            "this is the live cure: peer B holds no crown, alpha-A advertises one"
        );
    }

    #[test]
    fn an_existing_local_declaration_holds() {
        assert_eq!(
            decide(false, false, true, false),
            HeadDecision::Hold,
            "nobody is advertising anything better; keep what we have"
        );
        assert_eq!(
            decide(false, false, true, true),
            HeadDecision::Hold,
            "no peer hint means nothing to contest, election or not"
        );
    }

    /// THE LIVE CLASS (2026-08-02): both sides declare, no election has run.
    /// This used to `Hold` forever — and because `AdoptPeer` is the only
    /// automated minter of canonical links, holding meant the DHT arbiter never
    /// got a set to elect on. ~11.4k ids fleet-wide sat here.
    #[test]
    fn two_way_declared_without_an_election_contests() {
        assert_eq!(
            decide(false, true, true, false),
            HeadDecision::ContestPeer,
            "supply the election a candidate; do NOT decide the winner locally"
        );
    }

    /// QUIESCENCE — the property that makes this a cure and not a write storm.
    /// Once an election stands behind the row, a peer advertising the LOSING
    /// head must not provoke another mint: the question is settled on the DHT
    /// and this row is already obeying it.
    #[test]
    fn a_row_obeying_an_election_holds_against_a_peer_advertising_the_loser() {
        assert_eq!(
            decide(false, true, true, true),
            HeadDecision::Hold,
            "a converged corpus mints ZERO links per sweep — re-contesting a \
             settled election is exactly the permanent write storm this gate exists to stop"
        );
    }

    /// The config switch is a true rollback: OFF restores the pre-cure rule.
    #[test]
    fn contest_disabled_restores_the_old_hold() {
        assert_eq!(
            decide_head_action(false, true, true, false, false),
            HeadDecision::Hold,
            "per-pod OFF switch must yield exactly the prior behaviour"
        );
    }

    #[test]
    fn nothing_declared_anywhere_authors() {
        assert_eq!(
            decide(false, false, false, false),
            HeadDecision::Author,
            "today's behaviour, now safe: the authored root is not a declaration"
        );
    }

    /// The rule is total: every input maps to exactly one decision, and
    /// authoring happens ONLY when no head exists anywhere.
    #[test]
    fn author_is_reachable_only_when_no_head_exists_anywhere() {
        let mut authored = 0;
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    for e in [false, true] {
                        for enabled in [false, true] {
                            if decide_head_action(c, p, l, e, enabled) == HeadDecision::Author {
                                authored += 1;
                                assert!(
                                    !c && !p && !l,
                                    "Author must be unreachable when any head exists \
                                     (canonical={c}, peer={p}, local={l}, election={e})"
                                );
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(
            authored, 4,
            "exactly the four no-head-anywhere corners author (election and the \
             config switch are irrelevant when nothing is declared)"
        );
    }

    /// CONTEST never authors and never adopts — it is reachable only from the
    /// two-way-declared corner, and only with the switch on.
    #[test]
    fn contest_is_reachable_only_from_the_two_way_declared_corner() {
        for c in [false, true] {
            for p in [false, true] {
                for l in [false, true] {
                    for e in [false, true] {
                        for enabled in [false, true] {
                            if decide_head_action(c, p, l, e, enabled) == HeadDecision::ContestPeer
                            {
                                assert!(
                                    !c && p && l && !e && enabled,
                                    "ContestPeer escaped its corner \
                                     (canonical={c}, peer={p}, local={l}, election={e}, \
                                      enabled={enabled})"
                                );
                            }
                        }
                    }
                }
            }
        }
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

    /// QUIESCENCE, shape 2 (self-candidacy). The durable gate is
    /// `canonical_declared_at` on the row, but that only closes once the election
    /// PROJECTS. In the window between minting and projecting, the ledger is what
    /// stops a node re-nominating the identical head every sweep.
    #[test]
    fn self_candidacy_is_claimed_once_per_id_and_head() {
        let id = "quiescence:self-candidacy";
        let head = "uhCkk-own-head";

        assert!(
            claim_self_candidacy(id, head),
            "the first nomination must be allowed — this is the candidate that \
             gives the DHT election something to pick"
        );
        for sweep in 0..5 {
            assert!(
                !claim_self_candidacy(id, head),
                "sweep {sweep}: re-nominating the SAME head adds an identical candidate \
                 for no gain — a converged corpus must mint zero"
            );
        }
    }

    /// A genuinely NEW local head is a NEW candidate and must be nominatable —
    /// keying the ledger on the id alone would freeze a node out of the election
    /// forever the moment its own head legitimately moved.
    #[test]
    fn a_moved_local_head_may_be_nominated_again() {
        let id = "quiescence:moved-head";
        assert!(claim_self_candidacy(id, "uhCkk-head-A"));
        assert!(!claim_self_candidacy(id, "uhCkk-head-A"));
        assert!(
            claim_self_candidacy(id, "uhCkk-head-B"),
            "our head moved; that is a different candidate and the election must see it"
        );
    }

    /// The ledger keys two different ids independently — a shared key would let
    /// one id's nomination silently suppress another's.
    #[test]
    fn the_self_candidacy_ledger_does_not_collide_across_ids() {
        assert!(claim_self_candidacy("quiescence:id-one", "uhCkk-shared"));
        assert!(
            claim_self_candidacy("quiescence:id-two", "uhCkk-shared"),
            "same head hash under a different id is a distinct candidacy"
        );
    }

    /// The key is unambiguous: no (id, target) pair can be confused with another
    /// by concatenation. Uses a separator that cannot occur in either component.
    #[test]
    fn self_candidate_keys_are_unambiguous() {
        assert_ne!(
            self_candidate_key("a:b", "c"),
            self_candidate_key("a", "b:c"),
            "a naive join would collide these two distinct candidacies"
        );
    }

    /// QUIESCENCE, shape 1 (the durable gate) — restated against the shapes so
    /// the two are visibly complementary: the ledger bounds the transient window,
    /// `canonical_declared_at` closes it permanently.
    #[test]
    fn the_durable_gate_supersedes_the_ledger_once_the_election_projects() {
        assert_eq!(
            decide(false, true, true, true),
            HeadDecision::Hold,
            "once an election stands behind the row, contest is never reached at \
             all — the ledger is only needed before that lands"
        );
        assert_eq!(
            decide(false, true, true, false),
            HeadDecision::ContestPeer,
            "and before it lands, contest IS reached — which is exactly why the \
             ledger has to hold the line in the meantime"
        );
    }

    /// ELECTION-OBEY scope: the arm is probed ONLY for the conductor-missing
    /// class (no head answer at all). A row that got an answer — canonical or
    /// fallback — is already served by the existing arms, and probing it would
    /// spend a conductor round-trip per row per sweep for nothing.
    ///
    /// This mirrors the gate in `try_adopt_canonical_head`.
    #[test]
    fn election_obey_is_scoped_to_the_conductor_missing_class() {
        assert!(
            should_probe_election(false),
            "no head answer at all is exactly the class that may be hiding a \
             visible election — probe it"
        );
        assert!(
            !should_probe_election(true),
            "a row the conductor DID answer for is already served by the adopt/contest \
             arms; probing it spends a conductor round-trip per row per sweep for nothing"
        );
    }

    /// The obey path must stamp under the ELECTION's ordering in `HealCanonical`
    /// mode — never `Declare`. Proof of the bytes authorizes believing WHAT the
    /// head is; it never authorizes moving a row backwards. The tempting shortcut
    /// (bytes are proven, so just `Declare` them) is exactly the 2026-07-12
    /// backwards-heal regression.
    ///
    /// Asserted on the VERDICT function rather than by scanning source text: the
    /// property that matters is that an obey-shaped stamp is still arbitrated,
    /// and a source scan for `StampMode::` is both brittle and — as this test
    /// originally proved — capable of passing by accident when its slice runs
    /// past the end of the function it meant to check.
    #[test]
    fn election_obey_ordering_is_still_arbitrated_not_forced() {
        use crate::db::content_diesel::{canonical_move_verdict, StaleReason};

        // Forward election ⇒ the obey stamp moves.
        assert!(canonical_move_verdict(Some((9_000, false)), Some((1_000, false))).is_ok());
        // Older election ⇒ refused, even though the bytes were cryptographically
        // proven. Proof of WHAT is not permission to go BACKWARDS.
        assert_eq!(
            canonical_move_verdict(Some((1_000, false)), Some((9_000, false))).unwrap_err(),
            StaleReason::NotNewer,
        );
        // Staging election cannot displace an earned one, proven bytes or not.
        assert_eq!(
            canonical_move_verdict(Some((9_000, false)), Some((1_000, true))).unwrap_err(),
            StaleReason::Tier,
        );
    }

    /// QUIESCENCE, shape 3 (obeyed rows). Once the obey path stamps, the row
    /// carries `canonical_declared_at`, so the decision rule Holds — no re-obey
    /// attempt and, critically, no CONTEST of a question the DHT already settled.
    #[test]
    fn an_obeyed_row_neither_re_obeys_nor_contests() {
        assert_eq!(
            decide(false, true, true, true),
            HeadDecision::Hold,
            "a row that obeyed an election must not then contest it — that would \
             re-open a settled question every sweep"
        );
        // And the obey arm itself refuses a second move: `canonical_move_verdict`
        // declines an equal election (NotNewer), which the arm counts as
        // `stamp_refused` rather than treating as progress.
        assert_eq!(
            crate::db::content_diesel::canonical_move_verdict(
                Some((7_000, false)),
                Some((7_000, false))
            )
            .unwrap_err(),
            crate::db::content_diesel::StaleReason::NotNewer,
            "re-obeying the SAME election must not read as a move"
        );
    }

    /// A no-election answer must leave behaviour EXACTLY as it was — the arm
    /// returns `None` and the normal decision rule runs. This is what keeps
    /// wave 4 safe to land before the wave-3 window confirms anything.
    ///
    /// Expressed as the decision rule the fall-through lands on: with no election
    /// visible the obey arm returns `None`, so a conductor-missing row with a peer
    /// hint reaches exactly the verdict it reached before wave 4.
    #[test]
    fn no_visible_election_falls_through_to_the_unchanged_decision() {
        assert_eq!(
            decide(false, true, false, false),
            HeadDecision::AdoptPeer,
            "undeclared + peer declares ⇒ AdoptPeer, exactly as before wave 4"
        );
        assert_eq!(
            decide(false, true, true, false),
            HeadDecision::ContestPeer,
            "two-way declared ⇒ ContestPeer, exactly as before wave 4 — the obey arm \
             must not silently swallow ids it could not act on"
        );
        assert_eq!(
            decide(false, false, false, false),
            HeadDecision::Author,
            "nothing anywhere ⇒ Author, unchanged"
        );
    }

    #[test]
    fn contest_shape_source_labels_are_distinct_and_stable() {
        assert_eq!(ContestShape::PeerHead.source_label(), "contest_peer_head");
        assert_eq!(ContestShape::SelfHead.source_label(), "contest_self_head");
        assert_ne!(
            ContestShape::PeerHead.source_label(),
            ContestShape::SelfHead.source_label(),
            "the two shapes must be separable on the dashboard — which one \
             converges the fleet is the open question this metric answers"
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
