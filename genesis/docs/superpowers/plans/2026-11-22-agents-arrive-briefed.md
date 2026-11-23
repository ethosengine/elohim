---
title: "Agents arrive briefed — ambient best practice as a property of the system"
id: agents-arrive-briefed-plan
status: active
class: process-meta
date: 2026-11-22
steward: human:matthew
author: "Claude (session 6109194c, Fable 5.2 orchestrating), at the steward's direction; plan approved by the steward 2026-11-22"
serves: [agents-arrive-briefed, recall-reaches-authority]
tags: [arrival-brief, placement, guide-stars, done-ladder, memory-ceremony, hooks, capability-levels, always-on-agents]
cites: []
---

# Agents arrive briefed: ambient best practice as a property of the system

## Run order

0. **Covenant ask** (one question) → compare main checkout with `origin/dev` → `just status device`
   → check the CI runner grant's expiry (backlog records 2026-11-23T16:00Z).
1. **Slice 0 — per-call noise** (four hook fixes) and the byte baseline. No steward ask.
2. **Story and habit**, born red. Arrival-reader baseline (one tier, dry run first).
3. **Router landing**: the five-question router goes to the top of root instructions. Nothing cut.
4. **Native `placement`** (the brief), then hooks and launch prompts render it.
5. **Per-section cut landings** of root instructions, each with its move-ledger rows.
6. **Memory-ceremony and what touches it.**
7. **Sweep**, run through the updated ceremony as claimable commitments, on each device.
8. **Re-measure.** One gathered read for the steward.

Each numbered step lands on dev by itself once its own check passes.

## Context

The "25 habits" article (claude-guides.com, 2026-11-16) lists what one developer does by hand in
one session: one goal and a finish line, short memory, no stale instructions, no context dump,
summarize and move, find the reason fast. The steward's ruling: these become ambient — what every
agent in flight is handed or reaches in one search — and memory upkeep keeps it true.

His additions, each a requirement:
- **P2P.** "Those passive best practices assume one developer, one session… we're trying to
  realize them by thinking of artifacts as a system of EPR events, agents and resources." These
  are the practices of always-on agents, "contextualized from where sandboxed or ephemeral
  always-on agents are anchored".
- **Resolved at the target.** A conductor says "go work on X"; that invocation "sees the world
  from that epr-meta context point", and benefits from "passively composed context from every
  invocation".
- **Shaped to capability.** Model names are not levels; "opus/astra, sonnet/sol map onto a
  capability/capacity gradient that has a model-agnostic shape".
- **Done is a ladder** of atomic checks that ratchets up to a simulated person saying the result
  and the journey are good. It should reuse and feed back into EPR/VSM primitives.
- **Why is guide-stars**, layered into every agent's flight where it is invoked.

Decided with the steward this session: keep both memory stores and tend both; root instructions
become a router plus universal rules; land in slices.

**Finish line.** A fresh agent of each kind, given a one-line task at a target, answers five
questions from what it was handed or one search: *what am I for · what is done · why is it this
way · who decides · how do I run it.* Measured before and after by an arrival reader. Always-
loaded and per-call bytes are lower than today, metered, and refuse to grow back at pre-push.

## 1. Design decisions

### 1.1 The arrival brief
Seven independent readers found no document that defines what an agent is handed at start. The
dialogue-stasis spec already designs the node and says it "does not exist today":
`placement` (fields `station`, `commitment`, `upwardChain`, `nearestNeighbours`, `confidence`,
`recall`; never authority). **Build that design**; do not invent a second one.

| Part | Answers | `placement` field / source | Carried to a successor? |
|---|---|---|---|
| Goal and rung | what for, what is done | `commitment` + covering habit `checks:`; or `untasked` | yes |
| Guide-stars | why | `upwardChain`: system stars → directory `.epr-meta` body and habit invariant → ruling notes on the subject | yes |
| Authority and bound | who decides | cited pointers, labelled evidence: `halt` first, then principal, covenant, grant, lease, expiry; as may / must ask first / may never | **no — re-resolved**; reads `none held` until the successor's own covenant exists |
| Where to look | how to run | router: tree `gate`, `epr flow context <path>`, recall, berth inbox, seam map, MAP.md | yes |

