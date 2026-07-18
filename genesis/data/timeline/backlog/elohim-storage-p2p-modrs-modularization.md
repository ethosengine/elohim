---
id: "backlog-elohim-storage-p2p-modrs-modularization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Modularize elohim-storage src/p2p/mod.rs — 7450-line god-file over the 7000 hard LoC ceiling (finding 8b36709ba1e0): ordered pure-code-motion extractions of the inline impl P2PNode by concern"
slug: "elohim-storage-p2p-modrs-modularization"
written: "2026-07-18"
author: "rust-architect (architecture finding 8b36709ba1e0 — 2026-07-18 tree refresh)"
status: "open"
priority: "medium"
area: "elohim-storage/p2p-dataplane"
domain: "D-dataplane"
jobs: [elohim-edge]
finding: "8b36709ba1e0"
policy: "source-file-loc-ceiling@1"
relatedNodeIds:
  - "backlog-p2p-mod-loc-ceiling-decomposition"
  - "backlog-arch-dataplane-refactor-backlog"
  - "backlog-doorway-http-rs-modularization"
  - "feedback_swarm_composition_fresh_tree_build"
  - "feedback_signature_changes_grep_callers"
  - "project_container_cargo_environment_quirks"
cites:
  - elohim/elohim-storage/src/p2p/mod.rs
  - .claude/epr-meta/policies.yaml
  - genesis/data/timeline/backlog/p2p-mod-loc-ceiling-decomposition.md
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
tags: [architecture, refactor, p2p, loc-ceiling, god-file, mech, dataplane, tech-debt, mod-decomposition, code-health]
shift_objective: |
  Discharge finding 8b36709ba1e0 by draining elohim/elohim-storage/src/p2p/mod.rs
  (7450 lines, over the source-file-loc-ceiling@1 HARD ceiling of 7000) below the
  ceiling via pure code motion — zero behavior change, zero signature change, zero
  wire/protocol change. The one high-leverage move that clears the hard ceiling on its
  own: extract the swarm-behaviour-event dispatcher (`handle_behaviour_event`, ~1840 LoC,
  currently lines 3956–5795) plus the SwarmEvent handler (`handle_event`, ~151 LoC, lines
  3805–3955) into a new sibling `elohim/elohim-storage/src/p2p/node_swarm_events.rs` as a
  split inherent `impl P2PNode` block — child-module impl blocks see the parent module's
  private `P2PNode` fields automatically, so the only compiler-enforced change is bumping
  the two entry-point methods to `pub(super)` so mod.rs-resident `run()` can still call them
  (wrong visibility is a compile error, not a silent bug). That single extraction drops
  mod.rs from ~7450 to ~5460, under the 7000 hard ceiling, discharging the finding. Then
  continue the ordered, independently-landable extractions below toward the 3000 soft
  ceiling, ratcheting `loc-hard` in .claude/epr-meta/policies.yaml DOWN as the tail drains
  (never up). This is plain native Rust — NOT a zome, no DNA-hash move, no reinstall/partition
  trap — gated only by build + fmt + clippy + the crate lib test. Verify EVERY step on a
  CLEAN TREE from elohim/elohim-storage (this is the swarm-composition home; a DNA-worktree
  `just check` verifies a different workspace and misses field/variant references):
  `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build` + `cargo clippy -- -D warnings`
  + `cargo fmt --check` + the crate lib test + `cargo test export_bindings`, then sha256-diff
  the generated View TS to prove byte-identical. Serialize the steps — same file, same struct
  = merge-conflict zone; never run two steps (or the sibling dataplane items) concurrently,
  and confirm a clean tree with `git status`/`git diff` on mod.rs before dispatching each step.
---

# Modularize elohim-storage src/p2p/mod.rs — hard LoC-ceiling breach (finding 8b36709ba1e0)

`elohim/elohim-storage/src/p2p/mod.rs` is **7450 lines** as of 2026-07-18 — over the
`source-file-loc-ceiling@1` **hard ceiling of 7000** (soft 3000; `binding: observation`, so it
governs but never *blocks*; `dispatch-agent: rust-architect`). This is the escalated,
fingerprinted tier of the policy (finding `8b36709ba1e0`), not a soft edit-time nudge. The
mandate: canonicalize a modularization plan into the timeline backlog and drive it as bounded
work — never refactor mid-edit off the finding. This entry is that plan, re-scoped against the
live tree.

## Relationship to prior canonicalization (compose, don't fork)

