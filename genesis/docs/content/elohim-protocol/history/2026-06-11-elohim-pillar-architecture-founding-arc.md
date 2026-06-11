---
id: elohim-pillar-architecture-founding-arc
status: noted
tier: history
derived_from:
  - app/elohim-app/src/app/elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md   # retired to git 2026-06-11 (elohim-pillar island recompose)
  - app/elohim-app/src/app/elohim/ARCHITECTURE.md                   # retired to git 2026-06-11 (elohim-pillar island recompose)
cites:
  - CLAUDE.md
  - elohim-app-frontend-gospel | the shell gospel whose §Deployment Contexts is the live home for what ARCHITECTURE.md duplicated | sha256:1aed2111237ae7d0 | path: app/elohim-app/CLAUDE.md
  - elohim-pillar-gospel | the live pillar gospel that absorbed this retirement's drift-repairs (models/services/reach/contract tables) | sha256:2d9e49a724a24e9a | path: app/elohim-app/src/app/elohim/CLAUDE.md
  - app/elohim-app/src/app/elohim/models/protocol-core.model.ts
  - app/elohim-app/src/app/elohim/services/data-loader.service.ts
  - app/elohim-app/src/app/elohim/services/projection-api.service.ts
  - app/elohim-library/projects/elohim-service/src/client/elohim-client.ts
  - app/elohim-library/projects/elohim-service/src/connection/README.md
  - app/elohim-library/projects/elohim-service/src/angular/models/agent.model.ts
  - app/elohim-library/projects/elohim-service/src/angular/models/source-chain.model.ts
  - elohim/sdk/schemas/v1/enums/reach.schema.json
  - elohim-protocol-specification | the protocol canon whose three-pillar EPR taxonomy collides with this doc's five-app-pillar usage — the recorded name-collision hazard | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
  - genesis/docs/content/elohim-protocol/hardware-spec.md
  - qahal-api-spec-extraction-arc | sibling record — the qahal branch of the same 5e7e0b952 spec family; this record closes the family index | sha256:810ed38282d0cbc4 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-qahal-api-spec-extraction-arc.md
---

# The Elohim Pillar Architecture Founding Arc

## What they documented

Two architecture docs lived in the elohim pillar's directory, retired together
in the elohim-pillar island recompose:

**`ELOHIM_PROTOCOL_ARCHITECTURE.md`** (234 lines) was born 2025-12-08 in
commit 5e7e0b952 "refactor(spec): split specifications by pillar ownership" —
the same commit that extracted `QAHAL_API_SPECIFICATION_v1.0.md` from
`LAMAD_API_SPECIFICATION_v1.0.md`. It is the third member of that document
family and its *index*: the §Specification Documents table (lines 172-183)
catalogued the family, pointing at the lamad spec (retired 2026-06-11, commit
c8cb7ebe3, lamad island recompose) and the qahal spec (recomposed same day,
commit bf4367e98; see sibling record `qahal-api-spec-extraction-arc`). This
retirement closes the family index itself. The doc carried: the "Five
Pillars" diagram, the Model Ownership Matrix ("Each model has a canonical
location"), the §Shared Module tree, a 15-value `CrossPillarLinkType`
vocabulary, import conventions, a 5-step Holochain Migration Path, and five
constitutional principles deferring to the manifesto.

**`ARCHITECTURE.md`** (191 lines) arrived 2026-01-16 in e004a7624 — a *perf*
commit ("Replace eager content index load with lightweight readiness check"),
which is why its §Performance Considerations leads with
`DataLoaderService.checkReadiness()`. It documented the Progressive
Sovereignty deployment ladder (Browser/Doorway → Tauri/Direct → Federation
Node), the unified elohim-storage API boundary, the connection-strategy
pattern, the service-layer stack, and the geographic 8-value `ReachLevel`.
Both moved to their final path in 6195cdfe9 (2026-03-11, frontend
consolidation into `app/`) and were never substantively revised after birth.

