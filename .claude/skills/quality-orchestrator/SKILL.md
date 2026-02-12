# Quality Orchestrator — Team-Based Campaign System

Orchestrate the three-tier quality pipeline using Agent Teams. Agents receive **campaign-sized batches** (all issues for one rule across all files) and work continuously until done, maximizing context window utilization.

## Architecture

```
Team Lead (this conversation)
  ├── sweep-1..N (Haiku)  — mechanical campaigns (pattern replacements)
  ├── deep-1..N  (Sonnet) — contextual campaigns (type safety, code understanding)
  └── architect-1 (Opus)  — judgment campaigns (TODOs, architectural decisions)

Shared TaskList: each campaign = one task
Agents self-assign next campaign when done
Escalations flow up: sweep → deep → architect
```

## Quick Start

```bash
# 1. Pre-flight check
python3 .claude/hooks/pre-flight-check.py

# 2. Regenerate lint manifest
.claude/scripts/extract-lint-issues.sh

# 3. Generate campaign summary
python3 .claude/scripts/generate-campaigns.py --summary
```

Then follow the phases below.

---

## Phase 1: Pre-flight + Campaign Generation

### 1.1 Pre-flight Check
```bash
python3 .claude/hooks/pre-flight-check.py
```
- **RED build**: Stop. Fix build errors first. Quality agents can't distinguish pre-existing errors from regressions.
- **Dirty working tree**: Warn user. Recommend committing or stashing first.

### 1.2 Regenerate Lint Manifest
```bash
.claude/scripts/extract-lint-issues.sh
```
This produces `.claude/lint-manifest.json` with all current issues.

### 1.3 Generate Campaigns
```bash
python3 .claude/scripts/generate-campaigns.py --summary
```

Review the output. It shows campaigns grouped by tier with issue counts. Use this to plan team size.

For full JSON (used by task creation):
```bash
python3 .claude/scripts/generate-campaigns.py
```

---

## Phase 2: Team Spawn + Task Creation

### 2.1 Spawn Team
```
Teammate(operation: "spawnTeam", team_name: "quality-campaign", description: "Lint campaign batch")
```

### 2.2 Create Campaign Tasks

Load campaign data and create one task per campaign:

```bash
# Get campaign data as JSON
python3 .claude/scripts/generate-campaigns.py
```

For each campaign in the JSON output, create a task:
```
TaskCreate(
  subject: "Campaign: {rule_id} ({issue_count} issues, {file_count} files)",
  description: <campaign task description — see template below>,
  activeForm: "Fixing {rule_id} issues"
)
```

### Campaign Task Description Template

```
Campaign: {rule_id} ({issue_count} issues, {file_count} files)
Tier: {tier}
Fix pattern: {fix_hint}

Files:
- {filepath1}:{line1},{line2}
- {filepath2}:{line3}
...

Instructions:
1. For each file: Read, fix ALL instances of this rule, write the fix
2. Move to next file. Do NOT stop between files.
3. When done: TaskUpdate(status: "completed")
4. Check TaskList for next unassigned campaign. Claim it.
5. If a file needs deeper reasoning, note it and continue.

Issue IDs: {comma_separated_ids}
```

Use `python3 .claude/scripts/generate-campaigns.py --task-descriptions` to get pre-formatted descriptions.

### 2.3 Spawn Teammates

Determine agent counts from the campaign summary:

| Condition | Agents |
|-----------|--------|
| Mechanical campaigns > 0 | 1-3 sweep agents (Haiku) |
| Contextual/sonnet campaigns > 0 | 2-4 deep agents (Sonnet) |
| Judgment campaigns > 0 | 1 architect agent (Opus) |
| **Total cap** | **7 agents max** |

