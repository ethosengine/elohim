---
id: "backlog-p2p-mod-loc-ceiling-decomposition"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Decompose p2p/mod.rs god-file — hard LoC-ceiling breach (finding 8b36709ba1e0): split the 7859-line impl P2PNode into topical submodule files by pure code motion"
slug: "p2p-mod-loc-ceiling-decomposition"
written: "2026-07-03"
updated: "2026-07-06"
author: "rust-architect (source-file-loc-ceiling finding 8b36709ba1e0)"
status: "backlog"
priority: "medium"
finding: "8b36709ba1e0"
policy: "source-file-loc-ceiling@1"
relatedNodeIds:
  - "backlog-arch-dataplane-refactor-backlog"
  - "feedback_swarm_composition_fresh_tree_build"
  - "feedback_signature_changes_grep_callers"
  - "project_principle_p1_reconciliation_controller"
tags: [architecture, refactor, p2p, loc-ceiling, god-file, mech, dataplane, tech-debt, mod-decomposition]
cites:
  - elohim/elohim-storage/src/p2p/mod.rs
  - .claude/epr-meta/policies.yaml
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
shift_objective: |
  Land Phase 1 of the p2p/mod.rs decomposition: move `handle_event` +
  `handle_behaviour_event` (~2350 LoC) out of `elohim/elohim-storage/src/p2p/mod.rs`
  into a new `elohim/elohim-storage/src/p2p/node_swarm_events.rs` as a split inherent
  `impl P2PNode` block — pure code motion, zero behavior change, zero signature change
  (only bump the two entry-point methods' visibility to `pub(super)` so mod.rs-resident
  `run()` can still call them; the compiler enforces this). This one move drops mod.rs
  from ~7859 to ~5509 LoC, clearing the 7000 hard ceiling and the finding. Verify with a
  CLEAN-TREE build (this is the swarm-composition home — a DNA-worktree `just check` does
  NOT verify elohim-storage): `cd elohim/elohim-storage && RUSTFLAGS='--cfg
  getrandom_backend="custom"' cargo build` + `cargo clippy -- -D warnings` + `cargo fmt
  --check` + `cargo test export_bindings` and sha256-diff the 4 generated View TS files to
  prove byte-identical. Then optionally continue Phases 2-6 (each an independently-landable
  code-motion increment) toward the 3000 soft ceiling. Serialize the phases — same file,
  same struct = merge-conflict zone; never run two phases (or the sibling dataplane items
  #6/#10/#12/#15) concurrently. BEFORE starting Phase 1, confirm the tree is clean at
  `elohim/elohim-storage/src/p2p/mod.rs` — as of 2026-07-06 a live-ops session is mid-edit
  on the inventory-snapshot receive arm (`apply_snapshot` call site, ~line 6003, inside
  `handle_behaviour_event`'s Gossipsub/INVENTORY_TOPIC branch), which sits INSIDE this
  exact extraction target. Wait for that work to commit; do not code-move a region another
  session has uncommitted changes in (see Refactor-safety note 8).
---

# Decompose p2p/mod.rs — hard LoC-ceiling breach (finding 8b36709ba1e0)

`elohim/elohim-storage/src/p2p/mod.rs` is **7859 lines** (2026-07-03 scope was 7835; +24 from
legitimate concurrent feature landings since — the file keeps growing while this entry sits in
`backlog`, which is itself evidence for why the finding matters) — past the
`source-file-loc-ceiling@1` **hard ceiling of 7000** (soft 3000). This is the escalated tier of that
policy: a fingerprinted architecture finding (`8b36709ba1e0`), not a soft edit-time nudge. The policy `why` is the mandate:
a first-party file past the hard ceiling *corrodes its own governance* (line-anchor invariants drift
every edit, two-location sync traps grow ~1000 lines apart) and outgrows a single review context.
The instruction is exact: **canonicalize a modularization plan into the timeline backlog and drive it
as bounded work — never refactor mid-edit.** This entry is that plan. Do not refactor inline off the
finding; drive from here.

## Module-seam map (scoped 2026-07-03, refreshed 2026-07-06 — anchor to method names, not line numbers)

The file is three regions. The middle one is the whole problem.

- **Region A — header + types (~lines 1–1690, ~1690 LoC).** Module doc, imports/`pub mod` decls,
  `FederationError`, `DeliveryPeer`, `P2PConfig` (+`Default`), the `P2PNode` struct def (~154 fields,
  all private), `CachedIdentifyInfo`, `PeerMetrics`, `ReconciliationMetrics`(+`Snapshot`),
  `DrainStatusInfo`, `P2PStatusInfo`, `PeerInfoView`, `PeerListView`, the `P2PCommand` enum,
  `SyncPauseGuard` (+`Drop`), `P2PHandle` (+`impl`). The 4 `#[ts(export)]` View types
  (`DrainStatusInfo`, `P2PStatusInfo`, `PeerInfoView`, `PeerListView`) live here.
- **Region B — `impl P2PNode` (~lines 1691–7745, ~6055 LoC, 49+ methods).** The god-block. Clusters:
  - *Swarm event dispatch (~2400 LoC — the single biggest chunk):* `handle_event`,
    `handle_behaviour_event`. The latter is one ~2350-line `match` over
    `ElohimStorageBehaviourEvent`, already cleanly grouped per-plane (ShardProtocol, BlobProtocol,
    Kademlia, Mdns, SyncProtocol, EprProtocol, EprAtomProtocol, TrustProtocol, IdentityHandshake,
    ViewFederation, ShamirShare, Identify, AutoNat, Ping, Relay{Client,Server}, Dcutr, Gossipsub).
    **Live-edit hazard inside this cluster (2026-07-06):** the Gossipsub arm's `INVENTORY_TOPIC`
    branch (~line 6003) calls `crate::db::peer_blob_inventory::apply_snapshot` and is mid-edit on
    this shared tree — uncommitted receive-side-idempotency work (`git diff` shows +22/-1 in
    mod.rs) is adding a `SnapshotApplyOutcome::{Applied, Deduplicated}` match so a byte-identical
    re-delivery skips the per-hash commitment re-score instead of re-triggering
    `score_and_enqueue_snapshot`. This is the single highest-probability collision point for Phase 1
    — see Refactor-safety note 8.
  - *Command dispatch (~600 LoC):* `handle_command` (~520, 19 `P2PCommand` variants), `drain_publish_queue`.
  - *Reconcile/replication cycles (~600 LoC):* `hydrate_replication_state`, `initiate_sync_round`,
    `run_replication_cycle`, `drain_gap_queue`, `run_acquisition_reconcile`, `run_provide_reconcile`,
    `drain_acquisition_queue`, `refresh_status`.
  - *Request/response handlers (~900 LoC):* `handle_shard_request`, `verify_shard_locations`,
    `handle_sync_request`/`_response`, `resolve_epr_head_locally`, `handle_epr_atom_request`/`_response`,
    `handle_epr_request`/`_response`, `heal_content_row`, `heal_blob_bytes_if_absent`.
  - *Broadcast producers (~450 LoC):* `broadcast_inventory_snapshot`, `broadcast_salvage_capacity`,
    `score_and_enqueue_snapshot`, `set_last_gossiped_inventory`.
  - *Lifecycle/builders/accessors (~900 LoC):* `new`, `start`, `run` (the ~305-line `select!` loop),
    the `with_*`/`set_*` builders, `peer_id`/`is_connected`/`connected_peers`/`reconciliation_metrics`/
    `shutdown_sender`/`sync_manager`/`handle`/`dedup_stats`.
- **Region C — tests (~lines 7746–7859).** `bootstrap_peering_tests`, `reach_gate_tests`.

## The extraction mechanic (the load-bearing safety insight)

**Split inherent `impl P2PNode` across submodule files — do NOT introduce traits, do NOT change
signatures.** Rust permits multiple inherent `impl P2PNode { … }` blocks in different modules of the
same crate. Because `P2PNode`'s fields and private methods are declared in the `p2p` module root,
they are visible to *descendant* modules automatically — so a method body moved into
`p2p::node_swarm_events` can still read `self.swarm`, call `self.heal_content_row(...)`, etc., with no
churn. The only compiler-enforced adjustment: a method that **moves to a child module but is called
from mod.rs-resident code** must be bumped from private to `pub(super)` (or `pub(crate)`). Wrong
visibility is a *compile error*, not a silent bug — a safe failure mode. This keeps every phase pure
code motion: methods stay methods, same `self`-receiver, callers untouched. Keep all types in Region A
in-crate (mod.rs root or a `p2p/types.rs`) so the 4 ts-rs View exports stay path-stable. Preserve the
`pub use` re-exports so `elohim_storage::p2p::{P2PNode, P2PConfig, P2PCommand, P2PHandle, …}` — the
module's public API — is byte-for-byte unchanged.

## Phased bounded plan (each phase lands green independently; serialize them)

- **Phase 0 — guardrails (no code motion).** Capture baseline: LoC, full `cargo test` green,
  sha256 of the 4 generated View TS files, `cargo clippy` clean. Encode the verification protocol below.
- **Phase 1 — carve swarm event dispatch → `p2p/node_swarm_events.rs` (~2350 LoC out). CLEARS THE FINDING.**
  mod.rs ~7859 → ~5509, under the hard ceiling. Highest leverage, lowest risk (one contiguous block).
  **Gate:** do not start until the in-flight `apply_snapshot` idempotency edit (note 8) has
  committed — it sits inside this exact extraction target.
- **Phase 2 — command dispatch → `p2p/node_commands.rs`** (`handle_command`, `drain_publish_queue`; ~600 LoC).
- **Phase 3 — reconcile cycles + heal → `p2p/node_reconcile.rs`** (~800 LoC).
- **Phase 4 — request/response handlers → co-locate into the existing per-plane modules**
  (`shard_protocol.rs`, `sync_protocol.rs`, `epr_protocol.rs`, `epr_atom_protocol.rs`) as `impl P2PNode`
  blocks — puts each plane's wire handling next to its codec (~900 LoC).
- **Phase 5 — lifecycle/builders + broadcast producers → `p2p/node_lifecycle.rs` + `p2p/node_broadcast.rs`** (~900 LoC).
- **Phase 6 — types → `p2p/types.rs`** (P2PConfig, P2PCommand, P2PHandle, metrics, the 4 View types; in-crate, ts-rs-safe).

End state: `mod.rs` is a thin module root (doc header, `pub mod`, `pub use`, `struct P2PNode`) well under
the 3000 soft ceiling; each concern in a focused file. Phase 1 alone discharges the hard-ceiling
finding; Phases 2-6 ratchet toward the soft ceiling and are each afternoon-sized.

## Refactor-safety notes (honoring the policy `why` + gospel)

1. **This is plain native Rust, NOT a zome.** The policy `why`'s asymmetry: no DNA hash moves, no
   reinstall/partition trap, no DNA-lineage event needed. Gated by fmt/clippy/nextest/build alone.
2. **But it IS the swarm-composition home.** Verify with a CLEAN-TREE `cargo build` on elohim-storage
   (`RUSTFLAGS='--cfg getrandom_backend="custom"'`), never a DNA-worktree `just check` — see
   [[feedback_swarm_composition_fresh_tree_build]]. Parallel sessions on a shared tree hide missing code.
3. **Zero signature change ⇒ zero caller churn**, but still `rg` the method names crate-wide (incl.
   `tests/`) to confirm nothing references them via a path/`fn`-pointer — [[feedback_signature_changes_grep_callers]].
4. **ts-rs:** the 4 View types stay in the same crate, so module moves are ts-rs-safe; still run
   `cargo test export_bindings` and sha256-diff the 4 generated `.ts` files to prove byte-identical.
5. **Behavior-identical, wire-untouched:** no codec/protocol edits ⇒ no protocol-version bump, no compat test.
6. **Cargo target-pool + serialize:** `CARGO_TARGET_DIR` at the family slot; never concurrent cargo on
   one slot; run the phases one at a time (same file = merge-conflict zone).
7. **Public API preserved** ⇒ no gospel-tier surface migration owed; the CLAUDE.md / rust-architect
   "Key Files" tables reference `p2p/*.rs` generically, with no line-anchors into mod.rs internals.
8. **Live-ops collision inside the Phase 1 extraction target (flagged 2026-07-06).** A concurrent
   session is mid-edit on the inventory-snapshot receive arm — the `apply_snapshot` call site
   (~line 6003, inside `handle_behaviour_event`'s Gossipsub/`INVENTORY_TOPIC` branch) is gaining a
   `SnapshotApplyOutcome::{Applied, Deduplicated}` match to suppress commitment re-scoring under a
   gossipsub re-delivery storm. That code sits *inside* Phase 1's ~2350-LoC extraction block, so a
   code-move started while it's uncommitted maximizes merge-conflict surface in the single
   hottest corner of the file. Sequence discipline: (a) `git status`/`git diff` on
   `elohim/elohim-storage/src/p2p/mod.rs` before dispatching Phase 1 — proceed only on a clean tree
   (or a diff that does NOT touch the Gossipsub/inventory arm); (b) never `git stash`/`reset`/revert
   another session's ambient changes to "clear the way" — per the shared-tree git discipline, commit
   path-limited and let concurrent work land on its own; (c) if Phase 1 must start before that work
   commits, coordinate directly with the in-flight session rather than guessing whether the region
   is settled. This is a scheduling gate, not a design blocker — Phase 1 itself is unaffected once
   sequenced correctly.

## Composition with the existing dataplane backlog (compose, don't fork)

`arch-dataplane-refactor-backlog.md` (2026-06-11) already ranks the *design-tier* decomposition of this
file: **#10 ProtocolHandler trait** (hollow out `handle_behaviour_event` for Swarm-free unit tests),
**#12 CommandHandler dispatch** (hollow out `handle_command`), **#15 TimerArm select!-loop refactor**,
plus **#4 BootstrapRepeater**, **#5 BackpressureGate**, **#6 PeerContextCache** — with a strict
`10 → 12 → 15` chain and a "no concurrency with #6" caveat. This entry is deliberately *upstream and
complementary*: it adds the **mechanical, LoC-ceiling-clearing code-motion phase the 2026-06-11 survey
skipped** (it jumped straight to `[DESIGN]` traits). Land the mechanical split FIRST — it discharges the
finding at `[MECH]` risk and shrinks the blast radius of every later trait extraction (a 150-line file
is a far safer place to introduce `ProtocolHandler` than a 2350-line match). The two entries share the
same merge-conflict-zone discipline: serialize everything that touches `P2PNode`.

## Readiness notes

- **Escalation:** hard-ceiling breach (the fingerprinted tier), not a soft nudge — rank accordingly at
  refinement. `binding: observation` means it never *blocks*, so this is important governance debt, not
  a functional regression.
- **Ready now, sequencing gate open (2026-07-06):** Phase 1 is a self-contained, compiler-checked
  code move — no design decision blocks it — but do not dispatch it until the in-flight
  `apply_snapshot` receive-side-idempotency edit (Refactor-safety note 8) has committed on
  `elohim/elohim-storage/src/p2p/mod.rs`; re-check tree state at dispatch time, since this note may
  be stale by the time Phase 1 is picked up.
- **Area owners:** rust-architect (dataplane); the `reconcile/custody.rs` `LocalBlobStore`/`FetchKicker`
  trait pair is the proven in-crate IoC template cited by the sibling backlog for the later design phases.
- **Status gate:** author-canonicalized at `backlog`; awaits cartographer/operator promotion to `refined`
  before `/shift` picks it up (per timeline CONVENTIONS authority boundary).