## What survived

The founding doc's *organizing ideas* are now gospel, restated in stronger
homes:

- **The pillar organization itself.** The five Hebrew-named pillars became the
  root `CLAUDE.md` §Domain Pillars table — now SIX (doorway joined), plus an
  `avodah` directory in `app/elohim-app/src/app/` the doc never anticipated.
- **Model-canonical-location.** "Other pillars may re-export through barrel
  files but should not duplicate the model definition" survives as root
  `CLAUDE.md`'s "Import via barrel exports" rule.
- **DataLoaderService as the single adapter point.** "The single point of
  change for Holochain migration" (line 220) is live at
  `app/elohim-app/src/app/elohim/services/data-loader.service.ts` and still
  framed as "the ONLY service that knows about data sources" in the pillar
  gospel (`app/elohim-app/src/app/elohim/CLAUDE.md`).
- **The geographic ReachLevel.** ARCHITECTURE.md's 8 values (lines 164-171)
  match `protocol-core.model.ts` lines 50-58 exactly: private / invited /
  local / neighborhood / municipal / bioregional / regional / commons.
- **ReachEnforcer** is live (`elohim-client.ts:108` in elohim-library);
  **ProjectionApiService** is live (`projection-api.service.ts`); the
  **connection strategies** (Doorway/Tauri/Direct) are real code with their
  own home at `connection/README.md` — though ARCHITECTURE.md's §Related
  Documentation pointer to `connection/CLAUDE.md` is DEAD (no such file; the
  subject home exists under a different filename).

## How they rotted

1. **The §Shared Module went stale 17 minutes after authorship.** The doc was
   born 14:16:33; commit 23d29c2a2 ("merge shared/ into elohim/ pillar")
   deleted the documented `shared/` tree at 14:33:07 *the same day* — and
   touched the doc by exactly 2 lines (one import example,
   `@app/shared/services` → `@app/elohim/services`). The 15-line tree diagram
   and "When to Use Shared" guidance survived the commit that falsified them.
   All four listed services (affinity-tracking, governance, human-consent,
   profile) live in `elohim/services/` today; `shared/` now holds only
   `components/`.
2. **CrossPillarLinkType was replaced wholesale.** The doc's 15-value
   vocabulary (`content_governance` … `elohim_stewardship`, lines 139-167) and
   the live 14-value union (`protocol-core.model.ts:610-637`,
   `identity_authors_content` … `custom`) share NOT ONE value. The axis
   reorganized too: the doc's pairs led with lamad; the live set leads with
   imagodei (6 of 14 begin `identity_`). The type *name* and the
   pillar-pairing *shape* survived; the entire vocabulary turned over.
3. **The Model Ownership Matrix decayed in every row.** `elohim/models/` has
   20 files beside the barrel (matrix lists 9); its `agent.model.ts` and
   `source-chain.model.ts` MIGRATED to
   `app/elohim-library/projects/elohim-service/src/angular/models/` (commits
   0be9bddfb, 1e0397854 — Slice 2.1) — rot by relocation, not deletion.
   `app/elohim-app/src/app/lamad/` no longer exists (lamad is its own
   `app/lamad` bundle). Shefa — "uses Elohim models", zero own — has 11 model
   files of its own. Imagodei has 15 (matrix lists 3).
4. **"Five pillars" collided and lost.** Root CLAUDE.md counts six pillars;
   meanwhile the protocol spec's "three-pillar" means something else entirely
   — the EPR semantic dimensions lamad/shefa/qahal
   (`protocol-specification.md` lines 62, 1259-1270). Same word, two
   taxonomies (app-code organization vs addressable-unit semantics); recorded
   here as a standing name-collision hazard.
