# elohim-core ↔ reference-apps — Split Decision

> **Sprint 1 "Classify & Map" deliverable 2 of 3.** Decides the *shape* of the split; **execution is
> Sprint 5** (Workstream E). Read-only analysis; no code changed.
> Companions: [`boundary-map.md`](./2026-05-30-boundary-map.md) (deliverable 1),
> [`reliability-backlog.md`](./2026-05-30-reliability-backlog.md) (deliverable 3).
> Prior art: [`pillar-bundle-split-runbook.md`](../pillar-bundle-split-runbook.md),
> [`elohim-sdk.md`](../elohim-sdk.md). Read `feedback_understand_orchestrator_substrate_before_changes`.

## The question

Should the monorepo's CI (and possibly its layout) split into **elohim-core** — the substrate + the
Elohim SDK that *anyone* builds an EPR-app against — versus **reference-apps** — the protocol's own six
pillar apps that prove the contract end-to-end? So the core contract versions independently and third
parties (and we) consume a stable SDK rather than reaching into in-tree substrate source.

## Decision

**Yes — split into core and reference-apps, but execute it LAST (Sprint 5), and treat the
build/delivery harness as an explicit third concern, not as part of either side.**

The split is sound and the boundary is *mostly* clean at the substrate layer (Tier 3 has no
reference-app logic). It is **blocked today** by app↔app coupling and by a harness that hard-codes
delivery knowledge. The cure for both blockers is Sprints 3 (SDK bootstrap) and 4 (manifest-driven
delivery). **Splitting before those land would just relocate the cascade across a repo boundary, where
it is far more expensive to debug.** Hence: map (Sprint 1) → reliability (Sprint 2) → SDK contract
(Sprint 3) → manifest conformance (Sprint 4) → **split (Sprint 5)**.

### A framing correction that shapes the split: the harness is not a side

A mid-sprint operator note corrected a category error worth stating plainly: **CI/CD and the seeder are
not application layers.** They are *developer conveniences* for development, packaging, and delivery — and
the fact that so much delivery logic is hand-coded inside them is precisely the challenge this whole
cleanup exists to resolve. So the split is not a clean binary. There are **three** concerns:

| Concern | Members | Versioning role |
|---|---|---|
| **elohim-core** | DNAs, protocol schema, `@elohim/service`, storage-client-ts, epr-ts, `elohim/epr`, elohim-storage, doorway, steward, compute, bridges, elohim-elements, renderer plugins | publishes a **versioned SDK** + runs the substrate |
| **reference-apps** | elohim-app (Landing/CMS), lamad (LMS), shefa (mint), avodah (work), mishpat (governance), qahal (social), imagodei-portal | consume the **published SDK**; version independently |
| **delivery harness (shared tooling)** | orchestrator, per-project build-manifests, root Jenkinsfile/stageSpaBlobs, the seeder, deployments.json | **should get thinner** as the manifest matures; serves both sides; not a truth layer |

The harness is the crux of *why the split is hard*. Most split-blockers below are harness couplings.
The strategic point: **don't split the harness — dissolve the parts of it that hard-code delivery
knowledge into the SDK/manifest contract first (Sprints 3–4), then what remains is thin enough to
attribute cleanly to one repo or a shared tooling package.**

## Two axes, kept distinct

The boundary map classifies on a **role axis** (SDK / core / app-private / harness). The split is a
**repo/CI axis** (core repo / reference-apps repo / shared tooling). The bridge between them:

- **SDK + elohim-core (runtime)** → **core repo**.
- **app-private** → **reference-apps**.
- **delivery harness** → **shared tooling** (target: thin; ownership decided once thin).

So "is X SDK or core?" (deliverable 1) and "which repo does X live in?" (this doc) are different
questions with a deterministic mapping. A thing can be `elohim-core` by role and `core repo` by split
without being part of the *published* SDK surface.

## What moves where

### → core repo (publishes the SDK; versions independently)

- **Tier 1 contract:** all six DNAs (`elohim`/`lamad`, imagodei, mishpat, infrastructure, node-registry;
  hrea external), the protocol schema `elohim/sdk/schemas/v1/` including `app-manifest.schema.json` and
  the projection/pillar-projection *shapes*. `deny.toml`.
