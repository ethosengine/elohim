---
id: "backlog-dataplane-reanchor-dead-remaining-rekeyed-peer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Re-keyed dead anchors need a heal under the current key — dead_remaining survives a pod restart (mechanism corrected 2026-09-11: the rows are SETTLED-not-skipped, and nothing clears the dead verdict on a settled row)"
slug: "dataplane-reanchor-dead-remaining-rekeyed-peer"
written: "2026-09-06"
updated: "2026-10-19"
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
  - elohim/elohim-storage/src/services/reanchor_backoff.rs
  - genesis/data/timeline/backlog/alpha-adam-peer-meta-store-disk-io-error.md
  - genesis/data/timeline/backlog/reanchor-dead-remaining-stuck-vs-draining.md
  - .claude/scripts/runtime-harvest.py
  - https://elohim.host/admin/self-healing
  - https://elohim.host/db/p2p/conductor-diagnostics
  - https://doorway-alpha.elohim.host/p2p/status
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

## 2026-10-06 — recurrence at poll 189: F2 has landed, the population fell 9 → 1, and the residue is still unreadable

`2b4761b2eaf6` re-filed as a NEW ledger line (`first_poll: 189`) after the node restarted.
The ledger line:

```json
{"fp": "2b4761b2eaf6", "class": "provide-loop-dead-remaining-stuck", "node": "alpha-b",
 "provenance": "p2p-status:provide-loop",
 "line": "provideLoop.deadRemainingStuck reanchorDeadRemaining=1 reanchorPending=2 stuckSweeps=18",
 "status": "open", "first_poll": 189, "ts": "2026-10-06T19:42:25+00:00"}
```

Re-fetched live at 2026-10-06T19:43Z, `GET https://elohim.host/p2p/status .provideLoop`
(`/health` `uptime: 8160`, so the process started around 17:28Z):

```json
{"selfCidSource": "derived-libp2p-peer-id", "active": true, "reanchorPending": 2,
 "reanchorCompleted": 7, "reanchorFailed": 7, "reanchorCaughtUp": false,
 "reanchorDeadRemaining": 1, "stuckSweeps": 18, "deadRemainingStuck": true,
 "reanchorSkippedReach": 0, "reanchorSkippedContentType": 0}
```

`GET https://elohim.host/admin/self-healing` rules out the other mechanisms: admission
`shedTotal 0`, the single upstream `circuit: "closed"`, `errorStreak 0`. The projector reads
`caughtUp false` and `divergentAnchor 21`, but that is the separate projection-catchup concern
(`self-heal-adam-projection-catchup-exhaustion-full-arc.md`, fp `79f357281ca5`). It is not
this wedge.

### What changed since 2026-09-12

1. **F2 has landed.** Commit `421fa55ea` (2026-10-05, on `origin/dev`) stamps
   `dht_anchor_state = live` in the same transaction as an adoption. The function is
   `content_diesel::stamp_own_conductor_canonical_head`, called from
   `elohim/elohim-storage/src/services/head_adoption.rs` (the stamp sits beside the
   "liveness observation" comment, around line 2201, and is tested around lines 4354-4413).
   That closes root-cause sites 2 and 6 for the `Adopted` arm: an adopted row now leaves the
   dead population. The fall from 9 to 1 fits that change reaching adam. `/health` does not
   publish a git SHA, so the surface cannot prove which build is running.
2. **Held candidates now back off.** Commit `aa55b2e71`
   (`services::reanchor_backoff::note_held`, `reanchor_backfill.rs:436-445`) stops a
   `Held`/`Contested` row from being probed again every sweep. As a result,
   `reanchorCompleted` no longer equals `stuckSweeps × deadRemaining`, so the 2026-09-11
   arithmetic proof (1512 = 168 × 9) cannot be repeated on this surface.
3. **One row remains, and its arm cannot be named.** `reanchorFailed: 7` in 18 sweeps with
   both skip counters at 0 means the residue is either a `Held`/`Contested` row (site 7, and
   F3 has not landed) or a row whose re-author keeps failing. The endpoint cannot tell these
   apart. **F1 is the missing node.** It has not landed: `ProvideLoopStatus` still carries no
   per-arm `adopted`/`held` counts.
