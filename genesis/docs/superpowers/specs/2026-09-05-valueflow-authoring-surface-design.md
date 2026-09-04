---
title: "Valueflow Authoring Surface: claim, fulfil, rule, and the one-screen context"
id: valueflow-authoring-surface-design
status: Draft
class: design
domain: D9
sprint: valueflow-authoring
requires_env: [household-nodes]
context-tier: disclosed
steward: orchestrator
graduation-trigger: decompose-complete OR superseded-by-implementation
cites:
  - "epr-rea-valueflow-fabric | the three-layer fabric (knowledge, plan, observation) this spec adds a fourth standard layer to and whose Intent/Commitment/FlowEvent atoms the new verbs mint | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "run-plane-projection-observation-events | the note event this spec extends by two kinds (ruling, verdict) without moving any existing note content address | sha256:0781aecc53d8a0d0 | path: genesis/docs/superpowers/specs/2026-08-13-run-plane-projection-observation-events-design.md"
  - "actor-plane-inflight-identity-claims-design | the three-arm actor resolution (--as, session claim, git author) and the steward slot that claim and fulfill both reuse unchanged | sha256:6a6dee8249ae76ef | path: genesis/docs/superpowers/specs/2026-08-15-actor-plane-inflight-identity-claims-design.md"
  - "dev-system-equilibrium-stocks | the habit this slice serves: its outflow classifier keys on fulfills, which is why only DONE and DONE_WITH_CONCERNS discharge | sha256:5306c437d02200f2 | path: genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md"
  - "commitment-dispatch-puller | the designed-but-unauthorized puller this slice explicitly leaves out of scope while filling its identity hole | sha256:608803ebc8811e4a | path: genesis/docs/superpowers/specs/2026-08-13-commitment-dispatch-puller-design.md"
  - genesis/docs/content/elohim-protocol/value_scanner/epic.md
  - .epr-meta/habits-covenant.md
  - .epr-meta/dev-system-equilibrium.habit.md
---

# Valueflow Authoring Surface

## 0. What this is

The developer valueflow already projects intents and commitments from the repository, and it
already records notes and a2o fulfilments. What it cannot do is let an actor take one task,
discharge it from a task report, record a control decision as a record rather than as prose,
or hand a fresh agent the whole picture of one atom in a single read. This spec adds two write
verbs (`claim`, `fulfill --on`), one read verb (`context`), two note kinds (`ruling`,
`verdict`), three governance inject rules, and three native skill packages. Nothing new is
trained into a model. The method lives in the tool surface and in the skill, so the protocol
verbs stay the only designed friction and everything around them gets out of the way.

## 1. The concern and the measured friction

The REA authorship loop is intend, claim, produce, verify, fulfil, rule, ratchet. That loop is
the protocol's process, not a description of it. The lowest-level home for the loop is the
`epr` CLI (crate `elohim/eprfs/epr-cli`, ontology in `elohim/epr-rea`), then the `.epr-meta`
governance layer that says where authoring happens, then native skill packages projected to
every runtime.

**Why the friction sits where it sits (operator, 2026-09-05).** Friction is purposeful exactly
where a standard is held against individual bias: the seal, the review seat, the evidence-only
status flip, the notarized path, and the floor and ceiling bounds a scope declares on a stock.
Those are the places where one actor's perception could otherwise hijack the whole, so the
protocol makes them cost attention, witness and evidence on purpose. Everything else must be
frictionless, and not as a convenience: the marginal cost of applied knowledge trends toward
zero, and the substrate's performance edge is what lets the sensing (the learning pillar's
mastery paths, the psychometric instruments that read the perceived and ideal curves, the
register that reads reality) run continuously enough for policy to follow the readings. Slow
sensing is stale policy. So the verbs stay expensive and the surround (tool discovery, prompt
assembly, ledger duplication, context re-derivation) is driven toward zero.

Five frictions were measured while running the Holochain Evolution Epic between 2026-09-03 and
2026-09-05. Each one is a hole this slice fills.

