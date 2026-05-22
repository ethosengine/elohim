# Qahal MVP Roadmap

> **For agentic workers:** This is a **roadmap-tier** document, not a task-level plan. It maps the path from architectural vision to MVP landing across multiple sprints, with explicit brainstorming checkpoints between sprints. Each sprint produces its own brainstorm → spec → task plan trio at the location named in that sprint's section.

**Goal:** Land an MVP Qahal homepage for the median human steward — a household or emergent collective they can actually use — composed of our protocol primitives (reach, standing, lamad-attested mastery, commons-elohim shadow agent, shefa value cascade) rendered through the convergent UX shape (Hylo / Discord / Matrix / Moss / Element / Meta / Sub-Reddit homepage).

**Architecture:** Qahal is one primitive with a graduated capability surface gated by lamad-attested mastery on a Bloom's curve, with the rubric itself authored and governed by the collective's stewards as a versioned EPR. Two axes — reach (outward visibility) and standing (inward capability). One autonomous commons-elohim shadow agent per Qahal holds the commons share and convenes councils for commons-scale decisions. Friction-gradient limitarianism is baked into the substrate, recursing through the wisdom layer itself.

**Tech Stack:** elohim-storage (Rust, Holochain HDK/HDI), elohim-app (Angular 19), elohim-elements (Lit web components), elohim-library (graphos pattern library — Storybook), lamad pillar (assessment engine), shefa pillar (REA economic substrate), doorway gateway (Rust, manifest-driven routes), EPR substrate (first-class graph: nodes + couplings), iroh + libp2p (P2P data plane).

**Living memories the roadmap rests on:**
- `project_qahal_graduated_capability_surface` — the core architectural insight
- `project_commons_elohim_shadow_agent` — the per-Qahal commons-interest agent
- `project_friction_gradient_limitarianism` — anti-concentration as substrate
- `project_elohim_councils_capture_apex` — gospel-tier vision; wisdom holds the structural top
- `project_first_class_graph_pattern` — EPRs as nodes, couplings as edges
- `project_social_reach_nervous_system` — reach as earned, back-prop, quarantine
- `project_standing_composes_multiple_evidence_streams` — standing computation
- `project_elohim_app_as_composable_view_federation` — pillars as federated surfaces
- `project_doorway_manifest_driven_routes` — manifests declare HTTP routes
- `project_elohim_dna_as_sdk_boundary` — elohim DNA is the SDK contract

---

## The Unified Architectural Picture

```
                    COMMONS SURFACE (low friction, reach-gated)
                       ↑
   anyone in your   ───┤  reach gates outward visibility
   peer graph          │
                       ↓
                    ENGAGED LAYER (some standing, lamad-attested low-Bloom)
                       ↑
   earned through  ────┤  standing gates inward capability
   attested mastery    │  rubric = governable EPR authored by stewards
                       ↓
                    CONTRIBUTOR LAYER (mid-Bloom: Apply, Analyze)
                       │
                       ↓
                    STEWARD LAYER (high-Bloom: Evaluate, Create)
                       │
                       ↓
                    COMMONS-SHADOW ELOHIM (autonomous agent of commons interest)

                    ← friction gradient increases →
                       toward concentration
```

### Truth-layer ownership (across the three-layer truth model)

| Concern | DHT (notary) | libp2p / iroh (data ops) | doorway (web2 projection) |
|---|---|---|---|
| Qahal identity (Layer 0) | ✓ permanent anchor | — | served via manifest route |
| Qahal rubric (versioned) | ✓ steward-authored | — | served via manifest route |
| Standing (computed view) | inputs notarized | computed at query | exposed via API |
| Commons-elohim identity | ✓ tied to Qahal genesis | runtime sense-and-respond | proxied through doorway |
| Streams / chats | content EPRs notarized | content delivery | feed projection |
| Shefa flows | EconomicEvents notarized | flow propagation | aggregation dashboards |
| Capability decisions | rubric notarized | computed per request | enforced at gateway |

---

## Sprint Map

| Sprint | Focus | Lead agent team | Brainstorming checkpoint |
|---|---|---|---|
| 0 | Vision spec consolidation | main Opus session + storyteller (canonical narratives) | A — before Sprint 0.5 |
| **0.5** | **Scenario archaeology + archetype-aligned reclassification** | **general-purpose (Opus) + librarian + storyteller** | **A.5 — before Sprint 1** |
| 1 | Qahal homepage UX exploration | graphos-designer + component-architect | B — before substrate design |
| 2 | Substrate spine design — wire definitions written first; SoT classifications for the entities introduced here (Qahal=A, QahalRubric=A2, CommonsElohimGenesis=A2, all view projections=C) are detailed in the Sprint 2 section below | rust-architect + quality-architect | C — before substrate wire-up |
| 3 | Substrate wire-up (Qahal + rubric + standing + commons-elohim co-steward stub) | rust-architect + content-pipeline (lamad attestations) | D — before frontend wire-up |
| 4 | Frontend wire-up (Library B → real backend) | angular-architect + graphos-designer + component-architect | E — before MVP demo |
| 5 | Genesis content + canonical templates + a2o scenarios | content-pipeline + storyteller (Opus) | F — MVP demo gate |

**Sprint 0.5 (new):** Inventory the 76 existing `.feature` files in `genesis/a2o/features/`; classify each against the collective archetype catalog from spec Sections 4–6; surface orphans (no archetype fit) and gaps (archetypes with no scenarios); propose new archetype-primary directory taxonomy; connect each archetype to canonical narrative or stub. Output: `genesis/docs/plans/2026-05-22-scenario-archaeology-and-archetype-map.md`. Informs Sprint 1 (UX design has clear scenario base) and Sprint 5 (new-scenario authoring has concrete gap list). Estimated duration: 1-2 days of focused agent work + operator review.