5. **The Holochain Migration Path was overtaken by the real architecture.**
   Written as future ("All models are designed to migrate to Holochain entry
   types"), it was superseded by what actually landed: elohim-storage +
   doorway + DHT with the views.rs camelCase boundary (root CLAUDE.md §Data
   Flow; protocol spec). The 5-step list is a fossil of the pre-substrate
   imagination.
6. **Reach drift gained a fourth strand.** The DNA-notarized schema enum
   (`elohim/sdk/schemas/v1/enums/reach.schema.json`) is *relational*: private
   / self / intimate / trusted / familiar / community / public / commons —
   different from the TS geographic 8 that both these docs and
   `protocol-core.model.ts` carry. The known three-vocabulary drift (schema
   enum ≠ Rust `reach_earning.rs` ≠ resilience-epic Part V) thus has a fourth
   strand in the frontend. Recorded as drift evidence only — adjudication is
   the reconciliation work that gates the storage-stewardship-summary route.
7. **ARCHITECTURE.md's stack diagrams became near-duplicates.** Its service
   stack and data-flow examples are restated (current, maintained) in
   `app/elohim-app/src/app/elohim/CLAUDE.md` §API Boundary Architecture and
   `app/elohim-app/CLAUDE.md` §Deployment Contexts; the island doc was the
   un-refreshed copy.

## What never got another home

ARCHITECTURE.md's §Progressive Sovereignty Model table — the explicit mapping
of *deployment mode → data access → offline → sovereignty level* (Browser/
Doorway = hosted keys; Tauri/Direct = local SQLite + keys; Federation Node =
full + network contribution) — is only partially homed. The framing lives in
`genesis/docs/content/elohim-protocol/hardware-spec.md` §Progressive
Sovereignty: The Onboarding Journey, but as a *hardware/onboarding* ladder
(Visitor / Hosted User / App User / Node Operator). No live doc carries the
app-deployment-mode ↔ sovereignty mapping; root CLAUDE.md's four deployment
contexts list the modes without the sovereignty axis. RESIDUE: still true,
homed nowhere else.

OPEN QUESTION: the live pillar gospel
(`app/elohim-app/src/app/elohim/CLAUDE.md`) still lists `agent.model.ts` and
`source-chain.model.ts` in its Models table though both migrated to
elohim-library — does the recompose session refresh that table, or does the
migration get reverted/re-exported?

OPEN QUESTION: is the live 14-value `CrossPillarLinkType` itself anchored to
any substrate vocabulary (DHT link types, a manifest), or is it a second
generation of the same doc-only pattern that rotted the first?

## Why it matters for the future

- **Vocabulary tables in docs rot silently while the type system evolves in
  code.** The CrossPillarLinkType table looked authoritative for six months
  with zero surviving values. A doc that *copies* a union type out of a
  `.ts` file starts dying at the next edit to that file; the durable move is
  citing the source path, not transcribing values.
- **A doc can be falsified by the very commit that touches it.** 23d29c2a2
  updated the one line that broke compilation-adjacent examples and left the
  prose section describing the directory it deleted. Mechanical edits
  (imports, paths) get fixed; narrative sections (trees, guidance) don't —
  rot concentrates where no tool complains.
- **Org charts outlive inventories.** The pillar decomposition, the
  canonical-location rule, and the single-adapter-point principle all
  survived verbatim into gospel; every file-level inventory (model matrix,
  shared tree, spec index) was wrong within months. Architecture docs should
  state invariants and point at directories, not enumerate files.
- **The founding commit diagnosed its own disease.** 5e7e0b952's message:
  "only the specs had drifted during holistic integration work." The cure —
  splitting specs by pillar ownership — created three more documents that
  drifted the same way, family index included. Ownership boundaries don't
  prevent drift; refresh mechanisms (content-addressed cites, gates) do.
- **Both docs were write-once.** Neither received a substantive revision
  after its birth commit; each was authored as a snapshot during a refactor
  (spec split; perf change) and then abandoned by every later refactor that
  rearranged what it described. Snapshot docs belong in history-tier from
  day one.