A prior item — [[backlog-p2p-mod-loc-ceiling-decomposition]] (`p2p-mod-loc-ceiling-decomposition.md`,
2026-07-03, then 2026-07-06) — already canonicalized this SAME finding. Its plan is sound and its
extraction mechanic is load-bearing; this entry deliberately **carries it forward, does not
compete with it.** What changed since then, and why a refresh was warranted:

- **The file shrank 7859 → 7450** (~409 net lines out) from unrelated concurrent landings, but
  **Phase 1 never landed** — `handle_behaviour_event` is still inline (now lines 3956–5795), so
  the hard ceiling is still breached and the finding is still open.
- The prior item's **line anchors are stale** (it placed Region B at 1691–7745; the file is now
  7450). This refresh re-anchors every seam to 2026-07-18 line numbers and method names.
- The prior item's **live-edit hazard (its note 8** — the `apply_snapshot` receive-idempotency
  edit mid-flight in the Gossipsub/inventory arm) appears **settled** (no such uncommitted diff
  on the current tree), removing the Phase-1 sequencing gate — but re-verify at dispatch time.

**Reconciliation ask for the cartographer/operator:** these two entries are one finding with two
records. At refinement, collapse to one — keep whichever the operator prefers and retire the
other (this fresher re-scope, or the prior with its dataplane-backlog composition notes folded
in). Until then, **do not run both plans concurrently** — same file, same struct, same
merge-conflict zone. Captured here explicitly so the duplication is a decision, not a silent fork.

## What still lives inline in mod.rs (2026-07-18 seam map — anchor to method names, not lines)

There are already ~37 sibling modules under `src/p2p/`. mod.rs is now three regions; the middle
one (`impl P2PNode`, ~5520 LoC) is the whole problem.

**Region A — header + types/config/views (~lines 104–1702, ~1600 LoC).** Module doc, `pub mod`
decls + `pub use` re-exports, EPR-atom Kad constants (`kad_key_for_atom`), the `Pending*` map
type aliases + `FederationError` + `ReplicationGapQueue` + `RECOVERY_*_TOPIC` consts (~135–323),
`DeliveryPeer` (325–393), `P2PConfig`+`Default` (394–497), the `P2PNode` struct (~170 private
fields, 498–671), `CachedIdentifyInfo`/`PeerMetrics`/`ReconciliationMetrics`(+`Snapshot`)+
`now_unix_ms`/`median_rtt` (673–745), the **4 `#[ts(export)]` View types** `DrainStatusInfo`/
`P2PStatusInfo`/`PeerInfoView`/`PeerListView` (746–867), the `P2PCommand` enum (868–1058,
~190 LoC), `SyncPauseGuard`+`Drop` (1059–1072), `P2PHandle`+`impl` (1073–1685, ~613 LoC),
`extract_http_port` + `DRAIN_*` consts (1686–1702).

**Region B — `impl P2PNode` (~lines 1703–7222, ~5520 LoC, 49 methods).** The god-block. Clusters:
- *Swarm event dispatch (~1990 LoC — the single biggest chunk):* `handle_event` (SwarmEvent
  connection/listen/dial, 3805–3955, ~151) + `handle_behaviour_event` (3956–5795, **~1840**), one
  giant `match` over `ElohimStorageBehaviourEvent`, already cleanly grouped per plane.
- *Command dispatch (~665 LoC):* `handle_command` (3061–3580, ~520, 19 `P2PCommand` variants),
  `drain_publish_queue` (3581–3726, ~146).
- *Request/response handlers (~1030 LoC):* `handle_shard_request`+`verify_shard_locations`
  (5796–5856), `handle_sync_request` (5857–6040, ~184), `handle_sync_response` (6041–6244, ~204),
  `resolve_epr_head_locally`/`handle_epr_atom_request`/`handle_epr_atom_response`/`handle_epr_request`/
  `handle_epr_response` (6407–6739, ~330), `heal_content_row`+`heal_blob_bytes_if_absent`
  (6245–6406, ~162).
- *Broadcast producers (~440 LoC):* `broadcast_inventory_snapshot` (2622–2761), `broadcast_salvage_capacity`
  (2762–2837), `score_and_enqueue_snapshot` (2838–3048, ~211), `set_last_gossiped_inventory` (3049–3060).
