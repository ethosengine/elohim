---
title: "Spec/Plan Compaction Loop — Born-Linked Front, Self-Dissolving Back, Curated-Present History"
id: spec-plan-compaction-loop-design
status: Draft
created: 2026-06-02
tier: design-spec
class: process-meta
process_subdomain: doc-lifecycle
topic: [memory, compaction, lifecycle, decompose, stasis, comet, history, trajectory, born-linked, mempalace, no-dumping-grounds, pickup, session-surfacing, hooks]
cites:
  - unified-memory-loop-design | the loop this rides while correcting its single stasis metric to the three-zone comet shape | sha256:99100efd20d10129
  - memory-lifecycle-design | the product seed supplying the compact/merge/memorialize primitives this loop dogfoods for docs | sha256:b6545e6548573fa4
  - placement | the contract whose retired-language and doc homes this loop proposes six edits to | sha256:f84d7cb16bea9379
  - verification-result-index-design | the state store whose verification results gate when a spec may self-dissolve | sha256:8d6b292dafc4a44e
  - genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md
  - .claude/scripts/memory-kit/LIFECYCLE.md
  - converge-skill-design | the session-start what's-next moment whose manual surfacing step the PICKUP fire point (§4b) makes deterministic | sha256:3034b991de8d3d87
refines:
  - genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md  # rides its loop machinery; corrects its stasis metric + history model
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md  # canonical primitives: compact / merge / close-interval / memorialize / forget
canonical_seed: genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md
# DESIGN-STAGE: the edges below are PROPOSED, not yet applied. PLACEMENT.md still contains its
# `_retired/` language and the unified-loop spec still carries its composite stasis readout. This
# spec must not claim landed edges it has not made (its own front-link-reflects-reality discipline).
proposed_amendments:
  - genesis/docs/PLACEMENT.md   # PROPOSED — see §10.4 for the exact six edits; not yet applied
proposed_corrections:
  - spec: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md
    correction: "STASIS METRIC — replace the single P=T/H-style coverage composite (its `--stasis` composite-vs-benchmark model, `placement-audit.py` stasis_mode L533, with the PROPOSED `gospel-currency` dimension named in the unified-loop spec, L48–90) with the THREE-ZONE COMET SHAPE: a BLOAT shrink-target (ACTIVE pile vs canonical), a MUSEUM grow-target (curated history vs narrative), and a never-force-shrunk WORKING-MEMORY budget zone (§8)."
    status: PROPOSED (design-stage; not yet applied)
  - spec: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md
    correction: "HISTORY MODEL — replace the inherited 'archive/retire to history' sink with curated-present-pointer + decompose-to-zero-residue, retiring `history/_retired/` and the `.claude/archive/` junk-drawer as terminal destinations."
    status: PROPOSED (design-stage; not yet applied)
spawned_backlog:
  - placement-enforcement hook (.claude/hooks/, NOT YET BUILT)
  - delivery-status-poll (delivery-status-distribution.py SessionStart wiring, NOT YET BUILT)
  - pickup-semantic-surfacing hook (§4b — .claude/hooks/pickup-semantic-surfacing.py + settings.json UserPromptSubmit/PreToolUse wiring, BUILT 2026-06-04; gate paths verified by synthetic stdin tests, live-session efficacy pending next session start)
---

# Spec/Plan Compaction Loop

A discipline that wraps the superpowers lifecycle (`/brainstorm` → `superpowers:writing-plans` →
`superpowers:executing-plans`) so that spec and plan docs are **born linked** to the canonical seed they
descend from and **dissolve to zero residue** when their work concludes — keeping the doc corpus an
intentional, vision-true *memory* instead of a dump. It is not greenfield: it operationalizes substrate already
designed in `2026-06-01-unified-memory-loop-design.md` (the loop machinery) and
`2026-05-10-memory-lifecycle-design.md` (the lifecycle primitives), and it composes with them as a sibling, not
a fork.

## 1. The problem

The doc corpus is an **inverted comet**. A comet should be a small bright head (the live leading edge) trailing
a thin, distilled tail (memory). Ours is upside down.

The leading edge is enormous and healthy. The *canonical* head is **30 architecture docs (923 KB)** under
`genesis/docs/content/elohim-protocol/architecture/`; the full vision/narrative corpus (all of
`genesis/docs/content/elohim-protocol/`) is **219 docs (2.8 MB)**; and the codebase is **113,190 LOC**
(src-filtered, excluding `target/` and `node_modules/`) across `app/`, `elohim/`, `doorway/`, `steward/`, the
crates, and `bridges/`. (The real a2o `.feature` files are only **326 KB** — the prior "33.3 MB vision corpus"
was a miscount that swept the whole `genesis/a2o` directory of reports and screenshots into the head.) That head
is the thing memory exists to serve.

The tail that should trail it is almost nonexistent where it counts, and bloated where it shouldn't be:

- **Curated history**: `genesis/docs/content/elohim-protocol/history/` holds **4 docs / 13 KB** (`INDEX.md`
  plus three lessons). Against the 2.8 MB narrative corpus that is **0.46%** — the museum is essentially empty
  (it should *grow*, not shrink).
- **Active pile**: **134 docs / 5.7 MB** — `superpowers/specs` (44), `superpowers/plans` (63),
  `genesis/docs/plans` (27). (134 includes *this* spec; the count is a point-in-time snapshot and moves every
  time the FRONT fire point opens or the BACK fire point dissolves.) Against canonical architecture that is
  **6.2×**, and against all narrative **~2×** — the workbench is larger than the bench it sits on. These are not
  memory; they are *parked intentions*. Plans that landed, plans that died, and plans that forked off the same
  topic all sit in the live tree with equal weight, indistinguishable from work actually in flight. **This pile,
  and only this pile, is the bloat the loop shrinks.**
- **Archive junk-drawer**: `.claude/archive/` holds **77 files / 3.3 MB** — a date-stamped sink that nothing
  curates and nothing reads. It is **~254× larger than the curated museum** (3.3 MB vs 13 KB). The thing we
  throw away is two orders of magnitude bigger than the thing we keep on purpose.

So the corpus remembers by *accumulation*, not by *distillation*. The superpowers lifecycle is the dump pump:
every cycle emits a spec and a plan, both written as standalone artifacts, neither linked to the canonical seed
they descend from, and neither dissolved when the work concludes. The pile grows monotonically.
`genesis/docs/PLACEMENT.md` already names the three homes (CANONICAL = `content/elohim-protocol/architecture/`,
HISTORY = `content/elohim-protocol/history/`, ACTIVE = `superpowers/specs`+`plans`) — but nothing *moves* docs
between them at lifecycle boundaries, so ACTIVE is where everything lives and dies.

The cost is not merely disk. It is that **the corpus stops being a vision-true memory and becomes a search
hazard.** A planner opening a new topic cannot find the canonical seed underneath the dupes, and re-treads
anti-patterns whose lessons are buried in dead plans. The surfacing probe for *this very spec* demonstrated the
failure mode directly: the lexical `spec-coherence-index` (token-overlap) returned **0 matches** for
"decompose-self / dump / forget" while the canonical prior art — `2026-05-10-memory-lifecycle-design.md`, which
defines exactly those operations under the names compact / forget — sat right there. Same concept, different
vocabulary, therefore **invisible.** A memory you cannot surface at the point of decision is not memory; it is
sediment.

A burndown map of the active pile (40 clusters) shows the shape of the cleanup: **7 dead-arch clusters (~27
docs)** collapse to curated-history lessons (the biggest, "EPR Codec & Storage Foundation," is **13 docs → 1
lesson**); **7 multi-thread-dupe clusters (~19 docs)** collapse to **7 canonical seeds**; **4 mixed clusters
(16 docs)** split per-doc; **22 live-hot clusters (~71 docs)** stay on the lean edge. Projection: **the pile →
~71 hot + ~7 new seeds**, with **~43 docs compacting into a handful of curated lessons.** (The cluster map was
produced over a 133-doc snapshot; the pile is 134 today, and the FRONT fire point guarantees the count keeps
moving — "0 unassigned" was true *of that snapshot*, not a structural invariant.) The pile is not a fact of
nature. It is the absence of a back-pressure mechanism.

## 2. Reconciliation — this composes, it does not fork

This is a **born-linked sibling** of two existing documents, not a replacement for either. That this section
exists *at all* — opening by surfacing the canonical seeds it descends from and declaring the lineage before
proposing anything — is the structural dogfood of Principle 1: a new spec is born linked to its seeds,
compose-don't-fork enforced. The lineage claim is real; the *numbers* are a separate obligation, and **every
byte/doc/ratio in this revision was re-verified against disk on 2026-06-02** (the prior draft carried a
miscounted 33.3 MB head and a 1:2,642 inversion sourced from sweeping `genesis/a2o` reports into the vision
corpus — both are struck here). The discipline only holds if the front-links *and* the measurements reflect
reality; a born-linked front that cites stale numbers is exactly the failure mode this loop exists to refuse.

**What it COMPOSES (rides on, does not reinvent):**

- **The loop machinery** — `2026-06-01-unified-memory-loop-design.md` (commit 56e440536). That spec settled *one
  scoreboard* (`placement-audit`), the drain + menu readouts, and ceremony-as-trigger. The Compaction Loop is a
  discipline that **fires at two points on that same loop**; it adds no second scoreboard and no parallel
  cadence. Where the unified loop drives toward stasis, this spec defines *what born-linked creation and
  zero-residue dissolution mean as the loop's two fire points.*
- **The primitives** — `2026-05-10-memory-lifecycle-design.md` (CANONICAL) defines compact / merge /
  close-interval / memorialize / forget. The Compaction Loop's **three terminal fates are these primitives
  renamed**, not new operations.
