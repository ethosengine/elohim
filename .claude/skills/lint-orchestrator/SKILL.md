# Lint & Coverage Orchestrator Skill

Orchestrate the two-tier quality pipeline (quality-sweep → quality-deep) for lint fixes, test writing, and backlog generation.

## Pipeline Overview

```
Orchestrator checks: does module have existing specs?
     │
     ├─ YES → dispatch quality-sweep directly
     │
     └─ NO  → dispatch quality-deep to seed reference spec
                    │
                    ▼
quality-deep (Sonnet) - Reference Spec Seeding
     │
     ├─ Creates: 1 thorough spec with mock factories + // PATTERN: comments
     └─ Reports: sweep-ready sibling files list
                    │
                    ▼
quality-sweep (Haiku) - 1st Pass
     │
     ├─ Handles: ~80% (mechanical lint, basic tests)
     ├─ Copies: mock patterns from reference spec
     └─ Escalates: ~20% → quality-deep
                          │
                          ▼
quality-deep (Sonnet) - 2nd Pass
     │
     ├─ Finishes: Complex tests, refactoring
     └─ Creates: GitHub Issues for ~5% needing sprint work
                          │
                          ▼
GitHub Issues (backlog label)
     │
     └─ Sprint planning picks up P0/P1 items
```

## Coverage Workflow

Coverage annotations (`// @coverage: X%`) are embedded in source files and updated by `npm test`.

### Dispatch Pattern for Low Coverage Files

```bash
# Find files with low coverage (below 70%)
grep -r "@coverage:" elohim-app/src --include="*.ts" | grep -v ".spec.ts" | while read line; do
  pct=$(echo "$line" | grep -oP '\d+(\.\d+)?(?=%)')
  if (( $(echo "$pct < 70" | bc -l) )); then echo "$line"; fi
done
```

### quality-sweep Test Writing (Mechanical Only)
```
Write basic mechanical tests for:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Do NOT run tests. Write only:
- Method existence tests
- Simple input/output tests
- Observable return tests
- Property initialization tests

Escalate to quality-deep:
- Async flow testing
- Complex mock setup
- Error handling paths
- Business logic verification

Report with First-Pass Complete format.
```

### quality-deep Test Writing (Complex/Escalated)
```
Finish tests for:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Escalated from quality-sweep because: [reason]

Do NOT run tests. Cover:
- Async/Observable chains
- Error handling
- Edge cases
- Business logic

For items needing sprint work, create GitHub Issues:
- Search first: gh issue list --search "FILE.ts" --label "backlog"
- Create: [module/area] Title format with backlog label
- Report issues created in Second-Pass Complete format
```

## Quick Start

```bash
# Regenerate lint manifest
.claude/scripts/extract-lint-issues.sh

# Check issue counts
jq '[.[] | select(.tier == "mechanical")] | group_by(.ruleId) | map({rule: .[0].ruleId, count: length}) | sort_by(-.count)' .claude/lint-manifest.json
```

## Agent Dispatch Pattern

### 1. Get Issues by Rule Type
```bash
# Get files with most issues for a specific rule
jq -r '[.[] | select(.ruleId == "RULE_ID")] | group_by(.file) | map({file: .[0].file | split("/") | .[-1], path: .[0].file, count: length}) | sort_by(-.count) | .[0:10]' .claude/lint-manifest.json
```

### 2. Check for Uncovered Modules (Reference Spec Seeding)

Before dispatching sweep, check if target modules have existing spec files:

```bash
# Find modules with no spec files at all
for dir in elohim-app/src/app/MODULE/services; do
  specs=$(find "$dir" -name "*.spec.ts" 2>/dev/null | wc -l)
  sources=$(find "$dir" -name "*.ts" ! -name "*.spec.ts" 2>/dev/null | wc -l)
  if [ "$specs" -eq 0 ] && [ "$sources" -gt 0 ]; then
    echo "NEEDS SEED: $dir ($sources files, 0 specs)"
  fi
done
```

**If a module has zero specs**, dispatch quality-deep FIRST to create a reference spec:

```
Create a reference spec for the most representative service in:
/projects/elohim/elohim-app/src/app/MODULE/services/

This is a reference spec seeding task. Follow the Reference Spec Seeding
workflow in your agent prompt:
1. Scan the module for all untested files
2. Pick the best seed target (most dependencies, most representative)
3. Write a thorough spec with mock factories and // PATTERN: comments
4. Report the sweep-ready siblings list
```

Wait for deep to finish, then dispatch sweep for the siblings it listed.

### 3. Dispatch quality-sweep Agents (max 7 parallel)

Use the Task tool with:
- `subagent_type`: "quality-sweep"
- `run_in_background`: true

quality-sweep handles 80% mechanical work, escalates 20% to quality-deep.

**Important**: Sweep relies on existing spec files as reference for mock patterns and data shapes. If sweep is assigned a file in a module with no specs, it will escalate the entire file back to deep. To avoid this round-trip waste, always run the reference spec seeding step (step 2) first for uncovered modules.

### 4. Dispatch quality-deep for Escalations

After quality-sweep completes, dispatch quality-deep for escalated items:
- `subagent_type`: "quality-deep"
- `run_in_background`: true