- *Reconcile/replication cycles (~430 LoC):* `hydrate_replication_state` (3727–3804), `initiate_sync_round`
  (6766–6811), `run_replication_cycle` (6812–6860), `drain_gap_queue` (6861–6913), `run_acquisition_reconcile`
  (6914–6964), `run_provide_reconcile` (6965–7060), `drain_acquisition_queue` (7061–7117), `refresh_status`
  (7124–7222).
- *Lifecycle/builders/accessors (~910 LoC):* `new` (1705–1893, ~189), the `with_*`/`set_*` builders +
  `dedup*`/`agent_info_inbound_tx`/`peer_id`/`is_connected`/`connected_peers` (1894–2071),
  `reconciliation_metrics` (2072–2252, ~181), `start`+`run` (the ~305-line `select!` loop, 2253–2621),
  the thin accessors `shutdown_sender`/`sync_manager`/`handle`/`set_iroh_node_id` (6740–6765, 7118–7123).

**Region C — trailer (~lines 7223–7450, ~228 LoC).** `reach_gate_allows` (7223–7275) + its three
inline `#[cfg(test)]` modules (7276–7418), then the `impl gossip_dispatch::InventoryFetch for
P2PNode` hook (7423–7450).

## The extraction mechanic (load-bearing safety insight — carried forward)

**Split the inherent `impl P2PNode` across submodule files. Do NOT introduce traits, do NOT change
signatures.** Rust permits multiple inherent `impl P2PNode { … }` blocks in different modules of one
crate, and a **child** module of `p2p` (e.g. `p2p::node_swarm_events`) can read `P2PNode`'s private
fields and call its private methods automatically (privacy is "visible to the defining module and its
descendants"). So a method body moved into a child submodule still uses the same `self` receiver with
no churn. The only compiler-enforced adjustment: a method that **moves to a child module but is called
from mod.rs-resident code** must go private → `pub(super)`/`pub(crate)`. Wrong visibility is a
*compile error*, not a silent bug — a safe failure mode. Keep every Region-A type in-crate (mod.rs
root or a `p2p/types.rs`) so the 4 ts-rs View exports stay path-stable, and preserve the `pub use`
re-exports so `elohim_storage::p2p::{P2PNode, P2PConfig, P2PCommand, P2PHandle, …}` is byte-for-byte
unchanged. For the one giant `match` (`handle_behaviour_event`), the idiom is **extract-method-then-
move**: hollow each per-plane arm body into a `self.on_<plane>_event(...)` method (mechanical, in
mod.rs), then move those methods into the plane's existing sibling module — each plane independently
landable.

## Ordered, bounded, independently-landable extractions (serialize them)

Each step: move the cluster + any co-located `#[cfg(test)]` block to a sibling file, bump visibility
as the compiler demands, re-export if needed, verify green, commit. LoC deltas are approximate.

- **Step 0 — guardrails (no motion).** Baseline LoC, full crate lib test green, `clippy` clean,
  sha256 of the generated View TS. Re-check the tree is clean at mod.rs (prior item's note-8
  hazard appears settled; confirm).
- **Step 1 — swarm event dispatch → `p2p/node_swarm_events.rs`** (`handle_event` + `handle_behaviour_event`,
  ~1990 LoC out). **CLEARS THE HARD CEILING** (~7450 → ~5460). Highest leverage, lowest risk (two
  contiguous methods, one `pub(super)` bump each). Do this first.
- **Step 2 — reach gate → fold into the existing `p2p/reach_authorization.rs`** (`reach_gate_allows`
  + its three `#[cfg(test)]` modules, ~196 LoC out). Obvious home already exists — this is leaked
  reach-authorization logic; a pure function with self-contained tests, near-zero risk.
- **Step 3 — command dispatch → `p2p/node_commands.rs`** (`handle_command` + `drain_publish_queue`,
  ~665 LoC out).
- **Step 4 — request/response handlers → co-locate into the existing per-plane sibling modules**
  as `impl P2PNode` blocks (`shard_protocol.rs`, `sync_protocol.rs`, `epr_protocol.rs`,
  `epr_atom_protocol.rs`) + a small `p2p/node_heal.rs` for the two heal helpers (~1030 LoC out).
  Puts each plane's wire handling next to its codec.
- **Step 5 — broadcast producers → co-locate into `inventory_broadcaster.rs` / `salvage_gossip.rs`**
  (+ move the trailer `InventoryFetch` impl into `gossip_dispatch.rs`; ~470 LoC out).
- **Step 6 — reconcile/replication cycles → `p2p/node_reconcile.rs`** (~430 LoC out).
- **Step 7 — lifecycle/builders + status views → `p2p/node_lifecycle.rs` + `p2p/status_views.rs`;
  types → `p2p/config.rs` + `p2p/command.rs` + `p2p/handle.rs`** (P2PConfig, P2PCommand, P2PHandle,
  the metrics + 4 View types stay in-crate, ts-rs-safe). Ratchets toward the 3000 soft ceiling.

End state: mod.rs is a thin module root (doc header, `pub mod`, `pub use`, `struct P2PNode`) well
under the soft ceiling. Step 1 alone discharges the finding; Steps 2–7 each afternoon-sized.

## Refactor-safety notes (honoring the policy `why` + gospel)

1. **Plain native Rust, NOT a zome.** No DNA-hash move, no reinstall/partition trap, no DNA-lineage
   event. Cheap and reversible — gated by build + fmt + clippy + the crate lib test alone. This is
   exactly the asymmetry the policy `why` names: a first-party native file past the hard ceiling is
   pure governance/reviewability debt, not a correctness or migration risk.
2. **But it IS the swarm-composition home.** Verify with a CLEAN-TREE `cargo build` on elohim-storage
   (`RUSTFLAGS='--cfg getrandom_backend="custom"'`), never a DNA-worktree `just check` — the DNA
   worktree verifies a different workspace and misses `P2PNode` field/variant references. See
   [[feedback_swarm_composition_fresh_tree_build]]. Parallel sessions on a shared tree hide missing code.
3. **Zero signature change ⇒ zero caller churn**, but still `rg` each moved method name crate-wide
   (incl. `tests/`) to confirm nothing references it via a path — [[feedback_signature_changes_grep_callers]].
4. **ts-rs:** the 4 View types stay in-crate, so module moves are ts-rs-safe; still run
   `cargo test export_bindings` and sha256-diff the generated `.ts` to prove byte-identical.
5. **Behavior-identical, wire-untouched:** no codec/protocol edits ⇒ no protocol-version bump, no
   old↔new compat test owed.
6. **Cargo target-pool + serialize:** set `CARGO_TARGET_DIR` to this worktree's family slot; never
   run concurrent cargo on one slot; run the steps one at a time (same file = merge-conflict zone);
   `git status`/`git diff` mod.rs before dispatching each step and never `git stash`/`reset`/revert
   another session's ambient work to "clear the way." Container quirk: nextest may be absent — plain
   `cargo test` works, and never pipe a gate's output (a pipe masks cargo's exit code). See
   [[project_container_cargo_environment_quirks]].