1. Gap items become intents (`src/flow/project.rs:649` derives one `Intent` per gap item), but
   no verb claims an intent with an actor, and no verb fulfils a gap-item commitment from a
   task report. `fulfill` today reads only a2o sprint reports and discharges only
   `a2o:scenario-green` commitments. 614 commitments stand open. Task-level work has no drain
   path at all.
2. Notes exist (`src/flow/note.rs`) and land as `Cite` events on any atom, but nothing reads
   them back. `walk` filters lineage to `Produce`, so notes are invisible to every reader.
   Rulings therefore went to prose in three places at once: a gitignored `progress.md`, a spec
   section, and memory.
3. Every implementer and reviewer dispatch restated roughly two thousand tokens of constants:
   the cargo recipe, the commit trailers, the report contract, the admissibility clause.
4. A fresh agent re-derives "what is open on this atom, what seals it, what habit covers it,
   which gate runs it" by grepping, every single time.
5. The epic plan's gap-item file was stale. Fifteen items showed OPEN and Tasks 16 through 20
   had no items, because the decompose step was never re-run after the plan grew.

## 2. Vision grounding: the Scanner loop is this loop

The Value Scanner epic states the protocol loop as Story, Scan, Negotiate, Bundle, Story, and
its one rule is that Tommy does not log anything. The Elohim observes and records the story.
Care becomes computable, then valuable, then cultivatable. The evening story is a celebration
projection, never the record. The record is the REA flow the bundle minted.

The developer valueflow is the dogfood instance of exactly that loop. The surface must feel
like it:

| Scanner phase | Dev valueflow | Surface |
|---|---|---|
| Story (the mission assembles itself) | the atom's open intents, its habit, its gate, its rulings | `epr flow context <atom>`, the mission on one screen |
| Scan (each item, in context) | an actor takes a task | `epr flow claim`, where the brief IS the claim |
| Negotiate (personal, family, community, global) | implementer, reviewer, controller, operator | `note --kind verdict` (audit) and `note --kind ruling` (control) |
| Bundle (one code carrying payment, tokens, story, REA) | report plus commit SHAs plus gate evidence resolve into ONE event | `epr flow fulfill --on ... --report ...` |
| Story (evening celebration) | progress notes, spec sections, sprint result, habit delta | projections of the flow (slice 2) |

Four consequences the design honours.

**The actor does the act; the tool mints the record.** The human or agent writes the brief, the
code, and the report. The tool mints the commitment and the event. In this slice each seat skill
ends in exactly one verb, which is the bundle. In slice 2 the `.epr-meta` observer mints on
write, so nobody logs at all. That is the camera that sees.

**Ephemeral evidence never enters the flow.** Build logs and cargo output stay outside. The
event carries the evidence's content address and the gate line. That is observation graduating
to REA, not surveillance.

**Progressive permissions are reach.** An `agent:implementer@...` claim can fulfil. Only a
ruling flips a habit status. Only the operator ranks the register. The shape is the same one
that carries a person from age five to age fourteen.

**Ledger and play are different things.** `context` and `stocks` are the record: drain against
inflow, attribution through the steward slot. Tokens, badges, and the evening celebration are
play, which is a first-class human need. Play stays honest because every token is a projection
of a real REA flow, and reach bounds each player's exposure (operator ruling, 2026-09-05).
Slice-2 celebration projections may be playful. They never become the record.

## 3. Where habits sit in the REA, VSM, and EPR schema

The fabric spec has three layers: knowledge (process specs and recipes), plan (intents and
commitments), and observation (flow events). The habit register was born after that spec and is
none of those three, and the fabric spec never names it. This section states the placement.

