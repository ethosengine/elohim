---
id: "backlog-storyteller-coverage-sprint"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Storyteller coverage sprint — first wave (5-7 canonical stories) against 69 orphan features"
slug: "storyteller-coverage-sprint"
written: "2026-05-14"
author: "cartographer"
status: "refined"
priority: "high"
relatedNodeIds:
  - "memkit:2026-05-14/story-coverage-audit"
  - "memory:feedback_story_delivery_status_axis"
  - "memory:project_wisdom_resolves_into_epics"
  - "memory:project_three_temporal_perspectives"
  - "chronicle:2026-05-14-memory-ceremony-run-2"
  - "story:james-son--as-stewardee--stewarded-device-sync"
  - "backlog:persona-rename-canonical-flip"
tags: [storyteller, coverage, canonical-stories, substrate-narrative-gap, wave-5-retro]
shift_objective: |
  The story-coverage audit (2026-05-14) measures the substrate-narrative coverage: 73
  features on disk, 0 features canonically anchored by any story, 69 features fully orphan.
  Round 3 ceremony substrate updates landed the 5-stream composition pattern (epics + personas
  + scenarios + devices + historian consultation) as available methodology. This sprint, if
  selected by the storyteller's lens as the cycle's disposition, executes coverage authoring
  in two phases:
  PHASE 1 — author qahal-collective-governance (leverage 90.0, highest-leverage orphan; also
  introduces the non-human-persona pattern) as a methodology probe; append inline notes on
  how the 5-stream pattern landed; pause for operator confirmation. PHASE 2 — author the
  remaining 5 stories (jessica-attention-analytics, matthew-love-map, james-auth-lifecycle,
  elohim-agent-network-sentry, terrance-ssr-capability) only after Phase 1 validates the
  methodology. Each story uses the 5-stream pattern, includes `sourced_from:` frontmatter
  with historian-precedent citations, bootstraps `delivery_status: undelivered` per the
  floor pattern. Done when (a) all 6 stories at `status: canonical`; (b) INDEX.md
  two-axis rendering updated; (c) `story-coverage-audit.py` re-run drops orphan count by ≥ 18.
  Further coverage waves are signal-triggered by future ceremonies' lens reads.
---

# Storyteller coverage sprint — first wave (refined)

## Status

**Refined as of 2026-05-14, Round 3** — the substrate updates that make this work
addressable have landed:

- `story-coverage-audit.py` runs every ceremony Wave 1 prologue (neutral coverage data)
- Librarian surfaces the coverage numbers as Wave 1 substrate
- Cartographer reads the audit data alongside other Wave 1 signals per its per-cycle lens
- Storyteller has AUTHOR-CANONICAL-STORY as an available disposition class
- Historian consultation primitive defined (5th stream of composition — invoked when storyteller chooses to author)
- LIFECYCLE.md documents the 5-stream composition primitive as methodology reference

This entry is available substrate for the storyteller's lens. Whether the storyteller selects
canonical-story authoring as its Wave 2 disposition is its per-cycle judgment, weighed against
the other dispositions available. If selected, the AUTHOR-CANONICAL-STORY items become Wave 4
operator-decision items alongside the other ceremony outputs.

## Sequencing — two phases

The sprint splits into two phases by methodology-validation risk:

### Phase 1 — methodology probe (single story: qahal-collective-governance)

**Why first**: the 5-stream composition pattern is new (Round 3 substrate change). Before
authoring 5 more stories against it, we validate the methodology on the highest-leverage
target (leverage 90.0, 36 scenarios anchored in one stroke). The qahal-collective story
also introduces the **non-human persona pattern** (the collective itself as `subject`),
which deserves dedicated authoring attention rather than being batched.

**Done when**:
- Story authored at `genesis/data/stories/qahal-collective--as-governed--collective-governance.md`
- `sourced_from:` frontmatter populated with all 5 keys (or explicit inline rationale per empty key)
- Historian consultation completed BEFORE authoring; precedents cited in `sourced_from.historian_precedents:`
- Storyteller appends an inline methodology-validation note: did the 5-stream pattern work? Did historian consultation produce useful precedents? Any stream that fought back? (Feeds Round 4 ceremony retro.)
- Operator confirms `status: canonical`
- `story-coverage-audit.py` re-run; orphan count drops by ~36 (canonical only — adjacents tracked separately)

### Phase 2 — full coverage (remaining 5 stories)

**Trigger**: Phase 1 lands and operator confirms the methodology works. If methodology probe reveals
the 5-stream pattern is too heavy / too light / needs adjustment, Phase 2 is paused and the
substrate update happens first.

