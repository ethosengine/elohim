---
title: Unified Memory Loop — One Scoreboard, One Loop (Two Readouts), One Ceremony
id: unified-memory-loop-design
status: Draft
class: process-meta
process_subdomain: memory
topic: [memory, stasis, converge, loop, ceremony, context-coverage, orchestration, cadence-convergence]
cites:
  - "converge-skill-design | the dreaming-to-execution loop this partially supersedes by folding it into one scoreboard | sha256:3034b991de8d3d87 | path: genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md"
  - "verification-result-index-design | the system-state store this loop reads to auto-resolve the back half of a claim | sha256:8d6b292dafc4a44e | path: genesis/docs/superpowers/specs/2026-06-01-verification-result-index-design.md"
  - genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md
  - "memory-lifecycle-design | the comet-shaped product seed whose vocabulary this loop dogfoods for doc hygiene | sha256:6a3557a18e2ceb09 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md"
  - "placement | the contract defining the three doc homes this loop tends toward stasis | sha256:95be31e6724bb9f5 | path: genesis/docs/PLACEMENT.md"
  - .claude/workflows/memory-stasis-loop.js
  - .claude/skills/memory-ceremony/SKILL.md
  - .claude/skills/memory-kit/SKILL.md
  - .claude/skills/converge/SKILL.md
supersedes_partial: genesis/docs/superpowers/specs/2026-05-10-converge-skill-design.md
---

# Unified Memory Loop — One Scoreboard, One Loop (Two Readouts), One Ceremony