Rules:
- **Resolved from the target, not the sender's directory** — for guide-stars, the done rung and
  where-to-look. Authority is read from the record the agent acts under (its own or its
  dispatcher's covenant, its grant, its lease). The goal is read from the dispatch, commitment or
  contract. A dispatch carries a goal and a target address. (Valueflow already rules "pass
  addresses, not restated constants".) *(Narrowed 2026-11-22 after the story's blind reader found
  the unqualified rule contradicted the authority part.)*
- **Before the covenant.** A session's first brief shows authority as `none held`; when the
  steward answers, a source changed, so the brief is shown again once. A brief is re-shown only
  when a source changes, at a context reset, or when asked for.
- **System-level guide-stars travel with the checkout** (they live in the tracked root package),
  so a sponsored runner and a second device that hold the checkout can read them; only directory- and commitment-level lines can read `unmeasured`
  or `unknown — local to <device>`.
- **A brief is a read.** A participant with standing over a record may ask for the brief of an
  agent or runner acting under their name.
- **A read over records; no new store.** Each line carries its source address; the header prints
  the source CIDs so a changed source is visible. A missing part prints `unmeasured` and why.
- **Native composes; hooks only render.** A recorded rule on the hooks directory forbids a hook
  deriving its own orientation.
- **On cue.** Shown at start, at compaction and on resume (with the latest `recall remember`
  record: finding, open question, next action — "summarize and move"). Not per call.
- **Isolation list: no brief.** `blind-reader` (contract: one path, one profile), `stasis-keeper`
  (keeps its own incident packet), and the acceptor when it judges (artifact and trail addresses
  only). Keep and extend `SKIP_AGENT_TYPES`; a test asserts each gets nothing.
- **Succession** "carries what was said, never what was granted" (landing plane).

**Per kind of agent**

| Kind | Anchor | Delivery | Honestly empty today |
|---|---|---|---|
| Interactive session | covenant, session envelope | session start + compaction/resume render | goal until the covenant is answered |
| Subagent | parent's covenant; dispatch target | SubagentStart render, or `Target:` line + first act `epr flow context <target>` | — |
| Valueflow implementer / reviewer | commitment, brief | brief template + `epr flow context` | — |
| Shift step (`claude -p`, `codex exec`) | frozen contract (`intent`, `commitment`, `scope_cid`, `habit`) | **launch prompt** (`shift_runtime.prompt()`); hooks may be off | guide-stars resolve from habit + target; no schema change |
| Stasis keeper | coordinator's incident brief | unchanged (isolation list) | by design |
| Sponsored runner | delegates-compute grant | none read; the brief is a record its grant would cite | goal, guide-stars (reads no prompt) |
| Always-on, untasked | grant resolving to its principal | **pulled**: `epr flow context . --participate` (participation read), never pushed at start | goal reads `untasked` |

**Per runtime** — Claude: hooks + launch prompt. Codex: `AGENTS.md` hierarchy and launch prompt
only (nested `CLAUDE.md` is invisible to it; its hook events differ). Antigravity: skills only,
so each projected skill carries a one-line pointer to the placement verb.

### 1.2 Shaped to capability and capacity
`recall-contract.json` `lens_table` already has six levels (choices, bytes, scaffold:
"locate-and-hand-one-command" / "rank-and-let-choose") and a rule moving a reader up after three
journeys that reached authority and down after a mistaken assertion. It is keyed by model names
that are stale (`claude-fable-5-1`); this session read "stated none".

| | Lower level is given | Higher level is given |
|---|---|---|
| **Capacity** (holds, sustains) | small brief, content inlined | breadth, as addresses it opens |
| Options weighed | one path, one command | ranked alternatives, competing sources |
| Intent inferred | the steps and exact files | the goal |
| May decide | "if X, stop and report" | decide within the grant, say why |
| Notices unprompted | what it is pointed at | what is missing, stale, contradictory |

Capability (rows 2–5) and capacity (row 1) move apart: a large window with modest judgment gets
breadth but still one path and a stop condition. Bind names → level per runtime in
`genesis/agentic/agent-runtimes.json` beside C1–C4, effort, authority, independence. Role cuts
across level. A different lab's model at the same point is the more independent reviewer. What
each level needs is measured (arrival reader), not assumed (queue item 17).

### 1.3 Done, per kind of work, ratcheting
The repository already has a done contract per kind of work, VSM tags and a ratchet. Restated:

| Kind of work | Evidence (atomic) | Acceptance | Appreciation |
|---|---|---|---|
| Code | unit check → a2o scenario `@concern:` → simulacra on the mesh | tree `gate`; reviewer verdict | result + journey |
| Story / spec / doc | blind-reader loop; cites sealed | author's judgment under the covenant floor | result + journey |
| Habit station | its `checks:` | status flip on evidence only | — |

- A task names the highest rung it climbs to; that is its "done" in the brief.
- **Appreciation** = the existing appointed acceptor seat, given a named brief: would a typical
  person be pleased by the result and by how we arrived, reading the trail (corrections, review
  rounds, rework, steering). Recorded as one `--kind verdict` note (the audit channel, S3*); the
  rung reached goes in its reason, because the outcome is the closed pair approved /
  changes-requested.
- It is **testimony** under the steward's principal with the persona as provenance. It is never
  acceptance where a commitment requires the steward, and never a counted voice.
- **Feedback.** A poor journey = `changes-requested` + a `--kind correction` note on the surface
  that misled. That note shows in `epr flow context` NOTES for the next agent at that point, and
  the ceremony's Phase 0 reads it. `operator-surfaced-pain@1` folds only when the steward himself
  repeats a lesson already on record. The ratchet is the habit's `best_observed`.

### 1.4 What crosses devices
| Record | Crosses? | How |
|---|---|---|
| `.epr-meta` rules, habits, skills, packages, checks, stories, docs | yes | git |
| Collective memory content | yes (attribution does not) | git |
| Berth records, `gate-verdict@1`, `dev-landing@1`, `artifact-produce@1` | yes | peer channel allow-list |
| Correction / ruling / verdict notes, commitments, pain folds, measures | **no** | device-local flow store |
| Covenant clauses | **no** | local refs, until the steward rules on publishing |
| Private memory, transcripts, recall receipts | **never** | compiled floor: "thoughts are private; fruit is judged" |

So: **graduation into a tracked, governed surface is the only transport for a lesson.** A
correction note is a local cue toward it. A sibling's brief prints `unknown — local to <device>`
for parts that do not cross. Shared work is claimed with `berth claim lane:<id>`, which crosses.
Each device's private ring can only be swept by an agent on that device.

**Source-of-truth declaration (P2P design gate).** This plan adds no data entity: no DHT entry
type, no table, no route, no sync message. `placement` is an ephemeral, derived projection
(class C), recomputed from its sources on each read and never stored as truth. The sources of
truth stay where they are: tracked governed files (git), the device's flow store (notes,
commitments), brit covenant refs, berth records. The one new measure and the one new habit are
existing kinds. If execution finds it needs a stored record, stop and run `p2p-design-gate`.