- **Tier 2 SDK:** `storage-client-ts`, `epr-ts`, `elohim-views`, `elohim/epr`, `crates/{elohim-sdk,
  doorway-client,elohim-storage-client}`, `@elohim/service`, `@elohim/rea-runtime`, `@elohim/identity`
  (once its PENDING exports detangle), the renderer plugins, **all `app/elohim-elements/*`**.
- **Tier 3 runtime:** `elohim-storage`, `steward/node`, `elohim-compute`, `bridges/*`.
- **Tier 4 gateway:** `doorway-service`.
- **The SDK-owned standard bootstrap** that Sprint 3 creates (`provideElohimApp` / `bootstrapEprApp`).

### → reference-apps (consume the published SDK)

- `app/elohim-app` (Landing/CMS), `app/lamad` (LMS), `app/imagodei-portal` (auth portal),
  `genesis/landing` (static CMS), and the future shefa/avodah/mishpat/qahal pillar apps.
- The per-pillar **manifest instances** `elohim/sdk/domains/<pillar>/manifest.json` (the *shapes* stay in
  core; the *instances* travel with their app).
- `lamad-ui` (app pitch-surface), `doorway-app` (operator dashboard — app of the doorway, not the gateway).
- `steward/device` (Tauri shell — one deployment shape; the steward *archetype* is core, this wrapper is app).

### → shared tooling (thin target; serves both)

- `genesis/orchestrator/*`, the 11 `build-manifest.json` files, the root `Jenkinsfile`, `genesis/seeder`,
  `deployments.json`, `graphos` (Storybook).
- **The aspiration:** after Sprint 4, `stageSpaBlobs` and `seed-projections.ts` read everything from the
  app-manifest, so the harness no longer encodes any app-specific delivery knowledge — at which point it
  can live in the core repo as generic tooling that reference-apps invoke, or as its own published CLI.

## Split blockers (ranked) — what must be cut first

These are why Sprint 5 cannot be Sprint 2. Each maps to a leak in deliverable 1's register.

| # | Blocker | Evidence | Cleared by |
|---|---|---|---|
| **B1** | **Duplicate `ELOHIM_CLIENT` token** — any reference app importing `@app/elohim` services drags in the LOCAL token, not the SDK one → `NullInjectorError` across a repo boundary | L1: `elohim-app/.../elohim-client.provider.ts:19` vs `@elohim/service`; alias only in `lamad/app.config.ts:148` | **Sprint 3** (collapse at source) |
| **B2** | **lamad imports 16 concrete classes from `@app/elohim`/`@app/shefa`/`@app/imagodei`** at its composition root | L2: `lamad/app.config.ts:22–59` | **Sprint 3** (`provideElohimApp` owns them) |
| **B3** | **`elohim-app/app.config.ts:25` imports `LAMAD_HOLOCHAIN_CLIENT` from `@app/lamad`** — reference-app A → reference-app B (circular post-split) | L9 | **Sprint 3** (move `LAMAD_*` tokens to a shared SDK package) |
| **B4** | **`elohim` pillar imports 40+ `@app/lamad` models + 15 sibling-pillar symbols** — 174 ESLint pillar-boundary violations (warn-level) | L8; [[project_pillar_boundary_violations_backlog]] | dedicated pillar-boundary sprint (can overlap 3–4) |
| **B5** | **`content.service.ts` duplicated 948-line in `@app/elohim` and `@app/lamad`** | L4 | **Sprint 3** |
| **B6** | **`genesis` build-manifest cross-pipeline step deps** — `seed-content.depends` names `elohim:build-site-image` + `elohim-edge:{steps}` by qualified pipeline:step; the walker resolves them in one namespace | `genesis/build-manifest.json:41–48` | **Sprint 5** (inter-repo artifact-handoff protocol) |
| **B7** | **`stageSpaBlobs` lives in the *app* Jenkinsfile but performs *core seeding*** | `Jenkinsfile:223–339` | **Sprint 4** (manifest-driven) then move |
| **B8** | **`genesis/orchestrator/manifests/elohim-app/` lives in the core-CI tree** but is owned by the app | `Jenkinsfile:956,975` | **Sprint 5** (app owns its manifests) |
| **B9** | **`sophia`, `elohim-compute`, `doorway-app` have no `jenkinsPath`** — not orchestrator-dispatchable; sophia rides a hardcoded `sophia.Jenkinsfile` shim + `[build:sophia]` alias | `sophia/build-manifest.json`, `elohim/elohim-compute/build-manifest.json`, `doorway/doorway-app/build-manifest.json`; `orchestrator/Jenkinsfile:1460` | **Sprint 5** (register or publish) |
| **B10** | **`deployments.json` is shared** between edge k8s rendering and seeder peer-selection | `elohim/holochain/build-manifest.json:76` | **Sprint 5** (one owner + cross-repo ref) |
| **B11** | **Versioning model unresolved** — a single `VERSION` file is watched by both the DNA and app pipelines | `app/elohim-app/build-manifest.json:22`, `elohim/holochain/dna/build-manifest.json:23` | this doc (below) + Sprint 5 |

