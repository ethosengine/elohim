# Pillar Bundle Split Runbook

> **Canon status:** Operational. Read [elohim-sdk](epr:elohim-sdk) first — this runbook assumes the SDK boundary as its working substrate.

---

## §1 — Why this runbook exists

The lamad split (cross-pillar import cleanup sprint, 2026-05-25) was the **first** of seven planned pillar splits. The pillar-EPR decomposition design (`genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` §0) names the rest: **shefa, qahal, avodah, imagodei, account, doorway**. Each will face the same shape of work — 200+ cross-pillar imports to retire, library boundaries to honor, codegen targets to register, pre-push gates to wire, test specs to audit.

The lamad split paid the cost of being first. It surfaced gotchas that no spec had named. It taught lessons that this runbook captures so the next pillar splitter inherits them rather than re-derives them. The runbook is exhaustive on purpose: a splitter who follows it end-to-end produces a clean split; a splitter who skips a step produces the same gotcha that already cost the lamad split a day.

The runbook captures **operations**, not philosophy. For the philosophy of why each library exists, read [elohim-sdk](epr:elohim-sdk).

---

## §2 — Prerequisites before you start

A pillar split is a substrate-aware operation. Verify these before scoping the work:

- [ ] **Substrate baseline current.** The protocol primitives the new bundle depends on are landed on `dev`. Lamad's prerequisite was Z.D Phase 1 (commits `b2380b899`, `7f66391b6`, `bf2efd191`) — the REA compute-commitment schemas needed to be on `dev` before `@elohim/rea-runtime` could absorb them. Identify your pillar's equivalent substrate baseline and verify it landed.
- [ ] **SDK boundary is current.** [elohim-sdk](epr:elohim-sdk) §3 must accurately describe what is in each library before you split. If the SDK has drifted (a new library was added; a symbol was moved between libraries), update the SDK doc first. The runbook assumes the SDK doc is canon-true.
- [ ] **Pre-existing test debt audited.** The lamad integration of `design/peer-oauth-portal` surfaced two test files orphaned by post-refactor drift: `epr-link.component.spec.ts` (orphaned by commit `10516614e`'s Lit-wrapper refactor) and `app.routes.spec.ts` (orphaned by commit `effc26e04`'s lamad-bundle-split asserting `routes.length === 14`). **Always audit before splitting.** Run grep for canary patterns:
  ```
  grep -rln "ngOnChanges\|loadChildren.*<your-pillar>" app/<consumer-bundle>/src --include="*.spec.ts"
  grep -rln "routes.length.*===" app/<consumer-bundle>/src --include="*.spec.ts"
  ```
  Surface any obsolete specs before they block the pre-push gate.
