---
id: "backlog-dataplane-reanchor-dead-remaining-rekeyed-peer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Re-keyed dead anchors need a heal under the current key — dead_remaining survives a pod restart (mechanism corrected 2026-09-11: the rows are SETTLED-not-skipped, and nothing clears the dead verdict on a settled row)"
slug: "dataplane-reanchor-dead-remaining-rekeyed-peer"
written: "2026-09-06"
updated: "2026-09-11"
author: "story-harvest; mechanism corrected 2026-09-11 (runtime-triage, endpoint-confirmed)"
status: "backlog"
priority: "high"
self_heal_status: blocked
severity: medium
fingerprints: [2b4761b2eaf6]
nodes: [alpha-b]
relatedNodeIds: []
tags: [dataplane-convergence, reanchor, p1-reconciliation-controller, re-genesis, self-heal, provide-loop-dead-remaining-stuck, anchor-liveness, adopt-before-author]
cites:
  - https://elohim.host/p2p/status
  - elohim/elohim-storage/src/services/reanchor_backfill.rs
  - elohim/elohim-storage/src/db/content_diesel.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/elohim-storage/src/services/provide_loop_status.rs
  - genesis/data/timeline/backlog/reanchor-dead-remaining-stuck-vs-draining.md
  - .claude/scripts/runtime-harvest.py
---

> **2026-09-11 — runtime-triage mechanism correction (endpoint-confirmed).** The
> 2026-09-06 record below is accurate on SYMPTOM (adam, 9 rows, `deadRemainingStuck`
> surviving a restart) and its "Missing node" shape still stands. Its stated MECHANISM —
> "every candidate hits the reanchor sweep's skip-guards" — is **falsified by the live
> surface**: both skip counters read `0`. The rows are not skipped, they are **settled**
> every sweep by the adopt-before-author pre-flight, and nothing on the settled path ever
> clears the persisted `dht_anchor_state = 'dead'` verdict. See
> "## 2026-09-11 — triage of ledger fingerprint `2b4761b2eaf6`" at the end of this entry.

## Chain

`dataplane-convergence` / between "a content row's anchor is unresolvable under the current
conductor key" → "the row heals to a resolvable anchor (or a declared supersession)" /
**missing node: a controller-driven re-anchor under the CURRENT key for rows anchored by a
pre-re-genesis agent key whose chain nobody holds — today they just sit, stuck, forever.**

## Assertion + probe

Assertion: `dead_remaining` on a peer's reanchor sweep either (a) trends toward zero as rows
heal under the current key, or (b) is explicitly held with a declared supersession record —
never silently persists unchanged across a pod restart. Probe: `GET /p2p/status` on adam
before and after a pod restart — `reanchorDeadRemaining` and `deadRemainingStuck` must not
both survive unchanged (same nonzero count, still stuck) unless a supersession has been
filed for those specific rows.

## Current state — found live 2026-09-06

