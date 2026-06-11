---
id: avodah-pillar-gospel
cites:
  - avodah-domain-gospel | the work design subject this pillar renders — work-story/work-project vocabulary + the REA work-event coupling (renders, never redefines) | sha256:efee777a1e82a7fc | path: elohim/sdk/domains/avodah/CLAUDE.md
---

# Avodah Pillar — Work as Protocol Participation

Avodah (Hebrew: work / service / worship — the same word) is the **rendered slice of the
avodah process lens** (`avodah-domain-gospel` — *the process as canon*). This pillar's
slice is a **process-demonstrator**: it shows that work and contribution — projects,
boards, backlogs, stories, recurring tasks — are just another shape of the protocol's
content + REA substrate, rendered without new primitives. A work-project is a
`ContentNode`; a story is a `ContentNode`; moving a story to a terminal column emits an
REA economic event. This pillar introduces no new entry type.

> **shaped by:** D1 (EPR Envelope & Graph Substrate) — this slice is a *process-demonstrator
> subgraph*. It owns no architecture seed; it proves the substrate by riding on lamad's
> content primitives and shefa's REA events. See
> [Architecture seeds that shape me](#architecture-seeds-that-shape-me).
>
> Principle: "Avodah is protocol-as-process, not a pillar."

---

## Subject home & citation discipline (this pillar is a CONSUMER)

This pillar renders the avodah subject; the vocabulary (`work-story`/`work-project`, the terminal-column →
REA work-event coupling) is owned at the cited subject home `avodah-domain-gospel`
(`elohim/sdk/domains/avodah/`), which in turn assumes the protocol substrate + shefa. The rails here point
UP: the subject home assumes `elohim-protocol-specification` (the shared substrate) and `shefa-domain-gospel`
(the sibling lens whose REA primitives this pillar's work creates value through). Drift flows up the layers —
a substrate change reaches this surface through the domain's cites, not sideways through sibling app pillars.
The cite is content-addressed: a change at the subject home (or the substrate below it) drifts this gospel
STALE (spec: `2026-06-11-subject-routing-locus-graph-design.md`).

**Where code citations to the subject belong:**
- `../generated/metadata-types` (`WorkProjectMeta`/`WorkStoryMeta`) is DERIVED from the subject home — never
  hand-edit; regenerate. The schema owns notarized vocabulary; consumers conform — that truth-boundary
  applies to the REA *action value* too, not just metadata shapes.
- When code encodes the terminal-column → REA work event coupling, leave a `// subject: avodah-domain-gospel`
  breadcrumb (the coupling is subject-owned, not pillar-invented).
- If this surface's `avodah-domain-gospel` cite reads STALE, run the cite-refresh tooling — never hand-bump
  the fingerprint (`[[feedback_managed_surface_edit_discipline]]`).

## Why a process-demonstrator, not a pillar

A true pillar (lamad, imagodei, qahal, shefa) owns domain vocabulary, models, and a
manifest. This pillar's slice owns **no primitives of its own** — it composes existing ones:

- A **project** is a `ContentNode` with `contentType: 'work-project'`.
- A **story** is a `ContentNode` with `contentType: 'work-story'`.
- **Status / board column** lives in `ContentNode.metadata`, not in a new field.
- Moving a story to a **terminal column** (`isTerminal: true`, e.g. "Done") fires an REA
  economic event — work becomes a *contribution event* on the same substrate as content
  authorship or compute delegation.

What this thinness buys: demonstrated work (avodah) produces stewardship-eligibility
value (shefa) — the lamad↔avodah↔shefa loop. If work-management can be built from
`ContentNode` + metadata + an REA event with no new entry types, the substrate is
genuinely general. That value — not the absence of features — is the point of the pillar.

> **Scope note — the avodah LENS owns substrate this slice does not yet render.** "No new
> entry type" is true for *this work-board slice* (work-story/work-project). The avodah
> *domain* it renders (`elohim/sdk/domains/avodah/types/`, wired into `content_store`)
> carries the process lens's fuller vocabulary — service coordination (`ServiceRequest`/
> `ServiceOffer`/`ServiceMatch`), flow planning (`FlowPlan` family), mutual risk pooling
> (`CoveragePolicy`/`InsuranceClaim`) — the control room's substrate ahead of its
> surfaces. When you reach for "the avodah primitive," check whether you mean this slice's
> work-story or the lens's process-coordination layer (see `avodah-domain-gospel` §Two strata).

If you find yourself adding a new entry type, a new DHT shape, or a bespoke persistence
path in this pillar, **stop** — that is the signal you've left "process-demonstrator" and
should either (a) push the primitive down into the avodah domain/lamad/shefa where it
belongs, or (b) reconsider the design via the `p2p-design-gate` skill.

---

## The cross-pillar dependencies (read these to understand avodah)

This pillar is thin because it composes the substrate below it. The load-bearing imports:

| From | What avodah uses | Why |
|------|------------------|-----|
| `@app/lamad/models/content-node.model` | `ContentNode`, `ContentMetadata`, `ContentRelationshipType` | Projects and stories ARE content nodes; the `content_store` DNA entry they project from is the substrate that drifts this surface when it changes |
| `@app/elohim/services/storage-api.service` (`StorageApiService`) | `getContents`, `updateContent`, `createEconomicEvent` | All persistence + the terminal-column REA event |
| `@app/imagodei/guards/identity.guard` (`identityGuard`) | Route guard | Write routes require network auth |
| `../generated/metadata-types` | `WorkProjectMeta`, `WorkStoryMeta` | Schema-governed metadata shapes (codegen'd, never hand-edited) |

---

## Models

| Model | Purpose |
|-------|---------|
| `work-project.model.ts` | `BoardColumn`, `DEFAULT_BOARD_COLUMNS`, `parseWorkProjectMeta()`; re-exports `WorkProjectMeta` from generated types. Terminal columns (`isTerminal`) trigger the cadence reset + REA event. |
| `work-story.model.ts` | `WorkStoryStatus`, `WorkVisibility`, `WorkPriority`, `CadenceInterval`, `WorkCadence`, `parseWorkStoryMeta()`; re-exports `WorkStoryMeta`. Cadence drives recurring-task reset. |
| `models/index.ts` | Barrel |

**Metadata is schema-governed.** `WorkProjectMeta` / `WorkStoryMeta` come from
`../generated/metadata-types` (the schema is the source of truth). The `parse*Meta()`
functions only layer in defaults (default columns, default visibility/priority/status).
Do not redefine these shapes by hand.

## Services

| Service | Purpose |
|---------|---------|
| `AvodahApiService` (`services/avodah-api.service.ts`) | The only service. `getProjects()` / `getStoriesForProject()` fetch `ContentNode`s by `contentType`; `updateStoryStatus()` patches metadata and, on a terminal column, fires `createEconomicEvent({ action, contentId })`; `updateStoryField()` patches metadata fields. It is a thin adapter over `StorageApiService` — it owns no state. |

**`updateStoryField()` is a typed metadata path, not an open passthrough.** Patch only
fields the schema (`WorkStoryMeta`) governs — untyped JSON crossing the storage boundary
defeats the codegen'd contract.

## Routes & Components

Mounted under `/avodah` (`avodah.routes.ts`). Write routes are guarded by `identityGuard`
(imagodei); the home and project-list are open.

| Component | Route | Guard | Purpose |
|-----------|-------|-------|---------|
| `AvodahLayoutComponent` | (shell) | — | Layout wrapper |
| `AvodahHomeComponent` | `/avodah` | — | Landing page |
| `ProjectListComponent` | `/avodah/projects` | — | Project list |
| `ProjectBoardComponent` | `/avodah/projects/:id/board` | `identityGuard` | Kanban board (drag → status → REA on terminal) |
| `ProjectBacklogComponent` | `/avodah/projects/:id/backlog` | `identityGuard` | Backlog table |
| `StoryDetailComponent` | `/avodah/projects/:id/stories/:storyId` | `identityGuard` | Single story view/edit |
| `TaskListComponent` | `/avodah/projects/:id/tasks` | `identityGuard` | Recurring (cadence) task list |

Supporting (non-route) components: `story-card` (used by board/backlog).

## Key concepts

- **Board columns** — `DEFAULT_BOARD_COLUMNS` = backlog → todo → in-progress → review →
  done. The last is `isTerminal`. Columns live on the project's metadata; a project can
  override them. The `isTerminal` transition is the Active→Closed edge of the records
  lifecycle (D3): a closed story becomes a Record — the notarized contribution event.
- **Terminal column = contribution event** — moving a story to a terminal column is the
  moment work becomes economically legible. `updateStoryStatus(..., isTerminal: true)`
  emits the REA event whose Resource is shefa's stewardship-eligibility value (the
  lamad↔avodah↔shefa coupling — see shefa's coupling table). This is substrate-correct
  persistence of "work happened" — the event is notarized on the DHT (truth) and storage
  is the projection it reads back, not a private side table.
  - **Drift watch — the action value.** `'work'` IS a valid REA action — the notarized
    vocabulary is `REA_ACTIONS` (25 actions) in `content_store_integrity/src/lib.rs`,
    which includes `work` ("Contribute labor: stewardship, review, curation"). The real
    reconciliation: the code emits `action: 'work'` at the terminal-column moment where
    the avodah manifest's coupling declares `'produce'` (`onComplete`) — and the signal
    harness reads coupling from the manifest, so code↔manifest must converge. Secondary
    doc drift: `create-economic-event-input.schema.json` *describes* the action as
    "(use, consume, produce, transfer, cite)" — an incomplete description string (not an
    enum constraint) vs the zome's 25-action vocabulary. Multi-vocabulary reconciliation
    pattern: `[[project_reach_enum_drift_reconciliation]]`.
- **Cadence** — `WorkCadence` (daily/weekly/monthly/custom) drives recurring tasks: a
  terminal move resets the story to `backlog`/`todo` at `nextOccurrence`. Cadences are
  archetype-tunable in the broader protocol.
- **Visibility** — `WorkVisibility` (`private | community | exchange`) maps onto the
  protocol's 8-value reach vocabulary (`reach.schema.json`:
  `private, self, intimate, trusted, familiar, community, public, commons`). It is a
  reach *subset mapping*, not a faithful mirror — `exchange` is not a reach value — so it
  is a live reconciliation target against the reach enum as source of truth, not a
  settled access model (`[[project_reach_enum_drift_reconciliation]]`).

## Build & test

Avodah is part of the `elohim-app` build (it lives under `app/elohim-app/src/app/`):

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts avodah   # avodah unit tests
pnpm run build                                         # full app build incl. avodah
```

Reading a board back after a local seed shows empty? Suspect the DHT-anchor gap, not
avodah code: `hc:start:seed` bulk imports never DHT-anchor, so the provenance gate 404s
unanchored reads by design. The dev repair is the `p2p_published_at` SQLite backfill
(`[[project_local_stack_dht_anchor_gap]]`).

## Terminology

- "work" / "contribution" / "steward" — avodah is *avodah* (work-as-service); avoid
  "task management" framing where "contribution" is truer.
- A finished story is a **contribution event**, not just a closed ticket.

## Architecture seeds that shape me

This slice is a **process-demonstrator** — it has no architecture seed of its own. It is
constrained by the substrate seeds it rides on:

- **D1 — EPR Envelope & Graph Substrate** —
  `architecture/2026-04-21-elohim-core-graph-substrate-design.md`. Avodah proves the
  substrate's generality: work-projects/stories are EPR-shaped content atoms, no new
  entry types — the protocol watching itself work.
- **(via lamad, D3)** Records lifecycle — story status transitions mirror the
  Active/Closed state machine in
  `architecture/2026-05-24-records-lifecycle-design.md`; a terminal column is the
  Active→Closed edge.
- **(via shefa, D9)** REA economic events — the terminal-column work event is an REA
  event per
  `architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md`.

## See also

- Substrate REA primitive: `project_rea_compute_commitment_primitive`
- Content primitive it rides on: `app/lamad/src/app/claude.md` (the `ContentNode`)
- Storage API it adapts: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`
- Architecture graph (the GRAPH): `architecture/INDEX.md` — and the WALK: `architecture/MAP.md`