**Total estimated duration:** 4-6 weeks of focused work, depending on how much existing scaffolding the spine sprints can lean on.

**MVP exit criteria:** A median human steward (think: Matthew with the Bay Area Dawn Runners household + their faith community + their work team) can open the app, see a Qahal homepage in the convergent shape, take a lamad quiz that earns them standing in a specific Qahal, see their capability surface expand, and witness the commons-elohim's contextual view in the right nav. At least one fully-worked example Qahal (a household) is seeded.

---

## Sprint 0: Vision Spec Consolidation

**Goal:** Crystallize the conversation that produced this roadmap into a single gospel-tier spec document that future sessions read as canon.

**Why this exists as a sprint:** The vision is currently distributed across three memory entries and one roadmap document. Future sessions will not absorb the full vision from those alone. The spec is the *one place* the next contributor reads to understand what we're building and why.

**Brainstorming input:** This conversation. The three new memory entries. The architectural picture above. The previous research on nondominium (which gave us the Group vs OrgNDO frame to depart from). Existing Elohim memory on EPR, reach, standing, elohim-as-counsel.

**Agent team:**
- `quality-architect` (Opus) authors the spec; uses storyteller-pen discipline (canonical narrative grounded in real households/collectives from genesis stories)
- `storyteller` reviews for human-meaningful narrative (does a grandmother understand the vision?)
- Operator (Matthew) reviews and signs off

**Deliverables:**
- `genesis/docs/superpowers/specs/2026-05-22-qahal-architecture-vision.md` — gospel-tier vision spec covering:
  - The political/theological frame (covenant, fruit-back-on-tree, donut endstate)
  - The graduated capability surface model
  - Standing as computed view (algorithmic sketch)
  - Commons-elohim per Qahal
  - Friction-gradient limitarianism with recursion to wisdom layer
  - Two-axes design (reach × standing)
  - Imagodei lens recursion through Qahal context
  - Anti-colonization by mastery-attested gate
  - The three-layer truth model applied to Qahal
  - Open questions explicitly named for downstream sprints

**Exit criteria:**
- Spec exists, is reviewed, and is referenced from at least one CLAUDE.md surface
- Matthew confirms the spec captures the conversation faithfully
- Memory entries link to the spec as the canonical reference

**Duration:** 1-2 sessions of focused authoring.

---

## Brainstorming Checkpoint A — Before Sprint 1

**Invoke skill:** `superpowers:brainstorming`

**Inputs:** Sprint 0 spec. The Hylo/Discord/Matrix/Moss/Element/Meta/Sub-Reddit homepage shapes (operator pulls reference screenshots into `genesis/research/qahal-ux-references/` for the brainstorm).

