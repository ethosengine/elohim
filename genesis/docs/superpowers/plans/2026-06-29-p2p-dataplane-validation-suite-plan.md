---
id: plan-p2p-dataplane-validation-suite
title: P2P Dataplane Validation Suite — per-concern cucumber proofs, interpretable in Jenkins
status: Draft
class: protocol-canonical
domain: D5
sprint: dataplane-validation (verify-track + new concern suites)
requires_env: [household-nodes]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
cites:
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-proofs-plan.md
  - resilience-dimensions-proof-suite | 2026-06-12-resilience-dimensions-proof-suite-design | sha256:a89f58ec4906e152 | path: genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md
  - resilience-facings-select-fold-aggregate-design | Resilience Facings | sha256:738c9220d105e9e4 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - p2p-dataplane-sync-engine-design-arc | History: The P2P dataplane + sync-engine design arc (March 2026) | sha256:d509030b5f00acd0 | path: genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md
  - elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
---

# P2P Dataplane Validation Suite — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use `- [ ]`. Authoring `.feature` scenarios is Opus work (vision-bearing); step-def glue is Sonnet/Haiku. TDD-shaped: a concern's scenario lands **red against live peers**, then the implementation/proof turns it green.

**Goal:** Stop *guessing* whether the p2p dataplane works. Build a suite of cucumber/a2o scenarios **organized by dataplane concern**, run in Jenkins against live peers, each asserting on the **real exposed validation surfaces** (`/health`, `/sync/v1/*`, `/blob`, `/p2p/status`, `/api/v1/*`, `/metrics`, Loki). The pipeline's results become an **explorable, per-concern pass/fail surface** that both a human and the **agentic-developer loop** read to *know* — not infer — what's working. Granular and incremental: each concern is a nameable proof a fix can turn green, not one aggregated card.

**Architecture:** Three composed layers, one report. (1) **Deterministic floor** — the existing/P-PROOFS Rust integration proofs (`tests/*`) + `household_resilience.rs` matrix. (2) **Live cross-peer layer** — NEW per-concern `.feature` files under `genesis/a2o/features/dataplane/`, each tagged `@dataplane @concern:<name>`, driven by a shared surface-query step library that asserts on the exposed endpoints/metrics/logs against deployed peers (alpha + shem). (3) **Interpretability layer** — the a2o sprint-report aggregator buckets results **by `@concern:` tag** into a per-concern status matrix (machine-readable JSON + human MD), which the agentic-developer loop consumes as a measure surface. The resilience card is a *consumer* of this, not the suite itself.

**Tech Stack:** Gherkin/cucumber-js (`genesis/a2o`), TypeScript step defs + `PlaywrightDevice`/HTTP probes, the `build-sprint-report.ts` aggregator, Jenkins (edge + app pipelines), Rust (P-PROOFS integration tests, the peer-fallback fix), `.epr-meta` (the code compose-gate).

## Global Constraints

- **Cucumber is the vehicle.** Do NOT build a bespoke status element. Compose on `genesis/a2o` (features + steps + `build-sprint-report.ts`) and the existing sprint-report. (Per a2o/CLAUDE.md conventions: `@e2e @{domain}` tags, `Background: Given doorway "alpha"`, `@requires:<cap>` for substrate gating, `@browser-only` for Playwright.)
- **Surface-driven, never guess.** Every assertion reads a documented exposed surface (the per-concern surface table in Task list). No scenario asserts a behavior it cannot observe through a real endpoint/metric/log — if a concern has no surface, the suite's job is to add the surface FIRST (e.g. `/api/v1/sync/status`).
- **Concern taxonomy is the spine.** Every scenario carries exactly one `@concern:<name>` from the fixed vocabulary (below). The sprint-report buckets on it. A scenario without a `@concern:` tag is a defect.
- **Two consumers.** The per-concern result must be (a) human-explorable (sprint-report MD, one section per concern) AND (b) machine-readable (sprint-report JSON `byConcern{}` block) so the agentic-developer loop measures against it. Both come from one aggregator pass.
- **Red-first for gaps.** A concern with a known live failure (peer-fallback / blob-replication) lands its scenario RED against live peers first (CI shows the real gap), then the fix turns it green. Never author a green-by-omission scenario.
- **Compose, don't fork.** Extend P-PROOFS (deterministic proofs) + the resilience-dimensions a2o layer + the existing `genesis/a2o/features/resilience/*` — do not duplicate them; the new `dataplane/` tree is the *live cross-peer* layer that references them.
- **Env scope is per-scenario.** Doc-level `requires_env: household-nodes`; tag cross-tenant scenarios `@requires:shem` and cross-node-household ones `@requires:multi-node` (household M/J/J is itself multi-node — do NOT tag those `@requires:shem`). Both shem + household-nodes are currently AVAILABLE.
- **Commit-only; integrator pushes/merges.** No `git push` from implementation; CI is the gate.