> Current design direction: the [2026-09-09 revision](#2026-09-09--collective-memory-and-progressive-discovery)
> below supersedes the scoreboard-as-authority and gospel-rewrite-only framing.
> Earlier sections retain the historical proposal; the ceremony remains judgment,
> never a score. This revision serves `dev-system-equilibrium` and is not an implementation claim.

## The problem

Four things tend the same surface (the memory + doc graph) with the same four agents
(librarian, historian, cartographer, storyteller), with **no shared scoreboard** and
**redundant machinery**:

| Cadence | Agent / mode | Deliverable | Driver |
|---|---|---|---|
| `/hygiene-sweep` (memory-kit) | librarian — present | hygiene (budgets, dead cites, archive) | drift accumulators |
| `/converge` | cartographer — future | the "what's next?" ranked menu + plan edits | memory-kit reports |
| `/memory-ceremony` | storyteller-pen + 3 lenses — meaning | gospel-tier rewrites (1–2 surfaces) | `substrate-currency-audit.py` |
| `memory-stasis-loop` (new) | measurement → dispatches the above | the stasis score rising + backlog captured | `placement-audit.py --stasis` |

Three concrete redundancies fall out of that table:

1. **Two measurement scripts.** `placement-audit.py --stasis` (broad, whole graph) and
   `substrate-currency-audit.py` (narrow, ~60 gospel surfaces) both check citations and
   dead paths. Two scoreboards.
2. **Two orchestration loops.** `/converge` is a loop (measure corpus → cluster → dispatch
   agents → emit ranked menu); `memory-stasis-loop` is a loop (measure → dispatch agent →
   re-measure). Same skeleton, different scoring function.
3. **Four skills as entry-points over the same four agents.** The agents are the stable
   unit; the "cadences" are dispatch patterns over them. Four always-loaded skill
   descriptions for what are really *modes*.

## The decision

Collapse the surface to **three things, one of them shared**:

- **ONE scoreboard** — `placement-audit.py`, with `substrate-currency` folded in as a
  `gospel-currency` dimension.
- **ONE loop** — the drain readout ⟷ the menu readout of the *same* machine. Absorbs
  `memory-stasis-loop` + `/converge` + `/hygiene-sweep`'s dispatch.
- **ONE ceremony** — `/memory-ceremony`, the single human-gated judgment cadence. It stays
  **out** of the loop. The loop *triggers* it; the loop never *grades* it.

The dividing line, stated once: **everything automatable is the loop; the one thing that
needs a human's judgment and gate is the ceremony.**

## Why converge and the stasis-loop are one loop (the proof)

converge's five Phase-2 synthesis actions and the stasis-loop's pressure classes are the
**same operations** over the corpus:

| converge Phase-2 action | stasis-loop pressure class | the operation |
|---|---|---|
| add-as-outstanding | **capture** | surface latent work (decompose → gap-items / plan items) |
| mark-done | **claimed** → verify | grade a claim against evidence (verification-result index) |
| merge-redundant | **dedupe** | collapse overlap |
| remove-obsolete | **superseded** → history | distill + retire |
| surface-question | **needs-triage** | flag the undecided |

converge is the stasis-loop **scoped to plans and emitting a forward menu instead of a
score.** The only real differences are the **ranking function** (vision×readiness for a
human menu vs dimension-leverage for autopilot drain) and the **output artifact** (a ranked
next-menu vs a drained surface + score). Those are parameters and readouts of one loop, not
two loops.

## Component 1 — One scoreboard

`placement-audit.py` is the single measurement entry point. `substrate-currency-audit.py`
is folded in as one more dimension — **`gospel-currency`** — measuring drift on the ~60
gospel-tier surfaces (path-exists, process-status, missing-citation findings). It is not a
separate script the operator must remember; it is a column in `--stasis`.

This answers sub-question 3 directly: **substrate-currency IS subsumed by `--stasis`** — as
a dimension, not by deletion. The deep, narrow gospel audit becomes a measured slice of the
broad, shallow context-coverage score.

Implementation: either (a) `placement-audit.py` calls the substrate-currency logic and
ingests its findings as the `gospel-currency` dimension, or (b) the substrate-currency
checks migrate into `placement-audit.py` as a dimension function. (b) is cleaner long-term;
(a) is the cheaper first step. Decide at plan time.

## Component 2 — One loop, two readouts

The loop skeleton is unchanged from `memory-stasis-loop.js`:

```
measure scoreboard → rank pressure → dispatch the equipped agent
  (operator-gated APPLY for ops that mutate plans/gospel; autonomous for additive drains)
→ re-measure → repeat until convergence / stasis
```

It exposes **two readouts of the same machine**:

### 2a. Drain readout (autopilot)

- Ranks pressure by **dimension-leverage** (which coverage dimension is lowest / highest-ROI
  to drain). Whole-graph scope.
- Dispatches the equipped agent in **deterministic drain-mode**: capture → cartographer
  (decompose), needs-triage / mem-unlinked → librarian, superseded → historian,
  claimed → verification, regression → rework queue.
- These drains are **additive and reversible** (capture a gap-item, add a `cites:`, classify
  a status) → they run **autonomously**, no operator gate.
- Runs until convergence (numbers stop falling) or stasis (`at_stasis && uncaptured == 0`).

### 2b. Menu readout (human "what's next?")

- Re-ranks the **same captured backlog** by **vision×readiness** (converge's Phase 4 scoring,
  preserved verbatim). Plan-scoped.
- Emits the ranked **next-actions menu** the session-start UX consumes (converge's existing
  deliverable, unchanged in shape).
- The plan-mutating operations behind the menu (mark-done, remove-obsolete, merge-redundant)
  remain **operator-gated apply** — converge's Phase 3 gate survives the merge. The loop
  never autopilots a plan edit.

`/hygiene-sweep` = "run the loop, drain face." `/converge` = "ask the loop for its menu
face." Neither is a separate loop; both are faces of this one. This answers sub-question 1:
**the loop ABSORBS hygiene + converge as readouts** (they stop being separate skills); it
does **not** absorb the ceremony.

### The per-operation gate property

Gating is a property of the *operation*, not the *loop*:

| Operation | Gate |
|---|---|
| capture (decompose → gap-item), add `cites:`, classify status, dedupe-surface | autonomous |
| plan mark-done / remove-obsolete, archive/retire to history, gospel rewrite | operator-gated apply |

The loop carries this property per dispatch and pauses for the gate only where the op
mutates plans/gospel. (Workflows take no mid-run input — so the gated path returns proposals
and the operator gates them in conversation, exactly as the ceremony's workflow-mode already
does.)

## Component 3 — One ceremony (stays out; triggered, not graded)

`/memory-ceremony` is the single human-gated judgment cadence. It is the **only** thing in
the system that authors substrate-true gospel rewrites — which cannot be autopilot-graded
without re-creating the footgun its own SKILL.md deleted (*"the ceremony's measure is
rewrites delivered, not dimensions advanced"*; *"audit-number-as-success conflation"*).

The connection to the loop is **trigger, not grade**:

- **Trigger (the convergence-residual handoff).** The drain readout drives the deterministic
  dimensions to their ceiling and then **structurally converges below stasis** — because the
  judgment dimensions (`traceability` at 0.0%, `gospel-currency`) need authored judgment a
  script cannot produce. At convergence the loop **reports the residual gap, attributed to
  the floored dimensions**: *"converged at 0.62; the gap to 0.85 is traceability +
  gospel-currency — that's a /memory-ceremony job."* The residual is the signal; the operator
  invokes the ceremony.
- **Measure (unchanged).** Once running, the ceremony is measured by **rewrites-delivered +
  coherence-verify GREEN**, exactly as today. The dimensions it happens to raise are a
  read-only **after-effect indicator** on the scoreboard, never the ceremony's success
  measure. Trigger ≠ measure → the deliberate decoupling survives.

This answers sub-question 2: **ceremony gospel rewrites map to the traceability /
gospel-currency dimensions as a side-effect** — those dimensions are the ceremony's *trigger
signal* and a *post-hoc read-only indicator*, never its grade.

## What changes (migration)

1. **Scoreboard:** fold `substrate-currency-audit.py` into `placement-audit.py` as the
   `gospel-currency` dimension. Tune its manifest weight so the drain readout converges at a
   sensible floor (the floor is the trigger point for the ceremony — see Risks).
2. **Loop:** generalize `memory-stasis-loop.js` to carry the two readouts. Fold converge's
   Phase-4 vision×readiness scoring in as the **menu readout**; fold converge's plan-scoped
   synthesis (mark-done / add-outstanding / merge / remove / surface-question) in as the
   plan-scoped slice of the **drain readout**, preserving the operator-gated apply.
3. **Skills:** `/converge` and `/hygiene-sweep` become thin entry-points (aliases) that
   invoke the loop with a readout preset — or deprecated docs that point at the loop. This
   shrinks the always-loaded skill-description surface (a `skill-audit` win).
4. **Ceremony:** `/memory-ceremony` is unchanged except it gains an explicit "entered via
   loop convergence-residual" trigger note, and the loop gains a "ceremony-needed: <floored
   dimensions>" line in its convergence report.
5. **converge spec:** `2026-05-10-converge-skill-design.md` is **partially superseded** — its
   *separate-loop* framing is replaced by "menu readout of the unified loop," but its content
   (vision×readiness scoring, the menu shape, the operator-gated plan apply) survives intact.
   On landing, add `superseded_by:` pointer per PLACEMENT and a HISTORY record for the
   separate-loop framing.

## What we explicitly do NOT do (YAGNI / boundaries)

- **Do not fold the ceremony into the loop.** (Operator-ratified 2026-06-01.)
- **Do not re-introduce "dimension advanced" as the ceremony's success measure.** The
  ceremony is triggered by, never graded by, the score.
- **Do not delete the four agents.** They are the stable unit; the loop dispatches them by
  mode. The "cadences" were always just dispatch patterns over the four.
- **Do not autopilot plan edits or gospel rewrites.** Those keep their operator gate.

## Risks / open issues

- **`gospel-currency` weight tuning.** If weighted too high, the drain readout converges far
  below stasis and over-triggers the ceremony; too low and the ceremony is never signalled.
  The convergence floor IS the trigger threshold — tune the manifest weight so the floor
  lands where a ceremony is genuinely warranted. (Manifest-driven, no code change — proven by
  the `margin: 0.15→0.40` tuning.)
- **Menu readout still needs an agent.** vision×readiness is cartographer judgment, not pure
  determinism — the menu face dispatches the cartographer; only the drain face is
  fully autonomous.
- **converge's operator-gated plan apply must survive the merge** — the single biggest
  correctness risk in folding converge into the loop. The per-operation gate property
  (Component 2) is how it survives; verify it at plan time.
- **Alias vs delete for `/converge` + `/hygiene-sweep`.** Aliases preserve muscle memory and
  the session-start "what's next?" convention; deletion is cleaner but breaks references.
  Lean alias-then-deprecate.

## Acceptance criteria

- `placement-audit.py --stasis` reports a `gospel-currency` dimension; `substrate-currency-audit.py`
  is no longer a separate operator-invoked entry point (folded or aliased).
- The unified loop runs in both readouts: `drain` (autopilot, whole-graph, converges) and
  `menu` (emits the vision×readiness next-actions menu).
- The loop's convergence report names the floored judgment dimensions and explicitly says
  "ceremony-needed" when the residual is in `traceability` / `gospel-currency`.
- `/memory-ceremony` is unchanged in measure (rewrites + coherence-GREEN); the loop never
  writes a ceremony grade.
- `/converge` and `/hygiene-sweep` resolve to the unified loop (alias or deprecation pointer);
  the always-loaded skill-description surface shrinks.
- `2026-05-10-converge-skill-design.md` carries a `superseded_by:` pointer to this spec for
  its separate-loop framing.

## Sources

- `2026-05-10-converge-skill-design.md` — the loop being unified (menu readout origin).
- `2026-06-01-verification-result-index-design.md` — the claimed→verified drain (mark-done).
- `2026-05-28-in-flight-memory-coherence-design.md` — the cites edge / coherence dimension.
- `2026-05-10-memory-lifecycle-design.md` (CANONICAL) — comet-shaped lifecycle the scoreboard measures.
- `genesis/docs/PLACEMENT.md` — the contract the scoreboard enforces.
- `.claude/workflows/memory-stasis-loop.js` — the loop skeleton being generalized.
- `.claude/skills/{memory-ceremony,memory-kit,converge}/SKILL.md` — the cadences being collapsed.
- Operator dialogue 2026-06-01 — ratified: one loop (two readouts), ceremony stays out.

## 2026-09-09 — Collective memory and progressive discovery

### Boundary and purpose

EPRFS agent memory is the shared, governed memory capability of a collaborating
collective, first exercised in a local development workspace. Native agent memory
remains complementary: private working notes and runtime-managed stores are not
silently imported, made authoritative, or overwritten by package projection.
A new collective type or core EprKind is not established by this design.

Operator clarification: memory-kit is a transitional subsystem to be deprecated,
subsumed into two owners: the native-agent memory collective for shared knowledge
and collaboration, and EPRFS objects governed by `.epr-meta` for files, documents,
algorithms and code. Useful deterministic analyzers survive under those owners;
memory-kit does not survive as a third authority, cadence or queue. Retirement is
earned by replacement coverage, not by renaming the toolkit. The ceremony remains
the purpose-bearing tending experience spanning both owners.

The experience remains: orient → choose a concern → understand its story → follow
evidence → act → review → reconcile. The agent reads a compact, ephemeral projection
from governed memory. Purpose persists across navigation; history expands on demand.
The executor carries selection, provenance and lifecycle bookkeeping underneath.
Sacred Attention means warranted decisions with preserved uncertainty and less
avoidable investigation; lower token use or fewer files alone cannot establish it.

### Authority and subsystem ownership

| Surface | Authoritative home and integration boundary |
|---|---|
| Shared assertions and coherent topics | Existing content/claim/observation vocabulary, exact version references and evidence relationships; author/observer, steward, subject, governing scope and permitted reach remain distinct. Unresolved alternatives survive as alternatives. |
| Memory governance and selection recipes | Directory-local `.epr-meta` governance plus existing package/algorithm contracts. Declare defaults, selection, omissions, dependencies, authorship, permitted changes and method provenance. No unwired governance keys. |
| Ceremony and maintenance lifecycle | Existing Intent, Commitment, ProcessSpec, Process, fulfillment, review and acceptance records in EPR-REA. These record acts; they are not all core EprKind variants. |
| Context consumed in flight | Bounded recall/ceremony projections: purpose, selected assertion references, qualifications, omissions and expansion/actions. Their content is disposable; they confer no new authority. |
| Runtime memory integration | Adapters project selected managed context into a distinct runtime surface and accept proposed contributions back. Runtime-authored memory remains independently owned until explicit adoption. |
| Reports, boards and aggregates | Deterministic lenses over governed inputs and pinned observations. No writable parallel status or work queue. Judgment and opaque ranking remain attributed inputs, not invented deterministic explanations. |

An assertion is the smallest default contribution; a coherent topic can group it.
Content-derived versions retain derivation and supersession relationships. Paths,
run labels and scope names are locators, not evidence of identity or authority.
Concurrent contradictory contributions must not resolve by last writer, file mtime
or retrieval rank. The governing process selects an applicable version or records
that judgment remains open. A memory-specific package specialization is justified
only if selection, contribution, retention and projection semantics cannot be
expressed cleanly by existing packages. AgentDocPackage has useful machinery but
its current adapters assume agent instruction documents; portability is not proven.

### Holonic scope and graduation

Session, workspace, repository stewardship and wider collectives form governed,
potentially overlapping scopes. Reuse `.epr-meta` locality and inheritance for rules;
inheriting a rule never widens disclosure. Authorship, stewardship, membership,
subject rights, reach, version selection and byte availability are separate facts.
Subject-bearing derived memory inherits the standing of its inputs.

Push is a reach boundary at which a graduation can be evaluated, not an automatic
promotion of all local memory. A repository finding may graduate while private
investigation remains local. The proposed graduation names exact content, audience,
authorizing governance, attribution, review and available supporting evidence.
If necessary evidence cannot be disclosed, the receiving scope sees the limitation
and cannot receive an inflated claim of verification. Git tracking is not consent.
Local review is not peer attestation. Network publication needs its own existing
primitive mapping and gate before implementation; it is not delivered here.

### Ephemeral context with accountable provenance

Each context projection has a lightweight receipt connecting the actual reviewed
content to exact input versions, method version, purpose/scope and selection bounds.
Expandable detail explains selection where knowable, with uncertainty and material
omissions visible immediately. An opaque provider stays visibly opaque. Authorized
provider alternatives use the same contract without restructuring the ceremony.

Feedback targets either the source assertion or the projection: stale knowledge,
misleading compression, omitted contradiction and poor selection are different
failures. Feedback preserves the projection reference and the challenged passage;
correction reconciles affected relationships without rewriting prior decisions.
Ordinary projection bodies can expire. A consequential decision or challenge pins
sufficient exact context and provenance for review under the applicable retention
policy. A CID without retrievable bytes is insufficient; a receipt that exposes
private passages must not graduate merely because it is called metadata.

Continuation retains intent, baseline references, decisions, unresolved concerns,
visited evidence versions and the next useful action. Resume checks changed inputs
and reuses valid evidence rather than rerunning population audits. Projection
expiry must permit bounded regeneration or an honest unavailable result.

### Measurement and retirement

Reuse `genesis/scripts/memory-balance.sh`'s existing entry/close measurement purpose.
Bind an explicit baseline/close pair to each run, with method, scope, sampling window,
completeness and evidence references. Never pair with merely the latest snapshot.
Method changes require a new comparable series or an explicit non-comparison.
Native note timestamps currently derive from Git HEAD time; measurement evidence
must carry actual sampling times. Structured numerical ingestion is not implemented
by `flow note --kind observation`, which currently emits a unit run-note event.

Separate warranted outcomes and preserved uncertainty from attention costs and
physical burden. Count unique authored content, generated projections, retained
historical evidence and private operational storage separately. Report additions
as well as deletions; archival relocation is not net removal. Include the run's own
code/docs/reports, with a declared sampling cutoff and separately accounted closing
artifacts. Attribute shared-tree changes cautiously. Unknown token usage stays
unknown; bounded recall bytes do not account for direct shell reads or orchestration.

| Existing machinery | Disposition and retirement condition |
|---|---|
| Deterministic cleanup, dedupe, placement and currency analyzers | Keep as specialist lenses/actions; migrate observations and lifecycle references into the shared substrate. |
| Balance collector | Repair root/path-safe counting, authoritative/projection partitioning, bounded optional providers, snapshot identity and flag semantics before treating it as a trustworthy paired measure. Keep a compatibility entry point. |
| Memory-kit and ceremony instructions | Converge package-first around shared inputs, compatible ownership and cadence; remove contradictory rewrite limits, MEMORY.md editing guidance and repeat auditing after equivalent behavior is covered. |
| State ledgers, rankings and repeated report summaries | Derive from the same records; retire duplicate storage only after reader coverage and reproducibility are verified. Never mechanically label agent judgment deterministic. |
| Retention scripts and recall execution directories | Make reference-aware. Existing dated report deletion assumes Git recovery despite ignored directories; nondated recall-executions lacks that aging path. Preserve pinned evidence and verify actual recoverability before deletion. |
| Superseded topics and duplicate sections | Consolidate into evidence-bearing assertions and progressive links; retire originals only when provenance, qualification and discovery survive. |

The September ceremony has no contemporaneous paired balance snapshots; a later
snapshot cannot establish its net savings. Existing instruments are being integrated,
not retroactively treated as measurements they did not take.

### First integrated acceptance

Use one real shared concern and the existing ceremony entry point. Agent A contributes
an evidenced assertion; agent B arrives with only the context projection, recovers
purpose, follows evidence, recognizes a contested alternative and corrects either
the assertion or a misleading projection. Reset B's context and resume from the
continuation without repeating valid investigation. Review the outcome independently.

Exercise repository graduation with a local-only passage: the permitted finding and
its sufficient evidence can be selected, while restricted content and revealing
receipt data stay local. Exercise rejection, unavailable evidence, stale projection,
concurrent contradiction and replay; repeated reconciliation must not duplicate acts.
This is a local reach rehearsal, not proof of network enforcement.

Render familiar balance, stale-edge, placement and outcome reports from the same
pinned run inputs. Compare explicit baseline/close pairs and a controlled consolidation:
prove reduced active burden without losing discoverability or uncertainty, and count
retained storage plus measurement overhead. A fresh reader must explain both why an
edge was followed and what lay outside its view. Responsible unresolved completion
is valid; neither a score increase nor a one-item quota defines ceremony success.

### Design gate and remaining proof

Atlas routing: SDK composition owns artifact contracts; the client owns progressive
presentation; local runtime/resource governance owns bounded execution and retention.
This pass introduces private/local authored memory (B) and disposable projections (C).
No A/A2 entity, DHT entry, coordinator write, gossip signal or HTTP route is introduced:
new network heads at seed and one year are zero. No network stage is inferred from
local development. Later graduation must classify its actual payload and preserve
floor-protected authorization and counter-evidence at every declared network stage.

Concern canon: `.claude/epr-meta/concerns.yaml`, with active C2 and C6a enforcement
in `.claude/epr-meta/policies.yaml`. The integration has no contract tests yet; all
applicable concerns below are **partial**, not answered by this prose.

| Concern | Proposed boundary; remaining proof |
|---|---|
| C0 | Separate reach, declared version and availability; exercise local graduation. |
| C1 | Contribution/projection cannot self-accept; exercise independent decision paths. |
| C2 | Explicit version dependencies, no arrival-time authority; test concurrent replay. |
| C3 | Correct, contest, defer or stop; test a legal transition from each nonterminal state. |
| C4 | Distinguish absent, inaccessible, refused and unverifiable; test adapters. |
| C5 | Receipts and local reviews confer only their warranted scope; test misleading evidence. |
| C6a | Budget selection, providers, collection and retries; test cancellation and bounds. |
| C6b | Repeated contribution/reconciliation has no duplicate effect; test fixed points. |
| C7 | Advertised evidence and adapters must be available; test expired/missing bytes. |
| C8 | Retain selection provenance and typed, counted decision reasons; test correction routing and reason coverage. |
| C9 | Preserve actor/source lineage across adapters, key rotation and graduation; test attribution without orphaned state. |
| C10 | Pin method/schema versions; reject or count unknown fields; test incompatible snapshots. |
| C11 | Declare and count defer/refusal policy under external pressure; test backpressure and recovery. |
| C12 | Verify authorization at the acting boundary for content and receipts; test refusal paths. |
| C13 | Local acceptance does not imply network authority; successor and re-evaluation must be named at the graduation gate. Network mapping remains unbound. |
| C14 | Retain contested and unmeasured residuals; unexpected failures use existing runtime-findings/triage intake. Test witnessed stopping and recovery without a second incident queue. |

Implementation should extend existing EPRFS/recall/package seams under the existing
habit, not create another memory queue or a second overarching design. Relevant
source contracts are the doc-lifecycle-as-epr-development-substrate design, the
digital-memory-standing-ontology design, `elohim/epr/src/kind.rs`,
`elohim/epr-rea/src/model.rs` and the existing package projection machinery.