### 1.5 Already ruled; the plan fits these
No scheduler, no queue, no agent wakes another → upkeep is cue-driven and claimable. Silence from
a person is ordinary; quiet applies to asks and status, never to reds; an ask is shown once per
identity (P11–P13). An agent has "no memory beyond what it writes down". A covenant clause is "a
recorded promise, not a proof". **Not decided here:** what may interrupt the steward (row 58);
refusing writes outside the write-set (row 79); publishing covenant notes; the standing design;
the runner grant's pre-expiry sense.

## 2. Audit (done, read-only)

**Memory.** Private ring: 132 entries, 356 KB, ~2.6 new a day; 88 feedback; one 47 KB overnight
log; ~18 status-shaped; read by no lens, measure or hook. Collective ring: 291 files, index
23.8 KB (bound 24 KB), near-frozen since October. `device/` holds 17 stale one-way copies.
Main session gets the private index; subagents get 200 lines of the collective one. No repo rule
says when to save, what not to save, or when an entry leaves. The same correction was recorded
four times in ten days. False "symlinked" text: `.epr-meta/elohim/lenses/CLAUDE.md`,
`.claude/scripts/_lib/paths.py`, `pre-tool-memory.py`, `memory-review.py`, `dedupe-memory-scan.py`,
`recall-contract.json`, and the memory-ceremony package itself.

