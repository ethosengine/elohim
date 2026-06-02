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

## Watch-outs

- **A gherkin parse error aborts the WHOLE E2E run, not one scenario.** An unescaped `/` inside a scenario name or step (read by the parser as an empty regex alternation), a bare continuation line the AST rejects, or a malformed table will abort the entire feature-file load — every scenario in the run is lost and the build surfaces as `UNSTABLE` with a **blank cucumber report body** (empty report → UNSTABLE with no failing-scenario detail). When that happens, READ THE RAW E2E LOG FIRST (the parse error is there, not in the empty report). Backlog: a pre-push gherkin/cucumber grammar linter (`gherkin-prepush-lint`) to catch empty-alternation + bare-continuation before AST-abort drops the whole suite.
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
- **Slash commands**: `/gap-analysis` (sprint planning), `/generate-scenarios` (bulk from gap report), `/close-loop` (per-commit verification)
