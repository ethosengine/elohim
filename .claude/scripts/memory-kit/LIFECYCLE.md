---
decided: 2026-05-14
purpose: single source of truth for the memory team's tiers, ownership, flow, and what gets distilled vs preserved vs deleted
---

# Memory Lifecycle Map

The memory team operates on a coherent set of tiers. Each tier has clear ownership, durability, and rules for what enters and what leaves. This document is the operating spec; every memory-team agent links here from their "What you operate on" section.

## The tiers

| Tier | Holds | Where | Owner | Lifecycle |
|---|---|---|---|---|
| **Gospel** | Always-loaded operating instructions | `CLAUDE.md` (root + nested) | Operator (writes); librarian (proposes when drift accumulates) | Stable; audited when signal accumulates |
| **Surface of the comet** | Active dev work — live plans, specs, shifts | `genesis/plans/`, `genesis/docs/superpowers/specs/`, `.claude/shifts/` | Operator (writes); all agents read | Live during the work; **reviewed and distilled** at completion |
| **Working memory** | Crystallized lessons, project notes, feedback | `.claude/memory/` topic files + `MEMORY.md` index | Librarian (curates); operator (writes ad-hoc) | Curated weekly or signal-driven; each entry either graduates, memorializes, or archives |
| **Canonical stories** | Wisdom-tier human narratives | `genesis/data/stories/` | Storyteller (writes); operator (confirms canonical) | Stable; revised when shape changes |
| **Timeline — chronicle** | Memorable past moments | `genesis/data/timeline/chronicle/` | Historian | Append-only |
| **Timeline — roadmap** | Forward direction themes | `genesis/data/timeline/roadmap/` | Cartographer | Status-tracked (proposed → active → achieved/abandoned) |
| **Timeline — backlog** | Ranked Objective candidates | `genesis/data/timeline/backlog/` | Cartographer | Status-tracked (proposed → ready → in-shift → completed/closed) |
| **Isildur's-diary archive** | Distilled artifacts, retrievable when a story-pointer leads back | `.claude/archive/<YYYY-MM-DD>/` | Librarian (writes via cleanup-apply) | Permanent at this tier; further compression possible |
| **MemPalace index** | Searchable view across all tiers above | `/projects/elohim/.mempalace/palace` | Librarian (sync, re-mine); storyteller (graduation tunnels) | Re-mined on signal; one-time retroactive cleanup needed after substantive refactors |

## The flow

```
[surface of the comet — live work]
   plans, specs, shifts (full-text, free-form)
       │
       │  work completes; lesson crystallizes
       ▼
[working memory — topic-named memory entries]
       │
       │  storyteller decides per entry:
       │     ├── graduate    → canonical story carries the lesson; entry archives
       │     ├── memorialize → entry moves to deep archive with story-pointer back
       │     ├── hold        → entry stays active in working memory next cycle
       │     └── archive     → no graduation; preserved anyway (we tried; record it)
       │
       │  tiny clarifications and corrections allowed in transit
       │  (typo fixes, duplicate merge, obvious mistakes — librarian's
       │  discretion during dedupe)
       ▼
[Isildur's-diary archive — `.claude/archive/<YYYY-MM-DD>/`]
   what enters here has been distilled.
   raw artifacts get reviewed first.
```

In parallel:
- **Historian** writes chronicle entries when significant moments deserve a permanent record
- **Cartographer** writes roadmap entries (forward themes) and backlog entries (ready Objectives)

## Surface of the comet vs archive — the distillation discipline

Per [[project_memory_lifecycle_comet_shape]]: the head of the comet is hot (active work, present context, MEMORY.md). The tail dwindles (working memory ages; some graduates, others archive). The **memorialized core** endures (canonical stories + Isildur-tier archive).

The discipline at the boundary:

