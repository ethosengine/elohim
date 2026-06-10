# A2O - Acceptance-to-Outcome Testing

Executable BDD scenarios that verify the learner experience matches the Elohim Protocol vision.

## The Chain

```
genesis/docs/content/elohim-protocol/  Manifesto, epics, conceptual scenarios
        |
genesis/data/lamad/                    Seed data (JSON) derived from docs
        |
elohim-app/src/app/                    Runtime implementation (Angular)
        |
genesis/a2o/features/                  Executable acceptance scenarios (HERE)
```

Conceptual scenarios (in `genesis/docs/`) describe the human experience. Executable scenarios (here) test the system contracts that deliver those experiences.

## Feature File Conventions

- Tags: `@e2e @{domain}` on first line; add `@browser-only` for Playwright-dependent scenarios
- Background: always include `Given doorway "alpha" at "E2E_DOORWAY_ALPHA"`
- Humans: use named personas (`Given human "Terrance" is logged in on doorway "alpha" with device`)
- Add `@wip` to scenarios with unimplemented step definitions
- Add `@regression` to scenarios that guard against specific past bugs
- **Substrate scope: `@requires:<cap>`.** Tag a feature/scenario with `@requires:<cap>` when it needs a
  substrate dependency point declared in `genesis/manifests/cluster-state.yaml` (`shem`,
  `alpha-cluster-6peer`, …). This is the SETPOINT of a cybernetic scope reconciler with two arms that
  share one vocabulary, so toggling a capability on/off cascades automatically:
  - **Feature-level `@requires:<cap>`** (tag line above `Feature:`) ⇒ when `<cap>` is unavailable,
    `scope-reconcile.py` git-mv's the WHOLE feature to `held/` — out of the cucumber glob AND out of
    agentic-search/planning scope. Use when EVERY scenario needs the cap.
  - **Scenario-level `@requires:<cap>`** (tag line above a `Scenario:`) ⇒ the feature stays live and the
    `Before` gate in `steps/common.steps.ts` skips just that scenario at runtime (HELD, not failed). Use
    for MIXED features where some scenarios are household-testable.
  - **`shem` ≠ multi-node.** The household (matthew/jessica/james) is itself a 3-node cluster, so a
    cross-node/`@requires:multi-node` scenario among them is household-testable — do NOT tag it `@requires:shem`.
    Only the remote multi-tenant canvas (adam/caleb/pete/… or >3 independent peers) needs `shem`.
  - Toggle a capability with `scope-reconcile.py --set <cap>=off|on [--apply]`; the runtime arm derives
    from the same `cluster-state.yaml` (and the `ELOHIM_REMOTE_COMPUTE_STATUS` CI override for `shem`).
    Caps NOT in cluster-state (`@requires:doorway`, `@requires:seeded-content`) are fixture preconditions,
    not substrate gates — they're ignored by the scope reconciler. The primitive is
    `src/framework/fixtures/substrate-scope.ts`; design: `genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md`.

## What belongs in feature files

Feature files describe **what a human goes through** — learner experiences, timing/behavior constraints, recovery flows, peer-visible state. They are NOT the right place for: serialization bugs, schema codegen sync, type-system conventions, build-system mandates, or layer-internal contracts. Those belong in unit tests, schema-contract tests, pre-push hooks, or memory entries. The bar: "Did the human's experience change, or did I just learn something about the codebase?" Only the former harvests into a2o.

**Feature/scenario authoring is Opus work.** `Feature:` blocks, scenario titles, Given/When/Then narrative, frontmatter, and persona setup are load-bearing for vision alignment — Haiku produces scenario-shaped objects that pass tests but don't carry the story. Step definition wiring, fixture builders, and helper utilities are fine for Sonnet/Haiku. When dispatching agents for a2o coverage work, split the authoring (Opus) from the glue (Sonnet/Haiku).

## Watch-outs