- It also rides the existing tool/agent substrate: the `.claude/scripts/memory-kit/` scripts (`placement-audit`,
  `decompose`, `spec-coherence-index`, `cleanup-apply`, `memory-coherence-audit`, `prep-brainstorm`) and
  `LIFECYCLE.md`; the four agents (`librarian`, `historian`, `cartographer`, `storyteller`); the `/brainstorm`,
  `/shift`, `/deliver`, `/converge`, `/memory-ceremony` skills; and `writing-plans`.

The fate-to-primitive mapping, stated once and used throughout:

| This spec's terminal fate | Canonical primitive (`2026-05-10`) |
|---|---|
| subsume-into-living-surface | **compact** |
| subsume-into-story-subtext | **graduate** (to storyteller / canonical story) |
| curate-to-history | **close-interval** |
| clear (body → git / tiny-delete) | **forget** |
| horizontal N-thread merge (§9) | **merge** |
| `.claude/archive/` deep-archive (story-pointer only) | **memorialize** |

We rename for intent; we do not redefine for mechanism.

**What is genuinely NEW (the contribution — three things):**

1. **Front-link ENFORCED AT CREATION TIME.** Neither prior doc has the lifecycle *hook*. The unified loop
   measures and drains; the lifecycle doc names the primitives — but a new plan/spec is still born standalone
   and only *later* (maybe) reconciled. This spec makes the brainstorm-open fire point surface the existing
   canonical seed(s) and bind the new artifact to them *before* it is written, so it is born linked rather than
   retroactively threaded.
2. **Decompose-to-ZERO-RESIDUE as cardinal rule.** Concluding work does not *move* a plan to a quieter folder;
   it *dissolves* the plan until nothing plan-shaped survives in the live tree. This is the rule that retires
   `history/_retired/` and the `.claude/archive/<date>/` junk-drawer as destinations — under the
   No-Dumping-Grounds law, a slower dump is still a dump.
3. **History-as-CURATED-PRESENT-POINTER.** History is not a parking lot read by no one; every history record is
   a deliberate, textbook-quality lesson *and* surfaces as inline pointers (watch-out / paths-not-taken /
   anti-pattern) planted in the live planning surface, so the lesson meets the planner at the point of decision
   rather than waiting to be excavated.

It also takes two consuming dependencies as siblings: the **VERIFY-GATE** is the `verification-result-index`
(`2026-06-01-verification-result-index-design.md`) — "landed & working" claims are graded green by
ci-investigator before being remembered *as a green test*, because a plan cannot grade its own homework; and the
warm/hot temperature model that re-warms a curated record on regression is the in-flight coherence machinery
(`2026-05-28-in-flight-memory-coherence-design.md`) applied to placement.

**The TWO CORRECTIONS this spec makes to the unified-loop spec** (detailed in §8 and §7; both PROPOSED, not yet
applied):

- **(a) Stasis metric.** The unified loop currently scores stasis as a **single context-coverage composite**:
  its `--stasis` model (`placement-audit.py` stasis_mode, L533) averages coverage dimensions against a `1.0`
  benchmark; the unified-loop spec further *proposes* (does not yet implement) folding `substrate-currency` in as
  a `gospel-currency` dimension (unified-loop spec L48–90). **Correct to the THREE-ZONE COMET SHAPE** (§8): one
  shrink-target (the ACTIVE *pile* vs canonical truth), one grow-target (the curated *museum* vs narrative), and
  one never-force-shrunk *working-memory* budget zone. The single `P = T/H` ratio is **deleted** because it lumps
  curated working memory into the shrink-target and would force deletion of curated lessons; stasis is the comet
  *shape*, not one number trending toward zero.
- **(b) History model.** The unified loop inherited **"archive/retire to history"** as a terminal operation.
  **Correct to curated-present-pointer + decompose-to-zero-residue,** which
  kills `history/_retired/` and the `.claude/archive/` junk-drawer as destinations. The historian becomes a
  curator, not an archivist; `.claude/archive/` is demoted to genuine memorialized deep-archive (story-pointer
  retrievable) only — never a sink at the end of the loop.

## 3. The loop spine — two deterministic fire points (plus a read-only PICKUP fire)

The discipline is a **closed loop with two deterministic fire points** bracketing the superpowers lifecycle.
The FRONT fires when a brainstorm opens; the BACK fires when a plan's work concludes. The one-time burndown of
the existing active pile (§9) is just the BACK fire point run in batch over already-concluded work. The
unified-memory-loop tells you *what state the corpus is in and who to dispatch*; this spec tells you *what fires
when a brainstorm opens and when a plan's work concludes* — and enforces that the answer is born-linked at the
front and zero-residue at the back.

*(2026-06-04)* A third, **read-only** fire point — **PICKUP (§4b)** — covers the moment neither bracket
touches: a session re-opening on prior work ("where are we at with X?"). It surfaces semantic recall
deterministically at pickup-time but holds no lifecycle authority — it creates nothing and dissolves nothing,
so the two-point bracket model below is unchanged.

```
                       ┌──────────────────────────────────────────────────────────┐
                       │  CANONICAL SEED  (genesis/docs/content/elohim-protocol/    │
                       │   architecture/ — carries its own lineage edges)           │
                       │  past ──▶ present ──▶ future, linked THROUGH the seed       │
                       └──────────────────────────────────────────────────────────┘
                            ▲  surface seed(s)              subsume back  │
            ┌───────────────┴──────────────┐          ┌──────────────────┴──────────────┐
            │  ◆ FRONT FIRE POINT (§4)      │          │  ◆ BACK FIRE POINT (§5)           │
            │  (/brainstorm opens)          │          │  (work concludes / branch finish) │
            │  prep-brainstorm.py +          │          │  decompose.py  →  decompose-self   │
            │  spec-coherence-index.py       │          │  ZERO standalone residue          │
            │  (lexical) + MemPalace (sem.,  │          │                                   │
            │   JIT-scoped via historian)    │          │  three terminal fates per chunk:  │
            │                                │          │   1 SUBSUME → living surface       │
            │  new plan/spec BORN LINKED     │          │     (canonical/dev-doc/research/   │
            │  (compose-don't-fork,          │          │      TEST/SCENARIO/CODE) +inline   │
            │   ENFORCED at creation)        │          │     trajectory pointers            │
            └───────────────┬───────────────┘          │   2 SUBSUME → story subtext        │
                            │                          │   3 CURATE→history  or  CLEAR(git)  │
                            ▼                          └──────────────────┬──────────────────┘
                ┌───────────────────────┐                                 │
                │   EXECUTE              │   superpowers:executing-plans   │
                │   (in-flight in        │ ──────────────────────────────▶│
                │    superpowers/{specs, │   VERIFY-GATE before BACK:      │
                │    plans})             │   ci-investigator grades green; │
                └───────────────────────┘   plan can't grade own homework  │
                            ▲                                               │
                            │   regression along a verified_by edge         │
                            │   WARMS a curated record back to hot ◀────────┘
                            │   (PLACEMENT temperature: cold→warm→hot)
                  ┌─────────┴─────────────────────────────────────────────┐
                  │  RIDES: 2026-06-01-unified-memory-loop-design.md        │
                  │  single placement-audit scoreboard · drain+menu        │
                  │  readouts · ceremony-as-trigger · dispatch to the      │
                  │  equipped agent (librarian/historian/cartographer/      │
                  │  storyteller)                                          │
                  └────────────────────────────────────────────────────────┘
```

Between the fire points, work proceeds through `superpowers:writing-plans` → `superpowers:executing-plans`
exactly as today. In-flight specs and plans live in their ACTIVE homes (`genesis/docs/superpowers/{specs,plans}/`,
`genesis/docs/plans/`) per `PLACEMENT.md`. This spec adds nothing to the middle; it only brackets it.

## 4. FRONT fire point — born-linked surfacing (lexical + semantic, JIT-scoped)

When `/brainstorm` (the coherence-wrapped command at `.claude/commands/brainstorm.md`) opens, its Step 1 PRE
runs `prep-brainstorm.py` (84 lines), and the surfacing step runs **before** any design proposal to answer one
question: *which canonical seed(s) already cover this topic?* It surfaces them two ways, both real and
complementary, because **one lens is provably blind.**

### 4.1 Two lenses

- **Lexical (deterministic, cheap):** `spec-coherence-index.py --query "<topic>"` (152 lines) does token-overlap
  matching over the live surface and ranks every ACTIVE/CANONICAL doc with its PLACEMENT state, so the planner
  gets `CANONICAL → compose`, `SUPERSEDED → read the gotcha, design around`, `claimed-UNVERIFIED → don't assume
  it works`. This is the floor — it always runs, costs nothing, needs no warm index, never goes offline. It is
  what `prep-brainstorm.py` already invokes today.
- **Semantic (recall, JIT-scoped):** MemPalace embeddings (ChromaDB + `all-MiniLM-L6-v2`, offline; palace at
  `/projects/elohim/.mempalace/palace`), pulled **just-in-time** at the surfacing step via an **on-demand
  tool-load** that scopes exactly `mempalace_search` + `mempalace_check_duplicate` (`ToolSearch
  select:mempalace_search,mempalace_check_duplicate`), then released. This catches the
  same-concept-different-words case the lexical floor misses: the embedding similarity between "decompose-self"
  and `compact` is high even though token overlap is zero. (A works-today *fallback* dispatches to the
  MemPalace-equipped historian subagent — but that path over-imports; see §4.2.)

Why both: the surfacing probe proved the lexical index is **lexically brittle** — it returned 0 matches for
"decompose-self / dump / forget" while the prior art (`2026-05-10-memory-lifecycle-design.md`) sat right there
under different vocabulary, invisible. A born-linked guarantee cannot rest on a lens that goes dark exactly when
the planner is most at risk of re-minting an existing spec. Lexical is the always-available proof-of-effort
floor; semantic is the recall ceiling that defeats vocabulary drift.