- **Live work** is full-text, free-form. Plans are long. Specs explore. Shifts narrate moment by moment. This is OK at this tier.
- **Before archive**: review and distill. Strip what was hypothesis (now disproven or moot), tighten what remained load-bearing, surface the wisdom into working memory or a story, then archive the remainder.
- **Archive** is not a dumping ground. It's where artifacts go *after* we've extracted whatever wisdom they carry. If a plan died without producing wisdom, archive it anyway — the historian's record of "we tried X, it didn't pan out" is itself worth preserving. But the plan itself should be distilled at archive time, not raw.

A plan that gets archived without review becomes Isildur's-diary-with-200-pages-of-extraneous-genealogy. The whole point of the tier is that what's in there can be re-found *and* re-read meaningfully when the story leads back to it.

## What does NOT get destroyed

- Canonical stories — never deleted; superseded versions marked `status: retired`
- Timeline chronicle entries — append-only; never deleted (history is history)
- Active plans/specs/shifts — archived after distillation, not deleted
- Memory entries with graduated/memorialized wisdom — archived, not deleted
- CLAUDE.md surfaces — modified, not deleted (sections may be marked retired)

## What MAY be deleted (tiny, librarian-discretion)

Small-scale, no operator approval needed:
- Typo fixes (correction is functionally deletion + replacement)
- Duplicate entries during dedupe (after confirming both carry the same content)
- Stub files that were never filled in
- Generated artifacts that can be regenerated (already gitignored)
- Distilled-out paragraphs during the review-before-archive step (these were verbose hypothesis exploration; their wisdom either graduated or was discarded)

## What MAY be deleted (larger-scale, operator approval required)

- Memory entries deemed never-load-bearing (rare; default is archive)
- Stale CLAUDE.md sections
- Whole archived directories that are now redundant (e.g., superseded by stories)
- Anything destructive that affects shared state

**Default**: archive, don't delete. Tiny deletion is the exception inside cleanup ceremonies.

## Tier ownership matrix

| Tier | Reads | Writes | Modifies | Allowed tiny-delete |
|---|---|---|---|---|
| Gospel | All | Operator | Librarian (proposes); operator (approves) | No |
| Surface of the comet | All | Operator | Operator + cartographer (backlog → plan handoff) | No (review-then-archive) |
| Working memory | All | Operator + librarian | Librarian | Yes (typo, dup) |
| Canonical stories | All | Storyteller | Storyteller (drafts), operator (canonical) | No |
| Timeline chronicle | All | Historian | Historian | No |
| Timeline roadmap | All | Cartographer | Cartographer | Yes (mistakes only) |
| Timeline backlog | All | Cartographer | Cartographer | Yes (mistakes only) |
| Archive | All | Librarian (via cleanup-apply) | Operator | No |
| MemPalace | All | Librarian (sync, mine) | Storyteller (tunnels) | Sync prunes auto; manual delete only by operator |

## MemPalace — index that needs occasional retroactive cleanup

The palace mines content from working memory, plans, shifts, and elohim-protocol epic content. Drawers are embedding chunks; tunnels mark relationships (especially graduation tunnels from canonical stories back to memory entries they carry).

**Frozen embeddings**: a drawer is created at mine time and carries the content's state at that moment. The embedding does not auto-update when the source file changes.

**Ongoing maintenance**:
- `mempalace_sync` — prune drawers whose source files were deleted, moved, or gitignored
- `mempalace init <dir> --no-llm --yes --auto-mine` — re-mine. Content hashing skips unchanged files; only changed/new content is re-embedded
- Cadence: per-ceremony or signal-driven (after substantive refactor)

**One-time retroactive cleanup**: after a substantive refactor that changed content in-place (e.g., the 2026-05-14 Timothy → James/Terrance rename), the palace's embeddings reference stale name forms. The fix is to re-mine the affected wings. Wings to refresh after that refactor: `memory`, `plans`. Wings unaffected: `shifts` (historical, intentionally preserved), `elohim-protocol` (no rename touched its content).

## Cadence (locked in 2026-05-14 after first ceremony retrospective)

Signal-driven, not calendar-driven. The first ceremony ran 2026-05-14; all four retrospectives converged on signal-over-cadence.