**A habit is a standard. It is not an intent and it is not an event.** In Meadows' vocabulary a
habit is the goal, the reference level of a balancing loop over a stock, and it carries its own
sensor in its `checks:` field. The covenant's best-observed ratchet is her instruction to let
standards be enhanced by the best actual performance. In Beer's vocabulary a habit is System 5
identity, what this system reliably does, measured through System 3\* audit. Its `status` is the
algedonic signal. Its `active: true` is System 3 attention allocation, and the two-habit WIP
fence is a bound on the attention stock. In EPR terms a habit is an `.epr-meta` atom,
content-addressed by its body like any other file, declared where the behaviour lives, and
scoped by the cascade.

**In REA terms a habit is a scope.** It is ValueFlows `in_scope_of`, the container accountable
for a flow. Work that serves a habit is accounted to it, and its accounting is the observed
status, never the sum of events. Its `checks:` bind through the `@concern:` tag to validation
commitments whose fulfilment is the evidence. A status flip is a ruling with evidence, which is
covenant rule 4, so a flip is `note --on <habit atom> --kind ruling`, and the delta line in the
habit's evidence ledger is that ruling's prose projection.

So the fabric has four layers, not three: knowledge (how value moves), plan (what we promise),
observation (what happened), and standard (what we hold ourselves to). `.epr-meta` is the
governance layer across all four, saying where each is declared and which signal fires at
authoring time.

What this slice does with that placement is deliberately small and honest.

- `claim ... --serves <habit-id>` adds a `habit:<id>` slot. `in_scope_of` stays the plan
  document, so `walk`'s scoped-intents contract does not move. The flag is refused when the id
  is not in the register.
- `context <path>/<id>.habit.md` renders the habit as a scope: its status, its active flag, its
  checks, the open commitments carrying its `habit:` slot, the specs and plans whose `refs:` or
  cites name it, and its notes newest first.
- This section is the placement of record. Slice-2 graduations are named in section 12.

## 4. Ontology: two new note kinds, closed enum stays closed

`NoteKind` in `src/flow/note.rs:100` grows by two members. Nothing else in the ontology moves.

**`ruling`** is a control decision, System 3 and System 5 in VSM terms: accept, defer, order a
fix round, hold. Its slot-0 tag is `run:ruling`. The `--reason` text IS the ruling.

**`verdict`** is an audit outcome, System 3\*: a review seat's outcome on a delivery. Its
slot-0 tag is `run:verdict`. It requires `--verdict approved|changes-requested`, carried as a
positional slot `verdict:<value>` placed after the `reason:` slot. A verdict without the flag is
refused. The flag on any other kind is refused.

No new atom family lands in `epr-rea`. A note stays a `Cite` flow event with an empty `fulfills`
list, which is load bearing: notes never discharge anything, and the equilibrium outflow
classifier keys on `fulfills`. The slot vocabulary stays positional and additive, so every
existing note keeps its content address.

## 5. Write verbs

### 5.1 `epr flow claim`

```
epr flow claim --on <intent-cid | gap-id | path> --as agent:<role>@<model>
               [--brief <path>] [--serves <habit-id>] [--session <id>]
               [--supersede] [--json] [--root DIR]
```

**Resolution of `--on`**, in this order. A content address known to the sidecar, which must
resolve to an `Intent` or the call is refused. A gap id, matching the intent whose
`classified_as` slot 1 equals it, for example `plans__2026-09-04-...#16`. A repository path,
resolved to the document's canonical body address, then to the intents scoped by it; more than
one match is refused with all candidates named, because guessing which task an author meant is
the one thing a claim must never do.

**What it mints.** One `Commitment` (`elohim/epr-rea/src/model.rs:352`) with `state: Active`,
`provider` set to the resolved actor, `satisfies: [intent_cid]`, `in_scope_of` copied from the
intent, and `classified_as` carrying `gap:claimed`, the gap id, an optional
`brief:<body address of --brief>`, an optional `habit:<id>`, and finally
`steward:<git author email>`.

**Refusals.** An active, undischarged commitment already satisfying that intent refuses the
claim and names the incumbent, including the `tool:decompose-claim` commitments that the Python
decompose step still mints for items in the CLAIMED state. `--supersede` mints anyway and reports what it
superseded. Identity is the atom address, so re-running the same claim is a no-op.

