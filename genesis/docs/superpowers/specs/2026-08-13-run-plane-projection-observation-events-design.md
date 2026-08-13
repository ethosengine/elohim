---
title: "Run-plane projection & observation events — the sense/memory layer for a long-horizon run"
id: run-plane-projection-observation-events
tier: spec
status: Draft
class: process-meta
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete AND a note-then-project round-trip scenario passes green (an `epr flow note` appended in one session is re-projected into the next session's block without a conversation carry)
created: 2026-08-13
domain: D9
topic: [agentic-harness, context-engineering, projection, run-plane, epr-rea, flow-event, hooks, retire-when, observation]
cites:
  - genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md
  - genesis/research/context-engineering-primary-sources-cross-pollination-2026-08-13.md
  - genesis/manifests/habits.yaml
  - epr-rea-valueflow-fabric | EPR-REA ValueFlow Fabric | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md
  - genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md
  - elohim/epr-rea/src/model.rs
  - elohim/eprfs/epr-cli/src/flow/fulfill.rs
  - elohim/eprfs/epr-cli/src/flow/seal.rs
  - .claude/scripts/_lib/intervenor_census.py
---

# Run-plane projection & observation events

> **One-line:** the repo already holds the richer durable workflow object (REA commitments over
> EPR atoms, plus `habits.yaml`'s admission-controlled WIP fence) and runs the poorer loop — it
> renders that object once at session start and then runs on conversational memory. This spec
> closes both halves of the loop: a **write** leg (`epr flow note`) so a run-scale correction
> lands in the durable plane instead of the transcript, and a **read** leg (a per-prompt
> projection hook) so the durable plane is re-derived into context on every turn instead of
> drifting deeper into it.

## 1. Provenance and the seam this fills

This spec implements **rows 1, 2 and 5** of
`genesis/data/timeline/backlog/agentic-harness-borrows-backlog.md` — the per-turn run-state
projection (row 1, survey TAKE-1), the write-path discipline that is its stated precondition
(row 2, TAKE-2), and the run-scale failed-approaches field (row 5, TAKE-3, "the cheapest item in
the mint"). The three are one seam, not three: row 2 is named in the backlog as row 1's
precondition ("a projection loop over state nobody writes to projects nothing"), and row 5 is the
first and cheapest thing worth writing through it. Splitting them would ship a reader with nothing
to read.

The external evidence is graded, and the grades are load-bearing:

- **Arize's `PlanMessage`** (survey §1.5) is the corpus's **only** per-call injection precedent —
  the plan is lifted out of conversation history entirely, re-derived from durable state on every
  loop iteration, and injected at a fixed position so it cannot sink deeper into context as tokens
  accumulate. Mechanism 2 borrows exactly this and nothing more.
- **Anthropic's managed-agents session log** (§1.8) supplies the shape that makes the write leg
  worth building: the window is a *projection* of a durable append-only event log, so **compaction
  and handoff are the same operation** — two slices of one log. Our `.eprfs/status/flows.jsonl` is
  already that log (append-only JSONL, `elohim/epr-rea/src/store.rs:193`, `:206`; 4,866 lines /
  2,277,640 bytes at authoring). What we lack is a leg that writes run-scale observations into it.
- **`smsharma/clax`'s `CHANGELOG.md`** from the Anthropic science run (§1.2) is the whole design
  spec for row 5: the progress file records status, completed tasks, **failed approaches and why
  they failed**, with the rationale stated outright — without the dead ends, "successive sessions
  will re-attempt the same dead ends." The verbatim exemplar entry is the shape to hit: *"Tried
  using Tsit5 for the perturbation ODE, system is too stiff. Switched to Kvaerno5."* A fresh
  session inherits the **consequence** without re-reading the failure.
- **Symphony** contributes the around-the-run reconciliation cadence **only** and explicitly not
  per-call injection: its `WORKFLOW.md` renders the full state block on the *first* turn, and edits
  apply to future launches rather than in-flight sessions (§1.6). Survey **WATCH-10** names the
  promotion of that evidence into the injection claim as the first draft's error, and this spec
  does not make it: polling a tracker, refreshing an in-memory work object, and placing current
  state into the next inference request are three operations, and only Arize evidences the third.

**Habit served.** `dev-system-equilibrium` — the candidate habit currently **in admission** (Spec
B; row 7 of the same backlog, operator-gated because `genesis/manifests/habits.yaml` is capped at
max 12 with max 2 `active`, so a candidate displaces or waits). That habit's invariant is
rates-against-rates: every stock in the development system has drain ≥ inflow. This spec is its
**sense and memory layer** — the projection is how the loop perceives its own fence each turn, and
the note event is how a correction becomes a measurable inflow instead of a transcript artifact.
The relationship is deliberate and one-directional: this spec supplies the observations; it does
not compute the rate, does not touch `habits.yaml`'s covenant, and does not presume the admission
decision. If the habit is not admitted, the two mechanisms still stand on rows 1/2/5.

**What this spec does not add.** No new register. Survey **LEAVE-11** declines a `current.md`
outright — a fifth register with no reader is the precise anti-pattern our own gospel names, and
`CLAUDE.md`'s "What to work on" section says the same thing in its own words ("If you are about to
write a new register, ledger, or ranking script, the answer is almost certainly one of the four
above"). The projection block is a **derivation** of state we already keep;
`.claude/scripts/habits-status.py::headline` (`:129-160`) already computes most of it and
`epr flow status` (`elohim/eprfs/epr-cli/src/flow/walk.rs:405-471`) most of the rest.

## 2. P2P design gate

Two entities, both answered before any interface was drawn.

**(a) The run-state projection block — Ephemeral (C).** It is never persisted, has no identity, and
is a pure derivation from four inputs that are each already durable elsewhere: `habits.yaml`, the
saga chapter set (`genesis/a2o/features/dataplane/resiliency-saga/01..11-*.feature`),
`.eprfs/status/flows.jsonl`, and the session's own claimed commitments. Re-deriving it twice from
unchanged inputs must yield byte-identical output; that property is what makes a cache legitimate
(§4). No DHT entry type, no table, no route, no signal. A projection that acquired storage would
have become the fifth register LEAVE-11 declines.

**(b) Observation events — repo-plane appends, not DHT entities.** A note is a `FlowEvent`
appended to `.eprfs/status/flows.jsonl`, with identity `atom_cid()` — CIDv1 dag-cbor/sha2-256 over
the payload, envelope excluded (`elohim/epr-rea/src/store.rs:34-46`, `elohim/epr-rea/src/model.rs:18-20`)
— exactly like every other record on that path. This is the **offline floor** of the fabric
(`2026-07-18-epr-rea-valueflow-fabric-design.md` §4), whose graduation to a DHT `EconomicEvent` is
governed by recipe-declared policy and is explicitly out of scope here: a run-scale observation is
the 1000:1 observation end of the records-lifecycle gradient, not a crystallization candidate.

Answering the gate's five questions in order:

1. **Source of truth class:** (a) Ephemeral (C); (b) repo-plane append — durable-local, notarized
   nowhere. Neither is Notarized (A), Linked (A2), Private (B) or Attested-Private (B2).
2. **Does an entry type already exist?** No DHT entry type is involved, so no `#[hdk_entry_types]`
   read is required and none is added. The *record* kind already exists: `FlowRecord::Event`
   (`store.rs:22-31`). **No new `FlowRecord` variant is minted** — the note's kind is carried by
   classification, not by the enum.
3. **Head-plane cost:** zero. Nothing is published, gossiped, or head-elected. The local cost is
   one JSONL line per note; at a plausible run-scale rate (single-digit notes per session) this is
   noise against the existing 4,866 lines, and `SidecarFlowStore` re-verifies every record's CID on
   read (`store.rs:246-252`) so growth is bounded by honesty, not by policy.
4. **Identity:** content-derived. `atom_cid()` over the payload; no UUID, no slug, no
   agent-composite key. Two identical notes collapse to one CID — the existing `stage_or_count`
   dedup idiom in `fulfill.rs` already relies on exactly this.
5. **What creates it, and what projects it:** the new `epr flow note` leg creates it (§3); the
   run-projection hook projects it (§4). **No coordinator function, no zome, no DNA hash movement,
   no HTTP route, no signal.**

**The classification carrier — the one mechanical decision this gate forces.** `FlowEvent`
(`model.rs:377-393`) has no `classified_as` field; `classified_as` lives on `ResourceSpec`
(`model.rs:74-77`), which rides `Intent` and `Commitment` but not `FlowEvent`. The tag vocabulary
`run:failed-approach | run:correction | run:observation` therefore needs a carrier, and the choice
is constrained by two hard requirements: no new record kind, and no existing event's CID may move.
Both are satisfied by an **additive optional field** on `FlowEvent`:

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub classified_as: Vec<String>,
```

This is not an invention — it is the verbatim idiom `Commitment.bound` already uses for the same
reason, with the reason written into the source: *"Skipped when absent so that declaring this
vocabulary did not move a single existing commitment's CID"* (`model.rs:363-370`), pinned by the
test `an_unbounded_commitment_keeps_its_pre_bound_cid` (`model.rs:701`). The note leg's test set
must carry the sibling pin for events. The rejected alternative — smuggling the tag into
`Magnitude::Count{unit}` (`elohim/epr/src/witness.rs:141-152`) — is refused because `unit` is a
fold key: `count_in` filters `Magnitude::Count{unit}` by exact string match
(`elohim/epr-rea/src/stock.rs:470-475`), so a classification hidden in `unit` would silently
partition every stock fold that names a unit.

## 3. Mechanism 1 — `epr flow note`, the write leg

A new leg on the existing `epr flow` CLI (`elohim/eprfs/epr-cli/src/flow/mod.rs::run`, `:71-164`),
alongside `project | walk | status | seal | reseal | hold | fulfill`. It **writes**.

```
epr flow note --on <commitment-cid-or-path>
              --kind failed-approach|correction|observation
              --reason TEXT
              [--switched-to TEXT]
              [--json] [--root DIR]
```

`--root` and `--json` are the existing global opts, stripped by `parse_global` before leg-specific
parsing (`mod.rs:263-286`) — the leg parses only its own four flags.

**What it appends.** One `FlowRecord::Event` carrying a `FlowEvent`:

| field | value | why |
|---|---|---|
| `action` | `ReaVerb::Cite` | The closed verb set is `Use \| Consume \| Produce \| Cite \| Affirm \| Dismiss` (`elohim/epr/src/witness.rs:310-317`). A note produces no resource and discharges no promise; it *refers to* one. `Produce` would make notes count as output in every fold; `Dismiss` already means regression in `fulfill.rs:365-379`. |
| `resource` | `body_cid_of_file()` of the target when `--on` is a path (`mod.rs:360`), else the CID given | The atom the note is *about*, resolved by the same helper `fulfill.rs:332-334` uses. |
| `quantity` | `Magnitude::Count{value: 1.0, unit: "run-note"}` | One observation. A distinct unit string keeps notes out of every existing unit-keyed fold by construction (`stock.rs:470-475`). |
| `classified_as` | `["run:<kind>", "<target-label>", "reason:<TEXT>", "switched-to:<TEXT>"?]` | Tag first, subject second — the established two-slot convention (`project.rs:526-530` mints `["gap:<state>", item.id]`; scenario commitments mint `["a2o:scenario-green", rel]`, read back positionally at `fulfill.rs:200-212`). **Resolved at implementation (T1, commit 9082a42):** this table originally left the authored `--reason` body without a stated home; slots after the second carry it, `reason:`/`switched-to:`-prefixed so a reader distinguishes authored body from tag by construction. A VF-style `note` field was considered and deferred to the protocol-REA graduation path (where full ValueFlows `note` vocabulary exists) — two homes for authored text in the v1 projection plane would be worse than one documented convention. |
| `in_scope_of` | `repo_scope_atom()` (`mod.rs:366`) | Same scope every repo-plane record uses. |
| `process` | `None` | A note belongs to no recipe run. |
| **`fulfills`** | **empty** | See below. |
| **`satisfies`** | **empty** | See below. |
| `provider` / `receiver` | authoring agent / `repo_agent()` (`mod.rs:374`) | Mirrors `fulfill.rs:302-304`, where the CI agent is the provider and the repo the receiver. |
| `occurred_at` | see §7 Q1 — **open** | |

**`fulfills` and `satisfies` stay empty, and this is the load-bearing rule of the leg.** A note
**annotates** — it says something about the work; it does not discharge the promise. The
consequence is mechanical, not stylistic: the discharged set in both `walk.rs:170-179` and
`fulfill.rs:216-228` is derived as *any event whose `fulfills` names the commitment*, so a note
that populated `fulfills` would mark its own commitment fulfilled and silently retire live work.
Association is carried instead by `in_scope_of` plus the `classified_as` tag, which is exactly how
the projection re-finds it in §4. (The same discipline is already visible in the fabric: `Dismiss`
events carry an empty `fulfills` — `fulfill.rs:217-221` — precisely so a red run cannot discharge
anything.)

**Write-path precedent.** `fulfill` is the model for a leg that appends events
(`fulfill.rs:182-404`): open the store, read all records, resolve, stage into a `to_append` vec,
and append only at the end (`fulfill.rs:395-400`). **The failure-safety idiom to copy is `reseal`'s
two-phase resolve-then-append** (`seal.rs:251-270`): Phase 1 resolves *every* candidate and
collects every failure; if any failed, "reseal aborted, nothing appended"; Phase 2 appends only
once all resolutions succeeded. A note leg resolves exactly one target, so the phase split is
trivially satisfied — but the invariant it protects (never a partial-write window on the sidecar)
is the one that must survive into the note leg's error arms.

**Refusals.** An unresolvable `--on` target errors like `fulfill`'s `FlowError::UnknownResource`
(`fulfill.rs:333`) rather than appending an orphan. An ambiguous path match errors with the list
and never guesses (`fulfill.rs:281-289` — *"never guess"*). A byte-identical repeat note is a
true no-op by CID dedup, not a second line.

## 4. Mechanism 2 — the run-projection emitter

`.claude/hooks/run-projection.py`, registered on **`UserPromptSubmit`** and riding the existing
**`SessionStart`**.

**Delivery differs by event, and getting this wrong is a known way to ship a hook that never
lands.** On `UserPromptSubmit`, plain stdout reaches model context directly — no wrapper — as
`pickup-semantic-surfacing.py:205-206` does (`if via == "prompt": print("\n".join(lines))`), with
the reason stated in its own docstring at `:182-188`: on `PreToolUse` only the
`hookSpecificOutput` JSON form lands, and *"the tool-path injection was print()-based and never
landed"* (a documented prior bug). On `SessionStart` the emitter uses the JSON wrapper shape, of
which `delivery-gate.py:138-147` is the canonical instance:

```python
print(json.dumps({"hookSpecificOutput": {
    "hookEventName": "SessionStart", "additionalContext": context}}))
```

**Compaction is covered by the existing registration, not by a new event.** `.claude/settings.json`
declares seven `SessionStart` hooks and **no `source` matcher anywhere in the file** — so all seven
already fire on every `SessionStart` source including `compact`. An eighth needs no matcher either;
it inherits the same coverage. This is the mechanical reason the managed-agents framing holds here:
post-compaction the block is re-derived from the durable log rather than recovered from a summary,
which is what makes compaction and handoff the same operation for the projected state.
`UserPromptSubmit` is likewise a single unmatched block today (`settings.json:64-74`).

**Content — at most 20 lines**, in this order:

1. the **top red habit** and its first check, from `habits-status.py::headline`'s own selection rule
   (active red beats inactive red beats active unwired, `habits-status.py:136-139`) — reused, not
   re-implemented;
2. the **≤2 `active` habits** — the max-2 WIP fence, rendered as `habits-status.py:154` renders it;
3. the **saga frontier** — the first chapter of `01-device-awakens` … `11-pull-queue-retires`
   (`genesis/a2o/features/dataplane/resiliency-saga/`) not yet green in the per-concern rollup;
4. **this session's claimed commitments** — `commitment:claim:<gap>` rows (the label form minted at
   `project.rs:563`), which exist only for gap-items whose decomposed state is textually `CLAIMED`
   (`project.rs:548`);
5. the **last N unresolved `run:correction` / `run:failed-approach` events** from `flows.jsonl` —
   the §3 write leg's output read back, newest first;
6. one footer line (§5).

Nothing here is authored state; every line is a re-derivation. If an input is unreadable the
emitter drops that line and prints the rest — the degradation rule `habits-status.py:194-208`
already applies to itself (*"any failure degrades to printing no ratio line at all, never a
traceback at session start"*).

**Cache.** Keyed on `(habits.yaml mtime, flows.jsonl size, saga source mtime)`, all three cheap
stats, with the rendered block stored beside the other hook state. Target **<1s cached**; the hook
registers with a **10s timeout**. The budget is not theoretical: the `SessionStart` headline chain
already runs `load-project-context.py` → `placement-audit.py --headline` → four further
subprocesses, under a 5s outer registration whose own inner 25s budget is unreachable
(`load-project-context.py:86` vs `settings.json:14-16`) — a declared-vs-actual mismatch this hook
must not repeat. A cache miss on the `UserPromptSubmit` path must degrade to the cached-or-empty
block rather than block the turn.

**`retire-when` is a shipping requirement, not a footnote.** The backlog states it in those terms
(row 1: *"MUST ship with a `retire-when:`"*), the survey's §1.4/§4.6 gives the reason (Anthropic's
sprint scaffold was load-bearing on one model tier and dead weight four months later), and the
mechanism landed 2026-08-11 as item 16 of
`genesis/data/timeline/backlog/agentic-context-tooling-consolidation-queue.md` (commit `63e81325c`).
The counted form for a hook is a module-level `RETIRE_WHEN = "..."` constant — population 3 of the
intervenor census (`.claude/scripts/_lib/intervenor_census.py:19-21`) — and the census's own
self-declaration (`:63-68`) is the register to match. A bare `never` is refused at the gate
(`_validate_retire_when`, `.claude/scripts/_lib/epr_meta.py:222`), so the condition must be real
and checkable. **The load-bearingness stress test is the deliverable**: the retire condition is
stated as a model-tier claim — *this projection retires when a model tier holds the fence and the
frontier across a full session without it, evidenced by a paired run* — and it is re-asked at every
model-tier landing (survey WATCH-8).

## 5. Mechanism 3 — the write-path discipline

Row 2 is zero-tooling and is the precondition for everything above: a correction that must survive
compaction or reach the next session gets written to the durable plane; conversation is history by
definition. Survey §4.4 names the hole precisely — a mid-run operator correction lands in the
conversation, compaction is lossy summarization of exactly that region, and nothing re-injects it
afterward because habits and commitments were never told, so survival depends on the summarizer's
judgment ("an outdated judgment left in charge"). §4.2 types it harder: the operator's mid-run
correction **is the algedonic channel**, and if the pain signal reaches only the current draft and
not the state, *it was absorbed by the thing that caused it*.

The discipline is carried by the instrument, in two places and nowhere else:

1. **The projection block's footer line** teaches the write leg at the moment it is needed —
   verbatim shape: `a correction that must survive this window: epr flow note --on <commitment>
   --kind correction --reason "…"`. This is the same posture `delivery-gate.py:130-137` takes with
   its own advisory line: it informs, and it explicitly must not redirect the pilot.
2. **One `.epr-meta` rule at `class: inject`** — the advisory tier of the enforcement ladder
   (`ENFORCEMENT_CLASSES = ("deny", "ask", "inject", "measure", "dispatch")`,
   `.claude/scripts/_lib/epr_meta.py:162`; `inject` permits and advises,
   `epr_meta.py:1621-1623`). It nudges at edit time when a correction-shaped change lands with no
   corresponding note. It carries its own `retire-when:` like every other rule in the tree
   (`genesis/docs/superpowers/specs/.epr-meta:11`, `:17`, `:23`).

**No new register, no new file, no third home.** The instrument carries the discipline (survey
LEAVE-11). The `.epr-meta` route is the survey's own proposed landing for row 2 and is the
stigmergic System-2 shape the corpus already runs: the rule lives in the place, and the place
instructs whoever arrives.

## 6. Decomposition (gap-items)

- [ ] `epr flow note` leg in `elohim/eprfs/epr-cli/src/flow/` — arg parsing under `parse_global`,
      target resolution, event construction (§3)
- [ ] Additive `classified_as` on `FlowEvent` with `skip_serializing_if = "Vec::is_empty"`, plus the
      CID-stability pin mirroring `an_unbounded_commitment_keeps_its_pre_bound_cid` (§2)
- [ ] Note-leg tests: empty-`fulfills` invariant (a note never discharges its commitment),
      two-phase abort on unresolvable target, byte-identical-note dedup, `usage()` coverage (§3)
- [ ] `.claude/hooks/run-projection.py` — the ≤20-line renderer over the four inputs, reusing
      `habits-status.py`'s top-red selection rather than re-implementing it (§4)
- [ ] The cache: `(habits.yaml mtime, flows.jsonl size, saga source mtime)` key, <1s cached,
      degrade-to-empty on miss (§4)
- [ ] Hook registration in `.claude/settings.json` — `UserPromptSubmit` (plain stdout) and
      `SessionStart` (`hookSpecificOutput` wrapper), 10s timeout, no `source` matcher (§4)
- [ ] `RETIRE_WHEN` constant on the hook + the written load-bearingness stress test, authored
      **before** the projection ships (§4)
- [ ] The `class: inject` `.epr-meta` rule for the write-path discipline, with its own
      `retire-when:` (§5)
- [ ] `seam-registry.yaml` rows for the new decision surfaces — the note leg's target-resolution
      refusal arm and the projection's degrade-vs-emit decision — registered at birth per the
      registry's own honesty clause (`elohim/epr-rea/seam-registry.yaml:8-15`); this is p2p-gate
      Step 4's registration obligation, not optional documentation
- [ ] a2o scenario: **note-then-project round-trip** — a note appended in one session appears in the
      next session's projected block with no conversation carry (the graduation trigger)

## 7. Open questions

**Q1 — `occurred_at` discipline for an authored mid-run note.** Every timestamp on the projection
path is git-derived and never `now()`: `project.rs:351-356` states it outright (*"Deterministic and
history-derived like every other timestamp on this path — never `now()`"*, with the git author date
read at `:364`), and `fulfill` inherits its timestamp from the sprint report's `generated_at`
(`fulfill.rs:313`) rather than minting one. A note authored mid-run has **no commit yet** — the
discipline has no defined answer for a record that precedes its own commit. Proposal for the
epr-rea steward to rule on: RFC3339 **session time** with provenance carrying the session id,
sealed by the enclosing git commit later, so the record is honest about which clock it came from
instead of borrowing an unrelated commit's authority. **Resolved for v1 by the implementation plan
(Task 1 Step 3b): the git HEAD commit's date** — the note is dated by the tree it was authored
against, preserving the path's never-`now()` determinism. Known coarseness, accepted: notes
authored against the same head share a timestamp; intra-session ordering is carried by
`flows.jsonl` append order, and CID-dedup is unaffected (distinct reasons → distinct CIDs). The
session-time-plus-provenance alternative above stays recorded for the epr-rea steward, who may
still supersede the v1 choice when/if timestamp vocabulary is minted for this path.

**Q2 — should `run:observation` events be excluded from stock folds by default?** The `run-note`
unit already excludes them from every existing unit-keyed fold by construction
(`stock.rs:470-475`), so the question is not whether today's folds break — it is whether a *future*
fold that counts run-plane events should see observations at all, or only corrections and failed
approaches. Both readings are defensible: including them measures run-scale activity; excluding
them keeps the fold's subject to signals with a consequence. Recording it here rather than
answering it, because the default chosen becomes the semantics of the tag vocabulary.