- **A gherkin parse error aborts the WHOLE E2E run, not one scenario.** An unescaped `/` inside a scenario name or step (read by the parser as an empty regex alternation), a bare continuation line the AST rejects, or a malformed table will abort the entire feature-file load — every scenario in the run is lost and the build surfaces as `UNSTABLE` with a **blank cucumber report body** (empty report → UNSTABLE with no failing-scenario detail). When that happens, READ THE RAW E2E LOG FIRST (the parse error is there, not in the empty report). Backlog: a pre-push gherkin/cucumber grammar linter (`gherkin-prepush-lint`) to catch empty-alternation + bare-continuation before AST-abort drops the whole suite.
- **`deployments.json` is the single source of truth for "is this human exercise-able"** (`genesis/orchestrator/data/deployments.json`). Each human entry has `nodeTypes` and an optional `suspended: true`. Three code paths gate on it: deploy (manifest rendering), seed (`seed-humans.ts`), and test (`isHumanDeployed()` in `src/framework/fixtures/humans.ts` returns `'pending'` for suspended humans). Any new code path that exercises per-human conductor pods MUST respect this flag (pattern: `loadSuspendedNames()`, fail-open if unreadable). Paths that only go through doorway's hosted-pool don't need it.
- **Persona renames cascade across five surfaces**: canonical content (`humans.json`), generated downstream files with persona-keyed filenames (`account-packages/<persona>.json`, `human-<persona>.json` — generators write new but never delete old), non-generated filenames in a2o features/fixtures, test/orchestrator string literals, and cross-doc references. Before claiming a rename complete: `find . -type f -name "*<old>*"` + `grep -rn "<old>"` across all relevant extensions; diff generated indices vs canonical state.
- **Native `<dialog>` + `showModal()` modal migration gotchas (browser steps).** The top-layer modal fix (a `<dialog>` opened with `showModal()` renders into the browser top layer, above all stacking contexts, unaffected by ancestor `transform`/`overflow`) changes how you assert in step defs: (1) a native `<dialog>` has **no z-index** — `getComputedStyle().zIndex` returns `"auto"` (→ `NaN`); assert the `:modal` pseudo-class instead of a z-index value; (2) a synthetic `KeyboardEvent('Escape')` does **NOT** trigger the UA Escape handler — test close via the `(close)` event / `dialog.close()`, not by dispatching Escape; (3) backdrop-click is detected via `event.target === dialogEl`. (Full lesson: memory entry `feedback_native_dialog_top_layer_modal`.)

## Step Definition Patterns

- File: `steps/ui/{domain}.steps.ts` for browser steps, `steps/{domain}.steps.ts` for API steps
- Framework: `E2EWorld` (Cucumber world), `PlaywrightDevice` (browser automation)
- Guard: `requirePlaywright(this)` returns null in non-Playwright mode (return `'pending'` to skip)
- Selectors: import from `src/framework/pages/selectors.ts` (shared with page-model skill)

## Domain Mapping

| App Pillar | Feature Directory | Key Feature Files |
|-----------|-------------------|-------------------|
| lamad | `features/lamad/` | learning-journey, know-thyself-discovery |
| imagodei | `features/auth/` | auth-lifecycle, fixture-humans |
| elohim | `features/content/` | content-lifecycle |
| federation | `features/federation/` | cross-doorway-content |

## Tools

- **Coverage scanner**: `npx tsx scripts/scan-coverage.ts` — compares conceptual vs executable scenario coverage
- **Step skeleton generator**: `npx tsx scripts/generate-step-skeletons.ts` — stubs for undefined steps
- **Render & see (`look`)**: `pnpm look <url> [--as <FixtureHuman>] [--doorway <id|url>] [--wait-testid <id>] [--out <slug>] [--viewport WxH]` — renders a surface headless in Che, writes `reports/look/<latest|slug>/{shot.png,capture.json}`, and prints both paths. The fast "glance at the app" loop for agentic iteration; reuses `PlaywrightDevice` capture (console/pageerror/failed-requests). `--as` logs in as a fixture human first. First run needs `pnpm a2o:setup` (installs Chromium to the XDG cache once).
- **Operator view (`reports:serve`)**: `pnpm reports:serve` — serves `reports/` (look captures, screenshots, cucumber reports) on port 4201 = the Che `ui-playground` public endpoint. The operator opens that endpoint route in their browser and sees the SAME artifacts the agent reads — symmetric vision. Zero-dep static server (`scripts/serve-reports.ts`); port 4201 is a shared dev slot (mutually exclusive with a second dev server there).
- **Slash commands**: `/gap-analysis` (sprint planning), `/generate-scenarios` (bulk from gap report), `/close-loop` (per-commit verification)

## Authorized writes on shared alpha

Alpha is shared deployed state owned by real peers; agent writes happen only under an explicit
operator permission grant (settings/permissions dialog — never inferred from conversation).
Within a grant: test-persona content only (fixture humans), no bulk seeding, no destructive
flows; alpha state remains operator-owned (repo manifests are the cleanup surface). Authenticated
flows (`look --as <FixtureHuman>`, `test:browser` against alpha) are deliberate acts — the
default loop stays read-mostly. Granted 2026-06-10: the `look --as` / `test:browser` command
family (arc plan Task 0.1).