adam's provide loop shows `reanchorDeadRemaining: 9`, `deadRemainingStuck: true`. This
population **survived the #1432 pod restart**: `stuckSweeps` reset from 55 back down (fresh
counter after restart) then climbed straight back to 9 and re-asserted stuck, meaning the
restart did nothing to help these rows — they are anchored by a pre-re-genesis agent key
(from before the 2026-09-02 alpha re-genesis; see
`project_alpha_dna_migration_2026_09_02` memory) whose source chain no live peer holds, so
every candidate hits the reanchor sweep's skip-guards
(`elohim/elohim-storage/src/services/reanchor_backfill.rs` — the sweep's dead-vs-pending
split at the bottom of the function, `dead_remaining` recounted separately so "a
`dead_remaining` that never moves... is a seed-data correction waiting, not a heal in
progress" per the code's own comment). This is exactly that comment's predicted class,
observed live: a **re-keyed-peer class** of dead anchor that the current sweep logic
correctly detects as stuck but has no cure path for.

## Missing node (concrete shape)

Per P1 (`elohim-storage is a k8s-style controller that eagerly reconciles`), the cure is a
controller action, never a hand PATCH: either (a) a re-anchor pass that re-authors the row
under the CURRENT agent key when the old key's chain is confirmed unrecoverable, or (b) a
declared supersession record that lets `/p2p/status` report these rows as resolved-by-policy
rather than perpetually `deadRemainingStuck: true`. Either path needs a decision about what
"the current key" is authoritative to re-author on behalf of a dead pre-re-genesis identity
— an architect-level call, not a background mechanical fix, but the class itself (re-keyed
dead anchors from a re-genesis event) should have a named cure path rather than sitting as
an ever-present true/9 in `/p2p/status`.

## Note — already surfaced to the poller

The runtime-harvest poller (`.claude/scripts/runtime-harvest.py`, commit 9060c617d) now
files this class of exhaustion into the deterministic ledger
(`.claude/data/runtime-findings.jsonl`) rather than it going unnoticed; this backlog entry is
the canonical concern the poller's fingerprint should resolve to.

## Habit served

`dataplane-convergence` (`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`,
status: red, active: true) — invariant: "A peer that missed a deploy heals." A re-keyed dead
anchor is a peer that will never heal under the existing sweep logic; the habit's own guard
section already tracks re-genesis-era edge cases and this is a fresh, live instance.

## TODO (integrator)

`cites:` is a plain path list per this directory's existing convention (no fingerprint
invented). If cite-gen is later mandated for backlog rows, run it here rather than
hand-writing a fingerprint.

---

## 2026-09-11 — triage of ledger fingerprint `2b4761b2eaf6` (runtime-triage)

### What is exhausted

Ledger line (`.claude/data/runtime-findings.jsonl`, poll 67):

```json
{"fp": "2b4761b2eaf6", "class": "provide-loop-dead-remaining-stuck", "node": "alpha-b",
 "provenance": "p2p-status:provide-loop",
 "line": "provideLoop.deadRemainingStuck reanchorDeadRemaining=9 reanchorPending=9 stuckSweeps=168",
 "status": "open", "seen": 1, "first_poll": 67, "last_poll": 67,
 "clean_poll_streak": 0, "ts": "2026-09-11T13:14:04+00:00"}
```

Re-fetched live at 2026-09-11T13:17Z — `GET https://elohim.host/p2p/status` (storage peer
adam, B-side), condition still present:

```json
"provideLoop": {
  "selfCidSource": "derived-libp2p-peer-id",
  "active": true,
  "reanchorPending": 9,
  "reanchorCompleted": 1512,
  "reanchorFailed": 0,
  "reanchorCaughtUp": false,
  "reanchorDeadRemaining": 9,
  "stuckSweeps": 168,
  "deadRemainingStuck": true,
  "reanchorSkippedReach": 0,
  "reanchorSkippedContentType": 0
}
```

Surrounding surfaces are green, which rules the usual suspects out: `replication.caughtUp
true`, `pull.caughtUp true`, `projectionReconcile.caughtUp true` / `divergentAnchor 0` /
`healedTotal 27` / `sweeps 550`, `drain 3443/3443`, `connectedPeers 6`.
`GET /admin/self-healing` shows `admission.shedTotal 0`, the single upstream `circuit:
"closed"`, `projector.caughtUp true`; `GET /health` reports `conductor.connected true`,
`7/7` pools healthy. **The only red thing on this node is the dead-anchor arm.**

### Three facts the surface proves, and what they kill

1. **`reanchorSkippedReach: 0` and `reanchorSkippedContentType: 0`.** The 2026-09-06
   mechanism ("every candidate hits the skip-guards") is false, and so is the cure the
   `/p2p/status` field docs advertise ("*a seed-data correction is needed*",
   `provide_loop_status.rs` — the `dead_remaining_stuck` field doc). No seed-data
   correction can move these rows, because no row is being skipped for a non-canonical
   `reach` or `content_type`.