**Actor resolution** is the actor plane's three arms: `--as`, then the session claim, then the
git author. In practice `--as` is required, because a claim without an actor is not a claim.

**Outcome.** `ClaimOutcome { intent, commitment_cid, provider, brief, appended, superseded_by }`,
rendered human-readably by default and as JSON under `--json`.

### 5.2 `epr flow fulfill --on`, a second arm on one verb

The existing positional form, `epr flow fulfill <report.json>` over an a2o sprint report, is
untouched. The new flag form is:

```
epr flow fulfill --on <commitment-cid | gap-id> --report <path>
                 --status DONE|DONE_WITH_CONCERNS [--commit <sha>]...
                 [--as agent:<role>@<model>] [--session <id>] [--json] [--root DIR]
```

It emits one `Produce` flow event (`elohim/epr-rea/src/model.rs:377`) with unit `task-report`,
`fulfills: [commitment_cid]`, and `classified_as` carrying `report:<status>`, the label, an
`evidence:<body address of the report>` slot, one `commit:<sha>` slot per supplied SHA, and the
trailing `steward:` slot. `occurred_at` is the git HEAD author date, exactly as `note` sources
it, never a wall clock.

`NEEDS_CONTEXT`, `BLOCKED`, and `HOLD` are refused, with the hint to record them as
`note --kind observation`. Only `DONE` and `DONE_WITH_CONCERNS` discharge. This keeps the
dev-system-equilibrium guard honest, because that habit's outflow classifier keys on `fulfills`
and a non-discharging status must never look like a drain.

Gap-id resolution picks the newest active commitment whose `classified_as` carries that id.
Identity is the atom address, so a second fulfilment of an already discharged commitment reports
`already_fulfilled` and appends nothing.

## 6. Read verb: `epr flow context`

```
epr flow context <path | cid> [--notes N] [--json] [--root DIR]
```

`--notes` defaults to 5. The library entry point is
`pub fn context(root, target) -> FlowResult<ContextResult>`, so tests call the library the way
`tests/flow_edges.rs` already calls `walk`. This is the one screen a fresh agent otherwise
re-derives by hand. Eight sections, in this order.

1. **Identity.** Repository path, canonical body address, and the labels the sidecar carries.
2. **Intents.** Open intents scoped by this atom, taken from `walk`'s `scoped_intents`, each with
   its gap id and its state slot.
3. **Commitments.** Undischarged commitments on the atom: provider, state, brief slot, and the
   latest associated event. The occurred-at-then-append-order rule already implemented at
   `src/flow/fulfill.rs:474` is lifted to a shared module and reused here rather than copied,
   because two readers disagreeing about "latest" is precisely the bug that rule exists to
   prevent.
4. **Notes.** Newest first, `N` of them, each showing kind, actor, steward, reason, switched-to,
   and verdict slots. Read straight from the store as `Cite` events whose resource equals the
   atom's address and whose unit is `run-note`. `walk`'s `Produce` filter is NOT changed, so its
   JSON contract stays stable for every existing consumer.
5. **Seals.** Outgoing edges with their verdict (Ok, Governed, Stale, Held, Dangling) and the
   stale downstream count.
6. **Habit.** Entries of the generated register whose `checks:` or `refs:` mention the atom's
   path, plus any habit declared in the nearest ancestor `.epr-meta` directory. This is the first
   Rust reader of the register. The YAML file is the generated projection and is read as data.
   Printed: id, status, active, and the first check.
7. **Gate.** Walk up from the atom's directory to the nearest `build-manifest.json`, then take
   the `gate.projects` entry whose `dir` is a prefix of the atom's path, and print
   `just gate <name>` together with its cargo target directory and rustflags when declared. This
   is the first Rust reader of `gate.projects`.
8. **Governance.** The authority and cascade counts that `epr explain` already computes, reused
   rather than duplicated.