Triggers (any of):
- MEMORY.md byte-budget ≥ 90% of 24.4 KB threshold
- Audit-script substrate edit (mtime on `.claude/scripts/memory-kit/audit*`)
- 3+ sprint-results in one wing within 14-day window
- Manifesto edit
- Operator-invoked (`/memory-ceremony`)

Floor: monthly if no signal fires. Ceiling: biweekly upper bound (cartographer rec — don't over-run).

**Variants**:
- **Pre-shift readiness check**: lightweight (librarian + storyteller-disposition only)
- **Full pass**: all four agents, six waves, balance-sheet capture at Wave 0 + Wave 6
- **After substantive refactors** that affect palace embeddings: re-mine wing(s) + full pass

## The memorialize-vs-archive discriminator (added 2026-05-14)

When triaging dispositions, the boundary between **memorialize** and **archive-without-graduation** can blur on entries with technical forensic detail. The clean criterion (held up in first ceremony):

> **Does memorializing add retrievability the archive doesn't already give?**

- **Memorialize** when: the artifact's specifics will be searched-for via a story-pointer that doesn't exist yet (e.g., legacy env names, configuration constants, named characters); deep-archive routing matters for findability.
- **Archive-without-graduation** when: the artifact is superseded and links forward; any future reader will grep code/cargo errors, not search the story catalog. No story will ever lead back. Preservation is "we tried; record it," not "story will retrieve this later."

If you can't name the story that would eventually lead back, it's archive-without-graduation, not memorialize.

## The `graduate-pending` interstitial status (added 2026-05-14)

When the named graduation story exists at `status: draft` (not yet `canonical`), entries waiting on operator-confirmation accumulate in HOLD. The HOLD bucket bloats; entries that should graduate stay in active rotation.

**Resolution**: introduce a `graduate-pending` interstitial. An entry marked `graduate-pending` is:
- Named in the draft story's `graduates_memory[]` frontmatter
- Held in working memory (not yet archived)
- Auto-promoted to `graduate` when operator flips the story to `status: canonical`

Routing per Wave 2 storyteller decision:
- Story exists & canonical: **graduate** (librarian archives at next cleanup)
- Story exists & draft: **graduate-pending** (waits on operator-canonical)
- No story exists: **needs-new-story** (cartographer ranks "write story of X" as backlog Objective)
- Story exists but candidate isn't a fit: **hold** (revisit when shape clarifies)

The `graduate-pending` status removes the bloat without forcing premature graduation.

## Story composition primitive — the 5 streams (added 2026-05-14 Round 3)

The storyteller's load-bearing methodology for authoring any new canonical story. Story-summarization-from-scenarios alone produces narrative that drifts from substrate, persona, and precedent. The 5-stream pattern keeps composition anchored.

### The 5 streams

| # | Stream | Source | What the story must do with it |
|---|---|---|---|
| 1 | **Epic anchors** | `genesis/docs/content/elohim-protocol/` (`anchors_epics:`) | Body echoes at least one philosophical principle from each anchored epic — not decorative reference |
| 2 | **Persona records** | `genesis/data/humans/`, `genesis/data/collectives/`, `genesis/data/roles/` (`subject`, `role`, `characters:`) | Uses canonical persona language; never invents characterization that contradicts the record |
| 3 | **Scenarios** | `genesis/a2o/features/**/*.feature` (`feature:`, `adjacent_features:`) | Dramatizes behaviors that are actually scenario-anchored; new behaviors get a `When/Then` line or new adjacent_feature first |
| 4 | **Device archetypes** | `genesis/data/devices/` (`devices:`) | Honors device-as-actor with its record's affordances; device is not a prop |
| 5 | **Historian consultation** | Pre-write query to historian agent | Cites at least one historian-surfaced precedent in `relatedNodeIds:` or body footnote; receives "no-resonance" notes as positive signal (unprecedented territory) |

### Per-story `sourced_from:` frontmatter

Every new canonical story includes:

```yaml
sourced_from:
  epics: [...]
  personas: [...]
  scenarios: [...]
  devices: [...]
  historian_precedents: [...]
```

Each array MAY be empty if explicitly justified with an inline rationale comment (e.g., `devices: []  # pure governance narrative; no devices touched`). Empty arrays without comments are flagged by the librarian's story-currency audit (Wave 1 hygiene component).

### Coverage data as substrate (not prescription)

The librarian's `story-coverage-audit.py` script computes neutral coverage data:

- `features_on_disk` — total `.feature` files
- `features_orphan` — features not anchored by any canonical story
- `features_canonical_anchored` — features anchored by at least one canonical story
- per-orphan `leverage_score` — scenario count × adjacency factor
- per-canonical-story sourcing-completeness flags
- `dangling_feature_references` — canonical stories whose `feature:` triple does not resolve to a file on disk

The audit exposes data. It does not prescribe action. Each lens (storyteller, cartographer, historian, librarian) reads the same numbers and reaches its own conclusion per its own judgment in the context of the cycle's other signals. There is no fixed threshold that pre-determines when an agent should shift behavior; convergence (or divergence) across lenses on what the numbers mean is itself signal for the operator at Wave 4.

### Data flow — end-to-end

```
story-coverage-audit.py (Wave 1 prologue, every ceremony)
        │
        │  regenerates .claude/memory-kit/story-coverage-audit.json
        ▼
librarian Wave 1 dispatch — surfaces the numbers as neutral data
        │
        ▼
cartographer Wave 1 + Wave 3, storyteller Wave 2 — each reads the same data
        │
        │  each lens interprets per its own judgment in the cycle's context
        ▼
Wave 3 synthesis — cartographer recommends a /shift Objective based on full substrate
        │
        ▼
Wave 4 operator review — question set is derived from what lenses actually surfaced
        │
        │  operator decides → /shift dispatch with pre-authored Objective (whatever it is)
        ▼
... if storyteller authors a story, the next audit measures the new coverage
```

What the numbers mean for any given cycle is emergent across lenses, not pre-computed at substrate boundaries.

### Historian consultation — first-class primitive

The historian's role in the 5-stream pattern is not just "search for prior shape" — it's a **structured query/response contract**. See `.claude/agents/historian.md` "Storyteller consultation primitive" for the canonical query and response shapes. The storyteller pastes the response (with confidence tags) directly into `sourced_from.historian_precedents:`.

## The author/delivery axis split — `status:` vs `delivery_status:` (added 2026-05-14 Run #2)

After Run #2, four-way convergence surfaced that the storyteller's `status:` (draft / canonical / retired) and substrate-evidence are **orthogonal axes**. Author-status says *"the storyteller has finished composing; operator sealed it as canonical narrative."* Delivery-status says *"the feature this story dramatizes is actually visible to a human at some maturity level — `/deliver`'s tier-3 stewardship verdict has confirmed it."* See [[feedback_story_delivery_status_axis]].

**The unified delivery-status gradient** (most-delivered → least-delivered; `regression` is orthogonal-sideways):

```
stable                              ← held green long enough across releases to be load-bearing
regression                          ← was-stable, now broken (sideways; can apply anywhere right of wip)
active.latest-stable                ← released and marked stable in its current release-channel
active.beta                         ← released, hardening
active.alpha                        ← released, exploratory
wip                                 ← in active development
refined                             ← definition-of-done complete, ready to pull
backlog                             ← identified, not yet refined
envisioned                          ← idea-stage; in manifesto/vision, no backlog entry yet
```

This is the **same lifecycle as backlog `status:`** ({proposed → ready → in-progress → done}) extended at both ends: `envisioned` upstream of `backlog`; `wip → active.* → stable` downstream of `done`. Backlog entries, feature files, and stories share **one lifecycle vocabulary**, registered at `genesis/graphos/vocabulary.md` (TBD).

### Authority boundary — `/deliver` owns `active.*` and `stable` (CRITICAL)

`/deliver` (`.claude/skills/deliver/SKILL.md`) is the **only** authority that can confer states at-or-above `active.alpha`. Its tier-3 stewardship verdict (rendered screenshot judged against the FeaturePromise) is the falsifier `CI green` alone cannot supply — the whole point of `/deliver` is closing the gap *"CI green ≠ human-visible delivery."* If the memory-ceremony group could author `active.*`/`stable`/`regression` from raw cucumber JSON, that gap reopens.

**Clean authority split across the gradient**:

| State | Authority | Substrate evidence |
|---|---|---|
| `envisioned` | operator / manifesto | epic doc cites it; no backlog yet |
| `backlog` | cartographer (writes); storyteller (surfaces from stories' coverage gaps) | timeline/backlog/*.md exists |
| `refined` | cartographer (DoD complete + `shift_objective` filled) | backlog item ready for `/shift` |
| `wip` | sprint / agentic-developer | active shift in flight |
| `active.alpha` / `active.beta` / `active.latest-stable` | **`/deliver` ONLY** | tier-3 verdict `delivered` + plan_deliverables cited verbatim |
| `stable` | **`/deliver` ONLY** | graduates from `active.latest-stable` after holding green across N releases (working N = 5 sprint-reports) |
| `regression` | **`/deliver` ONLY** | sideways flip from any `active.*` or `stable` when visual/scenario signals turn red |

**What the memory-ceremony group keeps**:
- Storyteller authors story `status:` (draft / canonical / retired)
- Cartographer authors backlog/roadmap entries at `envisioned` / `backlog` / `refined` states
- Librarian gates graduations on `/deliver`'s verdict (requires `delivery_status >= active.latest-stable`)
- Historian surfaces precedents/risks involving `regression`

The memory-ceremony group is an **observer-and-bridge**, not an authoring party for `active.*`/`stable`/`regression`. It can forward evidence to `/deliver` (e.g., "story claims feature X; X is unauthored") as backlog candidates; it cannot mint the verdict.

### Disposition matrix — extended with delivery axis

| Disposition | Story `status:` required | `delivery_status:` (read from `/deliver`) | Librarian action |
|---|---|---|---|
| `graduate` (legacy) | canonical | (not checked) | Archives memory entry to `.claude/archive/<date>/graduated/` |
| `graduated-narratively` (new) | canonical | `< active.latest-stable` (anything `/deliver` has not confirmed delivered) | Archives memory entry; flags `delivery-debt` for cartographer to write/refine the backlog `write-the-feature` entry |
| `graduated-fully` (new) | canonical | `>= active.latest-stable` (`/deliver` has confirmed) | Archives memory entry; no debt flag |
| `graduate-pending` | draft | (not checked) | Holds in working memory; auto-promotes on operator-canonical flip |
| `memorialize` | (any) | (any) | Moves to `.claude/archive/<date>/memorialized/` with story-pointer |
| `hold` | (any) | (any) | Keeps in working memory next cycle |
| `archive-without-graduation` | (no story) | (n/a) | Archives without a story-pointer; preserves trajectory |
| `tiny-delete` | (any) | (any) | Two-signature (librarian-proposes + storyteller-confirms) |

**Librarian gate (new, Run #2 retro)**: librarian SHOULD downgrade `graduate` → `graduated-narratively` when the linked feature `delivery_status < active.latest-stable` as reported by `/deliver`'s verdict. The Run #2 james-son flip would have surfaced this: canonical narrative + nonexistent `.feature` = no `/deliver` verdict possible = `delivery_status: undelivered` (synthetic value: "below envisioned; story claims a feature that isn't on disk"). The graduation still completes because the narrative carries the lesson; the cartographer sees a `delivery-debt` backlog candidate ("author `stewarded-device-sync.feature` + run through `/deliver`").

### Ownership of the delivery axis

| Artifact | Who authors `status:` | Who writes `delivery_status:` |
|---|---|---|
| Story (`genesis/data/stories/*.md`) | Storyteller + operator (canonical flip) | **`/deliver`'s auto-poll bridge** (read-only to storyteller; never operator-authored). Story `delivery_status` is **derived**, never directly minted — the bridge aggregates feature-level verdicts via the policy in the next section. |
| Backlog (`genesis/data/timeline/backlog/*.md`) | Cartographer | Cartographer up to `refined`; `/deliver`'s auto-poll for `wip → active.* → stable` |
| Roadmap (`genesis/data/timeline/roadmap/*.md`) | Cartographer | Cartographer (theme-shaped; `delivery_status` rarely applies) |
| Feature (`genesis/a2o/features/**/*.feature`) | Whoever wrote the scenario | **`/deliver`'s auto-poll** (only `/deliver` can mint `active.*`/`stable`/`regression`) |

### Story-level aggregation from feature verdicts — weakest-link policy

A story's `delivery_status` is **derived**, not authored. `/deliver` writes feature-level verdicts to `.claude/deliver/manifest.json`; the `deliver-bridge` auto-poller computes the story-level aggregate from the verdicts on the feature(s) the story dramatizes.

**Contributing feature set per story**: `{canonical feature} ∪ {adjacent_features}` — both the `feature:` triple component and every entry in `adjacent_features[]` count. The story is only as delivered as its weakest-delivered contributing feature.

**Aggregation rule**: `delivery_status = min(contributing-feature delivery_status, by gradient order)`. Stated plainly: if any contributing feature is `undelivered`, the story is `undelivered`. If all contributing features are at `active.beta` except one at `wip`, the story is `wip`. The weakest link sets the story's level.

**Gradient order for the `min()`** (low → high, weakest-first):

```
regression  <  unknown  <  undelivered  <  pending  <  envisioned  <  backlog
            <  refined  <  wip  <  active.alpha  <  active.beta
            <  active.latest-stable  <  stable
```

**Sticky exception — `regression` propagates UP**: `regression` is sideways-orthogonal in the per-feature axis (a feature can flip from `active.beta` to `regression` and back). At the story-aggregation layer, **any contributing feature in `regression` forces the story to `regression`**, regardless of how the others are doing. Regression dominates. The rationale: a story whose narrative claims a delivered experience cannot itself be "delivered" while one of its load-bearing scenarios is broken; the story carries the regression until the substrate repairs.

**Who computes this**: the `deliver-bridge` auto-poller (`.claude/scripts/memory-kit/delivery-status-poll.py`, when authored). The storyteller never writes `delivery_status` on a canonical story directly above `wip`; the operator may set a `wip`-or-below floor when seeding a new story before any feature has been judged. Anything `active.alpha` or above must be derived from a manifest verdict.

**What `/deliver` does NOT do**: `/deliver` writes per-feature verdicts; it does not compute story aggregates. The aggregation is the bridge's job. This keeps `/deliver` focused on the rendered-experience falsifier and the memory-team's bridge focused on the read-side projection. `/deliver`'s feature-level writes are stable; the bridge re-derives story aggregates on each poll.

### Auto-poller — observer-and-bridge spec (compact)

**Role**: the auto-poller is NOT an authoring tool. It is a **bridge** that reads `/deliver`'s output and writes `delivery_status:` onto stories and backlog entries that link to features `/deliver` has judged. The memory-ceremony group reads the written values; it does not produce the verdict.

**Lives at**: `.claude/scripts/memory-kit/delivery-status-poll.py` (librarian-invoked; bridge surface, not authority surface)

**Reads** (the primary source is `/deliver`'s output; raw a2o reports only fill the floor of the gradient where `/deliver` has not yet judged):

1. **`/deliver` verdicts** (the load-bearing source):
   - `.claude/shifts/*-deliver-*/sprint-result.md` — final tier-3 verdict + `plan_deliverables` cited verbatim + scenarios + screenshot artifact
   - `.claude/shifts/*-deliver-*/iter*-verdict.md` — per-iteration verdicts (latest is current)
   - Each verdict ∈ {`delivered`, `partial`, `error_state`, `missing`}. The bridge maps:
     - `delivered` (single render) → `active.alpha`
     - `delivered` (two consecutive, one fresh-trigger — `/deliver`'s done criterion) → `active.beta`
     - `delivered` + visual validation tag `@elohim-visually-validated` confirmed in the sprint-report → `active.latest-stable`
     - `delivered` + green across last 5 `/deliver` sessions touching the feature → `stable`
     - `error_state` or `partial` after prior `delivered` → `regression` (preserves prior level in `regression_from:`)
2. **Floor signals only** (for features `/deliver` has not yet processed, never to overwrite a `/deliver` verdict):
   - File does not exist on disk → `undelivered`
   - File exists, no Gherkin steps → `envisioned`
   - File exists, has steps, never run → `backlog`
   - File exists, has steps, last cucumber-run skipped/pending → `refined`
   - File exists, last cucumber-run mixed pass/fail → `wip`
3. **Linkage substrate**:
   - `genesis/data/stories/*.md` frontmatter `feature:` slug
   - `genesis/data/timeline/backlog/*.md` frontmatter `relatedNodeIds[]` for feature linkage

**Substrate gap to flag (Run #2 finding)**: `/deliver` currently writes prose sprint-results, not a structured `delivery-manifest.json`. The bridge has to grep the sprint-result for verdict + feature path, which is brittle. **Backlog candidate**: `/deliver` should persist a structured artifact (`.claude/deliver/manifest.json` or per-shift `delivery-verdict.json`) keyed by feature path → {verdict, iteration, fresh-trigger, timestamp, sprint-result-path}. Until that exists, the bridge runs in best-effort mode and surfaces ambiguous cases to the operator.

**Writes** (bridge-only; never to feature files themselves — those belong to `/deliver`):
- Story frontmatter: `delivery_status: <gradient-value>` + `delivery_status_updated: <date>` + `delivery_status_source: deliver-bridge` (or `deliver-bridge-floor` when only floor signals available)
- Backlog frontmatter: same fields (when the backlog entry has a linked feature AND `/deliver` has judged it)
- Aggregate report at `.claude/memory-kit/<today>/delivery-status.md` for librarian audit surface — surfaces graduation-delivery-gaps and delivery-debt items for cartographer

**Cadence**: signal-driven, accumulator pattern. Triggers:
- PostToolUse Edit on any `.claude/shifts/*-deliver-*/**` artifact → increment delivery-drift counter (this is the load-bearing trigger — `/deliver` finished an iteration)
- PostToolUse Edit on any `*.feature` or `genesis/data/stories/*.md` → smaller increment (linkage substrate changed)
- Operator-invoked via `/memory-kit delivery-poll`
- Floor: Wave 1 of every memory-ceremony

**Output discipline**: only surface stories/backlog where `delivery_status` changed since last poll, OR where graduation-delivery-gap (canonical + `< active.latest-stable`) is detected. No-change is silent.

## Dimensions with stasis targets (added Round 4)

This is the canonical table cartographer consults at Wave 3 to draft the stasis implementation plan. Each dimension has a measurable substrate signal, an aspirational target, and named resolution agents. Wave 6 dispatches against this table; the chronicle records the achievement per dimension.

The stasis-progress invariant: every cycle must advance each non-deferred dimension by at least 20% of its outstanding drift (`current → current - 0.2 × (current - target)`), or explicitly mark it `deferred-with-rationale`. Silent demotion to baseline-noise (the Run #3 failure mode) is no longer acceptable.

| Dimension | Source | Target | Resolution agent(s) |
|---|---|---|---|
| CLAUDE.md DRIFTED-FACTUAL count | `claude-md-audit.md` | 0 | Librarian (proposes edits; operator confirms gospel changes) |
| CLAUDE.md OVER-BUDGET count | `claude-md-audit.md` | 0 | Librarian (with operator confirmation for substantive trim) |
| Cleanup-scan review flags | `cleanup-proposals.md` | 0 | Librarian (judgment subagent) + storyteller (disposition triage) |
| Agent catalog real findings | `agent-audit.md` (filter durable false-positives: TOOLS-MISMATCH 19/19, OVER-IMPERATIVE 18/19) | 0 | Librarian |
| Skill catalog overlap pairs (non-orthogonal) | `skill-audit.md` | 0 | Librarian (disambiguating description edits) |
| Story orphan-ratio | `story-coverage-audit.json` (`features_orphan / features_on_disk`) | 0 | Storyteller (authoring canonical stories) |
| Story sourcing-completeness flags | `story-coverage-audit.json` (per-story empty `sourced_from:` keys without inline rationale) | 0 | Storyteller (backfill `sourced_from:` or add justifying comment) |
| MEMORY.md byte size | `memory-review.md` | ≤24,400 bytes (24.4 KB) | Librarian (graduate stale entries; tighten index lines) |
| Surface:Archive ratio | `memory-balance.sh` | <100:1 | Storyteller (graduations to canonical story) + librarian (archive distillation) |
| /deliver pickup queue | `delivery-status-distribution.json` (count of `refined` or `wip` features awaiting `/deliver` verdict) | drains to 0 over time | `/deliver` (only authority that mints `active.*` / `stable` / `regression`) |

### Notes on this table

- **Operator-tunable**: cartographer can propose additions or removals (e.g., a new dimension surfaces in retrospective; a dimension proves to be noise and gets retired). Changes go through operator approval at Wave 4 of the ceremony that proposes them.
- **Targets are aspirational floors, not hard SLAs**: a target of 0 means "we keep advancing toward 0 until we reach it"; it does not mean "must reach 0 this cycle." The 20%-floor governs per-cycle pace.
- **Resolution agent owns the dimension end-to-end**: they propose the action at Wave 3 (via cartographer's plan), execute it at Wave 6, and report the achievement back into the chronicle table. If a dimension straddles agents (e.g., Surface:Archive ratio depends on both storyteller graduations and librarian archive), the cartographer names the primary agent and lists the co-agent in the proposed action.
- **The deferred-with-rationale option**: when a dimension's 20% genuinely can't advance this cycle (substrate dependency, blocked-by-upstream-work, effort-tier=large, out-of-cycle ownership), cartographer marks it `deferred-with-rationale` and chronicle records WHY. This prevents the silent-demotion failure mode while preserving agent agency. Valid deferral patterns are enumerated in `.claude/agents/cartographer.md` "Stasis implementation plan" section.
- **For very-small counts (1-3)**: "advance = touch at least 1" is an acceptable 20%-floor interpretation. Exact arithmetic on tiny counts produces misleading thresholds; the spirit is "make at least one meaningful move on this dimension or explain why not."

### Why these dimensions and not others

The dimensions enumerated above all share three properties:
1. They produce a numeric or enumerable signal from a deterministic audit script (no judgment-call counts)
2. They correspond to substrate the memory-team owns (not `/deliver`'s verdicts directly, not the operator's gospel-authoring)
3. They have been observed to drift over multiple cycles, or they are foundational health metrics (MEMORY.md size, surface:archive ratio) where staying inside the target is itself the work

Dimensions explicitly **not** included (and why):
- **MemPalace drawer count growth** — embedding accumulation is expected and bounded by source-file count, not a drift metric
- **Backlog entry count** — quantity is not a quality signal; cartographer ranks by vision×readiness, not by drainage rate
- **Chronicle entry count** — append-only; growth is intentional
- **Number of Wave 5 retrospective signals captured** — exploratory output; not a drift metric

If a candidate dimension fails these three properties, it doesn't belong here; it belongs in the broader audit substrate where lenses interpret it per cycle.

## Related

- [[project_memory_lifecycle_comet_shape]] — head + tail + memorialized core
- [[project_forgetting_as_design]] — forgetting is the design, not the failure
- [[project_subconscious_memory_tier]] — Isildur's-diary tier semantics
- [[project_wisdom_resolves_into_epics]] — story-compaction destination
- [[project_three_temporal_perspectives]] — history / development / roadmap triad
- [[reference_mempalace]] — substrate details + known constraints
- `.claude/agents/{librarian,historian,storyteller,cartographer}.md` — each agent's role in this lifecycle
- `genesis/data/stories/CONVENTIONS.md` — stories tier schema
- `genesis/data/timeline/CONVENTIONS.md` — timeline tier schema (chronicle + roadmap + backlog)
- `.claude/skills/memory-ceremony/SKILL.md` — Wave 3 stasis-plan template + Wave 6 execution flow