2. **`reanchorFailed: 0`, cumulative for the whole process lifetime.** The dead arm has
   never had a conductor re-author error. Despite adam's conductor being CPU-pegged for
   ~24h, these 9 rows are not failing at the conductor — **they never reach it.**
3. **`reanchorCompleted: 1512 = 168 × 9`, exactly.** `completed` is published as
   `reanchored + already_anchored + adopted + held`
   (`reanchor_backfill.rs:537`). Every single sweep counts all 9 dead rows as *settled*,
   with zero variance across 168 sweeps — and the post-loop recount
   (`reanchor_backfill.rs:511-520`) then finds the same 9 still `dead`.

Facts 2 and 3 together pin the outcome class. `Reanchored` and `AlreadyAnchored` both write
the anchor through `content_diesel::upsert_with_anchor`
(`project_existing_anchor` at `content_service.rs:573` mirrors the create/update
projection), whose REVIVE stamp at `content_diesel.rs:1286` sets `AnchorState::Live` — so
either outcome would have dropped the row out of `count_dead_anchor_content` **inside the
same sweep**, before the recount. A constant 9 for 168 sweeps therefore rules both out. The
9 rows end every sweep on the pre-flight's early-`continue` arms: `Adopted`, or
`Held | Contested`.

### Root-cause inventory

**The invariant that is broken: the persisted `dead` verdict has exactly one clear, and it
sits on a code path the settled rows never take.**

(Line numbers are as of `dev` @ 2026-09-11; `projection_reconcile.rs` has unrelated in-flight
WIP in the worktree, so re-anchor on the symbol names if they have drifted.)

| # | Site | Role in the wedge |
|---|---|---|
| 1 | `elohim/elohim-storage/src/db/content_diesel.rs:1286` (in `upsert_with_anchor_transaction`, fn at `:1112`) | The ONLY transition `dead → live`. Fires only when the conductor RETURNED an action hash. Its own comment names the stakes: "this is what retires a row from the DEAD-anchor heal arm… Without it a healed row would be re-selected every sweep forever." |
| 2 | `elohim/elohim-storage/src/services/reanchor_backfill.rs:378-392` | The dead arm's adopt-before-author pre-flight `continue`s on `AdoptOutcome::Adopted` (`:378`) and `Held \| Contested` (`:389`) **before** `update_via_conductor` — so site 1 is never reached, yet both are folded into `completed` at `:537`. |
| 3 | `elohim/elohim-storage/src/services/reanchor_backfill.rs:511-520` | The post-loop recount re-reads `count_dead_anchor_content`, which is still 9. `pending = remaining + dead_remaining` ⇒ `reanchorCaughtUp` can never go true. |
| 4 | `elohim/elohim-storage/src/db/content_diesel.rs:205` / `:230` | Selection and count are both `dht_anchor_hash IS NOT NULL AND dht_anchor_state = 'dead'` — a **persisted verdict with no expiry and no settle-clear**. |
| 5 | `elohim/elohim-storage/src/p2p/projection_reconcile.rs:2485` | The only WRITER of `dead`, in the ghost-witness sweep. Its own comment concedes the verdict can be stamped from a CACHED absence via `heal_backoff::should_replay` — so a row can be re-killed without a fresh conductor answer. |
| 6 | `elohim/elohim-storage/src/p2p/projection_reconcile.rs:2626-2636` | The ghost sweep — the second loop that could have healed these rows — has the SAME two `continue` arms (`adopted += 1` / `held += 1`) and the same omission. Both heal legs settle-without-reviving. |
| 7 | `elohim/elohim-storage/src/services/head_adoption.rs:1384-1408`, `:1429` | Why `Held` is sticky: a row already carrying a declaration backed by an election is settled (Hold), **and any transient DB-pool error also returns `Held`**. And `Absent`/`Unreachable` are collapsed at `:1429` — a conductor that will not answer cannot produce `Author`, so a saturated conductor SUSTAINS the hold rather than breaking it. |
| 8 | `elohim/elohim-storage/src/services/provide_loop_status.rs` (module doc + `dead_remaining_stuck` field doc) | The verdict logic is CORRECT (`is_dead_remaining_stuck`) but its attribution is wrong for this instance: it tells the reader the cure is a seed-data correction and points at two counters that read 0. The surface is honest about the stall and misleading about the cause. |