**Prompt layer.** Root instructions 71,564 B against a declared 12,000 B bound (a carried-over
constant; the sections worth keeping measure ~14.4 KB). One package body → `CLAUDE.md` and
`AGENTS.md`; `AGENTS.md` already exceeds Codex's doc cap. "Build & Test Commands" is 27 KB.
Root never names `epr flow context`, `epr flow concerns --corrections`, MAP.md or the chronicle.
108 packages; 14 of 52 hooks packaged. No gate checks projection drift.

**Hooks** (source-read, not run):

| Emission | Cause | Source to edit |
|---|---|---|
| "Seam guidance coverage gap" every call | matcher `.*`; adapter supports Write/Edit only | package `native-seam-guidance` (+ `.codex`) and `.claude/settings.json` matcher |
| Rust-style notice on non-Rust shell | substring `cargo ` / `.rs` | package `rust-style` |
| Old shed lines, no dates; `BERTH: unavailable` | cursor 0; unpruned ledger; inner waits exceed hook timeout | `.claude/hooks/ram-guard.py` (file) |
| 2.5 KB cargo block every start | always printed | `genesis/agentic/bin/pool-preflight.sh` (file) |
| `Intent: Session context`, constant finish, guide-stars cut | literal in `recall::initialize`; `ceremony.finish`; 6-line cap | native + `recall-contract.json` + `run-projection.py` (file) |
| `bootstrap: skipped` | 5 s / 6 s budgets; runs take 3.4–5.1 s | diagnose first |
| Answered "question stall" keeps escalating | any `?`; retirement path | package `dialogue-stasis-sense`; `session_stasis.rs` — diagnose first |

Packaged hooks are edited in package `source.body`, then
`package-projections.mjs project --write-fixtures --write-runtime --only Hook:<id>`, then `verify`.

## 3. Gaps

G1 no brief for any agent kind · G2 task goal and finish never reach the per-prompt slot ·
G3 guide-stars exist and reach no one · G4 no stated rung per task; nothing judges the journey ·
G5 steering has no path to a rule, habit, skill or check · G6 private ring untended; no saving
rule · G7 root instructions omit the done/why lookups and hold 57 KB of reference · G8 hooks
emit without relevance; no total meter (queue item 13) · G9 parent and subagent hold different
memory · G10 docs describe a layout that no longer exists · G11 unattended agents and non-Claude
runtimes are outside every hook · G12 the ceremony tends one ring and cannot graduate steering ·
G13 levels are keyed by stale model names.

## 4. Closing the gaps

### Step 1 — Slice 0: per-call noise (first hour)
- Narrow the seam hook's matcher to the tools its adapter supports; re-project.
- `rust-style`: fire on `cargo` as a command word or a `.rs` write target.
- `pool-preflight.sh`: one line and a pointer; the slot table moves to the first cargo call.
- `ram-guard.py`: advance the cursor at session start, print dates, cap the ledger, fit inner
  waits inside the hook timeout.
- Each with its test in `.claude/hooks/__tests__/`. Measure session-open and per-call bytes
  before and after (per-call from transcripts, offline). This is the first baseline.
- Deliver queue item 13 as written: a context meter = declared sum of root instructions,
  both memory indexes as loaded, and each SessionStart hook's output, emitted once by
  `load-project-context.py`; a `measures.yaml` row and ceiling. Reader: the habit's `checks:`
  line (`epr flow report --bound …`). Close item 13 in the queue.

### Step 2 — Story, habit, baseline
- `genesis/a2o/features/devflow/agent-arrives-briefed.feature`, `@concern:
  agents-arrive-briefed`. Scenarios: a session, a subagent dispatched at a target, a valueflow
  implementer and a shift step each answer the five questions from what they were handed; a
  successor gets the same carried parts and `none held` for authority; the isolation list gets
  nothing; a lower-level reader gets one path and a stop condition; a seeded poor verdict shows
  in the next agent's context at that point; a forced compaction keeps goal, rung and next
  action; a runner's brief is asserted from the record its grant cites. Blind-reader loop applies.
- Habit atom `genesis/agentic/.epr-meta/agents-arrive-briefed.habit.md`: red, not active;
  `checks:` = the scenario, the arrival reader, the context meter ceiling, root size check.
  `retire-when:` the session runtime carries the brief natively and the checks have been green a
  quarter. Re-project `habits.yaml`. Also moves `recall-reaches-authority` (red).
