# BDD Coverage Gap Analysis

Run the BDD coverage gap scanner and interpret the results for sprint planning.

## Steps

1. Run the scanner:
```bash
cd orchestrator/e2e && npx tsx scripts/scan-coverage.ts
```

2. Read the generated report:
```
orchestrator/e2e/reports/coverage-gap-report.json
```

3. Present an interpreted analysis with:

### Summary Statistics
- Total conceptual scenarios (from genesis feature files)
- Total executable scenarios (from Cypress + Cucumber-JS)
- Coverage percentage
- Number of epics with zero test coverage

### Top 5 Priority Gaps
For each of the top 5 entries in `prioritizedGaps`:
- **Domain** and **governance layer**
- **Conceptual density** (how many scenarios exist conceptually)
- **Rationale** for why this gap matters
- **Suggested feature file** path for new executable tests

### Sprint Recommendations
Using quality-architect user story format:

```
**As a** [persona from the gap's user types]
**I want** [executable E2E coverage for this domain]
**So that** [the loop closes and regressions are caught]

### Acceptance Criteria
- [ ] Feature file created at suggested path
- [ ] Step definitions use E2EWorld, DoorwayClient, BrowserDevice patterns
- [ ] At least N scenarios converted from conceptual to executable
- [ ] `npm run scan:coverage` shows improved coverage for this epic
```

### Genesis Features to Convert First
For each gap, list the specific genesis feature files (`featureFile` from the conceptual scenarios) that should be translated first. Prioritize files with the most scenarios.

### Loop Status
How many iterations of the scanner → generate → review loop have occurred (check git log for coverage-gap-report.json commits). Note the trend in coverage percentage.