**On the conductor saturation in the dispatch context:** it is *not* the proximate cause
(fact 2 — zero conductor errors on this arm), but site 7 makes it a plausible *sustainer*:
the pre-flight buys its evidence with `LocalResolve::Probe` / `ElectionResolve::Probe`, and
on a pegged conductor an unanswered probe degrades to "not canonical / no election", which
keeps a declared row on the `Held` arm indefinitely. The CPU peg is tracked separately
(`project_alpha_conductor_spin_root_cause`; cure on fork branch
`fix/sys-validation-unfetchable-deps-backoff`, NOT in the pinned 0.7 conductor submodule
`25dd2d0be`) and should not be conflated with this wedge — fixing the peg alone would not
clear a stale `dead` verdict on a settled row.

### Fix path

Ordered cheapest-first. All four are in `elohim-storage`; none needs the re-genesis
key-authority decision the 2026-09-06 "Missing node" section defers.

- **F1 — make the distinction readable from the endpoint (smallest, unblocks the rest).**
  The sweep-complete `tracing::info!` at `reanchor_backfill.rs:554-566` already carries
  `adopted` and `held`; `/p2p/status` does not. Publish them as two additive
  `provideLoop` wire fields (exactly the shape the 2026-08-22 skip-counter split
  established — optional in `p2p-status-view.schema.json`, `#[ts(type = "number")]`,
  schema-contract case). Then "stuck because settled-by-declaration" vs "stuck because
  skip-guarded" is one HTTP read instead of a Loki query, and the poller's `line` can say
  which. **This also settles `Adopted` vs `Held` for the live 9, which no endpoint can
  distinguish today.**
- **F2 — an adoption IS a liveness observation; stamp it.** On `AdoptOutcome::Adopted` the
  row now obeys a canonical head declared through the OWN conductor, which is the same
  class of evidence site 1 revives on. Call `mark_anchor_state(… AnchorState::Live)` for
  the id — best placed inside the adoption write in `head_adoption` so both callers
  (`reanchor_backfill.rs:378`, `projection_reconcile.rs:2626`) inherit it rather than
  growing two copies.