**Targets** (per the original list below — refresh from `story-coverage-audit.json` rankings if
the orphan landscape has shifted):

2. `human-jessica-spouse--as-attention-steward--attention-analytics` (leverage 24.0, lamad)
3. `human-matthew-manager--as-relationship-author--love-map-negotiation` (leverage 24.0, lamad)
4. `human-james-son--as-identity-claimant--auth-lifecycle` (leverage 25.0, imagodei)
5. `elohim-agent--as-network-sentry--network-health-posture` (leverage 19.0, elohim — second non-human persona)
6. `human-terrance-tutor--as-content-deliverer--ssr-capability` (leverage 28.0, content)

**Done when**:
- All 6 stories (Phase 1 + Phase 2) authored at `status: canonical`
- INDEX.md updated with all 6 rows in two-axis rendering
- Each story has `sourced_from:` block + historian-precedent citations
- Re-running `story-coverage-audit.py` drops orphan count by ≥ 18 (6 × ~3 avg adjacents) and canonical-anchored count rises from 0 to 6
- Catchup mode likely still active (orphan-ratio would drop from 0.94 to ~0.84) — Phase 3 sprint signal-triggers when next round of orphan clusters surface

## Why this matters

Substrate-truth has dramatically outrun narrative-truth: 73 features on disk, 0 canonically
anchored, 69 total orphans, 1 dangling reference (the one canonical story declares a feature
that doesn't exist). This is exactly the failure mode the wisdom entry
[[feedback_story_delivery_status_axis]] anticipated — author-status and delivery-status drift
apart silently. Story is the compaction artifact in the comet-shape of memory; without
canonical stories, the substrate's lived experience cannot graduate to load-bearing wisdom
and the corpus accumulates capability without meaning.

## Scope decision

**N = 6 stories**, one per major archetype, hitting six distinct pillars. Not 14-15 — that
would burn a full sprint on a single agent and miss the signal-driven cadence point.
Six is enough to (a) prove the canonical authoring pattern at scale beyond james-son,
(b) cut orphan count by ~18-24 features (canonical + adjacents), (c) seed each pillar
with at least one canonical anchor so future stories have neighbors. Subsequent waves
are signal-triggered.

## Proposed first-wave stories

Each is `<subject>--<role>--<feature>` (storyteller's canonical triple). Subjects are drawn
from the existing persona graph where possible; new subjects are flagged.

1. **`qahal-collective--as-governed--collective-governance`** — leverage 90.0, highest by
   far. Qahal pillar. Subject is the collective itself (a non-human persona — a community
   as first-class actor); role `as-governed` captures the experience of being inside a
   governing collective. Anchors 36 scenarios in one stroke.

2. **`human-jessica-spouse--as-attention-steward--attention-analytics`** — leverage 24.0
   (canonical) plus likely adjacent pickup of `attention-stewardship` family. Lamad pillar.
   Jessica is already in the james-son story characters; promoting her to subject in her
   own story makes the persona graph reciprocal.

3. **`human-matthew-manager--as-relationship-author--love-map-negotiation`** — leverage 24.0.
   Lamad pillar (relational learning archetype). Matthew is the canonical adult/manager
   persona; love-map is one of the highest-leverage relational features. Distinct from
   Jessica's attention-stewardship.

4. **`human-james-son--as-identity-claimant--auth-lifecycle`** — leverage 25.0. Imagodei
   pillar (auth/identity). Re-uses james-son in his secondary role (`as-identity-claimant`
   vs his canonical `as-stewardee`); proves the multi-role pattern from the conventions.
   Anchors auth-lifecycle + likely adjacents recovery-emergency-quorum and visitor-boundaries.

5. **`elohim-agent--as-network-sentry--network-health-posture`** — leverage 19.0. Elohim
   pillar. Subject is the elohim agent itself (the first-class non-human persona the
   protocol is named after). Anchors network-health-posture (19 scenarios) and likely
   adjacents in federation/peer-advertisement.

6. **`human-terrance-tutor--as-content-deliverer--ssr-capability`** — leverage 28.0. Content
   pillar (doorway/SSR). Terrance is the existing tutor persona; ssr_capability is the
   highest-leverage content-pillar orphan. Anchors content delivery experience without
   competing with the lamad archetypes above.

**Pillar coverage**: qahal (1), lamad (2), imagodei (1), elohim (1), content/doorway (1).
**Shefa pillar deferred** — orphan list shows no high-leverage shefa features; signal-driven
next wave when shefa features mature.

## What's blocking

- **Single-agent storyteller** — six stories is the upper edge of one /shift. Recommend
  the storyteller author two at a time (paired by pillar adjacency) over 3 passes inside
  the sprint, with cartographer doing per-pair INDEX.md updates and orphan-count
  re-measurement. If pace lags, the sprint can split at the natural pair-boundary.
- **Persona pre-existence** — qahal-collective and elohim-agent are new subject types
  (non-human). Storyteller may need a 10-minute "non-human persona schema" decision
  before authoring; resolve inline.
- **Feature-file dangling** — every canonical triple risks the same dangling-reference
  state james-son's story currently has. Acceptable: the audit's dangling list IS the
  cartographer's separate "feature-authoring backlog" generator. Don't block on this.

## What's ready

- Audit script (`story-coverage-audit.py`) gives the orphan ranking and re-measurement.
- Triple-frontmatter schema is proven (james-son story is the canonical exemplar).
- LIFECYCLE.md weakest-link aggregation lands today (operator-confirmed).
- Unified delivery-status gradient is in [[feedback_story_delivery_status_axis]] —
  every new story bootstraps `delivery_status: undelivered` per the floor pattern.
- INDEX.md two-axis rendering exists for james-son; just needs more rows.

## Convergence

This entry sits at the intersection of four signals from the 2026-05-14 ceremony:

- **Wave 5 retro convergence** — all four agents independently flagged delivery_status
  as the next-cycle axis; the wisdom entry already memorialized it.
- **Operator question** — "where's the storyteller backlog?" surfaced exactly this gap
  during Wave 5; the audit script was the deterministic answer.
- **Cartographer Wave 3 NEEDS-NEW-STORY** — Run #2 surfaced multiple memory entries
  flagged as "wants a canonical story to graduate"; this sprint creates the substrate.
- **Run #2 chronicle** — recorded the storyteller's first canonical authorship; this
  sprint extends the pattern from N=1 to N=7 (existing + 6).

## Definition of done

1. **6 canonical stories authored** at
   `genesis/data/stories/<subject>--<role>--<feature>.md` matching the six triples above
   (or storyteller-revised triples if a better archetype surfaces during authoring).
2. Each story has the full triple frontmatter (`subject`, `role`, `feature`) plus
   `adjacent_features[]` plus `delivery_status: undelivered` plus
   `delivery_status_source: deliver-bridge-floor` per the bootstrap pattern.
3. **Stories INDEX.md updated** with all 6 new rows in the two-axis rendering
   (author_status × delivery_status).
4. **Canonical feature files** — for each story, either the feature file already exists
   (preferred) OR the cartographer captures it as a separate backlog entry
   `author-<slug>.feature` for sprint pickup. Dangling is acceptable as long as it's
   tracked.
5. **Re-run `story-coverage-audit.py`** — orphan count drops by ≥ 18 (6 stories × 3
   avg adjacents) and canonical-anchored count rises from 0 to 6.

## Secondary substrate-quality items (for next-cycle pickup, not this sprint)

Three small items surfaced by the audit during this synthesis — each lightweight enough
that they don't need their own backlog entries; just naming them so the next cartographer
cycle sees them:

1. **`_clean()` helper graduation** — three callers across audit scripts now share an
   inline `_clean()` frontmatter-scalar helper; per `project_shared_lib_pattern` the
   ≥3-caller threshold is tripped. Promote to `.claude/scripts/_lib/frontmatter.py`
   as `clean_scalar()`.
2. **Audit script slug-collision rendering** — when two stories with different filename
   stems share a slug, the per-story coverage summary doesn't disambiguate (N=2 today;
   low priority but will bite at N≥5).
3. **Bridge-substrate wiring** — `.claude/deliver/manifest.json` is empty; the audit's
   `/deliver?` column lookup returns `—` for every orphan. Once /deliver verdicts
   accumulate, this column becomes informative and the audit can rank by delivery
   tension (story exists × delivery missing) rather than pure leverage.

## Authorability judgment

**Borderline — recommend pair-by-pair pacing, with a hard split point at story 3.** Six
canonical stories in one /shift is feasible for an Opus-tier storyteller with the proven
james-son schema in hand, but the qahal-collective and elohim-agent stories introduce the
non-human-persona pattern for the first time, which may need its own brief decision pass.
Realistic shape: stories 1-3 (qahal + 2 lamad) as one /shift; stories 4-6 (auth + elohim
+ content) as a second /shift; cartographer re-runs the audit between passes and surfaces
adjustments. If the operator wants one-shift discipline, drop to N=4 (stories 1, 2, 4, 5)
and signal-trigger the rest.
