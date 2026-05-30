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