- **Arrival reader**: an arrival mode in `.claude/workflows/recall-standing-reader.js` (edit its
  hardcoded `QUESTIONS`): launch a real subagent, a `claude -p` step and a `codex exec` step at
  a target with a one-line task, capture exactly what each received, ask the five questions,
  judge with a different tier. One tier as a dry run, then per level and per runtime. Record the
  method CID with each result.
- Publish this plan to `genesis/docs/superpowers/plans/` citing both habits; seal cites.

### Step 3 — Router landing (nothing cut)
Edit `source.body` of `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json`; re-project
`--only`; `verify`; `git diff` shows only the addition. The router: five questions, each with its
command or path; the done table (1.3); guide-stars and where they come from; "record a correction
on the surface that misled you"; the memory-saving rule (private ring: the steward's preferences
and device facts; never status or what the repo records; every entry born with an exit; a lesson
another agent needs is graduated). Rewrite the package's `composition` note to describe the
package-master edit path. Widen the pre-push projection regex with `^(CLAUDE|AGENTS)\.md$`,
the two other planted docs, `^\.claude/commands/`, `^\.claude/hooks/`.

### Step 4 — Native `placement`, then render it
**Implementation design (2026-11-23):**
`genesis/docs/superpowers/specs/2026-11-23-placement-read-and-notes-store-design.md` — two designs
and an adversarial review, then a fresh implementer's read of the first landing. It supersedes
the bullets below where they differ: six landings (L1 notes reader, closure index and honest
gate and habit resolvers; L2 one flow store per repository on a device; L3 the placement read
and its per-prompt render; L4 to L6 the other renders, commitment rungs, and the router
follow-up). One item goes to the steward: the story's first scenario gives the covenant
"may / must ask first / may never" clauses, and the covenant records only an affirmation, a
purpose with a write-set and rule variances.

- `elohim/eprfs/epr-cli` (`flow/memory/recall/{mod,journey,render}.rs`, `session_envelope.rs`):
  `placement` as the stasis spec designs it, with guide-stars as `upwardChain`, authority as
  cited evidence, level-shaped output from `lens_table`, `unmeasured` with a reason for a missing
  part. Seed the envelope's intent from the recorded purpose. `epr-cli` is not a published crate:
  **no version bump, no Edge roll**; confirm with the fleet-roll preflight by hand before
  assembling. The new executor digest refuses live recall sessions once; announce over berth.
- **Two preconditions found while verifying the router (2026-11-23), both required before "every
  invocation leaves the point richer" is true even on one device:**
  1. *A note outlives an edit of its subject.* `epr flow context <path>` reads notes by the CID of
     the file's current bytes (`flow/context.rs` `resolve_target`), so a correction drops out of
     the context read as soon as the file it names is edited; only `epr flow concerns
     --corrections`, which matches by label, still lists it. Observed: two open corrections that
     `concerns` lists show `NOTES (0 shown)` in `context` for the same paths. `placement` and
     `context` gather notes by subject label as `concerns` does, and mark a note whose bytes have
     since changed as such.
  2. *One notes store per repository on a device, not one per checkout.* The flow sidecar is
     `<checkout>/.eprfs/status/flows.jsonl`; a worktree has its own, so a note written where the
     work happens is invisible from the main checkout (observed: 0 corrections from the worktree,
     5 from the main checkout) and is deleted with the worktree. Resolve the sidecar through the
     repository's common git directory, or fold worktree sidecars into the device's store.
  Until both land, the router tells agents to record corrections from the main checkout and to
  read `concerns --corrections`.
- The context read's GATE line is `ambiguous` (seven candidates) for some paths and names a
  doubtful habit for others (observed on `genesis/agentic/bin/pool-preflight.sh`); `placement`
  must resolve one gate and one covering habit, or say plainly that it cannot.
- **One contract commit**: `recall-contract.json` (`guiding_context` → system stars with source
  CIDs and the layering rule; `lens_table` keyed by level; the symlink text) together with
  re-pinning `recipe` and every `in_scope_of` in `recall-questions.json` to the CID epr reports,
  and the `q-top-red` assertion kept valid.