**Reading of the table:** B1–B5 are *app-code* couplings cleared by **Sprint 3**. B6–B10 are *harness*
couplings — mostly cleared by **Sprint 4** thinning the harness, with the residue executed in **Sprint
5**. B4 (174 pillar violations) is the one genuinely large body of work that the split surfaces but does
not strictly require; it can proceed in parallel.

## Versioning model

**Recommendation: independent SemVer per repo, with the SDK as the published contract between them.**

- **core repo** publishes a versioned SDK bundle: `@elohim/service`, `@elohim/storage-client`,
  `@elohim/epr-ts`, `@elohim/rea-runtime`, the elements, the renderer plugins, and the
  `app-manifest.schema.json` — under **one coordinated SDK version** (the contract version a third party
  pins). The DHT/protocol-schema version (the notarized contract) is the *floor*; the SDK version tracks
  the consumable surface on top of it.
- **reference-apps** each pin a **compatible SDK version range** and version themselves independently.
- **the contract test** (Sprint 4's manifest-conformance + the existing `schema_contract.rs`/
  `export_bindings`) is the CI gate that proves an app's pinned SDK still satisfies the manifest.
- **build artifact handoff (B6):** core publishes image tags + the SDK package; the `genesis` seed/test
  pipeline consumes *artifacts* (a published SDK + tagged images), not in-tree step references. This is
  the new inter-repo protocol that B6 requires and that does not exist today.

Rejected alternative — **lockstep single version** (today's de-facto model): it is what couples
everything through one `VERSION` file and the genesis cross-pipeline deps. It makes "third party consumes
a stable SDK" impossible by construction, because every substrate change forces an app version bump. The
whole point of the split is to break this.

## What "done" looks like (Sprint 5 exit, restated for traceability)

- core + reference-apps build/version/deploy independently.
- a reference app builds against the **published** SDK (not in-tree source).
- orchestrator dispatch is clean + predicted (principle 7) with the cross-pipeline step deps replaced by
  an artifact-handoff contract.
- the harness encodes **zero** app-specific delivery knowledge (it reads the manifest).

## Open questions for the operator

1. **Repo topology:** one repo with two top-level workspaces + enforced boundary (lighter, keeps one CI
   substrate), or two physical repos (true independence, heavier coordination)? The blockers are identical
   either way; this is a coordination-cost call. *Recommendation: two workspaces first (prove the boundary
   with `deny.toml`-style enforcement), split repos only if independent release cadence demands it.*
2. **Are `elohim-compute` and `doorway-app` intentionally gate-only** (no `jenkinsPath`), or accidentally
   un-dispatchable? (B9) — needs an operator answer before Sprint 5.
3. **SDK distribution channel:** private registry vs git-tag vs tarball for the published SDK the
   reference-apps will pin? This determines the B6 artifact-handoff shape.
4. **Pillar-boundary debt (B4):** fund the 174-violation cleanup as its own sprint, or gate it (`warn` →
   `error`) incrementally as pillars graduate to EPR-apps?