- **F3 — `Held` must NOT be laundered to `live`** (it is not an observation of anything —
  see site 7's DB-error arm). Instead split the counter: publish the dead-arm rows whose
  last outcome was `Held`/`Contested` as a distinct `deadSettledByDeclaration` beside
  `reanchorDeadRemaining`. `caughtUp`'s tightening stays intact for genuinely unhealed
  rows, and the endpoint stops asserting "seed-data correction needed" about rows that are
  settled-by-declaration. This is the 2026-09-06 option (b) ("declared supersession")
  narrowed to the part that needs no key-authority ruling.
- **F4 — correct the two doc-comments** in `provide_loop_status.rs` that attribute the
  stuck verdict to the skip-guards, since the first live instance has both skip counters
  at 0. Cheap, and it stops the next reader re-deriving the falsified 2026-09-06 cure.

Gate on landing: `just gate elohim-storage` (pool slot + `RUSTFLAGS=--cfg
getrandom_backend="custom"`, `CARGO_BUILD_JOBS=1`), plus `pnpm -w run schema:codegen:ts`
and `cargo test export_bindings` for any additive wire field (F1/F3 touch six generated
`p2p-status-view.ts` distributions and `ProvideLoopStatus.ts`).

### Current decision — BLOCKED (2026-09-11)

Canonicalized, not fixed. Two blockers, both outside a background triage agent's authority:

1. **Worktree ownership.** Every one of F1–F4 edits `elohim/elohim-storage/src/**`, which
   another lane holds as in-flight WIP for this dispatch. The triage agent was explicitly
   scoped OFF those files, so the fix is written down here rather than applied.
2. **Runtime proof needs a binary roll.** Even once landed, the verdict is published by
   the running alpha binary; confirming the fix requires an operator-owned edge deploy to
   adam (`[build:edge]`) and a post-roll re-read of `/p2p/status`. Cluster ops and build
   dispatch are operator-owned.

The ledger line for `2b4761b2eaf6` is set `status: blocked` with `backlog:
dataplane-reanchor-dead-remaining-rekeyed-peer`, so the poller keeps suppressing dispatch
on this fingerprint while the condition stands. Re-check belongs to the stasis sweep.

**Do NOT hand-PATCH the 9 rows** (P1 — `elohim-storage` is a reconciliation controller;
the cure is a controller action). And do not relax the `pending` arithmetic: a node serving
rows under anchors its own conductor cannot resolve is genuinely not caught up. What is
wrong is that a SETTLED row keeps a stale death certificate, not that death is counted.

### Verification (what would close this)

1. F1 landed and rolled: `/p2p/status .provideLoop` carries the last sweep's `adopted` /
   `held` counts, and they name which arm holds adam's 9.
2. F2/F3 landed and rolled: `reanchorDeadRemaining` falls below 9 (adopted rows revive) or
   the residue is reported as `deadSettledByDeclaration` with `deadRemainingStuck` no
   longer standing for settled rows.
3. The poller stops re-observing `2b4761b2eaf6` for its closure window and DELETES the
   ledger line by disappearance — the closure evidence, not an assertion in this file.

Not verified as of 2026-09-11: the condition is live and unchanged (`stuckSweeps: 168`).

### Recurrence after a restart, 2026-09-12 — the wedge is durable, and now failing

`2b4761b2eaf6` re-filed as a NEW ledger line (`first_poll: 77`, `status` back to `open`)
because the poller's closure-by-disappearance worked exactly as designed across a pod
restart. Re-fetched live, `GET https://elohim.host/p2p/status .provideLoop`:

```json
{"active": true, "reanchorPending": 11, "reanchorCompleted": 27, "reanchorFailed": 6,
 "reanchorCaughtUp": false, "reanchorDeadRemaining": 9, "stuckSweeps": 3,
 "deadRemainingStuck": true, "reanchorSkippedReach": 0, "reanchorSkippedContentType": 0}
```

Three things changed, and each sharpens the record above:

1. **`stuckSweeps` 168 → 3, `reanchorCompleted` 1512 → 27.** The counters reset: adam
   restarted. Per the 2026-08-22 stuck-vs-draining design, the run counter resets and the
   node must **re-earn** the verdict over 3 sweeps — and it did, immediately. A restart is
   the broadest self-heal the node has, and it did not clear this. The wedge survives
   process lifetime, which is what a stale `dht_anchor_state='dead'` row in diesel would
   predict and a transient in-memory condition would not.

2. **`reanchorDeadRemaining` is still exactly 9.** Same nine rows, across a restart.

3. **`reanchorFailed` 0 → 6, `reanchorPending` 9 → 11.** New, and it does NOT fit the
   settled-not-skipped account alone: the 2026-09-11 triage recorded `reanchorFailed: 0`
   and `reanchorCompleted = 168 x 9` exactly, i.e. all nine rows counted settled every
   sweep with nothing erroring. Post-restart there are 2 additional pending rows and 6
   failures. Whether the failures are the 2 new rows retrying or the 9 old ones changing
   arm is **not distinguishable on the published surface** — which is the same
   observability gap F1 already proposes to close (per-arm `adopted`/`held` counts on
   `.provideLoop`). Noting it as evidence F1 is now load-bearing for diagnosis, not just
   for confirmation.

Still **blocked** on the same grounds: the fix path touches `elohim/elohim-storage/src`,
held as in-flight WIP by another lane, and runtime proof needs an operator-owned edge roll.
Ledger line restored to `status: blocked` with the pointer to this file.
