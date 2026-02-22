# Generate Executable BDD Scenarios from Gap Report

Read the BDD coverage gap report and generate executable feature files + step definition skeletons for the top priority gaps.

**Argument**: Number of gaps to generate for (default: 3). Pass as `$ARGUMENTS`.

## Steps

1. Read the gap report:
```
orchestrator/e2e/reports/coverage-gap-report.json
```

If the report doesn't exist, run the scanner first:
```bash
cd orchestrator/e2e && npx tsx scripts/scan-coverage.ts
```

2. For the top N gaps (from `prioritizedGaps`, where N = `$ARGUMENTS` or 3):

### a. Read Source Genesis Features
For each gap, find the genesis feature files that match the gap's domain + governance layer:
```
genesis/docs/content/elohim-protocol/{domain}/{user_type}/scenarios/{governance_layer}.feature
```

Read these files to understand the conceptual scenarios.

### b. Read Existing Executable Features as Style Reference
Read these files to match the executable style:
- `orchestrator/e2e/features/federation/cross-doorway-content.feature` — feature file pattern
- `orchestrator/e2e/steps/federation.steps.ts` — step definition pattern

### c. Generate Feature Files
Create `.feature` files at the `suggestedFeatureFile` path from the gap report.

**Translation strategy**: conceptual scenarios describe human experiences. Executable scenarios test the API contracts that would support those flows. For example:
- "Workers implement community meal program" -> test content creation + tagging + cross-doorway discovery
- "Workers welcome community stakeholder input" -> test content creation + governance metadata + search

Each generated feature should:
- Use `@e2e @{epic}` tags on the first line
- Include a Background with doorway health checks
- Translate 3-5 conceptual scenarios into executable ones
- Use Given/When/Then with the existing step patterns (human registration, content creation, content discovery)
- Add `@wip` tag to scenarios that need step definitions not yet implemented

### d. Generate Step Definition Skeletons
Create skeleton step files at `orchestrator/e2e/steps/{epic}.steps.ts`.

Each skeleton should:
- Import from `@cucumber/cucumber` (Given, When, Then)
- Import `E2EWorld`, `Human`, `BrowserDevice` from the framework
- Include stub step definitions matching the new feature file
- Mark incomplete steps with `// TODO: implement` comments
- Follow the pattern from `federation.steps.ts`

### e. Verify
For each generated file:
- Ensure the feature file parses (no syntax errors)
- Ensure the step file compiles (typecheck)

3. Report what was generated:
- List of new feature files with scenario counts
- List of new step definition files
- Which steps are fully implemented vs TODO
- Updated coverage estimate (run scanner again)
