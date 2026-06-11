---
id: avodah-pillar-gospel
cites:
  - avodah-domain-gospel | the work design subject this pillar renders — work-story/work-project vocabulary + the REA work-event coupling (renders, never redefines) | sha256:6ba5a52e1647a05b | path: elohim/sdk/domains/avodah/CLAUDE.md
---

# Avodah Pillar — Work as Protocol Participation

Avodah (Hebrew: work / service / worship — the same word) is a **reference
implementation**, not a true pillar. It demonstrates that work and contribution —
projects, boards, backlogs, stories, recurring tasks — are *not* a bespoke feature but
just another shape of the protocol's content + REA substrate. A work-project is a
`ContentNode`; a story is a `ContentNode`; moving a story to a terminal column emits an
REA economic event. Nothing here is a new entry type.

> **shaped by:** D1 (EPR Envelope & Graph Substrate) — avodah is a *demonstrator only*.
> It owns no architecture seed; it proves the substrate by riding on lamad's content
> primitives and shefa's REA events. See
> [Architecture seeds that shape me](#architecture-seeds-that-shape-me).
>
> Memory anchor: `project_avodah_pillar` — "Avodah is protocol-as-process, not a pillar."

---

## Subject home & citation discipline (this pillar is a CONSUMER)

This pillar renders the avodah subject; the vocabulary (`work-story`/`work-project`, the terminal-column →
REA `work`-event coupling) is owned at the cited subject home `avodah-domain-gospel`
(`elohim/sdk/domains/avodah/`), which in turn assumes the protocol substrate + shefa. The cite is
content-addressed: a change at the subject home (or the substrate below it) drifts this gospel STALE.

**Where code citations to the subject belong:**
- `../generated/metadata-types` (`WorkProjectMeta`/`WorkStoryMeta`) is DERIVED from the subject home — never
  hand-edit; regenerate. The "schema is the source of truth" note IS the code-side citation.
- When code encodes the terminal-column → REA `work` event coupling, leave a `// subject: avodah-domain-gospel`
  breadcrumb (the coupling is subject-owned, not pillar-invented).

## Why a reference impl, not a pillar

A true pillar (lamad, imagodei, qahal, shefa) owns domain vocabulary, models, and a
manifest. Avodah owns **almost no primitives of its own** — it composes existing ones:

- A **project** is a `ContentNode` with `contentType: 'work-project'`.
- A **story** is a `ContentNode` with `contentType: 'work-story'`.
- **Status / board column** lives in `ContentNode.metadata`, not in a new field.
- Moving a story to a **terminal column** (`isTerminal: true`, e.g. "Done") fires an REA
  economic event (`action: 'work'`) — work becomes a *contribution event* on the same
  substrate as content authorship or compute delegation.

This is the point of the pillar: if work-management can be built from `ContentNode` +
metadata + an REA event with no new entry types, then the substrate is genuinely
general. Avodah is the proof gallery for "collapse bureaucracy into protocol."

---

## The cross-pillar dependencies (read these to understand avodah)

Avodah is thin *because* it leans on others. The load-bearing imports:

| From | What avodah uses | Why |
|------|------------------|-----|
| `@app/lamad/models/content-node.model` | `ContentNode`, `ContentMetadata`, `ContentRelationshipType` | Projects and stories ARE content nodes |
| `@app/elohim/services/storage-api.service` (`StorageApiService`) | `getContents`, `updateContent`, `createEconomicEvent` | All persistence + the terminal-column REA event |
| `@app/imagodei/guards/identity.guard` (`identityGuard`) | Route guard | Write routes require network auth |
| `../generated/metadata-types` | `WorkProjectMeta`, `WorkStoryMeta` | Schema-governed metadata shapes (codegen'd, never hand-edited) |

If you find yourself adding a new entry type, a new DHT shape, or a bespoke persistence
path in avodah, **stop** — that is the signal you've left "reference impl" and should
either (a) push the primitive down into lamad/shefa where it belongs, or (b) reconsider
the design via the `p2p-design-gate` skill.

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
| `AvodahApiService` (`services/avodah-api.service.ts`) | The only service. `getProjects()` / `getStoriesForProject()` fetch `ContentNode`s by `contentType`; `updateStoryStatus()` patches metadata and, on a terminal column, fires `createEconomicEvent({ action: 'work', contentId })`; `updateStoryField()` patches arbitrary fields. It is a thin adapter over `StorageApiService` — it owns no state. |

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
  override them.
- **Terminal column = contribution event** — moving a story to a terminal column is the
  moment work becomes economically legible. `updateStoryStatus(..., isTerminal: true)`
  emits the REA `work` event. This is the substrate-correct way to record "work
  happened" — not a side table.
- **Cadence** — `WorkCadence` (daily/weekly/monthly/custom) drives recurring tasks: a
  terminal move resets the story to `backlog`/`todo` at `nextOccurrence`. Cadences are
  archetype-tunable in the broader protocol (see `project_cadence_archetype_tunable`).
- **Visibility** — `private | community | exchange`: the work-item's reach, mirroring the
  protocol's reach vocabulary rather than inventing an access model.

## Build & test

Avodah is part of the `elohim-app` build (it lives under `app/elohim-app/src/app/`):

```bash
cd app/elohim-app
pnpm exec vitest run --config vite.config.ts avodah   # avodah unit tests
pnpm run build                                         # full app build incl. avodah
```

## Terminology

- "work" / "contribution" / "steward" — avodah is *avodah* (work-as-service); avoid
  "task management" framing where "contribution" is truer.
- A finished story is a **contribution event**, not just a closed ticket.

## Architecture seeds that shape me

Avodah is a **demonstrator** — it has no architecture seed of its own. It is constrained
by the substrate seeds it rides on:

- **D1 — EPR Envelope & Graph Substrate** —
  `architecture/2026-04-21-elohim-core-graph-substrate-design.md`. Avodah proves the
  substrate's generality: work-projects/stories are EPR-shaped content atoms, no new
  entry types.
- **(via lamad, D3)** Records lifecycle — story status transitions mirror the
  Active/Closed state machine in
  `architecture/2026-05-24-records-lifecycle-design.md`.
- **(via shefa, D9)** REA economic events — the terminal-column `work` event is an REA
  event per
  `architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md`.

## See also

- Memory anchor: `project_avodah_pillar` (protocol-as-process, not a pillar)
- Substrate REA primitive: `project_rea_compute_commitment_primitive`,
  `project_collapse_bureaucracy_into_protocol`
- Content primitive it rides on: `app/lamad/src/app/claude.md` (the `ContentNode`)
- Storage API it adapts: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`
- Architecture graph (the GRAPH): `architecture/INDEX.md` — and the WALK: `architecture/MAP.md`