## The concern taxonomy (the suites of like-concerns)

Fixed `@concern:` vocabulary (one feature file per concern under `genesis/a2o/features/dataplane/`), derived from the dataplane feature map. Each row = a CI-legible suite.

| `@concern:` | What it proves (live, cross-peer) | Primary exposed surfaces |
|---|---|---|
| `content-sync` | An Automerge content doc authored on peer A converges to peer B | `GET /sync/v1/elohim/docs` (`total`, `documents[].heads`), `/p2p/status.sync_documents` |
| `blob-replication` | A blob/EPR authored on A is byte-present on B (custody replication), NOT just metadata | `GET /blob/{hash}` on A=200 ⇒ on B=200; `/db/content/{id}.blobHash` non-null on B; `/api/v1/diagnostics/inventory-parity` |
| `epr-projection-fallback` | A doorway with a cold blob falls back to a server-capable peer (or emits EPR-head-aware syncing status) — never bare `App not found` | `GET /` / `GET /api/v1/epr/{id}/nav-context`; (NEW) `GET /api/v1/sync/status` / a syncing-status body |
| `peer-mesh` | The mesh is connected + caught up across peers | `GET /health.p2p` (`peerCount`, `caughtUp`, `divergentAnchor`), `/p2p/peers` |
| `blob-durability` | RS reconstruct from any K-of-N; K survivors after one household loss (deterministic floor, P-PROOFS) | P-PROOFS `tests/*`; `GET /blob/{hash}`, `/health.blobs` |
| `keyspace-coverage` | Cluster arc union covers `[0,1)` as arcs shrink (P-PROOFS) | `GET /api/v1/status/arc-policy`; P-PROOFS arc test |
| `reconcile-inventory` | A reconcile pass fires + emits kicks/placement-gaps under custody gap; inventory parity holds | `/p2p/status` (`reconcile*Total`), `/api/v1/diagnostics/inventory-parity` |
| `federation-deploy` | An EPR resolves on **all** federation doorways post-deploy (kills the per-host stageSpaBlob crutch) | per-doorway `GET /db/content/{id}.blobHash` + `GET /` 200 across alpha-A AND elohim.host(alpha-B) |

---

## Task 1: Concern-tag taxonomy + sprint-report `byConcern` aggregation (foundation, no live deps)

**Files:**
- Create: `genesis/a2o/features/dataplane/README.md` (the concern vocabulary + how to add a suite)
- Modify: `genesis/a2o/scripts/lib/aggregate.ts` (add the `byConcern` rollup)
- Modify: `genesis/a2o/scripts/build-sprint-report.ts` (emit per-concern MD section + JSON block)
- Test: `genesis/a2o/scripts/__tests__/aggregate-by-concern.spec.ts`

**Interfaces:**
- Consumes: the existing cucumber JSON → `aggregate.ts` summary shape.
- Produces: `summary.byConcern: Record<string, { passed, failed, pending, scenarios: {name, status, surface}[] }>` keyed by the `@concern:` tag; an MD section "## Dataplane validation by concern" with one subsection per concern + a status glyph (✅/❌/◌).

