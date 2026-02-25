# Close the Dev-to-A2O Loop

Verify that recent implementation work has corresponding a2o acceptance scenarios. Generate missing scenarios from captured intent or git diff.

## Steps

1. **Read intent entries** from `.claude/data/dev-intent.jsonl`:
   - Each line is a JSON object with: `ts`, `domain`, `files`, `summary`, `learner_impact`, `scenario_hint`
   - If no entries exist, fall back to step 2

2. **Infer from git diff** (fallback when no intent entries):
```bash
git diff --cached --name-only
git diff --name-only
```
   - Map changed files to domains: `lamad/` -> lamad, `imagodei/` -> auth, `qahal/` -> governance
   - Read the diff content to understand what changed

3. **Map domains to a2o features**:

| Domain | Feature Directory | Existing Features |
|--------|-------------------|-------------------|
| lamad | `genesis/a2o/features/lamad/` | learning-journey, know-thyself-discovery |
| imagodei/auth | `genesis/a2o/features/auth/` | auth-lifecycle, fixture-humans |
| content | `genesis/a2o/features/content/` | content-lifecycle |
| federation | `genesis/a2o/features/federation/` | cross-doorway-content |

4. **Read existing scenarios** for the affected domain(s):
   - Read the relevant `.feature` files
   - Read the corresponding step definitions in `genesis/a2o/steps/`

5. **Identify gaps**: What was built that has no scenario coverage?
   - Compare the intent summary / diff against existing scenario descriptions
   - List specific behaviors that need scenarios

6. **Generate scenario updates**:
   - Follow conventions in `genesis/a2o/CLAUDE.md`
   - For existing feature files: add new `Scenario:` blocks
   - For new domains: create new `.feature` files with proper tags and Background
   - Tag new scenarios with `@wip` if step definitions don't exist yet
   - Generate step definition skeletons for any new steps

7. **Run coverage scanner** for before/after comparison:
```bash
cd genesis/a2o && npx tsx scripts/scan-coverage.ts
```

8. **Report results**:
   - Which feature files were updated/created
   - Which scenarios were added
   - Which step definitions need implementation (`@wip`)
   - Coverage delta from the scanner

9. **Clear processed intent entries** from `dev-intent.jsonl` (remove lines that were addressed)