### 4.2 MemPalace is JIT-scoped, never always-on

The semantic lens is **pulled at the surfacing step and released**, not carried as ambient tooling. This is the
No-Dumping-Grounds law (operator, 2026-06-02) applied to the context/tool surface, binding MemPalace twice over.
First, as a *store*, the index embeds only the cleaned surface (canonical seeds + curated history + graduated
stories + living docs/tests/scenarios) — never the pile of plans/dead-arch/dupes; this matches the standing
memory-kit rule that mem-palace ingestion is a deliberate graduation act and "raw/abandoned/superseded is never
embedded." Second, as a *tool*, an always-loaded MCP is itself a dump: the historian's frontmatter carries
~18 `mcp__mempalace__*` schemas, and hoisting them into every brainstorm turn to use two (`mempalace_search`,
`mempalace_check_duplicate`) is ~16 tool-schemas of dead weight in every prompt — the context-surface analogue
of the `.claude/archive` junk-drawer this whole loop exists to kill.

There are two ways to pull the semantic lens JIT, and they are **not** equivalent — owning that tradeoff is the
point of this subsection:

- **PRINCIPLED PATH (achieves the 2-tool scope): on-demand tool-load.** At the surfacing step the harness
  deferred-tools mechanism loads *exactly* `mempalace_search` + `mempalace_check_duplicate` (`ToolSearch
  select:mempalace_search,mempalace_check_duplicate`), the front-link runs them, and the schemas are released. This
  is the only path that actually delivers the "scope exactly 2 tools" goal — nothing else enters the prompt.
- **FALLBACK PATH (works today, over-imports): dispatch to the historian.** Dispatching the surfacing step to the
  `historian` subagent (`.claude/agents/historian.md`, already MemPalace-equipped) runs today with zero new
  wiring — but the historian's frontmatter re-imports **all ~18 `mcp__mempalace__*` schemas**, so it does **not**
  achieve the 2-tool scope; it pulls the same ~16 schemas of dead weight the principled path exists to avoid. The
  tradeoff is explicit: the fallback trades context-surface cleanliness for "runs now," and it is the temporary
  bridge until the on-demand tool-load is wired into the brainstorm seam (§12 open issue).

Either way the historian/loader surfaces the seeds, annotates them with any precedent/risk pointers, and the
scoped context is released. The brainstorm session carries the *surfaced seeds* as plain text, not the MCP —
exactly as `prep-brainstorm.py` today injects the lexical preload as text, not as live tooling.

### 4.3 The new plan is written ALREADY linked

Surfacing is necessary but not sufficient — the born-linked guarantee is enforced **at creation time** (the
genuinely-new contribution). The seam already exists: `/brainstorm` Step 4 (POST) mandates PLACEMENT frontmatter
with a `cites:` array — "the verifiable links back" — before the artifact lands. The front fire point makes that
array a **front-loaded, non-optional input**: the seeds surfaced by both lenses are written into `cites:` (and
the appropriate `refines` / `derived_from` / `compacted_from` lineage edges) as the spec is born. Concretely:

- `CANONICAL` match present → the brainstorm output is "add a section to `<canonical seed>`" with `refines:
  <seed>`, not a new file (Step 3 already prefers this).
- `SUPERSEDED` match → `cites:` the history record and the design routes around its documented gotcha.
- Only an **empty result from BOTH lenses** justifies a standalone spec with `cites: []`.

A spec that surfaced a `CANONICAL` match but composes a standalone doc anyway is a placement violation the BACK
fire point will catch; the planned placement-enforcement hook (§9, not yet built) makes "compose, don't fork" a
creation-time gate rather than an audit-time observation. This is what makes the corpus an intentional memory:
every new doc enters the tree already wired into the lineage (§6) that the seed will carry past → present →
future.

### 4.4 Staleness guard — frozen embeddings, index-age vs last-dissolve

The semantic lens has a sharp failure mode: **the MemPalace index is frozen at mine-time and does not
auto-update.** The live palace was last mined **2026-05-14** — it does not contain even yesterday's specs,
including `2026-06-01-unified-memory-loop-design.md` itself. A front-link that trusts a stale index will
confidently report "no prior art" for a topic specced last week and wave through a fork. That is worse than no
semantic lens, because it carries false authority.

So the front fire point runs a **staleness guard** before trusting the semantic result: it compares the index
mine-date against the timestamp of the last BACK-fire `decompose + mempalace_sync` re-mine (§5.4). If the index
is older than the last dissolve — i.e. plans have been decomposed and re-homed since the embeddings were built —
the guard **flags the semantic lens as STALE and degrades to lexical-only**, surfacing an explicit advisory
("MemPalace index dated 2026-05-14, last dissolve newer — semantic surfacing degraded; re-mine before trusting
it") rather than silently presenting incomplete recall as complete. A fresh-but-correct index and a
stale-but-internally-consistent index look identical to a cosine probe, so age is tracked independently. Until
the first burndown-clean → re-mine completes, the front-link runs lexical-floor + stale-flagged semantic, never
silently-authoritative semantic. The ordered sequence is **burndown-clean → re-mine → trust semantic
surfacing**, and the staleness guard keeps every front-link self-aware about which phase it is in.

### 4.5 MemPalace ingestion scope — mine the cleaned surface, never the pile

MemPalace is a **curated semantic index, not a vacuum.** It mines ONLY the cleaned, durable, multi-destination
surface — **canonical seeds, curated history, living dev/functional docs, stories, scenarios, and working
memory.** It NEVER ingests the transient ACTIVE pile, raw code lines, or the `.claude/archive/` junk-drawer.
Those are exactly the matter the BACK fire point dissolves; embedding them would re-import the dump into the one
index whose job is to surface durable prior art.

The legacy config is the anti-pattern made literal: it mines `memory/` + `plans/` + `shifts/` +
`elohim-protocol/` — i.e. it points straight at the pile. That config is what this scope **replaces.** The live
index is frozen (mined 2026-05-14) and pointed at the pile, so there is **nothing worth preserving** — a clean
**rebuild from scratch is acceptable and expected**, not a migration.

The build order is therefore: **burndown-clean → re-point ingestion at the cleaned surface → fresh re-mine →
only then trust semantic surfacing** (the same ordering the §4.4 staleness guard and §5.4 re-mine step enforce).

**The whole-workspace convergence endpoint.** The "cleaned surface" exclusion is a *transitional* boundary, not
a permanent one. Once the No-Dumping-Grounds law (§10.3) holds **workspace-wide** — no ACTIVE pile, no
junk-drawer, every store curated — then "cleaned surface" *is* "whole workspace," and pointing ingestion at
everything becomes coherent rather than reckless. That convergence is the steady state the loop earns: the scope
restriction exists precisely *because* the workspace is not yet clean, and it dissolves the moment it is.

## 4b. PICKUP fire point — session-pickup semantic surfacing (harness-gated, read-only)

*(Added 2026-06-04. Numbered 4b, not 5, so every existing §4.x/§5.x prose cite in `brainstorm.md`,
`CLAUDE.md`, and sibling specs stays valid.)*

The FRONT fires when a brainstorm opens; the BACK fires when work concludes. **Nothing fired when a session
re-opened.** The evidence instance (2026-06-03): the operator started a session with *"Where are we at in
delivering … light up the topology …?"* — a pickup-shaped prompt that routed straight to `Skill(deliver)`,
whose Step-1 context gather is **all lexical** (8 grep/glob locations, no recall lens). The `/converge` spec
already names this exact moment — *"session-start UX: human asks 'what's next?'"* — as step 3 of its cycle,
but the surfacing there is manual: it happens only if the agent thinks to dispatch a MemPalace-equipped
subagent. The PICKUP fire makes it deterministic, the same way `prep-brainstorm.py` made FRONT surfacing
deterministic. (The concept seed predates this section: the agentic-context-graph memory called today's
SessionStart hook "the un-positioned, un-weighted prototype" and prescribed *"many sharp position-aware
hooks"* — PICKUP is the first position-aware one. See the dogfood note below for why that memory is cited by
description, not by path.)

### 4b.1 Two harness surfaces, one session gate

The harness has no native "session pickup" event, so PICKUP composes two hook surfaces around one
once-per-session gate (flag file per parent-PID, the `pre-tool-memory.py` pattern):

- **Primary — `UserPromptSubmit`** (the only surface that sees the raw prompt before the model frames an
  approach). Eligible only on **prompts 1–3 of a session** (counter flag; a late-session "resume" already has
  context — injecting would be noise). Fires on a **pickup-vocabulary regex**: `where (are|were) we`,
  `pick(ing)? up`, `resume`, `continue (with|from|where)`, `status of`, `what'?s next`,
  `where did we leave`, `catch me up`, `state of (the|our)` — **plus slash pickups**
  `^/(deliver|shift|converge|plan)\b`, which gives those wrappers the semantic lens with zero per-wrapper
  edits (their handles — `/deliver light-up-the-topology` — are excellent embedding input). `/brainstorm` is
  **excluded**: it owns its own FRONT seam (§4.1) and would double-fire.
- **Fallback net — `PreToolUse` on `Grep|Glob|Agent`**, firing only when **(a)** no injection happened yet,
  **(b)** it is the session's *first* search-shaped tool call, and **(c)** the session is still inside the
  first-3-prompt window. A fresh session whose first move is *search* is the pickup shape the regex missed;
  the net's query material is the stashed first prompt **plus the distilled tool pattern** (often better
  embedding input than the raw prompt). Accepted cost: a fresh non-pickup session that opens with a search
  also gets surfaced — one-time ~3 s and topically-relevant neighbors; the cosine floor (§4b.3) mutes the
  truly-irrelevant case.