- Bind names → levels in `genesis/agentic/agent-runtimes.json`.
- Render: `run-projection.py` passes the prompt as the need and drops lines that repeat a default;
  `covenant-subagent-stamp.py` emits the placement head whether or not a covenant is held
  (verify first whether SubagentStart exposes the dispatch text; fallback is the `Target:` line
  and first-act rule, added to the `orchestrate` skill); the compaction/resume re-show.
- Launch prompts: `shift_runtime.prompt()`, the Codex hook projection, a pointer line in each
  Antigravity-projected skill. Fence peer and note text as untrusted; a ruling is authority only
  when validated as steward-issued (the keeper brief's existing rule).
- Valueflow: brief template gains `done:` and "Guide-stars in force"; the reviewer states the
  rung reached; the acceptor seat gains the appreciation brief.
- Second-device run on ethosengine before these lines are called met.

### Step 5 — Cut root instructions, section by section
**Move ledger first** (in the published plan): one row per section — heading → new home → what
loads it there (path read, hook, deny rule, or "deleted, enforced by X"). Rules:
- A rule triggered by a **command** (kubectl, `mcp__jenkins__triggerBuild`, push forms, `EXIT=$?`,
  the husky `test -x` refusal) stays in root, or first becomes a settings deny entry or hook.
- Keep, compressed to a line each with a pointer: the `cleanup:` and `scope:` headline triggers
  (native code names them as a contract; update `report.rs`'s comment if wording changes).
- Keep the heading "Session discipline and commit attribution" verbatim (the Codex preamble
  cites it) or edit the preamble in the same commit.
- A home must be visible to Codex: an AgentDocPackage with both projections, or a skill. *(An
  `.epr-meta` rule was listed here and is not one: `.codex/hooks.json` registers no resolver for
  those rules, so they are surfaced to Claude on Edit/Write only. Corrected 2026-11-23 from the
  move ledger.)* A README is not a home for a rule. ts-rs trap → `elohim/elohim-storage` and
  `elohim/sdk`; contentFormat rule → the seeder / lamad manifest directory.
- One landing per section (CI/CD, mesh reference, gate knobs, architecture, schema). Each
  carries `[doc:cut] <destination>` touching the package, `CLAUDE.md`, `AGENTS.md` and both
  fixtures; a check that the destination holds the text; updated back-references
  (`.epr-meta/manifest.md`, seam map, sync rules); a berth announcement first.
- **The ledger (2026-11-23).** 144 rows, each checked by a second reader; raw rows in
  `genesis/local-dev/agents-arrive-briefed/move-ledger-raw-2026-11-23.json`, and each cut's rows
  in its commit.

  | Group | Bytes reviewed | To keep in root | Rows | Corrected by the check |
  |---|---|---|---|---|
  | `just` and mesh forms | 15,086 | 610 (landed as 1,824, with three traps and a lease sentence) | 37 | 5 |
  | Gate knobs, pins, traps | 12,848 | 1,734 | 33 | 6 |
  | Brit, done, recall, workflow, the two session-start triggers | 22,359 | 6,189 | 36 | 2 |
  | Architecture, schema, gotchas, CI/CD, code style | 21,032 | 2,780 | 38 | 1 |

  Findings that bind the remaining cuts: nothing denies `kubectl` or the Jenkins trigger tools
  today (the settings file has no deny entries), so every command-triggered rule stays in root
  until a deny rule or hook exists; the gate-knob reference has no home yet (a "Local gate knobs"
  reference is proposed and must be a skill or packaged doc, not a README, to reach Codex); the
  libp2p skill contradicts the root file on `with_codec` and must be reconciled before that
  gotcha moves.
- **Target**: measured kept rules plus router (expected 18–22 KB), recorded as `best_observed`
  and ratcheted. Growth refuses at pre-push beside `doc-cut` (a size check on root
  `CLAUDE.md`/`AGENTS.md`). The 12,000 B constant goes to the steward as one question (step 8).

### Step 6 — Memory-ceremony and what touches it
Package `.epr-meta/elohim/packages/skills/memory-ceremony.json`, projected to all three runtimes.
Record its two stale lines as correction notes first, so the new Phase 0 runs on its own package.
- Open with the journey and its finish; mechanics below.
- **Two rings**, the floor that keeps one local, both tended; private sweeps are per device.
- **New phase — graduate steering.** Per feedback entry: is it enforced or findable where the
  next agent looks? If not, its home (`.epr-meta` rule by the lightest signal, habit, skill,
  check, canonical story); write it there with a back-pointer; the entry becomes a pointer or
  leaves; record the removal as the person's act and clear the `device/` copy.
- Phase 0 also reads repeat-steering observations. Phase 4 adds the arrival reader.
- Lenses (`memory-review.py`, `dedupe-memory-scan.py`, `cleanup-scan.py`) read the harness
  directory directly as a second ring. Correct every false symlink line.
- `storyteller`: wider graduate targets. `librarian`: the private-ring budget. `historian`:
  status-shaped entries become chronicle. `memory-stasis-loop.js`: private ring and "steering
  awaiting graduation" as dimensions. `story-harvest`, `elohim-epr-metafile`: accept a memory
  entry as a rule's source. Pickup surfacing: scope its query at done and why sources.

### Step 7 — The sweep (uses the updated surfaces)
`epr flow project` turns the sweep into gap items. Each batch is a valueflow commitment naming
its rung, claimed with `berth claim lane:<id>`, run through the revised ceremony, closed by an
appreciation verdict, one delta line in the habit atom.
1. Private ring on shem (132): a disposition for every entry — graduate (home) · keep with an
   exit · fold · chronicle · drop.
2. Private ring on ethosengine: the same, by an agent there.
3. Collective ring (274); index under its soft bound.
4. Nested instruction files over budget (`.epr-meta/elohim/lenses/CLAUDE.md` 35.7 KB,
   `genesis/a2o/CLAUDE.md` 12.8 KB) and every home that received moved text.
5. Hooks: run the existing `intervenor_census.py`; drain its `lacking` count (`retire-when`)
   for every hook this plan touched; fix or retire any that speaks without changing the next
   action.
6. Loop agents: the two workflows, nine commands, the triage agents and the shift and keeper
   prompts point a dispatched agent at its placement by address. Skills and agents are **not**
   swept for a "done rung" line; the rung lives in the task's brief.

## Asks of the steward (three)
1. **Step 0 covenant**: steward of record; this purpose; the write-set (packages, hooks,
   `.claude/settings.json`, the named native files, lenses, memory rings, the new story, habit
   and plan, the new instruction homes); the blind-reader floor for the new story; and approval
   that the recall contract's `guiding_context` is rewritten.
2. **One gathered read at the end of the sweep**: rewrites of his own words, the stances to be
   graduated, the proposed system stars.
3. **One question**: the root size bound — keep 12,000 B as a target to ratchet toward, or set
   it at the measured kept size.
Everything else is decided under the grant and reported over berth.

## How the work is run
A worktree under `/projects/elohim/.worktrees/`; `brit` for repository operations; campaign state
under `genesis/local-dev/agents-arrive-briefed/`. Fable holds design and review; mechanical
package and hook edits go to a Sonnet-level seat; the ceremony rewrite, the story and the native
design to an Opus-level seat; an independent review of every step to a Codex seat. Steps 2 and 7
run as workflows with each finding adversarially checked.

## Verification
- **Finish line**: arrival-reader results on the five questions, before and after, per level and
  per runtime (Claude session, subagent, `claude -p`, `codex exec`; an Antigravity skill read).
- **Per step**: hook tests · `package-projections.mjs verify` · `just gate memory-ceremony` ·
  `just gate orchestrator` · the scenario · `epr flow cites verify` on receiving docs ·
  fleet-roll and doc-cut preflights by hand on each candidate head.
- **Bytes**: context meter under its ceiling; root size check; `memory-index-bytes` under soft;
  private-ring count and bytes on both devices.
- **Live**: a fresh session and a dispatched subagent after step 4 — no per-call line, a goal
  that is this task's or absent, placement in the subagent; a mid-session prompt that changes
  the task shows the new goal or nothing; a forced compaction keeps goal, rung, next action.
- **Arc items**: census coverage equals the hook count; every private feedback entry has a
  disposition; repeat-steering observations stop climbing over a stated window; a layering
  fixture for guide-stars; a known-good and a known-poor acceptor fixture; isolation-list test.
- **Second device**: the same live read on ethosengine after it follows each landing.
- **Appreciation, applied to this work**: an Opus-level seat that did not do the work reads the
  result and the trail.