The human render stays at or under 40 lines. `--json` emits the whole `ContextResult`. A
content-address target skips sections 5 through 8, because those four sections are properties of
a file in the tree and a bare address has no path.

The JSON shape sketch:

```
ContextResult {
  identity:    { path, cid, labels[] },
  intents:     [ { cid, gap_id, state, raised_by } ],
  commitments: [ { cid, provider, state, brief, habit, latest_event } ],
  notes:       [ { cid, kind, actor, steward, reason, switched_to, verdict, occurred_at } ],
  seals:       { edges[], stale_downstream },
  habits:      [ { id, status, active, first_check, source } ],
  gate:        { project, command, target_dir, rustflags },
  governance:  { authority, cascade_depth, rule_count }
}
```

## 7. Governance: three inject rules where authoring happens

The root `.epr-meta/manifest.md` gains three rules of class `inject`, the lightest signal in the
ladder. The root is the right home because the concern is repository-wide developer-valueflow
authoring, and the root already hosts habit-declaration-at-birth. Each rule's `why` names the
verb the author should run.

- On a write to `.superpowers/sdd/**/task-*-brief.md`: a brief is a claim. Run
  `epr flow claim --on <gap-id> --as agent:implementer@<model> --brief <this file>`.
- On a write to `.superpowers/sdd/**/task-*-report.md`: a report is a fulfilment. Run
  `epr flow fulfill --on <gap-id> --report <this file> --status <DONE|DONE_WITH_CONCERNS>`.
- On a write to `.superpowers/sdd/**/progress.md`: rulings are notes. Run
  `epr flow note --on <gap-id or plan> --kind ruling --reason '...'`. The progress file is a
  projection, never the record.

Those paths are gitignored. Whether the compose gate fires on an ignored path is a question of
fact, not of design, so the implementation task verifies it against the live hook and records
the answer. If the hook skips ignored paths, the rules still document the pattern for a human
reader and the plan says so plainly rather than claiming an enforcement that does not run.

## 8. Skills: three native packages, one dispatch prompt shape

Three skill packages are authored under `.epr-meta/elohim/packages/skills/` with
`sourceRuntime: elohim-agent` and `master: package`, which makes the package JSON the source of
truth and the runtime files generated projections. Native packages get no generated governance
block, so each one is hand-written: `eprRef: epr:elohim-agent/skills/<id>`, policy
`capability-governance@1`, gates `epr-meta-resolver` and `elohim-agent:packages:verify`, ledger
`.claude/data/governance-findings.jsonl`.

No parameter ABI exists and none is added. The brief path, the commitment, the rulings in force,
and the base commit ARE the parameters, and they are passed in the dispatch prompt. The skill
body is the constant eighty percent.

**`valueflow-authoring`** is orchestrator-facing. It states the method verb by verb, and each
verb is one command: intend (decompose, then `epr flow project`), claim, produce (dispatch an
implementer with the implementer skill), verify (dispatch a reviewer with the reviewer skill),
fulfil, rule (`note --kind ruling`), ratchet (a habit delta line, then re-project the register).
It states the friction principle, the WIP fence, the dispatch prompt shape
("Invoke skill X. Brief: `<path>`. Commitment: `<gap-id>`. Rulings in force: ... Base: `<sha>`."),
and what not to do, which is restating constants or writing rulings as prose.

**`valueflow-implementer`** is a seat contract. Run `epr flow context` on each file the brief
names, before anything else. Claim the cargo berth before cargo and release it after. The gate
is the gate line the context printed, never a hand-typed recipe. Echo the exit status on its own
line. Carry the commit trailers. Report under the five statuses with a mandatory gate-evidence
line. Never ask the user. End with one verb: `epr flow fulfill --on ... --report ... --status ...
--as agent:implementer@<model>`, or `note --kind observation` for the three non-discharging
statuses.