- [ ] **Pnpm lockfile clean.** `pnpm install --lockfile-only` produces no diff. (When the split scaffolds a new library, you will re-run lockfile reconciliation; verify it is clean now so any post-split diff is yours, not someone else's.)
- [ ] **Doorway clippy clean.** `cargo clippy -- -D warnings` on `doorway/doorway-service` returns 0. A clippy regression in `doorway/src/server/http.rs:1257` (introduced by `542ca8f0b`, fixed in `8c66fa3ca`) blocked the elohim-edge Docker image for build 1002 during the lamad split's adjacent work. Verify clippy is green; if not, fix-before-push.
- [ ] **Donor bundle currently healthy.** `pnpm --filter <donor-bundle> build` and `pnpm --filter <donor-bundle> test` both green BEFORE the split. The split is hard enough without inheriting unrelated failures.

---

## §3 — The Eight-Disposition Taxonomy

Every cross-pillar import in the new bundle's source classifies into one of these eight dispositions. The taxonomy was established by the cross-pillar import cleanup sprint's Wave 1 manifest (`genesis/docs/superpowers/notes/2026-05-25-cross-pillar-cleanup-dispositions.md`) and is reproduced here as canon.

| Code | Disposition | Examples | Target |
| --- | --- | --- | --- |
| **L** | Move to `@elohim/service` library | `ContentService`, `DataLoaderService`, `EprResolverService`, agent models, governance signals | `app/elohim-library/projects/elohim-service/src/` |
| **C** | Move to `elohim-core` Lit element library | Cross-pillar UI primitives (`reaction-bar`, `graduated-feedback`, navigation) | `app/elohim-elements/elohim-core/src/` |
| **S** | Move to `@elohim/storage-client` | Wire-format types, IPLD shapes, integrity-anchor TS contracts | `elohim/sdk/storage-client-ts/src/` |
| **I** | Move to `@elohim/identity` | Identity guards, session services, profile model, attestation | `app/elohim-library/projects/elohim-identity/src/` |
| **R** | Move to `@elohim/rea-runtime` | REA event services, Commitment action types, signal-bearing primitives | `app/elohim-library/projects/elohim-rea-runtime/src/` |
| **H** | Consume via doorway HTTP API | Cross-pillar data fetches that should ride the substrate | `doorway/doorway-service/src/routes/` |
| **E** | Consume via EPR resolution | Cross-pillar content references via `<elohim-epr-link>` | `<elohim-epr-link>` Lit element |
| **D** | Duplicate intentionally | Distinct copies the pillars need to evolve separately (rare, justify per case) | In-pillar |
| **X** | Delete | Unused after deeper audit | — |

**Recommended sprint shape for splits with more than 20 cross-pillar imports:**

- **Wave 1** (1 cartographer-tier agent, sequential, ~30–45 min, blocking) — Produce the disposition manifest. Every import gets a row; per-disposition totals match the audit baseline; library-structure decisions and per-symbol justifications surface to the operator for resolution.
- **Wave 2** (5–7 agents, parallel, ~2–4 hours each, file-disjoint) — Execute the migrations. One agent per disposition slice. Slice 2.7 is documentation-only and runs in parallel with the code slices. Each slice commits its own milestone.
- **Wave 3** (1 agent, sequential after all of Wave 2, ~2 hours) — Cutover. Remove transitional path aliases. Verify bundle independence. Update SDK doc and this runbook with anything the split discovered.

For splits with fewer than 20 cross-pillar imports, the runbook still applies, but Wave 1 and Wave 2 can collapse into a single linear execution by one agent.

---

## §4 — The Bundle-Creation Checklist

When a new pillar bundle scaffolds out of `app/elohim-app/`, **every one of these items is required**. The checklist is exhaustive because the lamad split paid for each missing item with a pre-push gate failure or an integration regression.

### §4.1 — Workspace and package scaffolding

- [ ] **`pnpm-workspace.yaml`** — add the new bundle's path under `packages:`.
- [ ] **`package.json` minimum scripts.** Every new bundle ships with `build`, `start`, AND `test` scripts. (Gotcha: `app/imagodei-portal/package.json` shipped without `test`, blocking husky's worker-scan from discovering its 9 standalone-resolver tests. Captured in §6.)
- [ ] **Pnpm lockfile reconciliation** — `pnpm install --lockfile-only` after the workspace entry lands. Caught during the `design/peer-oauth-portal` integration when two branches added different workspace entries (`app/lamad` and `app/imagodei-portal`).
- [ ] **`angular.json`** — projects map entry for the new bundle (build target, test target, serve target).
- [ ] **`tsconfig.json` paths** — bundle-internal `@app/<bundle>/*` alias. (Bundle-internal aliases are kept; cross-bundle aliases get retired in Wave 3.)

### §4.2 — Codegen distribution targets

- [ ] **Lamad manifest codegen** — if the bundle consumes lamad manifest types, the bundle's `src/generated/` directory must be added to `lamad:codegen`'s distribution targets.
- [ ] **Schema codegen distribution** — add the new bundle's `src/generated/` directory to `GENERATED_OUTPUT_DIRS` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`.
- [ ] **Manifest codegen distribution** — same for any manifest-driven generated types the bundle consumes.
- [ ] **Verify** — `pnpm run schema:codegen:ts --verify` reports no drift; generated artifacts appear in the new bundle's `src/generated/`.

### §4.3 — Pre-push gate paths

- [ ] **`.husky/pre-push`** — the virtual gates that route work to typecheck/lint/test commands must learn about the new bundle. Auto-detect by changed-files patterns where possible; otherwise add an explicit gate for the new bundle's path.
- [ ] **Verify** — make a trivial change in the new bundle and confirm pre-push runs the bundle's gates (not a no-op).

### §4.4 — Spec file imports (CANARY AUDIT)

The lamad split surfaced **seven** spec files orphaned by the bundle split's structural changes. The pattern: specs that count entities or list routes are canaries — they encode pre-split structural assumptions and continue to assert them after the structure changes. Audit for these specifically:

- [ ] **Route-count specs** — `grep -rln "routes\.length.*===" app/<donor-bundle>/src --include="*.spec.ts"`. The lamad canonical case: `app.routes.spec.ts` continued to assert `routes.length === 14` after lamad moved out (commit `effc26e04`).
- [ ] **App-absolute router literals in the NEW bundle** — `grep -rn "'/<pillar>" app/<new-bundle>/src --include="*.ts" --include="*.html"`. Code that moves into a bundle served at `<base href="/<pillar>/">` keeps its app-absolute `['/<pillar>/...']` routerLinks/navigations from monolith days; inside the bundle they double the prefix in the browser URL (`/<pillar>/<pillar>/...`) AND fail route matching (router matches post-base-strip), so every internal nav lands on the catch-all. The lamad canonical case: ~30 sites across 20 files broke ALL bundle-internal navigation until the 2026-06-04 de-literalization (spec §12.0 of `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`; regression anchor `genesis/a2o/features/lms/deep-link-delivery.feature` "Legacy doubled URL"). Keepers to NOT rewrite: `<base href>` itself, SEO absolute canonical URLs, and doc comments describing the public URL surface.
- [ ] **Pillar-bound module specs** — `grep -rln "loadChildren.*<your-pillar>" app/<donor-bundle>/src --include="*.spec.ts"`. Module-tree specs that lazy-load the migrating pillar will break when the pillar's path moves.
- [ ] **Lifecycle-hook specs on refactored components** — when an Angular component is rewritten as a thin Lit wrapper (the pattern: `LoginComponent`, `AuthCallbackComponent`, `EprLinkComponent`), the old spec asserting `ngOnChanges` / `OnChanges` / internal-state behavior STAYS unless explicitly retired. Audit: `grep -rln "ngOnChanges\|OnChanges" app/<bundle>/src --include="*.spec.ts"` and check whether each spec's subject is still an Angular component.
- [ ] **Convention going forward:** a refactor that removes a lifecycle hook MUST update or replace the spec in the same commit. The runbook does not let lifecycle-hook spec orphaning re-occur.

### §4.5 — Lint manifest regeneration

- [ ] If the project has a lint manifest (e.g., for ESLint flat-config project boundaries), regenerate it after the bundle split. The boundary rule that enforces "no cross-pillar imports" needs the new bundle's path on the allowed-internal list.

### §4.6 — Storybook config (if the bundle has UI primitives)

- [ ] If the new bundle exposes cross-pillar UI primitives that migrate to `elohim-core` (per disposition C), the storybook discovery glob (`../projects/**/__docs__/**/*.@(stories.ts|mdx)`) must surface their Library A and Library B stories. See `app/elohim-library/CLAUDE.md` for the Library A / Library B story coverage rules.
- [ ] Both libraries must have stories for every new `elohim-core` element: `component-architect` writes the Library A default stories; `graphos-designer` writes the Library B designed stories.

### §4.7 — A2o feature scenarios

- [ ] URL routes change with bundle splits (the bundle's `<base href>` becomes `/<pillar>/`). Update `.feature` files that assert URLs accordingly.
- [ ] If the split changes navigation behavior (e.g., introduces an EPR-link card-flip where there used to be a hard navigation), add a regression scenario in the relevant `genesis/a2o/features/<pillar>/` directory.

### §4.8 — Styling / token contract

- [ ] **`src/styles.scss` imports, in order:** `elohim-core/base` (universal reset + a11y floor —
  without it the UA's default `body{margin:8px}` frames the whole viewport, the bug lamad shipped
  on 2026-06-05), `elohim-core/tokens` (palette + `color-scheme` + `:root[data-theme]` theme
  reactivity), and the chrome binding layer (interim per-bundle `_chrome-binding.scss`; migrates
  wholesale to the graphos-tokens artifact — see
  `genesis/data/timeline/backlog/bundle-styling-token-contract.md`).
- [ ] **Never define or duplicate `--lamad-*` (or any palette) tokens in the bundle.** The token
  layer is defined once; a bundle that re-declares values forks the theme.
- [ ] **Every bound `*-bg` must have a paired bound `*-fg`** in the binding layer — an unpaired
  surface puts scheme-mismatched system colors on bound backgrounds (the 2026-06-05 dark-mode
  regression; theme-authority spec §1 C2/C3). The elohim-core theme-contrast gate enforces this
  for chrome elements.

### §4.9 — Build-artifact tracking convention

- [ ] **Pick one policy and propagate.** `app/elohim-elements/elohim-core/dist/custom-elements.json` is git-tracked but `app/elohim-elements/elohim-imagodei/dist/` is gitignored. This inconsistency caused the elohim-imagodei tests to 404 on `custom-elements.json` until the operator ran `pnpm --filter elohim-imagodei run build` locally first. **Convention going forward:** every new bundle that produces a `custom-elements.json` either git-tracks it (matches `elohim-core`) or documents the build-first dependency in its README and CI. Pick; do not leave ambiguous.

---

## §5 — The Wave 1 / Wave 2 / Wave 3 Shape

### §5.1 — Wave 1: Disposition manifest

Single agent (cartographer-tier — needs vision/judgment for SDK boundary calls). Sequential. Blocking.

**Output:** a manifest at `genesis/docs/superpowers/notes/<date>-<pillar>-cleanup-dispositions.md` with one row per cross-pillar import. Columns: source path, symbol(s), import count, disposition, target location, notes.

**Process:**

1. Re-read the cross-pillar audit if one exists (or run `grep -rE "from '@app/(elohim|imagodei|qahal|shefa|doorway|avodah|generated|account)" app/<bundle>/src --include="*.ts" | sort -u`).
2. Classify each import per the §3 taxonomy. Default to **L** when unsure and flag for the slice agent to confirm.
3. Group rows by disposition code so Wave 2 slice agents see their slice immediately.
4. Surface library-structure questions for the operator at the top of the manifest. The lamad split's Wave 1 surfaced seven such items, including: which symbols belong in a NEW library vs. fold into an existing one; per-symbol historical-misplacement corrections (the `profile.service` case — primary-dependency-driven placement, not directory-driven); barrel-split decisions (the `@app/elohim/models` case — `REAAction`/`LamadEventType` go to `@elohim/rea-runtime`, the rest to `@elohim/service`); coupled-component migrations (the `EprLinkComponent` + `EprPopoverComponent` case — programmatically-instantiated pairs migrate together).

**Acceptance signal:** every import has a row; per-disposition totals match the audit; operator-input items resolved before Wave 2 dispatches.

### §5.2 — Wave 2: Parallel migration slices

Five to seven file-disjoint slice agents executing in parallel. Each agent owns one disposition.

**File-scope discipline:**

- 2.1 (L) → `@elohim/service` library + L-classified consumers
- 2.2 (C) → `elohim-core` Lit library + C-classified consumers
- 2.3 (I) → `@elohim/identity` library + I-classified consumers
- 2.4 (R) → `@elohim/rea-runtime` library + R-classified consumers
- 2.5 (S) → `@elohim/storage-client` + S-classified consumers + codegen distribution
- 2.6 (H/E) → substrate-API consumers (doorway HTTP, EPR-link)
- 2.7 (docs) → runbook + SDK doc updates

The public-api.ts of each library is touched by exactly one slice. Slice 2.7 is documentation-only and runs in parallel from the start.

**Per-slice discipline:**

- For each symbol: identify canonical implementation, copy with `*.spec.ts`, adapt cross-references, add to public-api, rewrite consumer imports, verify build, delete original, commit per symbol.
- Test specs migrate WITH their subjects. (Gotcha §6.4.)
- Bidirectional consumer rewrites — every L/C/S/I/R migration touches BOTH the new bundle AND the donor bundle.
- Library scaffolding commits separately from migration commits — keeps blame surface clean.

### §5.3 — Wave 3: Cutover

Single agent (general — needs full context to coordinate across slices). Sequential, after all Wave 2 milestones land.

**Steps:**

1. Verify each Wave 2 slice landed cleanly: `git log --oneline | grep -E "Slice 2\.[1-7]"` returns the expected milestone count.
2. **Audit which cross-pillar aliases are still in use** and partition into two sets:
   - **Aliases used by zero remaining imports** → remove from the bundle's `tsconfig.json`.
   - **Aliases used by remaining intentional imports** (composition-root `useExisting` wiring per §6.11; deliberate deferrals per the slice's acceptance report) → KEEP, with an inline comment that explains why and points at the relevant canon section.
   - Run `grep -rE "from '@app/(elohim|imagodei|qahal|shefa|doorway|avodah|generated|testing)" app/<bundle>/src --include="*.ts" | sed -E "s/.*from '(@app/[a-z]+).*/\1/" | sort -u` to enumerate the actually-used set.
3. Verify the new bundle builds: `pnpm --filter <bundle> build` succeeds. Note: under the composition-root pattern (§6.11), the bundle is not truly source-isolated from the donor — what it IS isolated from is unintentional cross-pillar imports in its own source.
4. Verify the donor bundle still builds (bidirectional check): `pnpm --filter <donor> build` succeeds.
5. Run the full pre-push suite. Schema tests, both bundles' tests, lint, format.
6. Update canon docs that reference cross-pillar imports. Update this runbook and `elohim-sdk.md` with anything this split discovered.
7. Commit the cutover milestone.

---

## §6 — Common Gotchas (CAPTURED)

The lamad split and the `design/peer-oauth-portal` integration produced these gotchas. Future splits MUST honor them.

### §6.1 — Bundle package.json minimum scripts

Every new bundle ships with `build`, `start`, AND `test` scripts. The `app/imagodei-portal/package.json` shipped without `test`, meaning husky's pre-push hook and the worker-scan could not discover its 9 standalone-resolver tests. The next bundle's `package.json` template MUST include all three scripts.

### §6.2 — Pnpm lockfile reconciliation

When `pnpm-workspace.yaml` gains a new bundle entry, the lockfile must be regenerated. `pnpm install --lockfile-only` is the fast path (~45s on this monorepo). Caught during the `design/peer-oauth-portal` merge because both branches added different new workspace entries (`app/lamad` and `app/imagodei-portal`); the lockfile diff merged poorly and required regeneration before the workspace was usable.

### §6.3 — Build-artifact tracking inconsistency

`app/elohim-elements/elohim-core/dist/custom-elements.json` is git-tracked but `app/elohim-elements/elohim-imagodei/dist/` is gitignored. The runbook MUST take a position: pick one policy, propagate it to all bundles, and document the build-first dependency where artifacts are not tracked. The elohim-imagodei tests 404 on `custom-elements.json` until `pnpm --filter elohim-imagodei run build` runs first.

### §6.4 — Test-spec orphaning on Lit-wrapper refactor

When an Angular component is rewritten as a thin Lit wrapper (the elohim-imagodei pattern: `LoginComponent`, `AuthCallbackComponent`, `EprLinkComponent`), the old spec asserting `ngOnChanges` / `OnChanges` / internal-state behavior STAYS unless explicitly retired. The pre-push gate catches it; the integrator pays the cost.

**Convention going forward:** a refactor that removes a lifecycle hook MUST update or replace the spec in the same commit. Source: `design/peer-oauth-portal` integration; specs orphaned by commits `10516614e` (EprLinkComponent refactor) and others.

### §6.5 — Route-count specs as canaries

`app.routes.spec.ts` asserting `routes.length === 14` continued to assert the pre-split count after lamad moved out of the elohim-app routes table (commit `effc26e04`). Specs that count entities or list routes are canaries of bundle splits. They belong in the §4.4 audit checklist as files to examine before every split.

### §6.6 — Clippy regressions from upstream merges

When pulling `origin/dev` forward during a sprint, a clippy `useless_conversion` in `doorway/src/server/http.rs:1257` (introduced by `542ca8f0b`, fixed in `8c66fa3ca`) blocked the elohim-edge Docker image build for build 1002. Husky's pre-push hook catches it locally if you build doorway with `-D warnings`. **Convention:** fix-before-push rather than push-and-watch-CI-burn.

### §6.7 — Reverse dependency on the app being split

`ElohimPresenceService` (a candidate for migration to `@elohim/service`) imports `LearnerContextService` from `@app/lamad`. If the service moved as-is, the library would depend on the app — wrong direction; libraries cannot depend on the apps that consume them.

**Pattern:** Define a `<Concern>Provider` interface in the destination library; have the app being split register the implementation. Worked example: `LearnerContextProvider` interface in `@elohim/service`, registered concretely by lamad's bootstrap. Future services that face the same direction problem follow this pattern.

Source: cross-pillar import cleanup sprint, Wave 2 Slice 2.1; flagged at Wave 1 manifest operator-input #2.

### §6.8 — Historically-misplaced symbols

`profile.service` lived under `app/elohim-app/src/app/elohim/services/` but its primary dependency was `@app/imagodei/models/profile.model`. The "elohim cross-cutting" placement was historical, not structural — the service had drifted from imagodei into elohim because nobody had pushed back at the time.

**Pattern:** Check each symbol's primary dependencies to decide library placement, not its current directory. Wave 1 of every split runs this audit; symbols whose primary deps point to a different pillar get rehomed during the split, not after.

Source: cross-pillar import cleanup sprint Wave 1 manifest operator-input #1.

### §6.9 — Bare-barrel splits

The `@app/elohim/models` barrel exported many symbols, but two of them (`REAAction`, `LamadEventType`) were REA-shaped and belonged in `@elohim/rea-runtime`, not `@elohim/service`. A naive "move the whole barrel to one library" would have either polluted `@elohim/service` with REA primitives or polluted `@elohim/rea-runtime` with consent and presence models.

**Pattern:** Enumerate what the barrel actually exports and route each export to its substrate-correct home. Barrels do not map 1:1 to libraries. If your manifest has rows that say "move this barrel," it is wrong; rewrite each row as a per-symbol decision.

Source: cross-pillar import cleanup sprint Wave 1 manifest operator-input #3; coordination note for Slices 2.1 and 2.4.

### §6.10 — Coupled-component migrations

`EprPopoverComponent` was programmatically instantiated by `EprLinkComponent`. The two were structurally one concern, not two separate primitives.

**Pattern:** When an Angular component is programmatically instantiated by another, the migration unit is the COUPLED PAIR, not each component independently. When lamad swapped to `<elohim-epr-link>` (disposition E), the popover concern moved INTO the Lit element rather than continuing as a separate Angular component. The coupling was already implemented in the Lit element; the migration just respected it.

Audit for this pattern by grepping for `ViewContainerRef.createComponent` and similar programmatic-instantiation APIs in the donor pillar's components.

Source: cross-pillar import cleanup sprint Wave 2 Slice 2.6 canary report; flagged at Wave 1 manifest operator-input #4.

### §6.11 — Composition-root pattern (the LAMAD_* / `useExisting` exit)

A subset of cross-pillar services cannot cleanly migrate to the SDK because they have remaining out-of-scope dependencies in their pillar of origin. The lamad split surfaced ~14 such services (AgentService, EprResolverService, StorageApiService, GovernanceSignalService, ElohimPresenceService, …). Rather than block the bundle split on a full migration of each service plus its dep graph, the cleanup sprint adopted the **composition-root** pattern.

**Pattern (the disposition called "P" in the lamad sprint, for "lamad-local with token bridge"):**

1. The new bundle defines a narrow Angular `InjectionToken<I<Concern>>` plus `I<Concern>` interface in its internal `interfaces/cross-pillar.interface.ts`. The interface names only the methods the new bundle actually calls — not the donor pillar's full surface.
2. The bundle's own consumer code injects via the token (`inject(LAMAD_X)`), not via the concrete class.
3. The bundle's `app.config.ts` (the composition root) registers `{ provide: LAMAD_X, useExisting: ConcreteDonorClass }`. The `useExisting` import is the only place the bundle source imports the donor's concrete class.
4. The donor pillar's class stays where it is. No migration required.

This trades full library independence for a clean dependency surface: the bundle's source has zero unintentional cross-pillar imports, the donor service can move to the SDK later without breaking consumer code (only `app.config.ts` changes), and the narrow interface documents the actual API contract the bundle relies on.

**Tsconfig retention consequence:** The bundle's `tsconfig.json` must retain `@app/<donor>/*` aliases for every pillar source the composition-root imports reach. When TypeScript compiles the bundle, it transitively compiles the imported donor source files — those files use the donor's `@app/<other>/*` aliases, which must resolve via the importing bundle's tsconfig. The composition-root pattern therefore breaks the "remove all cross-pillar aliases" Wave 3 step.

**Convention going forward:** the bundle's `tsconfig.json` may keep `@app/<other-pillar>/*` aliases IF AND ONLY IF every retained alias is justified by either (a) a composition-root `useExisting` wiring or (b) a documented per-deferral acceptance report. Every retained alias gets an inline comment.

Source: cross-pillar import cleanup sprint Slice 2.1c milestone (commit `02763beb3`); Wave 3 cutover discovery that alias removal broke transitively-compiled donor source.

### §6.12 — Token-bridge for concrete-class DI tokens

A variant of the composition-root pattern surfaces when the donor pillar already defines an `InjectionToken` whose `useValue`/`useExisting` is a concrete class with un-migratable deps. The lamad split's `ECONOMIC_EVENT_FACTORY` (defined in `@app/shefa/interfaces/economic-event-factory.interface.ts`, self-provided via `EconomicEventsApiService`) is the worked example.

**Pattern:**

1. Define the canonical `InjectionToken` in the SDK library (e.g. `@elohim/rea-runtime` exports `ECONOMIC_EVENT_FACTORY`).
2. The new bundle's `app.config.ts` registers `{ provide: SDK_TOKEN, useExisting: DonorConcreteClass }`.
3. The donor pillar continues to use its own token internally; the SDK token is the cross-pillar surface.
4. When the donor's concrete class clears its dep barriers and migrates fully, the donor-local token consolidates into the SDK token (single source of truth restored).

**When to use:** the concrete service has a dependency the SDK library cannot absorb yet, but the service's INTERFACE is stable enough to publish as an SDK token. Document the consolidation deferral in the slice's acceptance report and surface it in the runbook (this §) so the next splitter knows what's still pending.

Source: cross-pillar import cleanup sprint Slice 2.4 residual (commits `0fee42d4f`, `11361812c`, `ca4f6c0c5`); reported by the slice agent as token duplication to consolidate when Slice 2.1 EconomicEventsApiService deps clear.

### §6.13 — Narrow-interface drift checklist for inversion tokens

LAMAD_* tokens (§6.11) and SDK tokens (§6.12) carry narrow interfaces that mirror the donor pillar's public API. The narrow interface usually omits surface area the new bundle doesn't currently need. Two drift hazards surface from this asymmetry:

1. **Settle-wait / async-state methods.** The donor's full guard or service may include retry / settle logic that callers rely on. The narrow interface that the new bundle adopts may omit this method entirely. If the new bundle ever needs the settle behavior, either expand the narrow interface OR register a second richer token that delegates to the donor's full guard for that specific route.
2. **Type mirrors of string-union enums.** The narrow interface re-declares string-union types (e.g., `LamadIdentityMode = 'hosted' | 'steward'`) rather than importing from the donor. If the donor adds a value to its enum, both ends need updating; otherwise the narrow type silently lags.

**Convention going forward:** every inversion-token interface declares its mirror surfaces explicitly. The slice agent that defines the token documents in the milestone commit message a list of donor methods/types that the narrow surface DOESN'T cover, plus the rationale. The next splitter checks the drift list before extending the bundle's reach.

Source: cross-pillar import cleanup sprint Slice 2.3 residual report (commit `3247740d9`); the lamadIdentityGuard intentionally omitted the imagodei guard's settle-wait loop, captured as a known limitation.

### §6.14 — Client-side stateful-orchestrator anti-pattern (smell-to-fix)

The composition-root pattern (§6.11) and token-bridge (§6.12) work cleanly for SERVICE imports. They don't apply to ANGULAR COMPONENT imports — components are imported by class for template registration; you can't `useExisting` a component the way you can a service.

When a cross-pillar Angular component you'd otherwise want to retire encapsulates **substantial stateful orchestration** — API calls, submission flow, REA event creation, mediation logic, recognition wiring — that orchestration **does not have a legitimate client-side home in the elohim architecture**. It is a smell, not a pattern. The substrate-as-steward principle ([stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §3) and the thin-client discipline (`angular-architect` agent definition + manifest-driven doorway routes) together say:

> **Aggregated state, transaction orchestration, and substrate writes are backend concerns. The client is thin: it captures user signals, calls doorway/zome, and renders the result. Only purely client-side concerns — UX, accessibility, sense-and-respond — live in Angular.**

Client-side stateful orchestrators usually arose because somebody built UX + orchestration together as one Angular component before the backend route existed. That's a normal evolution shape, but the substrate-correct end state is:

| Layer | Owns |
|---|---|
| Backend (doorway route / zome handler / storage projection) | API orchestration, REA economic-event creation, signal aggregation, submission flow, mediation logic, recognition wiring |
| `elohim-core` Lit element | Stateless UI primitive — accepts pre-computed view state as `@property`, emits user-action events |
| Client (lamad/elohim-app/etc.) Angular code | Capture event → call doorway → render result. UX, accessibility, sense-and-respond ONLY. |

**The exit path is not "wait for a library service to wrap the orchestration." The exit path is moving the orchestration to backend.** Once doorway exposes the appropriate route (or the zome the appropriate coordinator function), the client side becomes thin enough that the Lit swap is trivially clean — the consuming component captures the Lit element's submit event, POSTs to doorway, the substrate does the work.

**When you encounter this during a split:**

1. Identify the orchestration that has no legitimate client home. List the API calls, the REA events being created, the aggregated state being computed.
2. **Open a backend-migration ticket** (NOT a "wait for library service" deferral). The ticket names the substrate-correct destination: a doorway route, a zome coordinator function, a storage projection, or some combination.
3. Document the deferral in the consuming component's import block, naming:
   - The Angular component imports retained (because backend isn't ready yet)
   - The blank-slate Lit equivalents that already exist in `elohim-core`
   - The backend-migration ticket
   - Pointer to this section
4. The split closes with these three imports remaining and documented; the backend-migration sprint closes them properly when it lands.

**Distinguishing from §6.11:** composition-root tokens are a legitimate pattern — they move a service across a pillar boundary at runtime DI without disturbing source-line cross-pillar count. §6.14 names a smell — the orchestration shouldn't be on the client at all, in any pillar.

**Worked example:** Slice 2.2b of the cross-pillar import cleanup sprint surfaced this. `app/lamad/src/app/components/content-viewer/content-viewer.component.ts` retains imports of `FeedbackMechanismGatewayComponent`, `GraduatedFeedbackComponent`, and `ReactionBarComponent` from `@app/qahal`. Slice 2.2b closure (commit `625d02a0f`) migrated `MechanismSelectionService` + `SignalAccumulationService` to `@elohim/service` (those two are pure helpers — pure-function-style mechanism-ladder derivation and signal-to-status flag derivation; legitimately client-side OR backend; migrated to library to be host-agnostic). The remaining three Angular components encapsulate ~1650 lines of true orchestration: API calls to `governance-api`, REA economic-event creation via `recognition-api`, form-submission flow, mediation dialog wiring, reaction aggregation. **Those 1650 lines should not live on the client in any bundle.** The substrate-correct destination is a doorway route (`POST /api/v1/governance/feedback` or similar) that creates the REA economic event server-side, projects the signal aggregate, and returns the updated view. The three component imports are retained as a backend-migration deferral pointing at this section. The deferral comment in content-viewer.component.ts L52-67 names this.

Source: cross-pillar import cleanup sprint Slice 2.2b closure (commit `625d02a0f`, 2026-05-28); operator correction `2b31aa62b...` reframed an initial "library service emerges" framing as the correct backend-migration framing.

---

## §7 — Worked Example: The Lamad Split

The cross-pillar import cleanup sprint (2026-05-25) is the foundational case study. Future splitters read this section as the "this is what it looks like" reference.

### §7.1 — The B0 audit

The pre-sprint audit (`genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md`) found 159 cross-pillar imports in `app/lamad/src/`. By the time the sprint dispatched, integration of `design/peer-oauth-portal` had drifted the count to 261 (228 `@app/elohim`, 28 `@app/imagodei`, 11 `@app/shefa`, 9 `@app/qahal`, 7 `@app/generated`). The pre-sprint readiness checklist captured the drift; Wave 1 manifest's per-disposition totals reflected the post-integration baseline, not the pre-integration audit.

**Lesson:** between audit and sprint, integration can drift the count significantly. Re-enumerate at sprint start.

### §7.2 — The Wave 1 manifest

Wave 1 (cartographer-tier, Opus) produced the manifest in ~30 minutes. Per-disposition totals:

- L (→ `@elohim/service`): 38 source modules, ~190 import lines, ~95 consumer files.
- C (→ `elohim-core`): 7 source modules, 11 import lines.
- S (→ `@elohim/storage-client` + bundle-local `generated/`): 10 source modules, 27 import lines.
- I (→ `@elohim/identity` NEW): 8 source modules, 28 import lines.
- R (→ `@elohim/rea-runtime` NEW): 6 source modules, 10 import lines.
- E (→ `<elohim-epr-link>`): 2 source modules, 2 import lines.
- H, D, X: empty.

Seven operator-input items surfaced (covered in §6.1–§6.10 — those gotchas are not retrospective inventions; they are what Wave 1 named).

### §7.3 — The Wave 2 parallel structure

Seven slices dispatched in parallel after operator resolved the seven manifest items. File-disjoint scopes; per-slice agent roles per `genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md` §Wave 2.

Slice 2.7 (this runbook + the SDK boundary doc) ran in parallel from the start because its work product had no source-code dependency on the other slices' deliverables.

### §7.4 — The Wave 3 cutover

Wave 3 (general agent, full context) audited the actually-used aliases in lamad post-Wave-2:

- `@app/shefa`, `@app/shefa/*`, `@app/generated/*`, `@app/doorway`, `@app/doorway/*`, `@app/avodah`, `@app/avodah/*`, `@app/testing`, `@app/testing/*` — zero remaining imports in lamad. **Removed** from `app/lamad/tsconfig.json`.
- `@app/elohim`, `@app/elohim/*` — 14 imports in `app/lamad/src/app/app.config.ts` for composition-root `useExisting` wiring of LAMAD_* tokens (§6.11). **Retained** with an inline comment block.
- `@app/imagodei`, `@app/imagodei/*` — 1 import in `app/lamad/src/app/app.config.ts` for the `LAMAD_IDENTITY` `useExisting: IdentityService` wiring (Slice 2.3 residual outcome). **Retained**.
- `@app/qahal`, `@app/qahal/*` — 3 imports in `app/lamad/src/app/components/content-viewer/content-viewer.component.ts` for the documented Slice 2.2b Lit-swap deferral (the swap would require ADDING new L-slice imports for MechanismSelectionService + SignalAccumulationService + GovernanceRecognitionService, net cross-pillar count goes up). **Retained**.

Wave 3 verified bidirectional builds. Verified full pre-push suite. Committed the cutover milestone.

**Lesson for §6.11**: removing all cross-pillar aliases is not always achievable under the composition-root pattern. The runbook's acceptance criteria (§8) reflect this: the metric that matters is "zero unintentional cross-pillar imports in the bundle's own source," not "zero aliases in the bundle's tsconfig." Every retained alias must be justified and inline-commented.

### §7.5 — The two canon docs

Slice 2.7 produced [elohim-sdk](epr:elohim-sdk) (the SDK boundary canon) and this runbook. The SDK doc named the five libraries; the runbook captured how to do the next split. Together they are the **durable artifact** of the cross-pillar import cleanup sprint — the lamad split was the first; the runbook is what makes the rest follow without re-deriving.

---

## §8 — Acceptance Criteria for a Complete Split

A pillar split is done when ALL of the following hold:

- [ ] **The new bundle builds.** `pnpm --filter <bundle> build` succeeds.
- [ ] **The donor bundle still builds (bidirectional check).** `pnpm --filter <donor> build` succeeds; donor's consumers of migrated symbols resolve through the SDK libraries, not pillar source.
- [ ] **The bundle's own source has zero unintentional cross-pillar imports.** Use `grep -rE "from '@app/(<every-other-pillar>)" app/<bundle>/src --include="*.ts"` to enumerate. Every remaining import is either (a) in the composition root (§6.11), (b) a documented deferral with a slice acceptance-report citation, or (c) a justified D-disposition copy. NO unintentional imports.
- [ ] **The bundle's `tsconfig.json` retained-aliases are each justified.** Aliases for pillars the bundle no longer imports from are removed. Aliases retained for composition-root or deferrals are inline-commented and pointed at the relevant canon section. (See §6.11.)
- [ ] **All pre-push gates pass.** Schema codegen freshness, both bundles' tests, lint, format, clippy on doorway, Rust workspace builds.
- [ ] **The SDK boundary doc is updated.** If the split created a new library or moved a symbol between libraries, [elohim-sdk](epr:elohim-sdk) §3 reflects the new state.
- [ ] **This runbook is updated.** If the split discovered a new gotcha, §6 captures it (with the commit SHA where known and the lesson named).
- [ ] **Operator dogfood verified.** If the bundle is web-served, the operator confirms the bundle loads from the doorway at the expected URL path (per the pillar-EPR design's MVP scope §8).

The bar for "complete" is the runbook itself remaining useful for the NEXT split. If your split surfaced something the runbook does not yet capture, the runbook is incomplete until you capture it.

---

## §9 — References

### Canon (this directory)

- [elohim-sdk](epr:elohim-sdk) — the SDK boundary that every disposition routes to.
- [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) — why substrate-as-steward demands bundle independence.
- [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) — the primitive `@elohim/rea-runtime` surfaces.
- [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) — the gradient `@elohim/identity` makes legible.

### Specs

- `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` — names the remaining pillar splits (shefa, qahal, avodah, imagodei, account, doorway).
- `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` — the Capability Profile primitive `elohim-core` elements honor.

### Plans + Notes

- `genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md` — the foundational sprint plan.
- `genesis/docs/superpowers/notes/2026-05-25-cross-pillar-cleanup-dispositions.md` — Wave 1 disposition manifest from the lamad split.
- `genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md` — B0 audit baseline (159 imports pre-integration).

### Operational references

- `app/elohim-library/CLAUDE.md` — Library A / Library B story conventions for any C-disposition element migration.
- `elohim/sdk/schemas/v1/views/CONVENTIONS.md` — wire-shape conventions for any S-disposition type migration.
- `elohim/sdk/schemas/scripts/codegen-ts.mjs` — codegen distribution targets (`INTERFACE_FILES`, `GENERATED_OUTPUT_DIRS`).
- `.husky/pre-push` — pre-push gate routing.

---

## §10 — Closing Note

The runbook is the second of the cross-pillar import cleanup sprint's two durable artifacts. The first ([elohim-sdk](epr:elohim-sdk)) named the boundary; this one captures the operation.

Every future pillar splitter inherits both. The cost of the lamad split is amortized across the remaining six splits — shefa, qahal, avodah, imagodei, account, doorway — and across every third-party pillar that may follow them.

The runbook is finished only when the next split does not need it. Each successful split that adds nothing to §6 is evidence the runbook is converging on completeness. Each successful split that adds a gotcha to §6 is the runbook doing its job — capturing what would have been re-paid in agent-hours and integration regressions.

Follow it. Extend it. The protocol's pillar architecture depends on the seam holding.
