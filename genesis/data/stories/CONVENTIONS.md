# Stories — Conventions

Canonical human stories that compose with the protocol's data layer (humans, devices, roles, features) and serve as **Tier 1 narrative anchors** in the experience-story EPR design (see `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md`). Stories are first-class protocol content — when seeded, each story file becomes a `ContentNode` with `contentType: experience-story`, addressable via EPR alias and linked into the lamad graph.

The **storyteller** agent (`.claude/agents/storyteller.md`) owns this directory. Other agents read; only the storyteller and the operator write.

## What a story is

A Tier 1 experience-story is a **stable narrative anchor** identified by a `(subject, role, feature)` triple. It accumulates worth over time as the test harness mints Tier 2 `story-point` attestations against it (per the design spec; out of scope for the authoring layer). The narrative anchor itself — what this directory holds — is the human-readable canonical text: roughly 500–1500 words that dramatize the protocol's behavior through one persona's experience in one role exercising one feature.

A story is **not**:
- A specification (that's the plan/spec layer under `genesis/plans/`)
- A Gherkin scenario (those are under `genesis/a2o/features/`)
- A manifesto epic (those are under `genesis/docs/content/elohim-protocol/`)
- An experience-moment (Tier 3, agent-scoped private, per-run)
- A story-point attestation (Tier 2, machine-minted, valenced)
- Marketing copy ([[project_values_forward_disclosure_accountability]])

A story succeeds when a reader who has never read a spec recognizes themselves in it — *"oh yeah, that's me."*

## Triple identity

Every canonical story is identified by a `(subject, role, feature)` triple. The triple is the story's identity — two stories with the same triple are the same story. Multi-feature human narratives are split into N stories with shared body text where appropriate.

| Element | What it is | Examples |
|---|---|---|
| `subject` | A `human-*` or `collective-*` id from `genesis/data/humans/` or `genesis/data/collectives/` — who the story is about. | `human-terrance-tutor`, `collective-maintainers` |
| `role` | A `role-*` id — the role the subject occupies in this story. | `role-as-stewardee`, `role-as-entrepreneur`, `role-as-maintainer` |
| `feature` | A Gherkin feature file path under `a2o/features/`, slugified. The canonical feature whose passing proves the story's experience is real. | `feature-stewarded-device-sync`, `feature-learning-journey` |

When seeded into DHT, these become typed Holochain Links: `:hasSubject`, `:inRole`, `:exercises`. The EPR alias is derived: `epr:experience-story/{subject-slug}/{role-slug}/{feature-slug}`.

## File layout

```
genesis/data/stories/
├── CONVENTIONS.md          # this file
├── INDEX.md                # storyteller-maintained catalog (by theme, character, role, feature, graduated memory)
├── <subject-slug>--<role-slug>--<feature-slug>.md   # one story per triple
└── ...
```

Filenames encode the triple via double-dash separators for unambiguous parsing while keeping the canonical short title in frontmatter. Example: `terrance-tutor--as-stewardee--stewarded-device-sync.md`. Operators may add a short slug field for nicer human references; the filename remains the triple.

## Frontmatter schema

Every story file begins with YAML frontmatter, then the prose body. The frontmatter is verifiable — every id reference must resolve to an existing record in the data corpus. A future `validate-stories.ts` (mirroring `validate-humans.ts`) will fail CI on dangling references.

```yaml
---
# ContentNode identity (matches the lamad ContentNode schema; seeded into DHT)
id: "experience-story-terrance-tutor--as-stewardee--stewarded-device-sync"
contentType: "experience-story"
contentFormat: "epr-composite"

# Triple (the canonical identity — three links when seeded)
subject: "human-terrance-tutor"           # → :hasSubject link
role: "role-as-stewardee"                # → :inRole link
feature: "feature-stewarded-device-sync" # → :exercises link

# Human metadata
title: "James and the Spoke"
description: "Witnessed evidence of human:terrance-tutor exercising feature:stewarded-device-sync in role:as-stewardee."
slug: "terrance-and-the-spoke"            # short reference; INDEX uses this
version: 1
written: "2026-05-14"
author: "storyteller"
status: "draft"                          # draft | canonical | retired

# Delivery axis — orthogonal to author-status. Auto-poller-maintained; NEVER operator/storyteller-authored.
# See `.claude/scripts/memory-kit/LIFECYCLE.md` "The author/delivery axis split" section.
# Values: undelivered | envisioned | backlog | refined | wip | active.alpha | active.beta | active.latest-stable | stable | regression
# active.*/stable/regression are conferred ONLY by `/deliver`'s tier-3 stewardship verdict.
delivery_status: "undelivered"           # read-only to storyteller — written by deliver-bridge
delivery_status_updated: "2026-05-14"
delivery_status_source: "deliver-bridge-floor"   # deliver-bridge | deliver-bridge-floor | operator-override

# EPR alias (derived; recorded for navigation)
epr_alias: "epr:experience-story/terrance-tutor/as-stewardee/stewarded-device-sync"

# Other characters and devices that appear in the narrative (not the subject)
characters:
  - "human-jessica-spouse"
devices:
  - "device-chromebook-edu"
  - "device-family-node-base"

# Adjacent features the narrative touches (not the canonical feature; for coverage discipline)
adjacent_features:
  - "imagodei/community-attestation.feature"
  - "qahal/household-sync-handshake.feature"

# Vision anchors — epics whose philosophy this story instantiates
anchors_epics:
  - "governance_layers/family.md"
  - "social_medium/community-attestation.md"

# Memory graduation (storyteller's curate authority)
graduates_memory:                        # memory entries that can defer to this story for their lesson
  - "project_stewarded_child_identity"
  - "project_household_fabric"
memorializes: []                         # memory entries that move to deep-archive, retrievable via this story's pointer

# ContentNode tags (used by seeding + discoverability)
tags:
  - "experience-story"
  - "@as-stewardee"                      # role tag, prefixed with @ per spec
  - "stewardship"
  - "ceremonial-ux"
  - "graduated-authority"

# relatedNodeIds — computed at seed time from subject + role + feature + characters + devices + anchors_epics.
# Do not hand-maintain.
---
```

### Field semantics

| Field | Meaning |
|---|---|
| `id` | Globally unique ContentNode id. Pattern: `experience-story-<subject-slug>--<role-slug>--<feature-slug>`. |
| `contentType` | Always `experience-story` for files in this directory. Lamad manifest must list this contentType. |
| `contentFormat` | Always `epr-composite`. Established core protocol format (see CLAUDE.md). |
| `subject` / `role` / `feature` | The triple. Each references an id that must exist. |
| `slug` | Short human reference for INDEX and conversation. Does not need to be globally unique within the directory if filenames disambiguate. |
| `status: draft` | Storyteller is working on it. Not a graduation target. |
| `status: canonical` | Operator has confirmed. Safe to seed; safe for memory graduation (with delivery-gate). |
| `status: retired` | Superseded by another story. Keep the file; it carries history. |
| `delivery_status` | Orthogonal axis: substrate-evidence that the feature the story dramatizes actually runs at some maturity. **Read-only to storyteller.** Written by the `deliver-bridge` auto-poller from `/deliver`'s tier-3 verdicts (`active.*`/`stable`/`regression`) or from raw a2o signals (`envisioned`/`backlog`/`refined`/`wip` floor only). See [[feedback_story_delivery_status_axis]]. |
| `delivery_status_updated` | Date the auto-poller last wrote `delivery_status`. |
| `delivery_status_source` | `deliver-bridge` (verdict from `/deliver`), `deliver-bridge-floor` (only a2o floor signal, no verdict yet), or `operator-override` (rare, for backfilling). |
| `epr_alias` | The resolvable EPR URL form. Derived from the triple but recorded so cross-references in other docs can be grep-stable. |
| `characters` | Additional `human-*`/`collective-*` ids that appear in the prose but are not the subject. |
| `devices` | Devices that appear in the narrative. |
| `adjacent_features` | Other Gherkin features the narrative touches. Feeds the storyteller's coverage gap reporting; not part of identity. |
| `anchors_epics` | Epic paths (relative to `docs/content/elohim-protocol/`) the story instantiates philosophically. |
| `graduates_memory` | Memory entry slugs (no `.md`) that the librarian can now archive safely — this story carries the lesson. |
| `memorializes` | Memory entries kept in deep archive ([[project_subconscious_memory_tier]]), retrievable via this story's pointer when needed. |
| `tags` | Includes `experience-story`, the `@-prefixed role tag`, and free-form theme tags. INDEX groups by themes. |

## Two orthogonal axes — author-status vs delivery-status

Every story has two independent axes that the librarian / cartographer / `/deliver` triangulate over:

| Axis | Values | Owner | Lifecycle |
|---|---|---|---|
| `status:` (author) | `draft` → `canonical` → `retired` | Storyteller authors; operator flips to canonical | Narrative truth — "is the story composed and sealed?" |
| `delivery_status:` (substrate) | `undelivered` → `envisioned` → `backlog` → `refined` → `wip` → `active.alpha` → `active.beta` → `active.latest-stable` → `stable` (with `regression` as orthogonal sideways) | Auto-poller (`deliver-bridge`) writes; `/deliver` confers `active.*`/`stable`/`regression` | Substrate truth — "does the feature this story dramatizes actually run?" |

A story can be `canonical, undelivered` (narrative sealed; feature not yet authored — Run #2 surfaced this on james-and-the-spoke). A story can be `draft, active.latest-stable` (feature delivered but storyteller still composing). A story can be `canonical, stable` (the desirable end state — narrative and substrate both at the load-bearing tier). The two axes never collapse.

**Authority boundary**: `/deliver` (`.claude/skills/deliver/SKILL.md`) is the only authority that can confer `active.*`/`stable`/`regression`. The memory-ceremony group (librarian/historian/storyteller/cartographer) **reads** the delivery axis to gate graduations and surface delivery-debt; it does not author the verdict. The `deliver-bridge` auto-poller transcribes `/deliver`'s sprint-result verdicts onto the story frontmatter; it is a bridge, not an authoring tool. See `.claude/scripts/memory-kit/LIFECYCLE.md` "Authority boundary" for the full table.

**Graduation gate (Run #2)**: librarian downgrades `graduate` → `graduated-narratively` when `delivery_status < active.latest-stable`. The narrative still graduates (the lesson is carried), but a `delivery-debt` flag goes to the cartographer's backlog (e.g., "author `stewarded-device-sync.feature` and run through `/deliver`").

**Lifecycle unification**: the `delivery_status` gradient is the **same lifecycle** as backlog `status:` ({proposed → ready → in-progress → done}) extended at both ends. `envisioned` lives upstream of `backlog`; `wip → active.* → stable` lives downstream of `done`. Stories, backlog entries, and feature files share one vocabulary, registered at `genesis/graphos/vocabulary.md` (TBD).

## Voice and length

- **Concrete over abstract**: name the person, the device, the moment. *"James's chromebook synced when his mom authorized it"* beats *"the system enables stewarded sync."*
- **Human register**: a parent reading at the kitchen table is the test reader. If they need a glossary, the story has failed.
- **Honest about friction**: ceremonial UX, deliberate waiting, attestation steps are features. Show them as such.
- **No Hebrew pillar names in narrative** ([[feedback_no_hebrew_pillar_names_in_narrative]]). *"Elohim"* stays as the protocol's name and the agent's name; *lamad/imagodei/qahal/shefa* translate to experience.
- **Length**: 500–1500 words. A story that wants more is usually two stories (two different triples sharing motifs).
- **Valence-aware**: stories can dramatize any of the seven valences from the EPR spec — `progress`, `discovery`, `regression`, `validation`, `witness`, `refinement`, `confirmation`. Failures are evidence, not defects. A story of a stewardee who couldn't sync because of legitimate household disagreement is a valid story — it dramatizes `validation` (the protocol correctly refused).

## INDEX.md

The storyteller maintains `genesis/data/stories/INDEX.md` as a curated catalog. Minimum sections:

- **By theme** — stories grouped by their `tags` (free-form theme tags, not `experience-story` or `@-role`)
- **By subject** — for each human/collective with at least one story, the stories where they are the subject
- **By role** — stories grouped by the `@-prefixed role tag` (one row per role across all subjects)
- **By feature** — for each feature with at least one experience-story, the stories that exercise it
- **By epic** — for each epic, the stories that anchor to it
- **By graduated memory** — for each memory entry that has graduated, the story that now carries it (librarian's safety check)
- **Coverage gaps** — themes/epics/(subject,role,feature) triples that have no story yet (cartographer surfaces these as candidate Objectives)

INDEX is maintained manually by the storyteller (consistent with the small N expected — the spec predicts ~50–200 experience-stories total, ever). A future enhancement is generation tooling; for now, curator-tended.

## Validator (future)

Planned: `genesis/seeder/src/validate-stories.ts` (mirroring `validate-humans.ts`). Checks:

- `id` matches the filename's triple
- `subject`, `role`, `feature` ids each resolve to existing data records or feature files
- `characters[]` and `devices[]` resolve to existing data records
- `anchors_epics[]` paths exist under `docs/content/elohim-protocol/`
- `graduates_memory[]` and `memorializes[]` slugs exist in `.claude/memory/`
- `tags` includes `experience-story` and exactly one `@-prefixed role tag` matching `role`
- `epr_alias` derives correctly from the triple
- No duplicate ids; no orphan stories absent from INDEX

Out of scope for this turn; add when the corpus is non-trivial.

## Relationship to Tier 2 and Tier 3 (per the spec)

The storyteller authors Tier 1 narrative anchors only. Tier 2 (`story-point` attestations) and Tier 3 (`experience-moment` per-run records) are produced by the test harness + discernment service, not by the storyteller writing prose. The storyteller's `mempalace_create_tunnel` authority maps to a *narrative-graduation* attestation when seeded — a special kind of Tier 2 link from a story to a graduated memory entry. This is not a `progress/discovery/regression` valence; it's an out-of-band curate attestation, distinct from the test-harness valences.

## Operator's role

Stories are canonical content. The storyteller drafts and revises; the operator confirms canonical status. Workflow:

1. Storyteller writes `status: draft` and surfaces for operator review.
2. Operator reads (or asks the storyteller to read aloud / summarize).
3. On approval, status flips to `canonical` and the storyteller mints graduation tunnels in mempalace.
4. Librarian then knows it's safe to archive entries listed in `graduates_memory`.
5. Seeder picks up canonical stories on next seed run; the ContentNode lands on DHT with derived links.

Stories should not slip into canonical without operator review. The graduation authority they carry over memory is too consequential to auto-promote.

## Related

- `.claude/agents/storyteller.md` — the agent that maintains this directory
- `.claude/memory/project_forgetting_as_design.md` — the principle this catalog serves
- `.claude/memory/project_memory_lifecycle_comet_shape.md` — head/tail/memorialized-core model
- `.claude/memory/project_subconscious_memory_tier.md` — Isildur's-diary tier (memorialize destination)
- `.claude/memory/project_wisdom_resolves_into_epics.md` — story-compaction as memory's destination
- `.claude/memory/feedback_a2o_narrative_is_opus_work.md` — narrative authoring is Opus work
- `.claude/memory/feedback_no_hebrew_pillar_names_in_narrative.md` — translation discipline
- `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` — the design spec this catalog implements (Tier 1 only)
- `genesis/data/humans/` — subject source of truth
- `genesis/data/devices/` — device source of truth (referenced in narrative)
- `genesis/docs/content/elohim-protocol/` — epic anchors
- `genesis/a2o/features/` — feature triple component + adjacent feature references