**`valueflow-reviewer`** is the other seat contract. The review package is a diff of base against
head plus the report. The admissibility clause bounds findings to what is fixable in the tree:
no history rewrites, no demands for test-driven-development evidence after the fact. Findings are
tiered Important or Minor. Conformance to the spec comes first. A missing gate line is Important.
End with one verb: `epr flow note --on <gap-id> --kind verdict --verdict approved|changes-requested
--as agent:reviewer@<model>`.

## 9. Out of scope, named

- Turning the progress file and the epic spec's status section into generated projections. That
  is slice 2, and its verb will be `epr flow ledger <atom>`.
- The commitment-dispatch puller. Designed, not authorized, not built here.
- Changing `walk`'s `Produce` filter. Its JSON contract stays where it is.
- A parameterised skill ABI.
- Changes to the Python decompose step. Its CLAIMED state keeps minting `tool:decompose-claim`
  commitments, and `claim`'s duplicate refusal names them rather than racing them.

## 10. Habit served, gate, install, dogfood evidence

**Habit served:** `dev-system-equilibrium`, declared at `.epr-meta/dev-system-equilibrium.habit.md`.
It is red because commitments fill far faster than they drain, and task-level fulfilment is the
missing drain. Closing this slice means one delta line in that atom and a re-projection of the
register. No status flip: one dogfood run is an event, not a rate.

**Gate:** `just gate eprfs`, declared in `elohim/eprfs/build-manifest.json` under
`gate.projects`. It runs format check, clippy across the workspace and all targets with warnings
denied, and the workspace test suite, with empty rustflags and a target directory at
`/tmp/eprfs-gate-target`. Claim the cargo berth first, because other sessions hold that lease
intermittently and the RAM guard sheds builds under pressure.

**Install:** `cargo install --path elohim/eprfs/epr-cli --root /opt/rust/cargo --target-dir
/tmp/eprfs-gate-target`. The container runs as root and `/opt/rust/cargo/bin/epr` is the
devspace-installed binary that every skill and hook invokes.

**Dogfood evidence, which is the acceptance:** re-run decompose on the Holochain Evolution Epic
MVP plan so Tasks 16 through 20 exist as intents; project; claim and fulfil the three landed
tasks (16, 17, 18) with their commit addresses and an implementer actor; record one ruling and
one verdict; run `epr flow context` on the plan and see all of it on one screen; and read the
commitments stock before and after so the drain is witnessed rather than asserted. Appends to the
flow sidecar are idempotent by content address and append-only, so this runs safely beside the
epic's own session.

## 11. Story

One a2o feature, `genesis/a2o/features/devflow/valueflow-authoring.feature`, tagged
`@concern:valueflow-authoring`. Four scenarios:

1. A claim on a gap id mints exactly one commitment, and a second claim on the same gap id is
   refused with the incumbent named.
2. A report with status DONE discharges the commitment, and a report with status BLOCKED does
   not.
3. A ruling and a verdict are both readable in `context`, newest first, with the verdict slot
   rendered.
4. `context` on a storage source file names the habit that covers it and the gate that runs it.

The feature carries `@wip` if no step definitions drive it, which is the honest state that the
run-plane feature already models. The Rust tests under `epr-cli/tests/` must cover the same four
assertions regardless, because the CLI-level test is the executing proof while the story waits
for its wiring. The a2o governance manifest attaches a context-blind reader review obligation to
any feature write, and that loop runs on this file.

## 12. Open questions

None. The decisions in sections 1 through 11 are closed and this spec is their record. What
remains is not an open question but a named slice-2 graduation:

- Habit status flips read as rulings by the register projector, so the ruling event becomes the
  authority and the delta line becomes its prose projection.
- The WIP fence expressed as a bounded commitment, using the bound vocabulary the fabric already
  carries, so attention allocation is measured rather than remembered.
- Habit delta lines generated from the flow rather than hand-written.
- `epr flow ledger <atom>`, which makes the progress file and the epic's status section
  generated projections.
- The `.epr-meta` observer minting claims and fulfilments on write, so nobody logs at all.