- [ ] **Step 1: Write the failing test** — feed `aggregate-by-concern.spec.ts` a synthetic cucumber JSON with 2 scenarios tagged `@concern:content-sync` (1 pass, 1 fail) + 1 `@concern:peer-mesh` (pass); assert `byConcern['content-sync'] == {passed:1, failed:1}` and `byConcern['peer-mesh'].passed == 1`.
- [ ] **Step 2: Run → FAIL** (`pnpm --filter @elohim/a2o test aggregate-by-concern` — no `byConcern` yet).
- [ ] **Step 3: Implement** the `byConcern` rollup in `aggregate.ts` (parse `@concern:` from each scenario's tags; bucket pass/fail/pending) + the MD/JSON emit in `build-sprint-report.ts`.
- [ ] **Step 4: Run → PASS.**
- [ ] **Step 5: Commit** `feat(a2o): per-concern dataplane validation rollup in the sprint-report`.

## Task 2: The surface-query step library (foundation)

A reusable step layer so every concern asserts the SAME way on the exposed surfaces (no per-feature HTTP reinvention).

**Files:**
- Create: `genesis/a2o/src/steps/dataplane.steps.ts`
- Create: `genesis/a2o/src/framework/dataplane/surfaces.ts` (typed probes for `/health`, `/sync/v1/*`, `/blob`, `/p2p/status`, `/api/v1/*`, `/metrics`)

**Interfaces:**
- Produces reusable steps, e.g.:
  - `Given peer "{name}" at "{HOST_ENV}"` (resolves alpha-A / elohim.host(alpha-B) / shem hosts)
  - `When I query {surface} on peer "{name}"` (surface ∈ the table; stores the parsed JSON in the world)
  - `Then peer "{name}" /health p2p.caughtUp is true` / `... peerCount >= {n}`
  - `Then /sync doc "{docId}" is present on peer "{name}"` (heads non-empty)
  - `Then blob "{hash}" is byte-present on peer "{name}"` (`GET /blob` 200)
  - `Then EPR "{id}" blobHash is non-null on peer "{name}"`
  - `Then resolving "{path}" on peer "{name}" does NOT return App-not-found` (the anti-hard-fail assertion)
  - `Then metric "{name}" on peer "{name}" {cmp} {value}`

- [ ] **Step 1:** Write `surfaces.ts` typed against the grounded surface fields (peerCount/caughtUp/divergentAnchor/projection.writer; sync total/heads; blob status; epr nav-context; arc-policy; inventory-parity; doorway M1–M5 + storage `p2p_*`/`reconcile_*`/`dedup_*`).
- [ ] **Step 2:** Write the step defs delegating to `surfaces.ts`; `requirePlaywright`-free (pure HTTP) so they run in API mode.
- [ ] **Step 3:** Smoke a trivial `@concern:peer-mesh` scenario locally against `E2E_DOORWAY_ALPHA` → green. Commit `feat(a2o): dataplane surface-query step library`.

## Task 3: `@concern:peer-mesh` + `@concern:content-sync` suites (the already-working baseline)

Author the two concerns that are LIVE today (lock them in as regression proofs; they should pass now).

**Files:**
- Create: `genesis/a2o/features/dataplane/peer-mesh.feature`, `content-sync.feature`

- [ ] **peer-mesh:** scenarios — mesh connected (`peerCount >= 2`, `caughtUp true`) on alpha-A AND elohim.host; `divergentAnchor` within tolerance. `@requires:multi-node`.
- [ ] **content-sync:** author a content node on peer A via `POST /db/content` (test-persona, non-bulk), then assert `node:{id}` is present (heads non-empty) on peer B's `/sync/v1/elohim/docs` within a deadline. `@requires:multi-node` (household); a `@requires:shem` variant for cross-tenant. Compose the producer/convergence facts from the content-sync-plane plan.
- [ ] Run against alpha; expect GREEN (these are live). Commit. These prove the suite mechanism works on known-good concerns before the red ones.

## Task 4: `@concern:blob-replication` + `@concern:epr-projection-fallback` — RED-FIRST (the live failure)

The concern that caught the real bug: elohim.host has the EPR head but `blobHash:null`; resolving returns `App not found`.

**Files:**
- Create: `genesis/a2o/features/dataplane/blob-replication.feature`, `epr-projection-fallback.feature`

- [ ] **blob-replication (RED now):** author/seed an EPR+blob on peer A; assert the blob is byte-present on peer B (`GET /blob/{hash}` 200 AND `/db/content/{id}.blobHash` non-null on B) within a convergence deadline. This FAILS today (elohim.host `blobHash:null`) → the suite makes the gap CI-visible.
- [ ] **epr-projection-fallback (RED now):** `Then resolving "/" on peer "elohim.host" does NOT return App-not-found` — instead returns the SPA (peer-fallback served) OR an EPR-head-aware syncing status (`{ eprHead, blob: { state: syncing|ready, bytes: x, of: y } }`). FAILS today.
- [ ] Run against alpha+elohim.host; CONFIRM RED (the proof has teeth). Commit `test(a2o): blob-replication + epr-projection-fallback dataplane concerns (red-first)`.
- [ ] **Capture the fix as its own work** (NOT this plan's scope to bloat): the storage peer-fallback + status surface + blob byte-replication is a separate implementation plan. Backlog `dataplane-peer-fallback-and-blob-replication.md` (domain D5) — the scenarios here are its acceptance gate.

## Task 5: `@concern:federation-deploy` — kill the per-host stageSpaBlob crutch

- [ ] Create `genesis/a2o/features/dataplane/federation-deploy.feature`: for the landing EPR, assert `GET /` returns 200 (not App-not-found) AND `blobHash` non-null on **every** federation doorway (alpha-A + elohim.host/alpha-B). FAILS today on elohim.host. This is the assertion that makes the per-host CI blob-upload shortcut un-shippable. `@requires:multi-node`.
- [ ] Commit.

## Task 6: `.epr-meta` code compose-gate — yell at the shortcut at edit time

Extend enforcement from design-docs (the existing p2p-design-gate) to the IMPLEMENTATION resolution path.

**Files:**
- Create/Modify: `elohim/elohim-storage/src/.epr-meta`

- [ ] Add a rule (per `elohim-epr-metafile` skill) scoped to the EPR/blob resolution path (`http.rs` app-resolution, `sharding.rs`, `epr/`): flag any edit that returns a hard `App not found` / `NotFound` for a **known-EPR-head, missing-blob** case **without** a peer-fallback (`race_fetch`) branch or an EPR-head-aware syncing-status return. Lightest signal that drives the directory toward the canonical "peers are the store; doorway projects with fallback" behavior — yells (PreToolUse) when an agent reintroduces the shortcut.
- [ ] Commit `feat(epr-meta): code-gate the peer-fallback invariant on the storage resolution path`.

## Task 7: Jenkins wiring + agentic-loop consumption

- [ ] **Jenkins:** add a `Dataplane Validation` stage that runs `cucumber-js --tags '@dataplane and not @wip'` against deployed peers (alpha + shem), emitting `sprint-report-dataplane.{json,md}`. Home: the edge `Jenkinsfile` (post-deploy, after `Verify Holochain Health`) — bash body in `scripts/ci/*.sh` (CPS size-limit gotcha), advisory `catchError→UNSTABLE` only until one green run, then gating. Predict dispatch with graph-walker; validate no over-build.
- [ ] **Agentic-loop consumption:** document (in `genesis/a2o/features/dataplane/README.md` + the agentic-developer measure section) that the loop reads `sprint-report-dataplane.json.byConcern` as a per-concern measure surface — a concern flipping ❌→✅ is forward progress; ❌ is a named candidate. (Mirrors the ci-findings ledger pattern; no new ledger — the per-concern block IS the surface.)
- [ ] Commit.

## Task 8: Compose the deterministic floor concerns (P-PROOFS + resilience-dimensions)

- [ ] `@concern:blob-durability` + `@concern:keyspace-coverage` + `@concern:reconcile-inventory`: these are largely the **deterministic** P-PROOFS integration tests + the resilience-dimensions a2o layer. Do NOT re-author — add thin `dataplane/` feature wrappers (or tag the existing `features/resilience/*` scenarios with the `@concern:` vocabulary) so they appear in the per-concern matrix alongside the live concerns. Verify P-PROOFS status via ci-investigator (CLAIMED vs landed) rather than rebuilding.
- [ ] Commit.

---

## Self-Review

**Spec coverage:** the user's ask = "cucumber suite in Jenkins validating every major dataplane feature, via the exposed surfaces, organized as interpretable suites-of-like-concerns for both human and agentic-loop, granular not aggregated." Covered: vehicle=cucumber/a2o (Constraints + Tasks 2-8); every major feature = the 8-concern taxonomy (from the grounded feature map); exposed surfaces = the per-concern surface table + Task 2 step library; interpretable organization = Task 1 `byConcern` sprint-report rollup; two consumers = Task 1 (human MD + machine JSON) + Task 7 (agentic-loop); granular/incremental = one feature per concern, red-first for gaps; not the aggregated card (the card becomes a consumer).

**Placeholder scan:** step signatures + surface fields are grounded (file:line came from the dataplane feature-map grounding); the few exact endpoint field names to confirm at implementation are flagged via the surface table (read the live `/health`,`/p2p/status` JSON before typing the assertion).

**Complementary work captured (not bloated in):** the actual peer-fallback + blob-replication + sync-status-endpoint IMPLEMENTATION is a separate plan (`dataplane-peer-fallback-and-blob-replication.md`, Task 4) — this plan builds its acceptance gate, not the fix. The resilience-card-as-consumer wiring and the `/api/v1/sync/status` endpoint are backlog/sibling-plan items.

**Type consistency:** `@concern:<name>` vocabulary is fixed (the taxonomy table) and used identically in Tasks 1, 3-5, 8; `byConcern` keys = those tags; `surfaces.ts` probes = the surface-table columns.