**Question to brainstorm:**
What is the minimum viable Qahal homepage shape that:
1. Exposes all the protocol primitives (commons stream, member ring, rules, standing, shefa surface, commons-elohim contextual view) at the simple-user tier
2. Has a clear path to power-user expansion (advanced panels: REA dashboard, attestation catalogue, feedback queue, graph-discovery, rubric editor)
3. Reads as familiar to anyone who has used the reference apps
4. Honors the friction-gradient (UI itself doesn't encourage concentration moves)

**Decisions to land:**
- MVP panel set (simple-user) vs deferred panels (power-user) — explicit cut list
- Layout (left-nav + tray vs top-nav + drawer vs hybrid) — decide based on the reference apps' convergence
- Composability surface (how do app-manifest-declared panels get added to a Qahal's tray?)
- Mock-data fixtures (what household + what collective do we render in Library B stories?)
- Simple → power-user gradient mechanism (capability-tier gate? user-preference toggle? both?)

**Outputs of checkpoint A:**
- `genesis/docs/superpowers/specs/2026-05-23-qahal-homepage-ux-spec.md`
- Reference screenshots committed in `genesis/research/qahal-ux-references/`
- Mock-data fixtures named (specific households, specific collectives, specific people from genesis stories)

---

## Sprint 1: Qahal Homepage UX Exploration

**Goal:** Produce a complete graphos Library B pattern story of the Qahal homepage rendered against realistic mock data, with simple-user and power-user variants for each panel claimed.

**Why this sprint comes first:** The shape determines everything downstream. We render the shape before we commit any DHT entries. If the shape is wrong, the substrate is wrong.

**Agent team:**
- `component-architect` — authors blank-slate Lit elohim-elements for each panel (Library A in `app/elohim-elements/`), with capability profile JSDoc tags, three precondition gates, default stories
- `graphos-designer` — composes elements into the Qahal homepage pattern story (Library B in `app/elohim-library/projects/graphos/`), binds Elohim brand tokens, demonstrates the convergent shape
- `angular-architect` — sanity-checks that the resulting elements can be embedded in elohim-app without architectural surprises (no commits yet; advisory only)

**Files & deliverables:**

**Library A (blank-slate primitives) — `app/elohim-elements/`:**
- `elohim-qahal-collective-tray.ts` (left nav with collective list)
- `elohim-qahal-tooling-tray.ts` (composable tooling panel inside collective tray; app-manifest-driven)
- `elohim-qahal-commons-stream.ts` (main viewer feed)
- `elohim-qahal-member-ring.ts` (imagodei-lensed member list)
- `elohim-qahal-context-rules.ts` (right-nav: community rules panel)
- `elohim-qahal-context-elohim.ts` (right-nav: commons-elohim contextual view)
- `elohim-qahal-context-discovery.ts` (right-nav: graph-discovery suggestions)
- `elohim-qahal-breadcrumbs.ts` (top nav: context-aware tooling)
- `elohim-qahal-standing-badge.ts` (small primitive — shows requester's standing in this Qahal)
- `elohim-qahal-capability-gate.ts` (wrapper primitive — gates content by capability tier)

Each element ships with:
- Capability Profile contract via JSDoc `@capability*` tags
- a11y + i18n + ua-prefs precondition gates passing
- Default stories: Unstyled, CustomTheme, every claimed lens (simple + power-user)

**Library B (designed pattern stories) — `app/elohim-library/projects/graphos/`:**
- `qahal-homepage-household.stories.ts` — the worked example: a Bay Area family household Qahal, simple-user view
- `qahal-homepage-household-power-user.stories.ts` — same household, all panels enabled
- `qahal-homepage-faith-community.stories.ts` — a different archetype (rubric varies)
- `qahal-homepage-open-source-project.stories.ts` — third archetype (rubric varies)
- `qahal-tooling-tray-app-manifest.stories.ts` — demonstrates composable tooling via app-manifest entries

**Mock data fixtures — `app/elohim-library/projects/graphos/src/fixtures/qahal/`:**
- `household-mock.ts` — household Qahal + rubric + members + stream content + standing values
- `faith-community-mock.ts` — same shape, different archetype
- `open-source-mock.ts` — third archetype
- Each fixture composes `QahalView`, `QahalRubricView`, `StandingView`, `CommonsStreamView`, `MemberRingView`, etc. (TypeScript view types matching what Sprint 2 will define)

**Exit criteria:**
- All Library A elements pass the three precondition gates (a11y, i18n, ua-prefs)
- All Library A elements have complete default story coverage (Unstyled + CustomTheme + every claimed lens)
- All Library B pattern stories render in Storybook without errors
- The three archetype stories visually convince that Qahal can carry meaningfully different collective shapes
- Simple → power-user toggle works in Storybook
- Operator (Matthew) reviews the stories and confirms the shape feels right

**Per-sprint plan landing:** `genesis/docs/plans/2026-05-24-qahal-homepage-ux-plan.md` (task-level, written after Checkpoint A)

**Duration:** 1-2 weeks. Front-loaded — get the shape right.

---

## Brainstorming Checkpoint B — Before Sprint 2

**Invoke skill:** `superpowers:brainstorming` + `p2p-design-gate` (MANDATORY — Qahal involves new data entities)

**Inputs:** Sprint 1 Library B stories. Mock-data fixtures (these are the implicit schema). The Sprint 0 vision spec.

**P2P design gate questions to answer for each entity:**

For `Qahal` (Layer 0 identity anchor):
- Notarized (A), derived (A2), agent-scoped (B / B2), or operational (C)? → A (notarized; permanent identity)
- Does a DHT entry type already exist? Check elohim DNA headroom (~73/100 in Lamad pillar; ~11/100 in Mishpat pillar). Where does Qahal sit?
- Identity: content-derived CID? Agent-composite? Slug-with-justification?
- Coordinator function: `create_qahal`? Signal: `qahal_created`?

For `QahalRubric` (versioned EPR):
- A2 (derived: a Qahal-authored rubric attached to the Qahal via coupling)
- Versioning model — chain via `RubricUpdates` link? Monotonic version field?
- Steward authorship validation — cross-zome call to standing computation?

For `StandingComputation` (operational, not stored):
- C (operational; computed at request time, not stored as entry)
- Inputs: lamad attestations, affinity signals, FeedbackSignal debits, current rubric version
- Where does the computation live? elohim-storage Rust service? Coordinator zome function? Mix?
- Caching — sliding window, invalidation on attestation events?

For `CommonsElohimGenesis` (notarized at Qahal birth):
- A (notarized; permanent record that this Qahal spawned a commons-elohim)
- Configuration mutability — config attached as versioned coupling?

**Decisions to land:**
- Final entry types and where they live (which DNA, which zome)
- Standing computation algorithm sketch (pseudocode, not implementation)
- Rubric template format (JSON schema first — schema-first IoC)
- Friction-gradient enforcement mix (soft standing-curve flattening vs hard protocol-floor refusal)
- HTTP route shapes for the elohim-app to consume (manifest-driven)
- View schemas (`QahalView`, `QahalRubricView`, `StandingView`, `MemberRingView`, etc.) matching the Sprint 1 mock fixtures

**Outputs of checkpoint B:**
- `genesis/docs/superpowers/specs/2026-06-01-qahal-substrate-spine-design.md`
- JSON view schemas in `elohim/sdk/schemas/v1/views/` (drafts)
- Standing algorithm pseudocode in the spec

---

## Sprint 2: Substrate Spine Schema (Schema-First)

**Goal:** Lock in the wire shapes, entry schemas, and computation algorithms for the substrate before writing any Rust. Schema-first IoC (`feedback_schema_first_ioc`).

**Why schema-first:** Per existing memory, writing JSON schema first lets Rust + TS comply rather than diverge. The schemas become the contract; codegen produces the structs.

**Agent team:**
- `rust-architect` — drives the schema design across DHT entries, view schemas, doorway routes, standing computation
- `quality-architect` (Opus) — sanity-checks schemas against vision spec; catches any unimplemented-feature gaps
- `p2p-design-gate` — invoked for each entity (already triggered by Checkpoint B)

**Files & deliverables:**

### Source-of-Truth Classifications (per `p2p-design-gate`)

Every entity introduced below carries an explicit SoT classification. These are the working assumptions to be confirmed at Checkpoint B's p2p-design-gate pass; if the gate revises them, the schemas change before Sprint 2 implementation begins.

**DHT Capacity Check (preliminary — must be confirmed at Checkpoint B):**
- Lamad DNA: ~73/~100 entry types (some headroom)
- Mishpat DNA: ~11/~100 entry types (substantial headroom — likely where Qahal lives, since governance pillar)
- Imagodei: 28/~100 (Qahal probably does not live here despite touching member identity)
- **Working assumption:** Qahal + QahalRubric + CommonsElohimGenesis land in **mishpat DNA** (governance pillar), not lamad or imagodei. Checkpoint B confirms.

### Schema deliverables — `elohim/sdk/schemas/v1/`

**DHT entries (notarized — Category A or A2):**
- `entries/qahal.schema.json` — Layer 0 identity anchor. **SoT: A (Notarized).** New DHT entry type; permanent; immutable post-creation; community must witness Qahal genesis. Lives in mishpat DNA (pending capacity confirmation). Storage projection has `dht_anchor_hash NOT NULL`.
- `entries/qahal-rubric.schema.json` — versioned rubric. **SoT: A2 (Derived).** Anchored via a `QahalToRubric` link from the parent Qahal entry; no standalone meaning without its Qahal. Link tag carries version number; rubric body stored as standalone entry payload linked from the Qahal anchor. Storage projection has `dht_anchor_hash` pointing to the parent Qahal's ActionHash.
- `entries/commons-elohim-genesis.schema.json` — commons-elohim instantiation record. **SoT: A2 (Derived).** Anchored via `QahalToCommonsElohim` link from the parent Qahal entry; created atomically with `create_qahal`; carries configuration hash + agent pubkey for the autonomous shadow agent. Same anchor pattern as rubric.

**Views (computed projections — Category C):**
- `views/qahal-view.schema.json` — Qahal homepage composed read. **SoT: C (Operational).** Computed from the Qahal entry (A) + latest rubric (A2) + standing inputs (B2) + reach gating (C). Reconstructed from source DHT entries on demand; SQLite projection caches for fast query but is not source of truth.
- `views/qahal-rubric-view.schema.json` — rubric viewer projection. **SoT: C (Operational).** Computed from QahalRubric entry chain (A2 with version history). Cache invalidated on rubric update.
- `views/standing-view.schema.json` — computed standing surface for a (human, qahal) tuple. **SoT: C (Operational).** Computed at request time from: lamad attestations (B2 — private quiz responses + public attestations), affinity signals (operational — computed from authoring/presence history), FeedbackSignal debits (A — notarized feedback events), current rubric version (A2). Sliding-window cache; invalidation on attestation/feedback events. Reconstruction strategy: re-walk attestation chain through current rubric.
- `views/member-ring-view.schema.json` — imagodei-lensed member list view. **SoT: C (Operational).** Composed from imagodei profiles (existing A entries) + per-viewer standing context (C) + Qahal-context lensing rules from rubric (A2). Reconstructed from source on demand.
- `views/commons-stream-view.schema.json` — feed view. **SoT: C (Operational).** Composed from reach-gated content EPRs (A entries) + reach computation (C) + viewer's standing in this Qahal (C). Materialized view; invalidated on relevant content/reach events.
- `views/qahal-context-elohim-view.schema.json` — commons-elohim contextual output. **SoT: C (Operational).** Live output of the commons-elohim runtime; ephemeral; no notarization (runtime sense-and-respond per `project_elohim_agent_sense_respond_architecture`). Reconstruction: commons-elohim re-observes from current Qahal state.

**Rubric template catalog — `elohim/sdk/schemas/v1/rubric-templates/`:**
- `household.template.json` — household stewardship rubric
- `faith-community.template.json` — religious community rubric
- `open-source-project.template.json` — OSS contributor rubric
- `regen-project.template.json` — regenerative agriculture project rubric
- `local-chapter.template.json` — federated local chapter of a parent Qahal

Each template specifies:
- Bloom-tier capability mappings (Remember/Understand/Apply/Analyze/Evaluate/Create → specific capabilities)
- Attestation requirements at each tier (which lamad quizzes count, weight, recency requirements)
- Affinity signal weights
- FeedbackSignal debit rules
- Friction-gradient thresholds (when does adding members start to flatten standing growth?)
- Commons-elohim configuration defaults

**Standing computation spec — in the design doc:**
- Pseudocode for `compute_standing(human, qahal, [as_of_time]) → StandingView`
- Algorithm: walk attestation chain → apply rubric weights → subtract feedback debits → check affinity threshold → return Bloom-tier capability surface
- Caching model + invalidation triggers
- Cross-zome call shape (standing as view, computed in elohim-storage or via doorway projection)

**Doorway route manifest — in `elohim/sdk/domains/qahal/manifest.json` (new pillar manifest):**
- `GET /qahal/{qahal_id}` → QahalView
- `GET /qahal/{qahal_id}/rubric` → QahalRubricView
- `GET /qahal/{qahal_id}/standing/me` → StandingView (callee-scoped)
- `GET /qahal/{qahal_id}/members` → MemberRingView
- `GET /qahal/{qahal_id}/stream` → CommonsStreamView (reach-gated)
- `GET /qahal/{qahal_id}/context-elohim` → QahalContextElohimView
- `POST /qahal` → create-qahal (steward of parent qahal authority required, or genesis bootstrap)
- `POST /qahal/{qahal_id}/rubric` → update-rubric (steward standing required)

**Contract test scaffold — `elohim/elohim-storage/tests/schema_contract.rs`:**
(Covers all SoT-classified entities declared above. No new entities introduced here — this is the validation harness for the classifications above.)
- One contract test per declared entity: Rust struct ↔ JSON shape
- Codegen freshness check (TS interfaces match the canonical wire definitions)

**Friction-gradient enforcement specification — in design doc §5:**
- Decision: BOTH soft + hard enforcement (per Sprint 1 brainstorm checkpoint)
- Soft: standing-curve flattening as Qahal size grows; commons-share auto-scales to absorb residual
- Hard: protocol refuses Agreement clauses giving any one receiver >X% beyond Y total members; rubric updates that centralize authority require council validation
- Recursive: same applies to councils — no single council can hold authority beyond threshold; sibling councils auto-convene

**Exit criteria:**
- All SoT-classified entities declared above exist as JSON wire definitions and pass JSON validation
- The Category C projections (computed views) match Sprint 1 mock fixtures (no shape drift)
- Rubric templates exist for at least household + faith-community + open-source archetypes
- Doorway route manifest declares all routes the Sprint 1 stories consume
- Standing computation pseudocode is reviewed and approved
- Contract test scaffold for the SoT-classified entities is in place (tests fail because Rust impl does not exist yet — that's correct)
- Operator reviews the spec and confirms it captures the vision

**Per-sprint plan landing:** `genesis/docs/plans/2026-06-02-qahal-substrate-spine-plan.md` (task-level, written after Checkpoint B)

**Duration:** 1 week. Schemas only — no Rust impl in this sprint.

---

## Brainstorming Checkpoint C — Before Sprint 3

**Invoke skill:** `superpowers:brainstorming`

**Inputs:** Sprint 2 schemas. The standing computation pseudocode. The Sprint 0 vision spec. Existing elohim-storage codebase patterns.

**Question to brainstorm:**
How do we wire the substrate without breaking what's already there? Which existing services do we extend vs which new ones do we author? What's the integration path with the lamad attestation pipeline (already partially built)? How does the commons-elohim agent get instantiated and run — does it live in elohim-storage, doorway, or a new elohim-agent process?

**Decisions to land:**
- Service ownership: which Rust service owns Qahal CRUD, which owns standing computation, which owns commons-elohim runtime
- Lamad attestation integration: how a quiz EPR becomes a standing input (event flow)
- Commons-elohim runtime: in-process actor in elohim-storage? Separate sidecar process? `elohim-agent` skill applied?
- Reach integration: how reach-gating composes with standing-gating at the doorway layer
- Storage projection: which views are computed on-read vs cached in elohim-storage Postgres
- Test strategy: sweettest for DHT entry validation, integration test for doorway routes, vitest for Angular adapters

**Outputs of checkpoint C:**
- `genesis/docs/superpowers/specs/2026-06-09-qahal-substrate-wire-up-design.md`
- Updated `elohim/sdk/domains/qahal/manifest.json` (with service binding info)

---

## Sprint 3: Substrate Wire-Up

**Goal:** Implement the substrate spine — Qahal + QahalRubric entries in DHT, standing computation as a view, doorway routes, commons-elohim stub, lamad attestation → standing pipeline.

**Agent team:**
- `rust-architect` — leads zome work + elohim-storage service + doorway routes
- `content-pipeline` — wires the lamad attestation → standing input event flow (this connects to existing lamad work, not greenfield)
- `quality-deep` — writes the integration tests (Sweettest for DHT, integration tests for doorway, contract tests for schemas)
- `code-reviewer` — reviews each PR before merge

**Files & deliverables:**

**DHT entries — `elohim/holochain/dna/elohim/zomes/integrity/`:**
- `qahal/src/lib.rs` (new integrity zome OR extension to existing one; Checkpoint C decides)
- `qahal/src/qahal_entry.rs` — Qahal struct + validation
- `qahal/src/qahal_rubric_entry.rs` — QahalRubric struct + validation
- `qahal/src/commons_elohim_genesis_entry.rs` — CommonsElohimGenesis struct + validation
- `qahal/src/link_types.rs` — link enum (RubricUpdates, QahalToCommonsElohim, AgentToQahalMembership, etc.)

**Coordinator zome — `elohim/holochain/dna/elohim/zomes/coordinator/`:**
- `qahal/src/lib.rs`
- `qahal/src/create_qahal.rs` → `create_qahal(input) → ActionHash`
- `qahal/src/create_rubric.rs` → `create_rubric(input) → ActionHash`
- `qahal/src/update_rubric.rs` → versioned update chain
- `qahal/src/get_qahal.rs` + `get_all_qahals_for_agent.rs`
- `qahal/src/get_rubric.rs` (latest version resolution via update chain)

**elohim-storage Rust service — `elohim/elohim-storage/src/services/qahal/`:**
- `qahal_service.rs` — handles HTTP routes, calls into DHT
- `standing_computation.rs` — implements `compute_standing(human, qahal)` algorithm
- `commons_elohim_runtime.rs` — sense-and-respond stub (per `project_elohim_agent_sense_respond_architecture`, gates in Rust, .ts sense-and-respond only)
- `qahal_views.rs` — composes views from DHT + storage projections + standing computation

**Doorway routes — manifest-driven via `elohim/sdk/domains/qahal/manifest.json`:**
- Routes are declared in the manifest; doorway resolves them at startup
- Per `project_doorway_manifest_driven_routes`: no direct HTTP code for routes that can be declarative

**Lamad attestation → standing pipeline:**
- Extend lamad pillar's existing `Recognition` callback handling to emit attestation EPRs
- Attestation EPRs carry: (human, qahal, rubric_version, bloom_tier, evidence_links)
- Standing computation walks recent attestations + rubric to derive capability surface

**Commons-elohim stub:**
- Instantiated at Qahal genesis (atomic with `create_qahal` coordinator function)
- Runtime: subscribes to Qahal events; emits `commons_elohim_observed` signals
- MVP behavior: tracks reach + standing flow, surfaces "this is what's happening" view; no arbitration yet (council convening is post-MVP)

**Tests:**
- Sweettest in `elohim/holochain/tests/sweettest/qahal/` — DHT entry validation + multi-agent consistency
- Integration tests in `elohim/elohim-storage/tests/qahal_integration_test.rs` — doorway routes + standing computation
- Schema contract tests pass

**Exit criteria:**
- A user can call `POST /qahal` and a Qahal entry lands in DHT
- A steward can create + update a QahalRubric (latest version resolves correctly)
- `GET /qahal/{id}/standing/me` returns a `StandingView` with a capability surface computed from real attestations
- A lamad quiz EPR results in an attestation that affects standing on the next computation
- Commons-elohim is instantiated at Qahal genesis and emits at least one `commons_elohim_observed` signal during the test scenario
- All Sweettest scenarios pass with multi-agent consistency (per `feedback_sweettest_cross_agent_consistency`)
- All schema contract tests pass

**Per-sprint plan landing:** `genesis/docs/plans/2026-06-10-qahal-substrate-wire-up-plan.md`

**Duration:** 1-2 weeks (substrate work is bounded by the schema-first contract).

---

## Brainstorming Checkpoint D — Before Sprint 4

**Invoke skill:** `superpowers:brainstorming`

**Inputs:** Sprint 1 Library B pattern stories. Sprint 3 substrate running with real DHT entries. Existing elohim-app pillar conventions (`@app/qahal` barrel etc.).

**Question to brainstorm:**
How do we wire the Library B pattern stories into elohim-app without breaking existing pillar boundaries? What new services does the `qahal` pillar need? Are there pillar boundary violations to fix as part of this work (per `project_pillar_boundary_violations_backlog`)?

**Decisions to land:**
- `@app/qahal` pillar barrel exports
- Storage-client SDK extensions (new types from schema codegen)
- Angular adapter shape (computed/derived fields per the existing rule: no wire transformation)
- Reactive state strategy (BehaviorSubject? Signals? Match existing pillar conventions)
- App-manifest-driven tooling tray (Sprint 1 wired the panels; now how does manifest declare them?)
- Routing — does Qahal homepage get its own route, or is it embedded in an existing page?

**Outputs of checkpoint D:**
- `genesis/docs/superpowers/specs/2026-06-17-qahal-frontend-wire-up-design.md`

---

## Sprint 4: Frontend Wire-Up

**Goal:** Take the Library B pattern stories and wire them to real backend data via storage-client SDK + Angular pillar services. Real Qahals render in real elohim-app.

**Agent team:**
- `angular-architect` — leads pillar service work + adapter layer + reactive state
- `graphos-designer` — adapts Library B stories from mock data to real data
- `component-architect` — ensures Library A elements still pass precondition gates with real data shapes
- `code-reviewer` — reviews PRs

**Files & deliverables:**

**Storage client SDK — auto-generated from Sprint 2 schemas:**
- `elohim/sdk/storage-client-ts/src/generated/qahal-view.ts`
- `elohim/sdk/storage-client-ts/src/generated/qahal-rubric-view.ts`
- `elohim/sdk/storage-client-ts/src/generated/standing-view.ts`
- (etc. — all view types from Sprint 2 schemas)

**Angular pillar — `app/elohim-app/src/app/qahal/`:**
- `qahal.module.ts` (barrel exports)
- `services/qahal.service.ts` — CRUD + view fetching
- `services/standing.service.ts` — reactive standing for current user
- `services/qahal-context-elohim.service.ts` — subscribes to commons-elohim signals
- `adapters/qahal.adapter.ts` — adds computed/derived fields only (no wire transformation per the rule)
- `pages/qahal-homepage.component.ts` — assembles the Library B pattern story with real data wiring
- `pages/qahal-homepage.component.html` — uses Library A elohim-elements directly
- `pages/qahal-homepage.component.scss` — minimal; binds to graphos brand tokens

**App-manifest tooling tray:**
- `elohim/sdk/domains/qahal/manifest.json` extended with `tooling_tray_panels` array
- Each panel declares: name, capability requirement, render-element, default-visibility
- Angular reads manifest at startup; renders panels dynamically based on user's standing in current Qahal

**Routing:**
- `/qahal/:qahalId` → QahalHomepageComponent
- Default route: redirects to user's primary Qahal (or onboarding if none)

**Tests:**
- Vitest unit tests for services
- Vitest unit tests for adapters
- Cypress E2E (Cucumber BDD) scenarios in `app/elohim-app/cypress/e2e/qahal/`:
  - `qahal-homepage.feature` — view a Qahal homepage
  - `qahal-standing.feature` — take a quiz, see capability surface expand
  - `qahal-context-elohim.feature` — commons-elohim contextual view updates

**Story-first discipline:** A2o scenarios in `genesis/a2o/features/qahal/` MUST exist before frontend tasks ship. Per `feedback_a2o_is_human_experience_not_dev_bugs`, these are human-experience scenarios — what does a household steward see and do?

**Exit criteria:**
- A real user (Matthew, in dev) can navigate to `/qahal/:qahalId` and see the convergent homepage rendered against real DHT data
- The standing badge reflects real computed standing
- Taking a lamad quiz in the app updates standing on the next page load
- The commons-elohim contextual view in the right nav updates as state changes
- Pillar boundary violations: no new violations introduced (per ESLint warn-level)
- All Cypress scenarios pass
- Storybook stories still pass with mock data (Library B continues to be the design contract)

**Per-sprint plan landing:** `genesis/docs/plans/2026-06-18-qahal-frontend-wire-up-plan.md`

**Duration:** 1-2 weeks.

---

## Brainstorming Checkpoint E — Before Sprint 5

**Invoke skill:** `superpowers:brainstorming`

**Inputs:** End-to-end Qahal homepage working with real data. The Bay Area Dawn Runners mock from `project_qahal_collective_view_slide45_reference`. Existing genesis household stories.

**Question to brainstorm:**
What is the *one specific Qahal* we seed for MVP demo? What is its rubric? Who are its members (real humans from genesis stories — Matthew's household, Bay Area Dawn Runners, others)? What stream content does it have? What standing distribution do we want to demonstrate (a new contributor, an engaged member, a steward, all visible in the member ring)?

**Decisions to land:**
- The worked example Qahal — household, faith community, or open-source project (probably household per existing genesis stories)
- Seeded members (specific imagodei profiles)
- Seeded rubric (one of the canonical templates, customized for this Qahal)
- Seeded attestations (specific humans at specific Bloom tiers)
- Seeded stream content (recent posts, recent flows, recent feedback signals)
- A2o scenarios that prove the MVP behavior end-to-end

**Outputs of checkpoint E:**
- `genesis/docs/superpowers/specs/2026-06-25-qahal-mvp-seed-content-design.md`

---

## Sprint 5: Genesis Content + Canonical Templates + A2O Scenarios

**Goal:** Seed a fully-worked example Qahal that demonstrates MVP behavior end-to-end, with a2o scenarios that prove it.

**Agent team:**
- `content-pipeline` — drives the seeding (uses elohim-import skill)
- `storyteller` (Opus) — authors the canonical narrative for the worked example; ensures it grounds in real households from genesis stories
- `quality-architect` (Opus) — authors a2o scenarios from MVP behavior

**Files & deliverables:**

**Canonical rubric templates (seeded into DHT):**
- `household.template.json` (already in catalog from Sprint 2; now seeded via seeder)
- `faith-community.template.json`
- `open-source-project.template.json`
- Minimum 3 templates, demonstrating that Qahal supports meaningfully different archetypes

**Worked-example Qahal — household:**
- Qahal identity: e.g., "Bay Area Dawn Runners + Dowell Household" (combining two of the existing references)
- Members: 4-6 humans from genesis stories with realistic imagodei profiles
- Standing distribution: 1 founder-steward, 2 engaged contributors, 1 new visitor (visible capability gradient)
- Stream content: 5-10 realistic posts (planning a run, sharing a recipe, asking for tool loan)
- Attestations: each member has a meaningful attestation history that explains their standing
- Commons-elohim: configured + running; emitting realistic contextual observations

**A2o scenarios — `genesis/a2o/features/qahal/`:**
- `qahal-homepage-first-visit.feature` — a household steward opens the homepage and recognizes their collective
- `qahal-standing-growth.feature` — a new contributor takes a quiz and sees their capability expand
- `qahal-commons-elohim-context.feature` — the commons-elohim's contextual view changes as the steward navigates
- `qahal-imagodei-lens.feature` — the member ring shows different facets of a person based on viewer's Qahal context
- `qahal-rubric-author.feature` — a steward updates the rubric and sees the change reflected in standing computations

**Seeder integration:**
- Extend `app/elohim-library/projects/elohim-service/src/cli/import.ts` to handle Qahal + Rubric seeding
- New skill workflow in `.claude/skills/elohim-import/` for Qahal content authoring
- Update `deployments.json` with the worked-example Qahal as a seed target (per `project_deployments_json_seed_or_skip_truth`)

**Exit criteria:**
- All canonical rubric templates seed without errors
- The worked-example Qahal seeds with all members, attestations, stream content, commons-elohim configuration
- All a2o scenarios pass (per story-first discipline)
- Matthew can open the dev app and demo the MVP to another human
- The demo lands: a non-technical observer recognizes the convergent shape AND understands what's different about it (the standing gradient, the commons-elohim view, the reach + standing axes)

**Per-sprint plan landing:** `genesis/docs/plans/2026-06-26-qahal-mvp-seed-content-plan.md`

**Duration:** 1 week.

---

## MVP Demo Checkpoint F

**Inputs:** End-to-end Qahal MVP running with seeded worked example.

**Gate:** Demo to a non-technical observer (Matthew's spouse, or a member of one of the genesis-story households). Two questions:

1. **Recognition:** Does it feel familiar? Does the shape (left-nav, tray, stream, member ring, right-nav) read as a known kind of app?
2. **Distinction:** Can the observer articulate, after 10 minutes of using it, what's *different* about this? Specifically:
   - Standing being earned, not assigned
   - The commons-elohim having a voice in the right nav
   - Reach being separate from standing
   - The Qahal's rubric being something the stewards themselves authored

**Pass criteria:**
- Both recognition AND distinction land
- The observer can describe (in their own words) why this is different from Discord/Hylo/Matrix
- The observer expresses interest in being part of (or starting) a Qahal of their own

**If pass:** MVP is real. Move to post-MVP horizon (federation, shefa cascade, merge/split, council arbitration, Hylo-style holonic graph, additional power-user panels).

**If partial pass:** Identify which axis failed (recognition vs distinction) and add a remediation sprint before declaring MVP done.

---

## Post-MVP Horizon (Not Detailed in This Roadmap)

These are explicit out-of-scope for MVP but the architecture supports them. Each gets its own roadmap when its time comes:

- **Sprint 6: Shefa value cascade.** REA economic events, ripple/commons split, Agreement clauses with BeneficiaryRef = NdoComponent, automatic cascade to commons-elohim. Integrates with the shefa pillar's existing REA work.
- **Sprint 7: Holonic federation (Hylo-shape).** Qahal-to-Qahal coupling, parent/child relationships, federated visibility rules, local-chapter bootstrapping.
- **Sprint 8: Council convening + arbitration.** Multiple commons-elohims convene for cross-Qahal decisions. Layered council resolutions interpretable at conversation level.
- **Sprint 9: Merge / split / succession.** Ephemeral EPR merge contracts. Standing-mapping across rubric versions. Lineage primitives (per `project_lineage_rna_upgrade_path`).
- **Sprint 10: Power-user panel suite.** REA dashboard, attestation catalogue editor, feedback queue inbox, graph-discovery suggestions, rubric editor IDE.
- **Sprint 11: Patreon/Open-Collective integration.** External monetary contribution to a Qahal via the doorway. Connects shefa cascade to web2 payment surfaces.
- **Sprint 12: Federation with external protocols.** AT Protocol / ActivityPub projection of Qahal commons surface (per `project_doorway_is_federation_surface_atproto`).

---

## Risks & Dependencies

### Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Sprint 1 UX doesn't converge on a satisfying shape | Medium | High | Use real reference apps as guardrails; brainstorm with mock data before committing to schema |
| Standing computation algorithm has performance issues at scale | Medium | Medium | Cache aggressively; sliding-window invalidation; doorway-layer projection |
| Commons-elohim runtime is harder than stub suggests | Medium | Medium | MVP scope is sense-and-respond only; arbitration is post-MVP |
| Lamad attestation pipeline needs more work than existing pillar provides | Medium | High | Audit lamad pipeline at Checkpoint B; widen Sprint 3 scope if needed |
| Pillar boundary violations cascade as we add `@app/qahal` | Low | Medium | Lint as warn-level; clean up violations in a dedicated post-MVP sprint |
| Friction-gradient enforcement is hard to implement at hard-floor tier | High | Medium | Soft enforcement (standing-curve flattening) is MVP; hard enforcement deferred to post-MVP if needed |
| Schema-first codegen creates drift between Rust and TS | Low | High | Schema contract tests catch drift; pre-push hook validates codegen freshness |

### Dependencies

- **EPR substrate**: We rely on `project_first_class_graph_pattern` being stable. If EPR substrate is shifting, pause this roadmap until it settles.
- **Reach engine**: Sprint 4 assumes reach is operational for commons-stream gating. If reach is incomplete, scope it tighter (no stream gating for MVP demo).
- **Lamad attestation pipeline**: Sprint 3 extends it. If lamad pillar work is in flight elsewhere, coordinate.
- **Doorway manifest-driven routes**: This roadmap assumes manifest-driven routing is operational (per `project_doorway_manifest_driven_routes`). Confirm at Checkpoint C.
- **Existing pillar conventions**: `@app/qahal` follows `@app/lamad` / `@app/imagodei` patterns. If those are inconsistent, address first.

---

## Open Questions Carried Forward

Questions that this roadmap intentionally defers. Each must be answered before its sprint can complete.

1. **Friction-gradient enforcement mix** — Soft + hard. Mix to be specified in Sprint 2. (Roadmap defers; spec lands at Checkpoint B.)
2. **Commons-elohim runtime location** — In-process actor (elohim-storage) vs sidecar process. Decided at Checkpoint C.
3. **Rubric versioning model** — Chain via `RubricUpdates` link vs monotonic version field. Decided at Checkpoint B.
4. **Standing caching strategy** — Sliding window vs full re-compute on attestation events. Decided at Checkpoint C.
5. **App-manifest tooling tray composability** — How third-party tools register panels. Decided at Checkpoint D.
6. **Imagodei lens recursion in the member-ring projection** (already classified Category C above) — How "view person X through this Qahal's context" is implemented. Decided at Checkpoint B (wire shape) and Checkpoint C (computation).

---

## Brainstorming Checkpoint Cadence

The checkpoints exist because the substrate is novel and the wrong call early causes expensive rework. Each checkpoint:

1. Invokes `superpowers:brainstorming` to explore the question
2. Invokes `p2p-design-gate` for any sprint that touches data entities (mandatory per CLAUDE.md)
3. Produces a spec document at `genesis/docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`
4. Operator (Matthew) signs off before the next sprint kicks off

| Checkpoint | Skill | Output spec | Gates sprint |
|---|---|---|---|
| A | brainstorming | `2026-05-23-qahal-homepage-ux-spec.md` | Sprint 1 |
| B | brainstorming + p2p-design-gate | `2026-06-01-qahal-substrate-spine-design.md` | Sprint 2 |
| C | brainstorming | `2026-06-09-qahal-substrate-wire-up-design.md` | Sprint 3 |
| D | brainstorming | `2026-06-17-qahal-frontend-wire-up-design.md` | Sprint 4 |
| E | brainstorming | `2026-06-25-qahal-mvp-seed-content-design.md` | Sprint 5 |
| F | demo gate | (demo session, not spec) | MVP signoff |

---

## TDD + Frequent Commits Discipline

Within each sprint, the per-sprint plan applies writing-plans discipline:

- Schema-first IoC (write schema before Rust/TS)
- TDD where the test surface is meaningful (Sweettest for DHT, Cypress BDD for UI, vitest for adapters)
- Frequent commits (one task = one commit; per writing-plans skill)
- Story-first for any frontend work (a2o scenarios exist before implementation per `feedback_a2o_is_human_experience_not_dev_bugs`)

The per-sprint plans will spell out the bite-sized 2-5 minute steps. This roadmap intentionally stays at sprint granularity.

---

## How to Resume This Roadmap

If a future Claude session needs to pick this up cold:

1. Read this roadmap document end-to-end
2. Read the gospel-tier vision spec at `genesis/docs/superpowers/specs/2026-05-22-qahal-architecture-vision.md` (Sprint 0 output)
3. Read the four memory entries listed at the top of this document
4. Check the sprint status table: which checkpoints have been passed, which specs exist, which plans are in flight
5. Resume at the next checkpoint or sprint
6. If the picture has shifted significantly since this roadmap was written, invoke `superpowers:brainstorming` to update before continuing

---

**Roadmap status: drafted 2026-05-21. Sprint 0 not yet started. Awaiting operator (Matthew) review before kicking off Sprint 0.**
