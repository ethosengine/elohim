---
id: "backlog-adopt-local-heal-second-guesses-arbitrated-winner"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adopt_local re-derives forward-ordering proof the conductor's arbitrated canonical answer already settled — gossip-only peers converge slower than Declare-receiving peers"
slug: "adopt-local-heal-second-guesses-arbitrated-winner"
written: "2026-07-31"
author: "claude (ch06 R1 decision execution)"
status: "open"
priority: "medium"
area: "substrate/content-versioning-authority"
domain: "elohim-storage"
jobs: [elohim, elohim-edge]
tags: [heal, adopt-before-author, canonical-head, refused_declared, head_adoption, projection-reconcile, saga-06]
cites:
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md
  - genesis/data/timeline/backlog/content-divergence-unhealable-without-canonical-heads.md
  - genesis/a2o/features/dataplane/resiliency-saga/06-heads-converge.feature
---

# adopt_local stamps HealCanonical even when the source IS the arbitrated winner, not merely a local echo

## The gap

`adopt_local` (`elohim/elohim-storage/src/services/head_adoption.rs:368-405`) is
the LOCAL-DHT arm of adopt-before-author: when a node's OWN conductor already
resolves a canonical head for an id, this function stamps it into the local
row instead of authoring a competing root. It always stamps with
`StampMode::HealCanonical` (`:383`), whose contract
(`content_diesel.rs`, cited from the module doc `head_adoption.rs:360-367`) is
deliberately conservative: it FILLS an undeclared row unconditionally, but
only MOVES an already-`declared` row when the caller can prove forward
ordering. That conservatism is correct for a node's own bare local `get` —
its own conductor answering "this is canonical" carries no proof that it is
newer than a peer's existing declaration.

But `adopt_local`'s LOCAL-DHT arm is reached specifically because the local
conductor's `resolve_content_head` / canonical-head zome path already ran
`select_canonical_winner`
(`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:2876`) — the
same in-zome, presence-based arbitration function the whole adopt-before-author
design already trusts as authoritative (see
`head_adoption.rs:40-42`: "ordering is arbitrated in-zome by
`select_canonical_winner`. Inventing a newest-wins election here would
reintroduce the head-flapping this exists to stop"). When the source of the
canonical answer IS that arbitrated winner — not a bare unarbitrated local
echo — `HealCanonical`'s forward-ordering gate re-derives a proof the DNA
already settled, and refuses to move an already-`declared` row that disagrees
with it. The row falls into `StampOutcome`'s "cannot be shown to supersede"
branch (`:399`) and the caller reports `AdoptOutcome::Held`.

## Why this matters: two convergence speeds for the same true answer

A peer that receives an explicit `POST .../canonical-head` (`StampMode::Declare`,
the PEER-HINT arm / `adopt_peer`, or the deploy `DECLARE_ONLY` channel) moves a
declared row immediately — `Declare` is exempt from the forward-ordering gate
by design. A peer that instead discovers the SAME already-arbitrated answer
via its own conductor's LOCAL-DHT arm (`adopt_local`) does NOT move an already-
declared row, even though the underlying zome-level arbitration is identical.
Gossip-only peers — ones that never receive an explicit Declare POST, only DHT
gossip that eventually lets their own conductor resolve the arbitrated winner
— can stay pinned on a stale declared value indefinitely, while Declare-
receiving peers converge promptly on the same true answer.

This is the mechanism behind matthew's 6071 `refused_declared` outcomes logged
against 8 `healed` in 12h (see
`content-divergence-unhealable-without-canonical-heads.md` Finding 1, and the
commit that first named the class, `b19f12014`, "fix(storage): convergence
excludes adjudicated divergence"). `b19f12014` (Leg A) now EXCLUDES the
`refused_declared` class from `elohim_projection_reconcile_converged` — a
refusal here is classified as CORRECT and permanent-until-a-canonical-channel-
fires, so the convergence gauge is no longer falsely gated on it. That fix
makes the symptom stop LOOKING like a converged-blocker; it does not close the
underlying gap named here. **This item is therefore a slower-convergence
concern (gossip-only peers lag Declare-receiving peers, sometimes indefinitely
absent an explicit Declare), not a converged-blocker** — the gauge can read
`converged` fleet-wide while this class of row still silently stays behind the
arbitrated answer.

## Candidate fix direction

Let `adopt_local` stamp with `Declare`-mode semantics (bypass the
forward-ordering re-proof) when the value it is adopting is provably the
conductor's own `select_canonical_winner`-arbitrated output — i.e., the
LOCAL-DHT arm's answer already passed the SAME in-zome arbitration a
`Declare` POST would have carried, so re-deriving forward-ordering proof at
the storage layer is redundant, not protective. This is narrower than making
`HealCanonical` universally Declare-strength (which would reopen the
backward-move regression `HealCanonical` exists to prevent — see
`head_adoption.rs:363-367`, the 2026-07-12 edge #1187/#1188 regression this
guarded against): the distinguishing signal is specifically "did the DNA
already arbitrate this," not "did my own conductor merely echo something."
The exact predicate for detecting "this canonical answer already carries
zome-level arbitration provenance" (vs. an ordinary un-arbitrated local read)
needs DNA-side design — plausibly a field on the resolved canonical answer
recording whether it passed through `select_canonical_winner`/
`declare_canonical_content_head` — before this can be implemented safely.

## Definition of done

1. A reproducing scenario: two peers, one receives a Declare, the other only
   ever resolves the same arbitrated answer via its own conductor — the
   Declare-receiving peer converges, the LOCAL-DHT-arm peer stays `Held`/
   `refused_declared` on a disagreeing already-declared row.
2. A DNA-side (or storage-side, if provable without a DNA change) signal that
   distinguishes "this answer already carries zome-arbitration provenance"
   from a bare local echo.
3. `adopt_local` stamps with `Declare`-mode semantics ONLY when that signal is
   present; the 2026-07-12 backward-move regression path stays fully guarded
   for the un-arbitrated case.
4. ch06 (`06-heads-converge.feature`) or a sibling scenario demonstrates the
   previously-stuck gossip-only peer now converges without ever receiving an
   explicit Declare POST.

## Status

Open. Not blocking — `b19f12014` (Leg A) already keeps this class out of the
fleet-wide convergence gauge, so this is a latent slower-convergence gap to
close on its own schedule, not a red blocking the saga today.