quality-deep finishes complex work, creates GitHub Issues for the 5% needing sprint work.

### 4. Agent Prompt Templates

#### Unused Variables
```
Fix ALL @typescript-eslint/no-unused-vars issues in:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

For each unused var:
- Unused import: remove it
- Unused parameter: prefix with underscore (_param)
- Unused variable: remove or prefix with underscore

Read the file, fix all issues efficiently.
```

#### Button Has Type
```
Fix ALL @angular-eslint/template/button-has-type issues in:
/projects/elohim/elohim-app/src/app/PATH/FILE.html

Add type="button" to all <button> elements missing it.
Read the file, fix all issues efficiently.
```

#### Nullish Coalescing
```
Fix ALL @typescript-eslint/prefer-nullish-coalescing issues in:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Replace || with ?? for null/undefined default values.
Read the file, fix all issues efficiently.
```

#### Floating Promises
```
Fix ALL @typescript-eslint/no-floating-promises issues in:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Add void prefix before unhandled promises.
Read the file, fix all issues efficiently.
```

#### No Empty Blocks
```
Fix ALL no-empty lint issues in:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Add descriptive comments to empty catch blocks:
catch (_err) {
  // Error handled silently - description of why
}
```

## Mechanical Issue Types (Haiku-appropriate)

| Rule | Fix Pattern |
|------|-------------|
| no-unused-vars | Remove or prefix with `_` |
| prefer-nullish-coalescing | `\|\|` → `??` |
| no-floating-promises | Add `void` prefix |
| button-has-type | Add `type="button"` |
| no-duplicate-attributes | `[class]` → `[ngClass]` |
| no-empty | Add comment to empty block |
| eqeqeq | `==` → `===` |
| no-console | Remove console statements |
| promise-function-async | Add `async` keyword |
| require-await | Remove `async` or add `await` |

## Contextual Issue Types (Sonnet-appropriate)

| Rule | Requires |
|------|----------|
| no-unsafe-member-access | Type inference, interface design |
| no-unsafe-assignment | Proper typing |
| no-explicit-any | Replace with specific types |
| no-unsafe-argument | Type guards or assertions |

## Workflow

1. **Regenerate manifest** after each batch completes
2. **Dispatch 7 agents max** to avoid crashes
3. **Commit after completions** to preserve progress
4. **Check remaining issues** before next batch

## Commit Pattern

```bash
git add -A && git commit -m "fix(lint): RULE_NAME - description

Files: X files changed
Issues: Y issues fixed

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

## Cost Optimization

- **Orchestration**: Use Sonnet (this skill)
- **Mechanical fixes**: Use Haiku agents
- **Type safety fixes**: Use Sonnet agents
- **Architectural decisions**: Use Opus

## Session Metrics

Track progress with:
```bash
# Total lint issues
jq 'length' .claude/lint-manifest.json

# By tier
jq 'group_by(.tier) | map({tier: .[0].tier, count: length})' .claude/lint-manifest.json

# Coverage below threshold
grep -r "@coverage:" elohim-app/src --include="*.ts" | grep -v ".spec.ts" | wc -l
```

## Tier Summary

| Task | Tier | Agent | Runs Tests? |
|------|------|-------|-------------|
| Reference spec seeding | seeding | Sonnet (quality-deep) | No |
| Mechanical lint fixes | mechanical | Haiku (quality-sweep) | No |
| Basic test writing | mechanical | Haiku (quality-sweep) | No |
| Type safety fixes | contextual | Sonnet (quality-deep) | No |
| Complex test writing | contextual | Sonnet (quality-deep) | No |
| Architectural decisions | judgment | Opus | No |
| **Batch test run** | - | npm test | **Yes** |

All test execution happens in batch via `npm test` before commit, which also updates coverage annotations.

## GitHub Issues Workflow (The 5%)

quality-deep creates GitHub Issues for work needing sprint planning.

### Check Existing Backlog
```bash
# View all backlog items
gh issue list --label "backlog" --limit 50

# Filter by priority
gh issue list --label "backlog,P1"

# Search by module
gh issue list --search "[lamad/" --label "backlog"
```

### Issue Format

**Title**: `[module/area] Brief description`
- `[lamad/path] Implement mastery-based suggestions`
- `[imagodei/auth] Add session timeout handling`

**Labels**:
- `backlog` (required)
- Priority: `P0`, `P1`, `P2`, `P3`
- Type: `architectural`, `feature-gap`, `security`, `tech-debt`

### Sprint Planning

```bash
# Review P0/P1 for next sprint
gh issue list --label "backlog,P1" --json number,title,labels

# Promote to sprint
gh issue edit 123 --milestone "Sprint-Name" --remove-label "backlog" --add-label "sprint-active"

# After sprint completion
gh issue close 123
```

### Backlog Hygiene

```bash
# Find stale backlog items (no updates in 30 days)
gh issue list --label "backlog" --json number,title,updatedAt | \
  jq '[.[] | select(.updatedAt < (now - 2592000 | todate))]'

# Bulk update priority
gh issue edit 123 124 125 --add-label "P2" --remove-label "P3"
```
