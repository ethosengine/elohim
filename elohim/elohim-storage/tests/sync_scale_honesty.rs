//! Standing RED for spine node `sync-scale-honesty` (genesis/manifests/spine.yaml).
//!
//! **Invariant under test:** the sync plane's cost is sub-quadratic and measured —
//! announce-on-change (not poll-only), head-diff transfer, healthy request-response
//! to every mesh peer.
//!
//! **What is broken today** (verified 2026-07-25 against `dev`):
//!
//! 1. `P2PNode::initiate_sync_round` (`p2p/mod.rs`) opens every round by sending
//!    `SyncRequest::ListDocuments { offset: 0, limit: SYNC_LIST_PAGE_LIMIT }` to
//!    every connected peer. The opener carries **no local state** — not our heads,
//!    not a digest, not a since-cursor. So the peer must answer by enumerating its
//!    whole corpus, every 60s, per peer-pair, *even when nothing changed*. Cost is
//!    O(peers x corpus) per round mesh-wide, forever, in the steady state.
//!
//! 2. `SyncRequest::AnnounceChange` exists in the protocol and both transports
//!    HANDLE it (`p2p/mod.rs`, `p2p_iroh/sync_backend.rs`) — but nothing in `src/`
//!    ever CONSTRUCTS one to send. There is no push path at all: the 60s poll is
//!    the only propagation mechanism the sync plane has.
//!
//! Both tests below fail today and go green when the cure lands. They assert
//! *properties*, not mechanisms — a head digest, a since-cursor, a bloom filter,
//! or a head-diff request variant all satisfy test 1; any real push path satisfies
//! test 2. Neither test prescribes a design.
//!
//! These call PRODUCTION planning functions (`p2p::sync_round`) that the real
//! round uses — not a test-local mirror. A mirror would let production drift green
//! while the wire stayed quadratic.
//!
//! **Why `#[ignore]`.** These are a STANDING red: they are expected to fail until
//! the sync plane is cured, which is the point. The elohim-storage pre-push gate is
//! `just gate` → `fmt-check clippy test`, and `just test` is a plain `cargo test`
//! that runs every target in `tests/` — so leaving these in the default sweep would
//! wedge the gate for everyone who touches this crate, and the pressure to "make the
//! red go away" would fall on whoever is unlucky enough to push next. `#[ignore]`
//! keeps them COMPILED on every gate run (so a refactor can't silently rot them)
//! while leaving execution to the explicit spine check. This is the opposite of the
//! `#[ignore]`-as-CI-no-op trap in the root CLAUDE.md: there the harm is an ignored
//! test that was believed to be running; here the spine node records exactly what
//! the check is and that it fails.
//!
//! Run (the spine check for node `sync-scale-honesty`):
//!
//! ```bash
//! cargo test --test sync_scale_honesty -- --ignored
//! ```

#![cfg(feature = "p2p")]

use elohim_storage::p2p::sync_round::{
    announcements_for_local_change, round_opener, DocHead, LocalCorpusState,
};
use elohim_storage::p2p::SyncRequest;
use libp2p::PeerId;

/// The sync-partition namespace the content producer writes under.
const NS: &str = "elohim";

/// A corpus big enough that "enumerate everything, every round" is obviously
/// the wrong shape — and realistic: the content back-fill projects the whole
/// content table, which is why `initiate_sync_round` needed page-cursor
/// follow-ups in the first place.
const CORPUS_DOCS: usize = 2_000;

fn corpus(n: usize) -> LocalCorpusState {
    LocalCorpusState {
        docs: (0..n)
            .map(|i| DocHead {
                doc_id: format!("node:concept-{i}"),
                heads: vec![format!("head-hash-{i}")],
            })
            .collect(),
    }
}

/// Serialize a request so we can compare two openers without asking the wire
/// protocol type to derive `PartialEq` for a test's convenience.
fn wire(req: &SyncRequest) -> String {
    serde_json::to_string(req).expect("SyncRequest is Serialize")
}

/// RED 1 — head-diff transfer.
///
/// A node that already holds the whole corpus must not open a sync round the
/// same way a node that holds nothing does. If the opener is identical, it
/// carries no information about what we have, so the peer has no choice but to
/// enumerate its entire corpus back to us — every round, forever, converged or
/// not. That is the scaling cliff: steady-state cost proportional to corpus
/// size rather than to the number of changes.
#[test]
#[ignore = "STANDING RED — spine node sync-scale-honesty; run with --ignored"]
fn the_round_opener_reflects_what_we_already_have() {
    let knows_nothing = corpus(0);
    let fully_synced = corpus(CORPUS_DOCS);

    let cold_open = round_opener(NS, &knows_nothing);
    let warm_open = round_opener(NS, &fully_synced);

    assert_ne!(
        wire(&cold_open),
        wire(&warm_open),
        "the round opener is identical for a node holding {CORPUS_DOCS} docs and a node \
         holding none — it carries no local state, so every peer must enumerate its \
         whole corpus every round (60s tick, p2p/mod.rs), even when nothing changed. \
         Steady-state cost is O(peers x corpus) instead of O(changes). Cure: make the \
         opener a function of local state (head digest / since-cursor / bloom / \
         head-diff variant) — any of those satisfies this test."
    );
}

/// RED 2 — announce-on-change.
///
/// When this node applies a change of its own, connected peers must learn about
/// it by being told, not by waiting up to 60s for their next poll. The protocol
/// already carries `AnnounceChange` and every transport handles it on receipt —
/// there is simply no send site. Until there is, propagation latency is bounded
/// below only by the poll interval, and the poll is the sole propagation path.
#[test]
#[ignore = "STANDING RED — spine node sync-scale-honesty; run with --ignored"]
fn a_local_change_is_announced_to_connected_peers() {
    let peers: Vec<PeerId> = vec![PeerId::random(), PeerId::random()];

    let announcements =
        announcements_for_local_change(NS, "node:concept-1", "change-hash-abc", &peers);

    assert_eq!(
        announcements.len(),
        peers.len(),
        "a locally-authored change produced {} announcements for {} connected peers. \
         SyncRequest::AnnounceChange is defined and handled by both transports, but \
         nothing in src/ ever constructs one — the 60s poll is the only propagation \
         path the sync plane has. Cure: push an announce to connected peers at the \
         local-change site and wire it to this planner.",
        announcements.len(),
        peers.len()
    );

    for peer in &peers {
        let for_peer: Vec<_> = announcements.iter().filter(|(p, _)| p == peer).collect();
        assert_eq!(
            for_peer.len(),
            1,
            "peer {peer} should be told exactly once about a local change"
        );
        assert!(
            matches!(for_peer[0].1, SyncRequest::AnnounceChange { .. }),
            "expected AnnounceChange, got {:?}",
            for_peer[0].1
        );
    }
}