4. **F4 has not landed, and it is not a comment-only edit.** The misleading text ("a seed-data
   correction is needed … `reanchorSkippedReach` / `reanchorSkippedContentType` name the
   likely reason") is the doc of a field on a `#[derive(TS)] #[ts(export)]` struct
   (`provide_loop_status.rs:173-215`). It is emitted into
   `elohim/sdk/storage-client-ts/src/generated/ProvideLoopStatus.ts`, into
   `p2p-status-view.schema.json`, and into four generated `p2p-status-view.ts` copies
   (elohim-app, elohim-identity, elohim-service, lamad). It ships with F1/F3 as one
   codegen-bearing change.

### Current decision — still BLOCKED (2026-10-06)

The 2026-09-11 worktree-ownership blocker is gone: `elohim/elohim-storage/src` is clean on
`fix/coordinator-acceptance-contract`. Two blockers remain:

1. **F1/F3/F4 are one wire-shape change, which needs a sprint lane rather than a background
   agent.** The change adds `provideLoop` fields (per-arm `adopted`/`held`, and
   `deadSettledByDeclaration`) and corrects the field docs. It touches the schema, the Rust
   struct, schema-contract cases, `cargo test export_bindings`, `pnpm run schema:codegen:ts`,
   and the generated copies (about 10 files). It needs the full `just gate elohim-storage`
   lane (`CARGO_BUILD_JOBS=1`). F3 also carries a semantic choice: whether a row settled by
   declaration counts toward `caughtUp`.
2. **Runtime proof needs an edge roll to adam,** which the operator owns. The roll should also
   expose the build SHA so the next triage can tell whether a fix is deployed.

Ledger line `2b4761b2eaf6` is set `status: blocked`, `backlog:
dataplane-reanchor-dead-remaining-rekeyed-peer`. The plan sketch for the sprint is F1 + F3 + F4
together, in `elohim-storage`, with one schema bump. Then roll, re-read `.provideLoop`, and
confirm one of two outcomes: the residue row is named and settled-by-declaration (not
stuck), or `reanchorDeadRemaining` reaches 0.

Not verified as of 2026-10-06T19:43Z: the condition is live (`stuckSweeps: 18`,
`reanchorDeadRemaining: 1`).

## 2026-10-07 — recurrence at poll 246: the residue arm is now named (Held), and the population is growing again

`2b4761b2eaf6` re-filed as a NEW ledger line (`first_poll: 246`, 2026-10-07T12:37Z) after
adam restarted on the `e68c909` edge roll. The finding line:
`provideLoop.deadRemainingStuck reanchorDeadRemaining=5 reanchorPending=5 stuckSweeps=3`.

Re-fetched live at 2026-10-07T12:41Z, `GET https://elohim.host/p2p/status .provideLoop`
(three reads between 12:41:32Z and 12:42:29Z, identical):

```json
{"selfCidSource": "derived-libp2p-peer-id", "active": true, "reanchorPending": 5,
 "reanchorCompleted": 12, "reanchorFailed": 4, "reanchorCaughtUp": false,
 "reanchorDeadRemaining": 5, "stuckSweeps": 4, "deadRemainingStuck": true,
 "reanchorSkippedReach": 0, "reanchorSkippedContentType": 0}
```

Same read: `projectionReconcile` `divergentAnchor 60`, `converged false`, `failed 1`,
`sweeps 49`. `GET /admin/self-healing`: admission `shedTotal 0`, the single upstream
`circuit: "closed"`, `errorStreak 0`, `conductor-6` `Degraded`. The A-side
(`https://doorway-alpha.elohim.host/p2p/status`) reads `reanchorDeadRemaining 0`,
`deadRemainingStuck false` at the same time: this is still adam only.

### Which build is running (the 2026-10-06 gap, closed for this roll)

The storage process's own startup line, read from Loki
(`{namespace="elohim-alpha", pod="elohim-adam-alpha-0", container="elohim-node"}`):

```
2026-10-07T08:33:12.226634Z INFO elohim-storage starting version=0.1.0 commit=e68c909 build_time=2026-10-07T08:11:03Z
```

`421fa55ea` (F2, adoption stamps `live`) and `aa55b2e71` (held backoff) are both ancestors of
`e68c909f5` (`git merge-base --is-ancestor`). **F2 is deployed on adam and the wedge still
re-earns its verdict.** The 9 → 1 fall recorded on 2026-10-06 was F2; what remains is not the
`Adopted` arm.

### The arm is Held, every sweep, for every row

`reanchor_backfill: sweep complete` lines from the same Loki stream (13 in the 3h window from
10:00Z; 9 read, listed here; `reanchored`, `already_anchored`, `adopted`, `failed`, `remaining`
and both skip counters are 0 in all nine):

| time (Z) | held | held_backoff | dead_candidates | dead_remaining |
|---|---|---|---|---|
| 10:00:33 | 0 | 2 | 2 | 2 |
| 10:06:18 | 1 | 1 | 2 | 2 |
| 10:10:00 | 1 | 1 | 2 | 2 |
| 10:32:16 | 1 | 1 | 2 | 2 |
| 10:35:44 | 0 | 2 | 2 | 2 |
| 11:06:03 | 3 | 2 | 5 | 5 |
| 11:12:37 | 1 | 4 | 5 | 5 |
| 11:14:50 | 0 | 5 | 5 | 5 |
| 12:40:15 | 2 | 3 | 5 | 5 |

In every sweep read, `held + held_backoff == dead_candidates`. No row was adopted, authored or
failed. That answers the question the 2026-10-06 entry left open ("its arm cannot be named"):
the residue sits on the `Held | Contested` arm of the adopt-before-author pre-flight
(`elohim/elohim-storage/src/services/reanchor_backfill.rs:436`), or on the replay of that
verdict (`reanchor_backoff::should_skip`, `:354`). Root-cause sites 2, 4 and 7 of the
2026-09-11 inventory stand unchanged for this arm; F3 is the fix that addresses it and it has
not landed (no `deadSettledByDeclaration` in `elohim/elohim-storage/src` or the view schema).

What the logs do NOT say is WHY the pre-flight holds — a row settled by a declaration backed
by an election, or the `Held` that `head_adoption` returns on an unanswered conductor probe or
a DB-pool error (site 7). No `Held`-reason line at WARN was found in the window.

### The population is growing: the ghost sweep stamped three more rows dead after the roll

```
2026-10-07T10:35:44.452409Z WARN projection-reconcile[ghost-witness]: rows anchored to actions this conductor cannot resolve — marked dead … marked=2
2026-10-07T10:57:11.578264Z WARN projection-reconcile[ghost-witness]: rows anchored to actions this conductor cannot resolve — marked dead … marked=1
```

(`elohim/elohim-storage/src/p2p/projection_reconcile.rs:3046`, the only writer of `dead`.)
`dead_candidates` went 2 → 5 at the next sweep. So this is no longer only a fixed set of
pre-re-genesis rows waiting for a cure: on the current build the writer is adding rows, and
every added row lands on the same Held arm. The same stream carries a recurring WARN,
`projection-reconcile: conductor get failed; retry next sweep` with `error: "Request timeout:
heal conductor call exceeded per-attempt timeout 25s"` (seen on `custody-blob-*` ids; not
counted). A conductor that times out is the condition under which site 5 can stamp `dead`
from a cached absence and site 7 degrades a probe to `Held`. That link is a hypothesis here,
not a finding: the logs read do not tie the three marked ids to a timed-out call.

Two smaller observations from the same read:

- Between 11:14:50Z and 12:40:15Z no `sweep complete` line was found, while sweep START lines
  (`re-authoring NULL-anchor and DEAD-anchor content via conductor`, `dead_candidates: 5`)
  appear every 3–6 minutes from 12:06Z. Sweeps that start and never report completion fit
  the `witness_sweep_budget` timeout dropping the `run_once` future
  (`projection_reconcile.rs:2856-2893`). The budget-exceeded WARN itself was not queried.
  If so, `stuckSweeps` undercounts stalled time: it only advances on a completed sweep.
- `/p2p/status` reports `reanchorFailed: 4` while every sweep line read shows `failed: 0`.
  Four of the 13 sweeps in the window and everything between 08:33Z and 10:00Z were not read,
  so the four failures are unlocated, not contradicted.

### Current decision — still BLOCKED (2026-10-07)

Unchanged in kind, sharper in scope:

1. **F3 (+ F1, F4) is the fix for the arm that is actually holding the rows**, and it is one
   wire-shape change across the schema, the Rust struct, the schema-contract cases, both
   codegens and the generated copies. It carries a semantic choice (whether a row settled by
   declaration counts toward `caughtUp`) and needs the full `just gate elohim-storage` lane.
   Not applied by this triage: the dispatch barred heavy cargo and commits, so an edit could
   not have been gated.
2. **A new question precedes F3: why does `Held` fire.** If it is site 7's unanswered-probe
   degradation, F3 would relabel rows as settled-by-declaration that are in fact unprobed.
   F1 should therefore publish the hold REASON (declared-with-election vs probe-unanswered
   vs DB error), not just a per-arm count. Until that is readable, F3's counter must not
   absorb rows whose hold came from a non-answer.
3. **Growth needs its own read.** Three rows marked dead in the first 2.5h of this process
   is a rate, not a residue. Whether those verdicts rode a fresh conductor ABSENT or a
   cached one (`heal_backoff::should_replay`) decides whether the writer or the conductor
   is the thing to fix. The conductor-timeout side belongs with
   `self-heal-adam-projection-catchup-exhaustion-full-arc.md`.
4. **Runtime proof needs an edge roll to adam**, operator-owned.

Ledger line `2b4761b2eaf6` (`first_poll: 246`) is set `status: blocked`, `backlog:
dataplane-reanchor-dead-remaining-rekeyed-peer`.

Not verified as of 2026-10-07T12:42Z: the condition is live (`stuckSweeps: 4`,
`reanchorDeadRemaining: 5`). The Loki lines above were read through a delegated query, not
re-read line by line by the triage author.

## 2026-10-07 — reproduced on the household after a recast (local repro, no fleet needed)

After a `MESH_RESET=1` start (operator-granted, to shed 108 accumulated hosted agents) and the
Act I prologue, matthew's storage row for `elohim-host-landing` read `dhtAnchorState: dead` with a
`dhtAnchorHash` that no record on the new network backs; `reanchor_backfill` swept
`held_backoff: 65, remaining: 13, reanchored: 0` every pass; and the prologue's stage leg failed
its `blobHash` PATCH with `update_content: canonical root history unavailable — PENDING` for 50+
minutes while the conductor was otherwise caught up (`conductor_missing: 0`, `caught_up: true`).
That is this concern's Held arm (`AdoptOutcome::Held` never clears `dead`), seen on seeded content
whose carried anchor points at a network that no longer exists — exactly what a fresh household
is. So the household is a repro for F1/F3/F4: run the prologue on a recast household and watch
`reanchor_backfill: sweep complete … held_backoff` stay flat. Consequence that day: the pre-push
serving receipt (`epr-app-deliverability`) could not be minted on the tip, and the push went out
with the hook bypassed under the operator's grant, receipt owed. Logs:
`genesis/local-dev/testdrive-20261007/receipt-lane-{3,4}.log`, `household-dowell/logs/matthew.log`.

## 2026-10-07 — FIXED on dev (683fb16a7), proven on the household

Operator ruling the same evening: fix it, don't record it. Landed in one commit:

- **F5 (the cure):** `head_adoption` distinguishes ABSENT from UNREACHABLE. A declared row's
  backing records are read locally and, on a local miss, once with the network strategy; only
  when a responsive conductor answers Absent for every backing hash is the declaration
  UNBACKED on this network, and the row is re-decided as undeclared (AdoptPeer or Author).
  Timeouts and DB-pool errors stay Held with backoff. A peer echoing the same unbacked head is
  not fresh evidence.
- **F3:** Held is never laundered to live; the residue behind `reanchorCaughtUp` and the stuck
  detector excludes rows settled by a backed declaration and rows awaiting their channel.
- **F1:** per-arm counts on the wire — `reanchorAdopted`, `reanchorHeld`, `reanchorHeldBackoff`,
  `reanchorHeldUnbacked`, `reanchorHeldUnanswered`, `deadSettledByDeclaration`,
  `reanchorAwaitingChannel` (optional, schema contract case, TS regenerated).
- **Site 5:** the ghost-witness sweep stamps `dead` only from a fresh conductor answer.
- The stale-anchor heal in `update_via_conductor` matches the zome's current
  "canonical root history unavailable" answer.
- **F4:** the docs name settled-by-declaration instead of the skip-guards.

Household proof (`receipt-lane-5.log`, polluted recast): first sweep `held_unbacked: 25,
reanchored: 25`, `reanchorDeadRemaining` 52 → 27; every wedged stage leg landed first try;
after the deliverability story `deadSettledByDeclaration: 27, reanchorHeldUnanswered: 0,
reanchorCaughtUp: true, deadRemainingStuck: false`. Clean recast (`receipt-lane-6.log`):
`PROLOGUE_EXIT=0`, loop reads 0 dead / caught up / not stuck, `epr-app-deliverability` 5/5
scenarios, receipt `sprint-report-household-20261007T232705Z-683fb16a`. The alpha-b fingerprint
`2b4761b2eaf6` should disappear from the poller once the edge roll carries 683fb16a7; until the
roll, the ledger row stays `blocked` on evidence, not intention.

## 2026-10-08 — ON THE FLEET (edge #1578, storage `1.0.0-dev-b29b7248`)

All seven alpha storage pods rolled 10:00Z to 10:40Z. Read at 12:10Z from `/p2p/status .provideLoop`:

| doorway | deadRemainingStuck | stuckSweeps | reanchorDeadRemaining | reanchorHeld (unbacked / unanswered) | reanchorFailed | reanchorPending | reanchorCaughtUp |
|---|---|---|---|---|---|---|---|
| doorway-alpha (matthew, ethosengine) | false | 0 | 0 | 0 (0 / 0) | 0 | 0 | true |
| elohim.host (adam, shem) | false | 1 | 9 | 9 (0 / 9) | 339 | 94 | false |

The A side is the clean read of this fix: no dead rows, no held rows, caught up. The B side is
downstream of the disk incident, not of this seam: every one of adam's 9 held rows is
`reanchorHeldUnanswered` — a conductor probe that timed out — and adam's conductor is the one
logging SQLite 778 on a full ZFS dataset (`alpha-adam-peer-meta-store-disk-io-error`, DELTA
2026-10-08). Held is not laundered to live, which is F3 holding on the fleet. Re-read elohim.host
after the operator sets `refquota` on adam's dataset and recycles the conductor; until then its
numbers are the disk's, and the ledger row `2b4761b2eaf6` clears by the poller's own clean streak.

## 2026-10-09 — recurrence at poll 195: 16 rows, every one an unanswered conductor probe (the disk, not the seam)

`2b4761b2eaf6` re-filed as a NEW ledger line (`first_poll: 195`, 2026-10-09T03:51Z):
`provideLoop.deadRemainingStuck reanchorDeadRemaining=16 reanchorPending=99 stuckSweeps=7`.

Re-fetched live at 2026-10-09T03:52Z, `GET https://elohim.host/p2p/status .provideLoop`
(`/health` `uptime: 42230`, `pools_healthy 6/7`):

```json
{"active": true, "reanchorPending": 99, "reanchorCompleted": 83, "reanchorFailed": 833,
 "reanchorCaughtUp": false, "reanchorDeadRemaining": 16, "stuckSweeps": 7,
 "deadRemainingStuck": true, "reanchorSkippedReach": 0, "reanchorSkippedContentType": 0,
 "reanchorAdopted": 0, "reanchorHeld": 0, "reanchorHeldBackoff": 16,
 "reanchorHeldUnbacked": 0, "reanchorHeldUnanswered": 16, "deadSettledByDeclaration": 0,
 "reanchorAwaitingChannel": 0}
```

Same instant, A side (`https://doorway-alpha.elohim.host/p2p/status`): `reanchorDeadRemaining 0`,
`reanchorCaughtUp true`, `deadRemainingStuck false`. `GET https://elohim.host/admin/self-healing`:
admission `shedTotal 0`, the single upstream `circuit: "closed"`, `conductor-4` `Degraded`.

### Reading

The F1 counters do their job: the arm is named on one HTTP read. **16 of 16 dead rows are
`reanchorHeldUnanswered`** — the adopt-before-author probe timed out against adam's conductor.
`reanchorHeldUnbacked 0` and `deadSettledByDeclaration 0` mean the F5 cure had nothing it could
decide: no conductor answered Absent or Present. F3 is holding as designed (an unanswered row is
not laundered to `live`, and correctly still counts as stuck). `reanchorFailed 833` sits on the
never-authored arm (`reanchorPending 99 = 83 never-authored + 16 dead`) — the same conductor
refusing work.

The cause is the operator-side disk incident
(`genesis/data/timeline/backlog/alpha-adam-peer-meta-store-disk-io-error.md`): per the dispatch
context, adam's and eve's conductor datasets are at zero free and crash-looping, and gertrude's
conductor slot is dead. The growth from 9 (2026-10-08) to 16 fits more rows being stamped while
probes go unanswered; no reanchor-seam code change is indicated.

### Change made by this triage

The poller's finding line (`.claude/scripts/runtime-harvest.py`, `_provide_loop_stuck_finding`)
now carries `heldUnanswered=`, `heldUnbacked=` and `settledByDeclaration=`. The fingerprint is
node + class + provenance, so it is unchanged; the next recurrence names its arm in the ledger
line itself, and a line reading `heldUnanswered == reanchorDeadRemaining` routes straight to the
conductor/disk concern.

### Current decision — BLOCKED (2026-10-09) on operator disk action

Cure: the operator restores headroom on adam's and eve's conductor datasets (refquota and snapshot
pruning; see the disk-incident DELTAs), recycles those conductors, and restores gertrude's slot.
No kubectl from this lane. Ledger line `2b4761b2eaf6` (`first_poll: 195`) set `status: blocked`,
`backlog: dataplane-reanchor-dead-remaining-rekeyed-peer`.

### Verification (what would close this)

After the disk action, `elohim.host/p2p/status .provideLoop` reads `reanchorHeldUnanswered 0`, and
the 16 rows either revive or move to `reanchorHeldUnbacked` (re-decided) or
`deadSettledByDeclaration`, with `deadRemainingStuck false`. The poller then deletes the ledger
line after its clean streak. Not verified as of 2026-10-09T03:52Z: the condition is live.

## 2026-10-09 — re-filed at poll 258: the disk is cured, the conductor is back, and the verdict the poller relayed was an hour old

The ledger row closed by the poller's clean streak after the 2026-10-08 read and was re-filed
fresh (`seen 1, first_poll 258`, 11:54Z). Same concern, same file; nothing forked.

### What is exhausted

Ledger line: `provideLoop.deadRemainingStuck reanchorDeadRemaining=16 reanchorPending=101 stuckSweeps=4`
(node `alpha-b`, provenance `p2p-status:provide-loop`).

`https://elohim.host/p2p/status .provideLoop`, re-fetched 12:44Z, 12:45Z, 12:46Z and 12:48Z, the
same every time:

```json
{"reanchorPending":101,"reanchorCompleted":32,"reanchorFailed":336,"reanchorCaughtUp":false,
 "reanchorDeadRemaining":16,"stuckSweeps":4,"deadRemainingStuck":true,
 "reanchorHeld":0,"reanchorHeldBackoff":16,"reanchorHeldUnbacked":0,"reanchorHeldUnanswered":16,
 "deadSettledByDeclaration":0,"reanchorAwaitingChannel":0}
```

`https://doorway-alpha.elohim.host/p2p/status .provideLoop` (matthew) at 12:44Z: `reanchorDeadRemaining 0,
stuckSweeps 0, deadRemainingStuck false, reanchorCaughtUp true`. The A side is still clean.

### What changed since 2026-10-08 (Prometheus + Loki, read-only, 12:46Z)

- **The disk blocker is cleared.** `kubelet_volume_stats_available_bytes` for
  `holochain-data-elohim-adam-alpha-0` read 0 at every 15-minute sample through 10:45Z and
  14,679,408,640 at 11:00Z; capacity 20 GiB, used 6.7 GB. Eve, gertrude and susan stepped to
  about 14.8 GB available in the same hour. Adam's conductor: 0 lines of `code: 778`, 0 of
  `database is locked`, 12,026 lines in all from 11:06Z to 12:46Z.
- **Adam's conductor is a new pod, started 11:01:10Z, 0 restarts** (the pod before it had 10).
  `/db/p2p/conductor-diagnostics` on elohim.host: 5 running cells, membership authoritative
  (529 reads, 4 failed), 213 agents, every role `zomePath: live`. Its dominant log lines are
  kitsune2 gossip-round and iroh stream timeouts: a conductor three days behind, catching up.
  Eve's conductor is still restarting (7 since 11:07Z); doorway B reads `conductor-6` and
  `conductor-4` as reconnecting.
- **Adam's storage pod was not rolled.** It started 08:56Z, so the counters above belong to one
  process across both the hours with no conductor and the catch-up.
- **Storage's heal calls time out against the catching-up conductor:** 93 timeout lines in 100
  minutes, every sampled one `Request timeout: heal conductor call exceeded per-attempt timeout
  25s` from `projection_reconcile`.

### The defect this read found: a sweep that runs out of budget freezes the surface

Loki, `elohim-adam-alpha-0`: the five most recent `reanchor_backfill` lines are all the
start-of-sweep line (12:20:31Z, 12:28:49Z, 12:36:05Z, 12:40:23Z, 12:43:45Z, each
`candidates: 62, dead_candidates: 19`) and none is `sweep complete`. In the same 26 minutes the
published snapshot did not move, and it names 16 dead rows where the sweep is working 19.

The sweeps are running and not finishing, and the surface had no way to say so:

- `elohim/elohim-storage/src/p2p/projection_reconcile.rs:181` — `WITNESS_SWEEP_BUDGET` is 120 s.
- `projection_reconcile.rs`, `witness_bootstrap` — `tokio::time::timeout(pacing.witness_sweep_budget, run_once(..))`;
  on elapse the `run_once` future is dropped.
- `elohim/elohim-storage/src/services/reanchor_backfill.rs`, `run_once` — the only
  `state.publish_reanchor_sweep(..)` on a non-empty sweep sits after the candidate loop and the
  recount. A dropped future never reaches it; neither does a recount error.
- `elohim/elohim-storage/src/services/provide_loop_status.rs`, `publish_reanchor_sweep` — the
  stuck detector advances only there.
- `.claude/scripts/runtime-harvest.py`, `_provide_loop_stuck_finding` — relays the latest
  sample's `deadRemainingStuck` with no way to know how old the sample is.

A conductor too slow to finish a sweep inside 120 s therefore pins `/p2p/status .provideLoop` at
the last sweep that did finish. The `stuckSweeps: 4, deadRemainingStuck: true` filed at 11:54Z
was earned earlier in this process's life and has not been measured again since. It is the last
thing the loop managed to say, not a current reading.

Whether each of those five sweeps ended on the 120 s budget or on an error was not read (the
`sweep exceeded wall-clock budget` line was not counted); both arms skip the publish, and the
fix covers both.

Read beside it and not changed: `reanchor_backoff.rs` holds an unanswered row for
`REANCHOR_HELD_BACKOFF_SECONDS` = 900 s ("3 sweeps at the 300s cadence") and
`DEAD_REMAINING_STUCK_SWEEPS` is 3, so one unanswered probe per row earns the verdict with no
second attempt. The module says a skip leaves `deadRemainingStuck` untouched on purpose and
`an_unsettled_residue_beside_settled_rows_still_reads_stuck` pins it. Reading rule:
`reanchorHeldBackoff` = `reanchorHeldUnanswered` = the residue means "asked once, nobody
answered, waiting out the window".

### Fix path

Landed on dev with this entry:

- `ProvideLoopStatus.reanchorSweepsAbandoned` (wire, additive): consecutive sweeps that ended
  without publishing since the last published one. `ProvideLoopState::note_reanchor_sweep_abandoned`
  increments it and moves nothing else. No counts arrived, so the verdict is neither advanced
  nor cleared. `publish_reanchor_sweep` zeroes it.
- `witness_bootstrap` calls it on both no-publish arms (budget elapsed, sweep error); the boot
  pass in `main.rs` calls it on its error arm.
- `elohim/sdk/schemas/v1/views/p2p-status-view.schema.json`, the schema contract test and the
  generated TS carry the field.
- The poller appends `sweepsAbandoned=N (snapshot stale)` to the finding line when the node
  reports it. The fingerprint keys on node, class and provenance, so a standing row is not re-filed.

Not built: clearing or re-deriving the verdict from a sweep that published nothing. That is a
decision about what a wedge is when nobody can be asked.

### Current decision — fix applied on dev; the node condition is catch-up (2026-10-09)

The blocker named on 2026-10-08 (the operator's `refquota` move on shem) is done and is no
longer what this row waits on. Adam's conductor has been up since 11:01Z and is catching up
three days of gossip; until its calls answer inside 25 s the sweep keeps running out of budget
and the published snapshot stays where it is. No operator action is owed for this fingerprint.

Ledger `2b4761b2eaf6`: `triaged`. On re-encounter the poller cites this section. Once the edge
roll carries the field, the surface tells two outcomes apart:

- a sweep finishes and the residue has moved: `stuckSweeps` restarts and the row closes by
  disappearance;
- a sweep finishes, `reanchorSweepsAbandoned` reads 0, and the same rows are still
  `reanchorHeldUnanswered`: that is a current stuck verdict against a conductor answering other
  calls, and it is a question for `head_adoption`'s probe.

### Verification

Not verified on the fleet. The field reaches adam with the next edge roll, and the exhaustion
itself is still live as of 12:48Z. Local evidence: the unit test
`an_abandoned_sweep_ages_the_snapshot_and_moves_nothing_else`, the schema contract case, and the
`elohim-storage` gate. Fleet read owed after the roll: `elohim.host/p2p/status
.provideLoop.reanchorSweepsAbandoned` present, and either 0 beside a moving residue or non-zero
beside a frozen one.

## 2026-10-19 — re-filed at poll 213: the snapshot is current, the conductor answers, and three rows stay unhealed

`2b4761b2eaf6` re-filed as a NEW ledger line (`seen 1, first_poll 213`, 2026-10-19T02:28Z). Same
concern, same file; nothing forked.

### What is exhausted

Ledger line (node `alpha-b`, provenance `p2p-status:provide-loop`):
`provideLoop.deadRemainingStuck reanchorDeadRemaining=5 reanchorPending=11 stuckSweeps=4 heldUnanswered=4 heldUnbacked=0 settledByDeclaration=0`.
The line carries no `sweepsAbandoned=` suffix, so the verdict was fresh when the poller read it.

`https://elohim.host/p2p/status .provideLoop`, polled once a minute from 03:23Z to 03:38Z (16
reads). The first read, and the read after the verdict was earned again:

```json
03:23Z {"reanchorPending":10,"reanchorCompleted":9,"reanchorFailed":0,"reanchorDeadRemaining":5,
        "stuckSweeps":2,"deadRemainingStuck":false,"reanchorHeldBackoff":4,
        "reanchorHeldUnanswered":2,"deadSettledByDeclaration":2,"reanchorHeldUnbacked":0,
        "reanchorAdopted":0,"reanchorSweepsAbandoned":0,"selfCidSource":"derived-libp2p-peer-id"}
03:27Z {... "reanchorCompleted":10,"reanchorDeadRemaining":5,"stuckSweeps":4,"deadRemainingStuck":true,
        "reanchorHeldBackoff":4,"reanchorHeldUnanswered":2,"deadSettledByDeclaration":2,
        "reanchorSweepsAbandoned":0}
```

Sweeps published at about 03:25Z (`stuckSweeps` 3) and 03:27Z (4). Nothing published from 03:27Z
to 03:35Z. At 03:35Z `reanchorSweepsAbandoned` went to 1, meaning one sweep ran out of its 120 s
budget. The verdict came back with `reanchorSweepsAbandoned 0`, so it is a current reading, not
an old one.

At the same time:
- `https://elohim.host/health`: `uptime 29856`, `pools_healthy 7/7`, `connected_workers 4/4`,
  `p2p.caughtUp true`, `divergentAnchor 151`.
- `https://elohim.host/admin/self-healing`: admission `shedTotal 0`, the one upstream (adam
  storage) `circuit: "closed"`, `errorStreak 0`. All seven `conductor-N` peers read `Degraded`.
  Warmup `lastError`: gertrude's storage `/api/v1/cache/stream` could not be reached.
- `https://elohim.host/db/p2p/conductor-diagnostics`: membership is authoritative with 80
  running cells and 0 failed reads. 3 of adam's 5 iroh connections have
  `recv_message_count 0` even though they have sent 14k to 58k messages each.
- `https://doorway-alpha.elohim.host/p2p/status .provideLoop` (matthew): `reanchorDeadRemaining 0`,
  `deadRemainingStuck false`, `reanchorCaughtUp true`. doorway-alpha's upstream circuit to
  matthew storage read `closed`, `errorStreak 0` at 03:23Z. The open-circuit, `errorStreak` ~210
  state reported in the dispatch was no longer live at that read.

### Reading

This is the second outcome predicted by the 2026-10-09 decision: a sweep finishes,
`reanchorSweepsAbandoned` reads 0, and rows are still held unanswered against a conductor that
answers other calls (`reanchorFailed 0`, pools 7/7).

- The population has shrunk: 16 dead rows (2026-10-09) down to 5, of which the F5 cure settled 2
  by declaration (`deadSettledByDeclaration 2`). That leaves an unhealed residue of
  `5 − 2 = 3` (`provide_loop_status::unhealed_dead_residue`).
- Of those 3, two are `reanchorHeldUnanswered`: the adopt-before-author pre-flight asked and no
  answer came back. For a backing-record read this is
  `head_adoption.rs` `DeclarationBacking::Unanswered` (the
  `"network backing-record read went unanswered"` arm). The third residue row is in no held
  counter (`reanchorHeld 0`, `Unbacked 0`, `Failed 0`, both skip counters 0). Between the 03:23Z
  and 03:25Z reads, `reanchorCompleted` rose by 1 while `reanchorDeadRemaining` stayed at 5.
  Either one row is re-authored every sweep and then counted dead again, or a different row
  revived while another went dead. The HTTP surface cannot tell these apart.
- The detector is behaving as designed. `next_unchanged_sweeps` / `is_dead_remaining_stuck`
  (`provide_loop_status.rs:196-216`) compare the residue across sweeps. The tests
  `an_unsettled_residue_beside_settled_rows_still_reads_stuck` and
  `a_fully_skip_guarded_sweep_publishes_the_stuck_shape` pin that a residue which does not move
  reads as stuck, whatever the reason. Reading noted, not changed: the sweep cadence observed
  here is about 2 min, not the 300 s that `REANCHOR_HELD_BACKOFF_SECONDS` = 900 s assumes
  (`reanchor_backoff.rs:29`). An unanswered row therefore sits out roughly 7 sweeps before it is
  asked again, and the verdict is earned after 3. Changing that is a decision about what "stuck"
  means. It is not a threshold fix.

### Root-cause inventory

- `elohim/elohim-storage/src/services/head_adoption.rs` (`DeclarationBacking::Unanswered`,
  `PreflightVerdict::held(HoldBasis::Unanswered)`): the two rows rest on network reads that
  return no answer.
- `elohim/elohim-storage/src/services/reanchor_backoff.rs:29`: the 900 s hold window is sized for
  a 300 s cadence.
- `elohim/elohim-storage/src/p2p/projection_reconcile.rs:181,3166-3240`: the witness sweep runs on
  the reconcile tick with a 120 s budget. The 03:35Z sweep was abandoned.
- `elohim/elohim-storage/src/services/provide_loop_status.rs:183-216`: residue and detector, as
  designed.
- Fleet: one-way iroh connections on adam's conductor, and gertrude's storage unreachable from
  doorway B. Either would explain backing-record reads that are never answered.

### Missing node (named, not built)

Chain: `deadRemainingStuck` → (arm named by counters) → *which rows, and which read went
unanswered* → cure owner. The middle node exists only in Loki: no HTTP surface lists the residue
row ids or the read position (head, election or record) that went unanswered. This triage had no
Loki access, so it could not name the three rows or explain the residue row that sits in no held
counter. If this concern is reopened as a code task, a bounded observability addition is a
capped list of residue row ids with their `HoldBasis` and unanswered read position on
`/p2p/status` (or an admin read). That touches the view schema, the schema contract and the
generated TS. Not built here: it is a wire change, and it belongs with whoever owns the
dataplane-convergence habit, not a background lane.

### Current decision — BLOCKED (2026-10-19) on fleet network health plus a Loki read

No storage-code defect is indicated. The surface is fresh and the detector is correct. The
residue is 2 network reads that go unanswered, plus 1 row the surface cannot classify, on a
degraded alpha fleet. The cure has two parts:
1. Restore the alpha network paths: adam's one-way iroh connections and gertrude's storage
   reachability. This is operator or fleet work. No kubectl from this lane.
2. Do a Loki read on `elohim-adam-alpha-0`, target `elohim_storage::services::head_adoption` /
   `reanchor_backfill`, to name the 3 rows and the unanswered read position.

Ledger `2b4761b2eaf6` (`first_poll 213`) is set to `status: blocked`,
`backlog: dataplane-reanchor-dead-remaining-rekeyed-peer`. The poller will not dispatch again
while the line is present. It closes the line by disappearance once a clean streak follows the
residue draining.

### Verification (what would close this)

`elohim.host/p2p/status .provideLoop` reads `reanchorHeldUnanswered 0`. The unhealed residue
(`reanchorDeadRemaining − deadSettledByDeclaration`) is 0, or it moves sweep to sweep, with
`deadRemainingStuck false` and `reanchorSweepsAbandoned 0`. Not verified as of 03:38Z: the
condition is live (`stuckSweeps 4`, `deadRemainingStuck true`).