Both surfaces inject **at most once per session, combined.**

### 4b.2 CLI engine, not MCP — a load-bearing fact

The MemPalace **MCP server is not wired into the main session** (it is per-subagent: historian / librarian /
cartographer / storyteller). A `ToolSearch select:mempalace_search` in the main loop returns nothing — this
brainstorm hit that wall directly. The §4.1 FRONT lens works because skills can dispatch equipped subagents;
a *deterministic hook* cannot. So PICKUP shells to the CLI:

```bash
python3 .claude/scripts/memory-kit/mempalace-currency.py --status   # staleness first: pure stdlib, ms
mempalace --palace <repo>/.mempalace/palace search "<query>"        # measured ~3 s wall
```

The ~3 s cost is exactly why the gates exist: once per session, never per prompt. Hook timeout: 15 000 ms.

### 4b.3 Injection contract

- **Cosine floor:** top hit < 0.35 → **silent no-op** (exit 0, nothing injected). Noise is worse than nothing.
- **Header carries currency, always:** `PICKUP SURFACING (semantic recall — fresh)` or
  `(… — DEGRADED: index N file(s) behind front-link, last mine <date>)`. Degraded is **surfaced, never
  silent, never skipped** — the §4.4 staleness-guard discipline applied at pickup-time.
- **Body:** top-4 hits — source path, room, cosine, ≤2-line snippet each. Budget ≤ ~1.2 k tokens.
- **Footer rule line:** *"Recall hints, not truth — verify each source against disk before acting on it."*
  This is the Stale-Memory-Override guard, and it earned its place immediately (next note).

### 4b.4 Read-only discipline + dogfood evidence

PICKUP **surfaces; it never binds.** It creates nothing, dissolves nothing, and does not apply the Step-2
composition rule — if a pickup escalates into design work, FRONT fires with its full rule as today. It is a
recall lens at the loop's third natural moment, not a third lifecycle authority.

The designing brainstorm itself proved both halves of the contract: **(the gap)** the 2026-06-03 pickup
prompt got zero semantic recall on a topic with rich palace coverage; **(the staleness guard)** the semantic
lens for *this very section* surfaced `project_agentic_context_graph_model.md` — the concept seed — which
**no longer exists on disk**; it survives only in the 92-files-behind index. Presented without the DEGRADED
banner, that hit would have been a live citation to a deleted file. The banner is not decoration; it is the
difference between recall and hallucinated authority.

**Landed as (2026-06-04):** `.claude/hooks/pickup-semantic-surfacing.py` (one script, `--event prompt|tool`,
stdin JSON per surface) + two `settings.json` entries (`UserPromptSubmit`; `PreToolUse` matcher
`Grep|Glob|Agent`). Gate paths (window, once-per-session, `/brainstorm` exclusion, no-stash net refusal,
dedupe) verified by synthetic stdin tests on build day; the original 2026-06-03 gap-prompt replayed against
the live palace and surfaced the resilience-profile badge/icon design plans under the DEGRADED banner.

## 5. BACK fire point — self-dissolve to zero residue

When work concludes (a branch finishes via `superpowers:finishing-a-development-branch`, or a plan is otherwise
done/dead), the **`decompose-self`** routine runs `.claude/scripts/memory-kit/decompose.py` and dissolves the
plan until **nothing plan-shaped survives in the live tree** — the cardinal **decompose-to-zero-residue** rule.
There is no `history/_retired/` graveyard and no `.claude/archive/<date>/` junk-drawer sink; per the
No-Dumping-Grounds law, *every chunk lands in a living surface or is cleared.*

### 5.1 The router: detect → analyze → route

**Detect** (deterministic, three triggers, no cadence):

- **PostToolUse signal** — the not-yet-built **placement-enforcement hook** (§9) extends the existing
  `claude-md-drift-signal.py` accumulator pattern: when an edit lands a terminal `status: landed | superseded |
  abandoned` (with `landed_commit:`) into an ACTIVE doc, it **queues** that doc for decompose-self. The hook
  only queues — it never mutates.
- **`/shift` final step** — the agentic-developer shift runs decompose-self on the plan(s) it concluded as its
  closing act, so a shift cannot end leaving a parked plan behind.
- **Surfaced loose plans** — the `placement-audit.py` drain readout names ACTIVE docs whose work is dead/landed
  but un-decomposed; each is fired through the router. The one-time burndown of the active pile (§9) is this
  back fire point run in batch.

**Analyze** reuses `decompose.py` verbatim: it already splits a doc into bounded, **cited** gap-items (checkbox
tasks, requirement bullets; else flagged for agent decomposition — and CHECKED ≠ VERIFIED). The back fire point
adds, per chunk, a **fate decision** and a **destination home**.

### 5.2 The three terminal fates, their homes, and the edges each writes

Every chunk routes to exactly one of three fates (the renamed lifecycle primitives from §2). A chunk's verified
behavior is remembered **AS a green test/scenario or as merged canon — never as a parked plan.**

**Fate 1 — SUBSUME INTO LIVING SURFACE** (= `compact`). Verified, still-true content folds into the live surface
that owns it, with inline trajectory pointers.

| Chunk kind | Destination home | State |
|---|---|---|
| Cross-cutting design truth (gospel-tier) | `genesis/docs/content/elohim-protocol/architecture/` (+ INDEX row, `tier: architecture`) | exists |
| Developer/functional how-it-works (build steps, API surface, env-blocked notes) | a functional-doc home **distinct from `superpowers/specs/`** | **open issue (§9)** — today such content scatters into loose `genesis/docs/{integration,setup}/` and orphan files |
| Research / market / prior-art | `genesis/docs/research/` (+ `/projects/research/` assets) | exists |
| **Verified behavior** | a **green a2o scenario** (`genesis/a2o/features/<pillar>/*.feature`) and/or a **passing test** | exists (the canonical resting place for "landed & working") |
| The change itself | **code** in `app/`/`elohim/`/`doorway/`/`steward/`/crates/`bridges/` | exists (113,190 LOC, src-filtered) |

*Edges:* the destination gains an inline `compacted_from:` + a one-line **"why we turned"** trajectory pointer;
the gap-item gains `superseded_by: <home-path#anchor>`. A scenario/test gains a `verified_by:` back-edge so a
future regression along it **WARMS the curated record back to hot** (§6).

**Fate 2 — SUBSUME INTO STORY SUBTEXT** (= `graduate`, storyteller-owned). A chunk whose lesson is human
narrative graduates into a canonical story at `genesis/data/stories/`. *Edges:* story gains `derived_from:`;
gap-item gains `graduated_to:`.

**Fate 3 — CURATE-TO-HISTORY or CLEAR** (= `close-interval` / `forget`).

- **Curate** → one **distilled (textbook-quality) lesson** at `genesis/docs/content/elohim-protocol/history/`
  (`tier: history` + INDEX row), bidirectionally linked to the canonical it qualifies, *and* its hot-context
  pointer planted inline in the live planning surface (§6). This is where the 7 dead-arch clusters (~27 docs)
  land.
- **Clear** → the raw body goes to **git** (recoverable, via `distills:` pointers), or a tiny stub is deleted.
  `cleanup-apply.py` is **corrected** here: it currently `shutil.move`s targets into `.claude/archive/<date>/`;
  under this spec it writes the distilled history lesson and lets the body retire to git. `.claude/archive/` is
  demoted to genuine memorialized deep-archive (story-pointer retrievable) only — never a back-fire destination.

### 5.2b Per-class fate routing (the table above is the `protocol-canonical` branch)

The §5.2 three-fates table assumes the chunk is **substrate** (verified → `a2o/<pillar>` + pillar code,
durable truth → `architecture/`, lesson → `history/`). That is the **`protocol-canonical`** branch only.
`decompose.py` now reads each spec's `class:` (the FRONT gate stamped it; the resolver is
`_lib.subject_routing`, the registry `.claude/subject-routing.yaml`) and routes per-class:

- **`process-meta`** → the `architecture/` and `a2o/<pillar>` legs are **NULL** (do not force them — that
  is the D4 collision, `history/2026-06-02-d4-name-collision`). Durable truth → the matching **CLAUDE.md
  gospel-diff** (by `process_subdomain`); the capability → a **`.claude/` tool**; a tried-and-failed
  mechanism or tool-design rationale → `history/` (LIVE, judgment-gated, `type:history-gotcha`); the body
  → git. The unifying rule: **discard the FORM, keep any RESIDUE carrying reusable reasoning** — class
  picks the home, reasoning-value decides whether a record is written (stub-then-grade default).
- A mis-class (e.g. `domain: D#` + all-`.claude/` targets) makes `decompose.py` **fail loud**, not
  silently route process residue into substrate legs.

Full design: `genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md`.

### 5.3 Mechanical-AUTO vs judgment-GATED apply

The gate is a property of the **operation**, inherited from the loop's per-operation gate model (corrected from
the unified-loop's "archive/retire to history" row):

| Operation | Apply |
|---|---|
| link chunk → its existing canonical seed; update `cites:` | **AUTO** |
| write a **history-lesson stub** (placeholder pointer awaiting curation) | **AUTO** |
| retire a chunk **body → git** (the clear path) | **AUTO** |
| **file backlog** — residual open work → `genesis/data/timeline/backlog/` (`spawned_backlog:` edge) | **AUTO** |
| **canonical-seed rewrite** or **horizontal merge** (N threads → ONE seed; §9) | **GATED** (operator-approved; only `/memory-ceremony` authors substrate-true gospel) |
| any **deletion** of a doc | **GATED** |