**Sizing guidance:**
- ~2 campaigns per sweep agent (they're fast)
- ~5-8 campaigns per deep agent (more reasoning per file)
- 1 architect handles all judgment campaigns

Spawn each teammate using:
```
Task(
  subagent_type: "quality-sweep",  # or "quality-deep" or "quality-architect"
  team_name: "quality-campaign",
  name: "sweep-1",  # unique name per agent
  prompt: "You are sweep-1 on the quality-campaign team.
    Check TaskGet for your assigned task and begin working.
    When done, use TaskList to find and claim the next available campaign."
)
```

### 2.4 Assign Initial Campaigns

For each spawned agent, assign their first campaign:
```
TaskUpdate(taskId: <campaign_task_id>, owner: "sweep-1")
```

Then message them to start:
```
SendMessage(type: "message", recipient: "sweep-1",
  content: "Your first campaign is assigned. Check TaskGet and begin.",
  summary: "Start first campaign")
```

Remaining campaigns stay unassigned — agents self-assign from TaskList when done.

---

## Phase 3: Monitoring

### Passive Monitoring
Messages from teammates arrive automatically. No polling needed.

**When a teammate completes a campaign:**
- They mark it completed via TaskUpdate
- They check TaskList for the next unassigned campaign at their tier
- They self-assign and continue

**When a sweep agent reports escalations:**
- Their completion message lists files that need deeper reasoning
- Create escalation tasks for deep agents:
  ```
  TaskCreate(
    subject: "Escalation: {rule_id} residuals ({count} files)",
    description: "Files escalated from sweep:\n{file_list}\nReason: {reason}"
  )
  ```
- Assign to an idle deep agent or leave unassigned for self-assignment

**When a deep agent creates architect-tier escalation:**
- They create the task directly via TaskCreate
- The architect picks it up from TaskList

### Progress Checks

Use TaskList periodically to see overall progress:
```
TaskList  → shows all campaigns with status
```

### Handling Stuck Agents

If a teammate goes idle without completing their campaign:
- Check their last message for errors
- If context window exhaustion: create a follow-up task with remaining files
- If blocked on a file: investigate and provide guidance via SendMessage

---

## Phase 4: Commit Checkpoints

Commits happen at natural boundaries to preserve progress and prevent merge issues.

### When to Commit
1. **All mechanical campaigns complete** — commit the sweep results
2. **Every ~8 contextual campaigns complete** — intermediate checkpoint
3. **All contextual campaigns complete** — commit the deep results
4. **Judgment campaign complete** — commit architect results
5. **Final** — cleanup commit

### Commit Procedure
```bash
# Stage all changes
git add -A

# Regenerate manifest to compare
.claude/scripts/extract-lint-issues.sh

# Commit with descriptive message
git commit -m "fix(lint): {tier} campaign batch — {N} issues fixed

Campaigns completed: {list}
Issues fixed: {count}
Remaining: {remaining}

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

### Between Checkpoints
Teammates continue working during commits. Since campaigns target different rules (different lines), concurrent edits to the same file are non-overlapping and merge cleanly.

---

## Phase 5: Shutdown + Cleanup

### 5.1 Verify Completion
```
TaskList  → confirm all tasks show "completed"
```

### 5.2 Final Commit
```bash
git add -A && git commit -m "fix(lint): final campaign cleanup

Co-Authored-By: Claude Opus 4.6 <noreply@anthropic.com>"
```

### 5.3 Regenerate Manifest for Final Count
```bash
.claude/scripts/extract-lint-issues.sh
python3 .claude/scripts/generate-campaigns.py --summary
```

### 5.4 Shut Down Teammates
Send shutdown requests to all active teammates:
```
SendMessage(type: "shutdown_request", recipient: "sweep-1", content: "All campaigns complete")
SendMessage(type: "shutdown_request", recipient: "deep-1", content: "All campaigns complete")
...
```

Wait for shutdown confirmations.

### 5.5 Cleanup Team
```
Teammate(operation: "cleanup")
```

### 5.6 Final Report

Report to user:
```
## Quality Campaign Complete

### Results
| Metric | Before | After | Fixed |
|--------|--------|-------|-------|
| Total issues | {before} | {after} | {fixed} |
| Mechanical | {before} | {after} | {fixed} |
| Contextual | {before} | {after} | {fixed} |
| Judgment | {before} | {after} | {fixed} |

### Campaigns
- Mechanical: {count} campaigns, {agents} agents
- Contextual: {count} campaigns, {agents} agents
- Judgment: {count} campaigns, {agents} agents

### Escalations
- Sweep → Deep: {count}
- Deep → Architect: {count}
- Architect → GitHub Issues: {count}
```

---

## Campaign Sizing Reference

### Context Window Budgets
| Agent | Model | Context | Safe Campaign Size |
|-------|-------|---------|-------------------|
| quality-sweep | Haiku | ~200K tokens | 20-40 files |
| quality-deep | Sonnet | ~200K tokens | 15-25 files |
| quality-architect | Opus | ~200K tokens | 15-25 files |

Each file read ≈ 1-3K tokens. Fix + write ≈ similar. Plus agent instructions, hooks output.

### Sub-Campaign Splitting
`generate-campaigns.py` automatically splits large rule groups:
- By module (first path after `src/app/`) if > max files
- Into chunks if a single module exceeds the limit
- Current data: largest module-rule combo is 24 files (fits in one campaign)

---

## Tier Reference

| Tier | Agent | Model | Fix Type |
|------|-------|-------|----------|
| mechanical | quality-sweep | Haiku | Pattern replacements (unused vars, `\|\|` → `??`, formatting) |
| contextual | quality-deep | Sonnet | Type safety (unsafe-*, explicit-any, type guards) |
| sonnet | quality-deep | Sonnet | Mixed rules not in tier map (deprecated, a11y, exceptions) |
| judgment | quality-architect | Opus | Strategic review (TODOs, identical functions) |

---

## Existing Infrastructure (Preserved)

These tools continue to work alongside the team system:

| Tool | Purpose | Usage |
|------|---------|-------|
| `extract-lint-issues.sh` | Regenerate manifest | Before campaigns, at checkpoints. Use `--project` for per-project extraction |
| `generate-campaigns.py` | Group issues into campaigns | Phase 1 |
| `lint-orchestrator.py` | Status tracking, escalation queries | `show-status`, `get-escalations` |
| `pre-flight-check.py` | Build health validation | Phase 1 |
| PostToolUse hooks | Auto-lint on every edit | Continuous (automatic) |

---

## Multi-Project Campaigns

The quality system supports all four projects. Use `--project` to scope campaigns.

### Per-Project Commands

| Project | Extract | Campaigns | Lint Command |
|---------|---------|-----------|-------------|
| elohim-app | `.claude/scripts/extract-lint-issues.sh` | `--project elohim-app` | `npx eslint src --ext .ts,.html` |
| doorway | `.claude/scripts/extract-lint-issues.sh --project doorway` | `--project doorway` | `RUSTFLAGS="" cargo clippy` |
| doorway-app | `.claude/scripts/extract-lint-issues.sh --project doorway-app` | `--project doorway-app` | `npx eslint src --ext .ts,.html` |
| sophia | `.claude/scripts/extract-lint-issues.sh --project sophia` | `--project sophia` | `pnpm lint` |
| all | `.claude/scripts/extract-lint-issues.sh --project all` | (no filter) | — |

### Project-Specific Caveats

**Doorway (Rust):**
- Always use `RUSTFLAGS=""` — system RUSTFLAGS break native cargo builds
- Clippy issues map to `clippy::*` ruleIds in the manifest
- Rust files use module extraction: `doorway/src/<module>/`

**Sophia (React):**
- sophia is a git submodule — changes need separate commits inside `sophia/`
- Test files use `.test.ts`/`.test.tsx` (not `.spec.ts`)
- Uses pnpm monorepo — packages extracted as `packages/<name>/`

**Doorway-App (Angular):**
- Same Angular patterns as elohim-app
- Smaller codebase — typically fits in fewer campaigns

### Multi-Project Workflow

```bash
# 1. Extract from all projects
.claude/scripts/extract-lint-issues.sh --project all

# 2. Review per-project summary
python3 .claude/scripts/generate-campaigns.py --summary

# 3. Run one project at a time
python3 .claude/scripts/generate-campaigns.py --project doorway --summary
python3 .claude/scripts/generate-campaigns.py --project sophia --tier mechanical --summary

# 4. Or run all together — campaigns auto-sort by tier
python3 .claude/scripts/generate-campaigns.py --task-descriptions
```

---

## Partial Session Support

If you need to run a partial session (e.g., just mechanical fixes):

```bash
# Generate only mechanical campaigns
python3 .claude/scripts/generate-campaigns.py --tier mechanical --summary

# Spawn team with only sweep agents
# Create only mechanical campaign tasks
# Skip deep/architect entirely
```

Similarly for targeted module or project work:
```bash
# Filter by project
python3 .claude/scripts/generate-campaigns.py --project doorway --tier mechanical

# Filter campaigns by examining the JSON output
python3 .claude/scripts/generate-campaigns.py | \
  python3 -c "import sys,json; [print(json.dumps(c)) for c in json.load(sys.stdin) if 'lamad' in str(c.get('files',{}))]"
```

---

## Cost Optimization

| Work Type | Model | Cost Tier |
|-----------|-------|-----------|
| Campaign generation | Scripts (free) | None |
| Mechanical fixes | Haiku | Lowest |
| Type safety fixes | Sonnet | Medium |
| Strategic judgment | Opus | Highest |
| Orchestration | Main conversation | Opus (minimal — passive monitoring) |

The team lead (main conversation) does minimal work after Phase 2 — just handling escalations and commits. Most token spend is in the agents doing actual fixes.