7. **Public API preserved** ⇒ no gospel-tier surface migration owed. The CLAUDE.md / rust-architect
   "Key Files" tables reference `p2p/*.rs` generically with no line-anchors into mod.rs internals, so
   no honesty-matrix churn. If a later step changes the *public* module surface, note it in the commit.
8. **Ratchet the ceiling DOWN, never up.** After each step lands and mod.rs shrinks, lower `loc-hard`
   in `.claude/epr-meta/policies.yaml` toward `loc-soft: 3000` so the file can never silently re-inflate
   past its new floor. The policy's job is to trend the file down — raising the ceiling to fit a
   growing god-file is the failure this finding exists to prevent.

## Composition with the design-tier dataplane backlog

[[backlog-arch-dataplane-refactor-backlog]] ranks the *design-tier* decomposition of this file
(#10 ProtocolHandler trait to hollow `handle_behaviour_event` for Swarm-free tests, #12 CommandHandler
dispatch, #15 TimerArm `select!`-loop refactor). This entry is upstream and complementary: land the
**mechanical, LoC-ceiling-clearing code-motion** FIRST — a 150-line file is a far safer place to
introduce a `ProtocolHandler` trait than a 1840-line `match`. All three share the same merge-conflict-
zone discipline: serialize everything that touches `P2PNode`.

## Readiness notes

- **Escalation:** hard-ceiling breach (fingerprinted tier `8b36709ba1e0`), not a soft nudge — rank
  accordingly at refinement. `binding: observation` ⇒ important governance debt, never a functional
  regression.
- **Ready now:** Step 1 is a self-contained, compiler-checked code move — no design decision blocks it.
  Re-check tree state at dispatch (this note may be stale by pickup) and reconcile the duplicate
  prior item first.
- **Area owner:** rust-architect (dataplane).
- **Status gate:** author-canonicalized at `open`; awaits cartographer/operator promotion to `refined`
  (and the duplicate-reconciliation decision) before `/shift` picks it up, per timeline CONVENTIONS.