Auto operations never touch gospel or delete; they only link, stub, retire-to-git, and file. Everything that
*rewrites canon* or *destroys* returns a proposal and waits for the operator — exactly as the ceremony's
workflow-mode already gates.

### 5.4 The ordered MemPalace re-mine step

Because the index is **frozen at mine-time (2026-05-14)** and embeddings do not auto-update, re-mine is an
**ordered, post-dissolve step** — never concurrent with it:

1. **dissolve** — route all chunks to their fates; the live tree now holds **no plan-shaped residue** for this
   work.
2. **`mempalace_sync` prune** — drop index vectors for the now-gone plan/spec files (their semantic ghosts must
   not surface in a future front-link).
3. **re-mine the CLEAN surface** — embed only the compacted output (canonical seeds, curated history lessons,
   graduated stories, living docs/tests/scenarios). MemPalace stays a curated semantic index over the cleaned
   surface only, never a vacuum over the pile.

The §4.4 staleness guard records index-age vs. last-dissolve; the one-time build order is identical:
burndown-clean → re-mine → only then trust semantic surfacing.

### 5.5 The verify-gate (a plan cannot grade its own homework)

Routing to Fate 1's *verified-behavior* / *canon* home requires external grading; the other paths do not:

- **dead / superseded / abandoned** → curate to history **NOW**. No CI needed — there is no working claim to
  grade.
- **"landed & working"** → graded **green by ci-investigator** (fed by the not-yet-built **delivery-status-poll**,
  i.e. the SessionStart wiring of `delivery-status-distribution.py`): the area's tests pass, the relevant
  pipeline is green and **not cascade-masked** (`feedback_cascade_halt_masks_failures`), and any required
  soak/parity window actually ran clean. Only then is the chunk remembered AS a green test/scenario and merged
  to canon. A checkbox or a self-asserted "landed" is a CLAIM, never sufficient (the iroh gates proved it).
- **env-blocked** → **BLOCKED-BY-ENV hold**: the chunk stays queued (not curated, not merged) with the blocker
  recorded.

This mirrors the protocol's own law that **reach is earned, not self-asserted**
(`project_reach_earned_genesis_seeder_grades_homework`): a plan earns retirement by external evidence, the same
way content earns reach.

## 6. Trajectory — a cross-cutting property, two fidelities

Trajectory is first-class and carried at **two fidelities** so that lineage is both walkable and fully
recoverable:

- **Distilled (in the live docs):** walkable lineage edges — `supersedes` / `superseded_by` / `refines` /
  `derived_from` / `compacted_from` / `spawned_backlog` — plus a one-line "why we turned." The **canonical seed
  carries the lineage**; past → present → future is linked *through* the seed, never scattered across dead plans.
- **Full (in git):** the raw plan body. Clearing a chunk deletes it from the live tree but leaves it recoverable
  from history; nothing load-bearing is lost, only un-parked.

Trajectory is also the warming edge. A regression detected along a `verified_by` lineage edge **warms a curated
history record back to hot** — the same cold → warm → hot temperature model `PLACEMENT.md` already applies to
docs (its `verified_by:` warm-cascade). History is therefore a curated **present pointer**, not a graveyard: a
lesson re-enters the live planning surface exactly when a regression makes it relevant again.

## 7. History — a curated museum, present at the point of decision

### 7.1 The discipline in one line

History is not where dead work goes to rest; it is **distilled lessons planted where the next planner will trip
on them.** Every history record is a deliberate, textbook-quality artifact *and* it surfaces as **inline
pointers in the live planning surface** (the canonical seed), so the lesson meets the planner at the moment they
approach burned ground — not in a register they never open. The historian is a **curator, not an archivist**
(`.claude/agents/historian.md`: "You don't write to [the archive]… you consult").

This corrects the inherited model in `2026-06-01-unified-memory-loop-design.md` (L139) and in `PLACEMENT.md`
(L50, L65, L68–69), both of which still describe **"retire to `history/_retired/`"** — a parking-lot pattern.
Under the No-Dumping-Grounds law, `history/_retired/` is itself a dumping ground; the directory does not yet
exist on disk, and this spec strikes the *concept* before it is ever created. Distilled lessons live in
`content/elohim-protocol/history/` as first-class records; raw bodies live in git. Nothing is *retired* — it is
**decomposed to zero residue**, and the fraction worth remembering is **re-planted hot**.

### 7.2 Two surfaces per lesson, neither optional

A history record discharges its duty only when **both** of these exist. **Honest status: only the first half
exists today; the second half is aspirational** — there are **zero working instances** of a live doc carrying
mirrored inline pointers. That is acceptable for a design spec, but it must be stated plainly rather than
implied as current practice.

1. **The distilled record** in `history/` — **this half exists.** Textbook-quality: one-sentence lesson, the
   paths that were drifted onto, why they were wrong, the canonical it points back to, and `distills:` pointers
   to the raw bodies in git. The shape already exists and is the template — see
   `2026-06-01-dht-is-a-notary-not-a-byte-store.md`, which collapses **three** abandoned/pivoted plan bodies into
   **one lesson** with bidirectional `canonical:` links to the two specs it informs, registered as one row in
   `history/INDEX.md`.
