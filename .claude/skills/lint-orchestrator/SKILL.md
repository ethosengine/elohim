# Lint & Coverage Orchestrator Skill

Orchestrate parallel Haiku agents to fix lint issues and write basic tests. Escalate complex work to Sonnet. Run with Sonnet for cost-effective batch processing.

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

### Haiku Test Writing (Mechanical Only)
```
Write basic mechanical tests for:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Do NOT run tests. Write only:
- Method existence tests
- Simple input/output tests
- Observable return tests
- Property initialization tests

Escalate to Sonnet:
- Async flow testing
- Complex mock setup
- Error handling paths
- Business logic verification

Report with Test Outcome format.
```

### Sonnet Test Writing (Complex/Escalated)
```
Write comprehensive tests for:
/projects/elohim/elohim-app/src/app/PATH/FILE.ts

Escalated from Haiku because: [reason]

Do NOT run tests. Cover:
- Async/Observable chains
- Error handling
- Edge cases
- Business logic

Report gaps and design suggestions.
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

### 2. Dispatch Haiku Agents (max 7 parallel)

Use the Task tool with:
- `subagent_type`: "general-purpose"
- `model`: "haiku"
- `run_in_background`: true
- `allowed_tools`: ["Read", "Edit"] (minimal for speed)

### 3. Agent Prompt Templates

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
| Mechanical lint fixes | mechanical | Haiku | No |
| Basic test writing | mechanical | Haiku (linter) | No |
| Type safety fixes | contextual | Sonnet | No |
| Complex test writing | contextual | Sonnet (test-generator) | No |
| Architectural decisions | judgment | Opus | No |
| **Batch test run** | - | npm test | **Yes** |

All test execution happens in batch via `npm test` before commit, which also updates coverage annotations.