2. **The inline pointer**, *planted in the live canonical seed*, at the decision point the lesson guards — **this
   half does NOT exist today; it is the contribution of this section, and it has no working instance yet.** Even
   the exemplary dht-notary record carries an excellent "Hot-context pointer (the one sentence to remember)"
   block — but it lives **inside the history record**, where a planner reaching for a DHT write will never see
   it. The two canonical specs it claims to inform carry **zero** inline `watch-out / paths-not-taken /
   anti-pattern` markers. The lesson is shelved in the museum, not posted on the burned ground. The discipline
   *proposes* to close that gap: every history record's hot-context pointer is **mirrored as an inline block in
   its `canonical:` targets**, keyed by the decision it guards ("If you are about to reach for a DHT record… stop
   and read"). The bidirectional link becomes a bidirectional *placement*: the lesson is in the museum **and** at
   the door. No such mirrored block has been planted yet — the pattern is designed here, not demonstrated.

This is the structural reason the front-link (§4) composes with the back: history is not a separate store to be
searched but **content already woven into the seeds the brainstorm front-link surfaces.** A planner who pulls
the canonical seed pulls its anti-patterns for free.

### 7.3 Three inline-pointer kinds

| Pointer kind | Plants where | Says |
|---|---|---|
| **watch-out** | the canonical decision point | "this layer/op looks right and is wrong — here's the result we got" |
| **paths-not-taken** | the canonical design-alternatives section | "we evaluated X; rejected for Y; do not re-propose without new evidence" |
| **anti-pattern** | the canonical pattern/convention it inverts | "this shape recurs and fails; recognize it by its silhouette, not its keywords" |

Each carries a back-link to the full distilled record and, through it, the `distills:` git pointers. The purpose
is singular: **stop re-treading anti-patterns.** One curated record + three planted pointers is cheaper than the
fourth drift.

### 7.4 Kill the junk drawer

The current state is the inversion, measured: the curated museum holds **4 docs / 13 KB**, while
`.claude/archive/` holds **77 files / 3.3 MB** — the dump outweighs the museum by **~254×**. Under
decompose-to-zero-residue:

- **Raw bodies → git** (full fidelity, recoverable via `distills:`). Nothing plan-shaped survives in the live
  tree.
- **`history/_retired/` is never created.** PLACEMENT.md's "verified-stable → `history/_retired/`" rule is
  rewritten: verified-stable behavior is remembered **as a green test/scenario** (Fate 1), and the *lesson* of
  the path that got there is remembered **as a planted history pointer** (Fate 3). A verified plan leaves **no
  doc** behind — it leaves a passing test and, if it taught something, a watch-out.
- **`.claude/archive/` is demoted to memorialized-deep-archive only** — story-pointer retrievable, never a
  vacuum over the pile. Its `memorialized/` and `graduated/` subtrees are the only legitimate residents; the
  dated raw plan dumps alongside them are exactly what the law forbids and what the burndown dissolves. MemPalace
  indexes the **cleaned** surface, never `.claude/archive/`'s raw pile.

## 8. Stasis = the comet's SHAPE — three zones, three disciplines (the corrected metric)

### 8.1 The correction, stated plainly

The unified-memory-loop spec measures stasis as a **single context-coverage composite**: `placement-audit.py`
stasis_mode (L533) averages coverage dimensions — capture, status, well-formed, memory-linked,
CLAUDE.md-rightsized, history-bidirectional, traceability, MEMORY.md-budget — against a `1.0` benchmark,
declaring "at stasis" when the score clears `1 − STASIS_MARGIN` (margin `0.15`, L78/L192 → threshold `0.85`),
and that same spec *proposes* (does not yet implement — grep finds no such dimension in the current
`placement-audit.py`) folding `substrate-currency` in as a `gospel-currency` dimension (unified-loop spec
L48–90). **Two things are wrong with reducing stasis to one number.**

First, **every one of those coverage ratios can saturate to 1.0 while the corpus stays exactly as bloated as it
is today.** A spec is "captured" the moment it has a decompose record; "well-formed" the moment it carries a
link; "status'd" the moment it carries a state. None of those measure **how much plan-shaped matter is sitting
in the live tree** — the actual disease.

Second — and this is why a *single* proportion is also wrong — an earlier draft of this very spec tried to
replace the composite with one ratio, **`P = T / H`** (distilled tail over leading edge, target `1.5% ± 0.5%`).
That `P` is **DELETED here too.** It lumps **curated working memory into the same `T` it wants to shrink toward a
proportion**, so the moment the corpus is "over band" the loop is told to shave the tail — which means
**deleting curated lessons to hit a number.** A metric that rewards destroying the museum to satisfy an
arithmetic target is the inverse of what the comet model wants. The museum is *supposed* to grow; working memory
is *supposed* to be a crystallized tail, not a shrink-target. One ratio cannot express three opposite
disciplines.

**The correct model is the comet SHAPE: three zones, each with its OWN discipline.** Stasis is not a number
trending toward zero or toward a single percent — it is **all three zones in-band simultaneously.**

> **NOTE (design-vs-impl):** `placement-audit.py` does **not** yet compute the three zone ratios below. This
> three-zone dimension set is **PROPOSED**; the section specifies what the corrected `--stasis` should emit.

### 8.2 The three zones, each with its own discipline

| Zone | What it is | Discipline | Driving measure (today → target) |
|---|---|---|---|
| **ZONE 1 — BLOAT** *(the only shrink-target)* | the ACTIVE pile (44 specs + 63 plans + 27 genesis/plans = 134 docs / 5.7 MB) | **DECOMPOSE until LEAN** relative to canonical truth | `\|pile\| : \|canonical architecture\|` = **6.2× today** → **≤ 1×**. Also report `\|pile\| : \|all narrative\|` = **~2× today**. This is the burndown's success metric. |
| **ZONE 2 — MUSEUM** *(must GROW, not shrink)* | curated history (`history/`, 4 docs / 13 KB) | **FILL** it — it is starved | `\|history\| : \|all narrative\|` = **0.46% today** → grows from *starved* toward *proportional* (a small directional band of a few percent — deliberately NOT a brittle single hard-coded percent). |
| **ZONE 3 — WORKING MEMORY** *(never force-shrunk)* | `.claude/memory/` (249 files / 842 KB; `MEMORY.md` = 47.7 KB) | **HYGIENE within budget** — it is legitimate crystallized tail, NOT bloat | `MEMORY.md` index ≤ **24.4 KB** budget + topic-file hygiene (its existing discipline, unchanged). |

**Stasis condition = all three zones in-band at once:** the pile is **lean** (Zone 1 ≤ ~1× canonical), the museum
is **proportional-and-growing** (Zone 2 climbing out of 0.46% toward a few-percent band), and working memory is
**within budget** (Zone 3 ≤ 24.4 KB index + clean topic files). No single ratio; a shape.

### 8.3 Why the OLD targets — "pressure → 0" and "single `P = T/H`" — were both wrong

- **"Pressure → 0" rewards motion, not compaction.** The prior stasis loop drove markers toward zero (pressure-dir
  occupancy, `_retired` dumps). Both can hit zero **while the 134-doc, 5.7 MB active pile sits untouched.** Worse,
  the cheapest way to empty a pressure dir is to shove a doc into `.claude/archive/<date>/` — exactly how that
  junk-drawer reached 77 files / 3.3 MB. Draining-to-zero makes the **graveyard the relief valve.** The tail is
  *supposed to exist* (deliberate forgetting, `project_memory_lifecycle_comet_shape.md`, is "hold a curated tail
  sized to the head," not "delete everything"); a loop chasing zero fights the design.
- **"Single `P = T/H`" force-shrinks the wrong zone.** Collapsing curated history + working memory into one `T`
  and chasing a percent means the loop, when "over band," is told to **cut curated lessons** — the museum it
  should be *filling* and the working memory it should *never* force-shrink. It also hides the actual disease:
  the bloat is the **pile**, which `P` excludes entirely from its numerator and denominator, so `P` can sit in
  band while 5.7 MB of parked intentions rots in the live tree. One ratio, three opposite disciplines, is
  category error.

The three-zone shape fixes both: the **only** shrink-target is the pile (Zone 1); the museum is a **grow**-target
(Zone 2); working memory is a **budget-hygiene** zone (Zone 3), never force-shrunk.

### 8.4 The comet, in real numbers (re-measured 2026-06-02)

| Body | What it is | Size today |
|---|---|---|
| **Leading edge** (head) | 30 architecture docs (canonical) / **923 KB**; all narrative = 219 docs / **2.8 MB**; codebase = **113,190 LOC** (src-filtered) | 923 KB canonical · 2.8 MB narrative · 113K LOC |
| **ZONE 2 — Museum** (curated tail, must grow) | curated history (`history/`) — 4 docs | **13 KB** |
| **ZONE 3 — Working memory** (crystallized tail, budget-hygiene) | `.claude/memory/` — 249 files; `MEMORY.md` = 47.7 KB (index budget ≤ 24.4 KB) | **842 KB** |
| **ZONE 1 — Active pile** (the BLOAT — the only shrink-target) | 44 specs + 63 plans + 27 genesis/plans = 134 docs | **5.7 MB** |
| **Junk-drawer** (NOT a zone — anti-pattern to dissolve) | `.claude/archive/` | 77 files / **3.3 MB** |

The defining pathology is the **inversion of Zone 1 against the head**: the ACTIVE pile is **6.2× the canonical
architecture** (5.7 MB : 923 KB) and **~2× all narrative** (5.7 MB : 2.8 MB) — the workbench is larger than the
bench it sits on. Meanwhile Zone 2 is **starved at 0.46% of narrative** (13 KB : 2.8 MB) — the museum is nearly
empty. (The bogus "33.3 MB head / 1 : 2,642 inversion" of the prior draft came from counting the whole
`genesis/a2o` directory — 29 MB of reports and screenshots — as the vision corpus; the real a2o `.feature` files
are only 326 KB.)

### 8.5 The three measured ratios and their bands (PROPOSED dimension set)

`placement-audit.py --stasis` should compute three independent ratios, each with its own band — **not** one
blended number:

1. **Zone 1 — pile leanness.** `R1 = bytes(ACTIVE pile) / bytes(canonical architecture)`, where canonical
   architecture = `genesis/docs/content/elohim-protocol/architecture/` `*.md` bytes. **Today `R1 = 6.2×`; target
   `R1 ≤ ~1×`.** Also report the secondary `bytes(pile) / bytes(all narrative)` = ~2× today. This is the
   burndown's success metric and the **primary** verdict: `BLOATED` while `R1 > 1`.
2. **Zone 2 — museum proportion.** `R2 = bytes(history/) / bytes(all narrative)`, where all narrative = all
   `*.md` under `genesis/docs/content/elohim-protocol/`. **Today `R2 = 0.46%`; target = "grows from starved
   toward proportional" — a directional few-percent band, deliberately NOT a single brittle percent.** Verdict:
   `STARVED` while `R2` is far below the band; the historian is dispatched to lift it.
3. **Zone 3 — working-memory budget.** Unchanged from today: `MEMORY.md` index ≤ **24.4 KB** + topic-file
   hygiene gates. This zone is **never** a shrink-target driven by R1/R2; it is held to its own budget. Verdict:
   `OVER-BUDGET` only against the 24.4 KB index ceiling, never against a comet proportion.

`at_stasis = R1_lean AND R2_proportional AND Zone3_in_budget AND hard_gates_pass` — **gate, don't average**, so a
strong zone can never mask a weak one. The transient `.claude/archive/` (3.3 MB) and the pile's raw bytes that
have not yet dissolved are reported on a separate **"undissolved mass"** line (5.7 MB pile + 3.3 MB archive =
9.0 MB today) — the BACK fire point's actual workload, named beside the verdict, the honest readout the old
single-number metric hid.

The JSON readout (L590) gains `pile_to_canonical_ratio`, `pile_to_narrative_ratio`, `museum_to_narrative_ratio`,
`memory_md_index_bytes`, and `undissolved_mass_bytes`, so `memory-stasis-loop` and `/converge` can rank each
zone's pressure independently and dispatch the equipped agent (librarian/burndown to shrink Zone 1, historian to
fill Zone 2, librarian to hold Zone 3 to budget). The composite-print path (L596) is replaced by the three
per-zone verdict lines; `has_canonical_link` (L240/L542/L665) continues to gate Zone-2 records as bidirectionally
linked. Targets live in `context-coverage.yaml` alongside `targets.benchmark`/`targets.margin` (read via
`_mfval`, L192), so the operator re-tunes each band without editing `placement-audit.py`.

## 9. The one-time burndown (catch-up = the BACK fire point in batch)

The seven principles describe a steady-state loop, but the loop arrives late: there are already **134 active
docs / 5.7 MB** (44 specs + 63 plans + 27 genesis/plans, this spec included) that were born *before* the FRONT
hook existed and have no BACK fate. The burndown is not a separate migration — it is **the BACK fire point run
once, in batch, over the accumulated pile.** Same `decompose.py`, same three fates, same verify-gate; only the
cardinality differs. After it runs once, the steady-state loop keeps the pile from re-accumulating. It is the
move that **fills the empty museum while draining the buried workbench.**

### 9.1 The 40-cluster map, and why ordering is causal

The pile decomposes into **40 clusters**, produced by `spec-coherence-index.py` (lexical) cross-checked against
the `placement-audit.py` temperature model. **The "40 clusters / all docs covered / 0 unassigned" tally is a
dated point-in-time snapshot (taken over a 133-doc pile; the pile is 134 today), not a structural invariant** —
the FRONT fire point opens new brainstorms and the BACK fire point dissolves concluded ones, so the count moves
continually; the burndown re-clusters whatever the pile holds when it runs. The class **is** the fate, and the
fate **dictates the order**:

| Class | Clusters | Docs | Terminal fate | Op (§ below) |
|---|---|---|---|---|
| **dead-arch** | 7 | ~27 | curate-to-history → distilled lesson + inline pointer | VERTICAL burn-down |
| **multi-thread-dupe** | 7 | ~19 | merge → 1 canonical re-surfaceable seed each | HORIZONTAL merge |
| **mixed** | 4 | 16 | per-doc split (some live, some curate, some clear) | both |
| **live-hot** | 22 | ~71 | stay on the lean edge — front-linked in place, no move | none |

The sequence **dead-arch → multi-thread-dupe → mixed → live-hot** is causal, not cosmetic:

1. **Dead-arch first (7 clusters, ~27 docs → curated history).** Superseded/abandoned architectures — no CI, no
   green-test claim to grade. Per the verify-gate they go *straight* to history NOW: each cluster collapses to
   one distilled lesson plus inline pointers planted in the live planning surface. Curating these first produces
   the **first real museum content** the front-link can later surface, and removes dead mass *before* the merge
   step must reason about it. The headline win: the **"EPR Codec & Storage Foundation" cluster is 13 docs → 1
   lesson** (its verified behavior already lives as green storage tests and as the canonical
   `2026-06-01-dht-is-a-notary-not-a-byte-store.md` record, so the plan bodies carry zero remaining truth and
   fall to git). Other dead-arch: **Light-Up-Topology (6 → 1)**, **Scenario-Archaeology (3 → 1)**.
2. **Multi-thread-dupe second (7 clusters, ~19 docs → 7 canonical seeds).** With dead mass gone, the HORIZONTAL
   merge has a clean field: N live threads on one topic collapse into ONE concise canonical seed that *becomes
   the re-surfaceable seed* the FRONT fire point hands the next brainstorm. The seven: **Peer-OAuth-Portal (4 →
   1)**, **Sweettest (3 → 1)**, **Doorway-SSR (3 → 1)**, **Capability-Profile (3 → 1)**, **Doorway-Hub-Edge (2 →
   1)**, **App-Manifest (2 → 1)**, **Conductor-Agent-Info (2 → 1)**. Each seed carries its distilled lineage
   (`supersedes` / `compacted_from` + "why we turned"); the dupe bodies fall to git.
3. **Mixed third (4 clusters, 16 docs → per-doc split).** These resist a single fate and must be handled after
   the clean classes, because the split depends on what already landed in history (step 1) and which seed already
   exists (step 2). Each doc is decomposed chunk-by-chunk into Fate 1/2/3. The four: **EPR-3.5** (trust/compute
   gradient), **Recovery**, **SDK-Boundary**, **Memory-Coherence**.
4. **Live-hot stays (22 clusters, ~71 docs).** The lean edge of genuinely in-flight work. The burndown does not
   move these — it retro-fits the FRONT link in place (born-linked enforcement applied retroactively), so when
   each concludes it has a BACK fate waiting.

### 9.2 Projection

The burndown takes **the pile (~134 today) → ~71 hot + ~7 new canonical seeds**, with **~43 docs compacting
into a handful of curated-history lessons** (cluster sizes are from the 133-doc snapshot and will be re-derived
against the pile's actual contents at run time). Net standalone-plan residue trends toward **zero**: nothing
plan-shaped survives in the live tree that isn't actively in flight. The museum goes from 4 docs to a curated set of textbook lessons
each bidirectionally linked to a canonical; `.claude/archive/` is demoted to memorialized deep-archive only.

### 9.3 Operator gate: the converge-style approval menu

No cluster mutates without operator approval (a plan cannot grade its own homework — and neither can the
burndown grade itself). The batch presents a converge-style ranked menu, one row per cluster, in the order
above. Each row is decision-ready:

```
[dead-arch]  EPR Codec & Storage Foundation   13 docs → 1 history lesson
             fate: curate-to-history (no CI — superseded by dht-is-a-notary canon)
             lesson: "DHT notarizes; bytes live in the quilt. Codec-as-byte-store was the wrong layer."
             bodies → git (recoverable);  inline pointer → planted in superpowers/specs planning surface
             [approve] [defer] [edit-lesson] [show-diff]
```

The menu is generated by `placement-audit.py`; proposed mutations are staged by `decompose.py`; nothing is
written until `cleanup-apply.py` runs against the operator's approved rows — per-cluster, never all-or-nothing.
"Landed & working" claims in mixed clusters route through `ci-investigator` for a green grade before being
remembered as a test and merged to canon; env-blocked chunks take a BLOCKED-BY-ENV hold. Dead-arch and dupe-body
rows need no CI and approve fast — this is why they lead: the menu front-loads the cheap, high-mass wins so the
museum fills on the first approvals. After the menu drains, the burndown hands off to the re-mine step (§5.4):
dissolve → `mempalace_sync` prune → re-mine the now-clean surface.

## 10. Enforcement, the No-Dumping-Grounds law, and the PLACEMENT.md amendment

The loop's two fire points need a deterministic floor that *makes drift visible at the moment it happens* and
*refuses a quiet graveyard to drift into.* That floor is two un-built mechanisms plus one amendment.

### 10.1 The PostToolUse placement-drift hook (not yet built)

`PLACEMENT.md` §"Enforcement" already specifies this hook as deterministic and "to build, extends memory-kit,"
and it does not yet exist (present hooks: `claude-md-drift-signal.py`, `claude-md-structural-signal.py`,
`memory-coherence-signal.py` — no placement hook). This spec adopts it unchanged in mechanism and binds it to
the BACK fire point. It mirrors `claude-md-drift-signal.py`: PostToolUse matcher `Edit|Write`, single-digit-ms
cheap path, walks up from the edited file, increments a counter into `.claude/memory-kit/`, defers all judgment
to a script run at ceremony/session-start. The placement variant fires on any write under an ACTIVE home and
asks one deterministic question against frontmatter: **does this ACTIVE-home doc carry a terminal status
(`landed | superseded | abandoned`) yet still live plan-shaped in the live tree?** That is *placement drift* —
the BACK fire point's tripwire. The hook does not move the file (mutations are operator-gated per §5.3); it only
counts.

### 10.2 SessionStart surfacing of past-due work

The count surfaces at SessionStart through the **existing** budget headline (`placement-audit.py` headline_mode,
L611), not a new readout. That line already prints `MEMORY BUDGET` (no-status · unlinked-memory ·
claimed-unverified | pressure-dirs) plus the decompose-coverage review queue. This spec adds one field:
**"N plans past-due to decompose"** (ACTIVE-home docs with terminal-but-undissolved status). The companion
delivery-status surfacing — *"landed & working"* claims awaiting the verify-gate — is computed by the
already-present `delivery-status-distribution.py`, whose SessionStart poll is one of the two not-yet-wired
pieces. Wiring both into the one headline keeps a single SessionStart readout: *how many plans are overdue to
dissolve, and how many "done" claims are ungraded.*

### 10.3 The No-Dumping-Grounds universal law

The operator's law (2026-06-02): **there are no dumping grounds — anywhere.** A dumping ground is any store that
accumulates without curation. This spec binds the law to every store the loop touches:

- **No `history/_retired/`.** PLACEMENT.md routes verified-stable work there (L50/L65/L68–69); the directory does
  not yet exist, so we strike the concept before it is created. History is a curated museum (§7), not a
  retirement bin.
- **No `.claude/archive/<date>/` sink.** `.claude/archive/` (77 files / 3.3 MB) outweighs the museum (13 KB)
  by ~254×. That inversion *is* the dumping ground; it is demoted to memorialized deep-archive (story-pointer
  retrievable) only.
- **Not the ACTIVE pile.** The 134-doc / 5.7 MB pile is the workbench, not a store — every doc must carry a next
  state or it is debt.
- **Not MemPalace.** A curated semantic index over the cleaned surface only; plans/dead-arch/dupes/superseded are
  never embedded (per the memory-kit standing rule).
- **Not the context/tool surface.** Tools are JIT-scoped to the step that needs them (§4.2). All four memory
  agents carry MemPalace MCP, but it is never promoted to always-on.

### 10.4 The PROPOSED PLACEMENT.md amendment

Six edits to `genesis/docs/PLACEMENT.md` — **PROPOSED, not yet applied** (this is a design spec; PLACEMENT.md
still contains its `_retired/` language and the `.claude/archive/` sink role on disk today):

1. **Strike `history/_retired/`** — every occurrence in §"Retirement requires VERIFICATION" (L50, L56, L68–69)
   and §"Plans are not atomic" (L63–69). Verified-stable work is **subsumed into a living surface** with
   trajectory pointers inline (= `compact`); verified behavior is remembered **as a green test/scenario**, not a
   parked record. (The `verified_by:` warm-cascade, L82–90, is retained unchanged.)
2. **Strike the `.claude/archive/<date>/` sink role** in `.claude/scripts/memory-kit/CLAUDE.md` ("cleanup
   destinations") — redefine it as memorialized deep-archive only.
3. **Add the cardinal rule — decompose-to-zero-residue.** A concluded plan is decomposed until nothing
   plan-shaped survives in the live tree. No graveyard, no junk-drawer.
4. **Replace the single "retire" arrow** (§"Lifecycle", L19–28) with the three terminal fates (= compact /
   graduate / close-interval+forget; the renamed primitives of `2026-05-10-memory-lifecycle-design.md`).
5. **Add history-as-curated-present-pointer** (§7): every history record is (a) a textbook-quality distilled
   artifact AND (b) surfaced as inline pointers planted in the live planning surface.
6. **Correct the stasis metric to the three-zone comet shape** (§8): strike the single composite-vs-1.0
   dimension model (`placement-audit.py` stasis_mode, L533; composite print L596) for three independent zone
   ratios — pile-leanness (shrink), museum-proportion (grow), working-memory budget (hold) — each gated, not
   averaged. (This same correction is PROPOSED for `2026-06-01-unified-memory-loop-design.md`, whose stasis
   readout inherited the single-score model; it is **not yet applied** to either file.)

## 11. Acceptance criteria

1. **Front-link enforced at creation.** Every spec/plan born after this lands carries lineage frontmatter
   (`supersedes`/`superseded_by`/`refines`/`derived_from`/`compacted_from`/`spawned_backlog` + a one-line "why
   we turned") pointing at the canonical seed(s) the FRONT step surfaced; empty lineage is admissible *only* when
   both lexical (`spec-coherence-index.py --query`) **and** semantic (MemPalace) surfacing returned zero seeds.
   Falsifiable: grep new-doc frontmatter; any standalone doc with a non-empty surfacing result fails.
2. **Zero plan-shaped residue at conclusion.** After a plan's decompose-self runs, `placement-audit.py
   --headline` reports **0** ACTIVE-home docs with terminal status still living plan-shaped (the "N plans
   past-due to decompose" field reads 0). Falsifiable: the headline field. **Verifiable once the
   placement-enforcement hook lands** (§10.1, not yet built) — the "N plans past-due" field is produced by that
   hook's counter; until it ships this criterion has no number to read.
3. **No dumping grounds exist.** `genesis/docs/content/elohim-protocol/history/_retired/` does not exist;
   `.claude/archive/` total bytes do not exceed the curated-history total bytes by more than a stated
   deep-archive allowance; MemPalace contains zero embeddings whose source path is under an ACTIVE home.
   Falsifiable: `find`/`du` + an index-source audit.
4. **History records are dual-form.** Every file under `history/` is bidirectionally canonical-linked (the
   existing `has_canonical_link` check in stasis_mode) AND has at least one inline pointer planted in a live
   planning surface. Falsifiable: the audit's `history: bidirectional canonical link` dimension reads 100%, plus
   a pointer-presence grep.
5. **Three-zone comet shape is the stasis verdict.** `placement-audit.py --stasis` reports three independent,
   gated zone ratios — pile-leanness (Zone 1, shrink), museum-proportion (Zone 2, grow), working-memory budget
   (Zone 3, hold) — not a single blended composite and not a single `P = T/H`. Falsifiable: the JSON gains
   `pile_to_canonical_ratio` / `pile_to_narrative_ratio` / `museum_to_narrative_ratio` / `memory_md_index_bytes`
   / `undissolved_mass_bytes`, the verdict reads per-zone (`BLOATED` / `STARVED` / `OVER-BUDGET` / `AT STASIS`),
   and `at_stasis` is the AND of the three zones, not an average. **Verifiable once the three-zone dimension set
   is added to `placement-audit.py`** (PROPOSED, §8.5 — the current script computes the single composite, not
   these three ratios).
6. **Verify-gate holds.** No plan grades its own homework: every doc transitioning to a living-surface green
   test/scenario carries an external grade (ci-investigator / CI ref / soak window) per PLACEMENT.md §"Retirement
   requires VERIFICATION"; env-blocked work sits in BLOCKED-BY-ENV hold, not history. Falsifiable: each
   subsumed-as-verified doc cites external evidence; none cite themselves.
7. **Index-after-clean ordering observed.** The one-time build runs burndown-clean → `mempalace_sync` prune →
   re-mine; semantic surfacing is trusted only after re-mine. The staleness guard flags index-age vs
   last-dissolve. Falsifiable: a stale index (older than the last dissolve) raises the flag instead of silently
   feeding the FRONT-link.
8. **One SessionStart readout.** The two past-due counts (plans-to-decompose, claims-to-grade) surface through
   the single `--headline` budget line, not separate polls. Falsifiable: SessionStart emits one budget block.
   **Verifiable once the placement-enforcement hook AND the delivery-status-poll land** (§10.1/§10.2, both not
   yet built) — the plans-to-decompose count comes from the hook and the claims-to-grade count from the
   `delivery-status-distribution.py` SessionStart wiring; neither field exists until both ship.

## 12. Risks / open issues

- **Dev-doc / functional-doc home undefined.** Fate 1 routes verified behavior into "canonical / dev-doc /
  functional-doc / research / test / scenario / code", but PLACEMENT.md's three homes name no dev-doc location.
  Either these fold into CANONICAL (`content/elohim-protocol/architecture/`) or a fourth home is added —
  unresolved, and a fourth home risks re-opening a dumping ground.
- **Three-zone bands unvalidated.** `STASIS_MARGIN` (`placement-audit.py` L78/L192) was tuned to a
  *dimension-average* threshold, not to any of the three zone ratios; carrying it over verbatim is almost
  certainly wrong. Zone 1's `≤ ~1× canonical` target is directionally clear but the exact "lean" cutoff is
  unset; Zone 2's "grows from 0.46% toward a few-percent band" is deliberately left as a directional band rather
  than a brittle single percent, but the band's edges are unvalidated against a starved starting point where
  *any* museum growth looks like progress and can mask under-curation; Zone 3's 24.4 KB index budget is the one
  band that is already established. None of the three is yet computed by `placement-audit.py` (§8.5 is PROPOSED).
- **JIT-scope mechanism for MemPalace in brainstorming.** The `/brainstorm` command currently has no MemPalace
  wiring (confirmed: no `mempalace` reference under `.claude/commands/brainstorm.md` or the brainstorm skill),
  and `prep-brainstorm.py` runs only the *lexical* `spec-coherence-index.py --query`. The two semantic paths are
  not equivalent (§4.2): the **principled** on-demand tool-load (`ToolSearch
  select:mempalace_search,mempalace_check_duplicate`) is the only one that achieves the 2-tool scope, but it is
  **un-wired into the brainstorm seam**; the **fallback** historian dispatch runs today but **over-imports all
  ~18 `mcp__mempalace__*` schemas**, so it does not meet the scope goal. Until the on-demand load is wired, the
  front-link must accept the over-importing fallback or run lexical-only.
- **MemPalace re-mine cost + staleness.** The index is frozen at mine-time (2026-05-14) and does not auto-update;
  the BACK fire point must run `mempalace_sync` prune + re-mine as an ordered step, but re-mine cost over the
  cleaned surface (and acceptable cadence) is unmeasured. Until the first burndown-clean → re-mine completes,
  semantic surfacing mines a *dirty* index — the staleness guard mitigates but does not eliminate this window.
- **Lexical brittleness is load-bearing, not incidental.** The surfacing probe returned 0 matches for
  "decompose-self / dump / forget" while the prior art sat right there. The lexical floor alone will miss
  vocabulary-shifted seeds; the FRONT-link's correctness therefore *depends* on the semantic half being wired and
  fresh — making the two MemPalace risks above load-bearing.

## 13. Sources

- `genesis/docs/PLACEMENT.md` — the contract this spec amends (homes table; lifecycle L19–28; verification gate;
  `_retired/` L50/L63–69; `verified_by` warm-cascade L82–90; enforcement-hook spec).
- `genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md` — the loop machinery this rides on;
  the two PROPOSED corrections target its single-composite stasis readout (incl. the proposed `gospel-currency`
  dimension, L48–90) and its "archive/retire to history" model.
- `genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md` — the canonical
  primitives (compact / merge / close-interval / memorialize / forget) the three fates rename.
- `genesis/docs/superpowers/specs/2026-06-01-verification-result-index-design.md` — the verify-gate dependency.
- `genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md` — the warm/hot temperature
  model the regression-warming edge applies to placement.
- `.claude/scripts/memory-kit/placement-audit.py` — stasis_mode L533 (the single dimension composite corrected
  to the three-zone shape; JSON emit L590, composite print L596); `has_canonical_link` L240/L542/L665 (gates
  Zone-2 bidirectional linkage); `STASIS_MARGIN` L78/L192, `_mfval` L185/L192; headline_mode L611 (SessionStart
  budget line). NOTE: the three zone ratios are PROPOSED — the current script computes none of them (no
  `gospel-currency` / `comet` / per-zone code present; verified by grep).
- `.claude/scripts/memory-kit/prep-brainstorm.py` (84 lines) — FRONT pre-step; lexical-only surfacing via
  `spec-coherence-index.py --query`.
- `.claude/scripts/memory-kit/spec-coherence-index.py` (152 lines) — the lexical prior-art index.
- `.claude/scripts/memory-kit/decompose.py` — the spec/plan → cited gap-items routine `decompose-self` extends.
- `.claude/scripts/memory-kit/cleanup-apply.py` — operator-gated mutation; corrected to stop moving targets into
  `.claude/archive/<date>/`.
- `.claude/scripts/memory-kit/delivery-status-distribution.py` — the not-yet-polled delivery-status source
  feeding the "claims-to-grade" count.
- `.claude/hooks/claude-md-drift-signal.py` — the PostToolUse accumulator pattern the placement-drift hook
  mirrors.
- `.claude/commands/brainstorm.md` — Step 1 PRE / Step 4 POST seam the front-link extends (no MemPalace wiring
  today).
- `.claude/agents/{historian,librarian,cartographer,storyteller}.md` — the four memory agents, all MemPalace-MCP
  equipped.
- On-disk measurements (re-verified 2026-06-02): canonical architecture = 30 docs / 923 KB; all
  vision/narrative (`genesis/docs/content/elohim-protocol/`) = 219 docs / 2.8 MB; codebase = 113,190 LOC
  (src-filtered, no `target/`/`node_modules/`); a2o `.feature` files = 326 KB; `history/` = 4 docs / 13 KB;
  `history/_retired/` absent; working memory (`.claude/memory/`) = 249 files / 842 KB (`MEMORY.md` = 47.7 KB,
  index budget ≤ 24.4 KB); active pile = 44 specs + 63 plans + 27 genesis/plans = 134 (this spec included;
  point-in-time snapshot); `.claude/archive/` = 77 files / 3.3 MB; key ratios — pile : canonical = 6.2×, pile :
  all-narrative = ~2×, history : narrative = 0.46%; MemPalace index mined 2026-05-14. (Supersedes the prior
  draft's 33.3 MB / 141-doc / 12.6 KB / 540K-LOC / 1 : 2,642 figures, which miscounted the `genesis/a2o`
  whole-directory as the vision corpus.)
