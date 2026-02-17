---
name: pipeline-diagnostics
description: Diagnose CI/CD pipeline failures, analyze Jenkins build logs, and troubleshoot deployment issues. Use when builds fail, checking pipeline status, investigating errors, or understanding deployment health.
---

# Pipeline Diagnostics

This skill helps diagnose CI/CD pipeline issues for the Elohim project using the Jenkins MCP integration.

## Pipeline Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     Elohim CI/CD Orchestration                    │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  GitHub Push                                                      │
│      ↓                                                            │
│  Orchestrator  ←── Analyzes changesets, triggers dependencies     │
│      ↓                                                            │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │  Parallel Builds (if dependencies met):                     │ │
│  │                                                              │ │
│  │  elohim-holochain  →  DNA builds, hApp packaging            │ │
│  │  elohim-edge       →  Doorway, storage containers           │ │
│  │  elohim-app        →  Angular build                         │ │
│  └─────────────────────────────────────────────────────────────┘ │
│      ↓                                                            │
│  elohim-genesis  ←── Seed validation & deployment                 │
│      ↓                                                            │
│  Health Checks  ←── Post-deployment verification                  │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

## Jenkins Job Reference

| Job Name | Purpose | Triggers |
|----------|---------|----------|
| `elohim-orchestrator` | Changeset analysis, pipeline coordination | GitHub webhook |
| `elohim-holochain` | Rust DNA compilation, hApp packaging | Orchestrator |
| `elohim-edge` | Docker containers for doorway, storage | Orchestrator |
| `elohim-app` | Angular build, static assets | Orchestrator |
| `elohim-genesis` | Content seeding, verification | After app/edge |
| `elohim-steward` | Tauri desktop app (manual) | Manual only |

## Using MCP Tools for Diagnostics

### Check Jenkins Status
```
Use mcp__jenkins__getStatus to check Jenkins health
```

### Get Job Information
```
Use mcp__jenkins__getJob with jobFullName="elohim-holochain"
Use mcp__jenkins__getJob with jobFullName="elohim-app"
Use mcp__jenkins__getJob with jobFullName="elohim-genesis"
```

### Get Build Details
```
Use mcp__jenkins__getBuild with jobFullName="elohim-holochain"
  (omit buildNumber for latest)

Use mcp__jenkins__getBuild with jobFullName="elohim-holochain" buildNumber=123
  (specific build)
```

### Analyze Build Logs
```
Use mcp__jenkins__getBuildLog with jobFullName="elohim-holochain"
  limit=-100  (last 100 lines)

Use mcp__jenkins__searchBuildLog with:
  jobFullName="elohim-holochain"
  pattern="error|failed|Error"
  ignoreCase=true
  contextLines=3
```

### Check Test Results
```
Use mcp__jenkins__getTestResults with jobFullName="elohim-app"
  onlyFailingTests=true
```

## Common Failure Patterns

### DNA Build Failures

**Pattern: WASM compilation error**
```
Search logs for: "error\[E" or "cannot find" or "unresolved"
```

**Common causes:**
- Missing RUSTFLAGS for getrandom backend
- Incompatible dependency versions
- Syntax errors in zome code

**Fix checklist:**
1. Check `RUSTFLAGS='--cfg getrandom_backend="custom"'` is set
2. Verify `Cargo.lock` is committed
3. Check zome source for compile errors

### Angular Build Failures

**Pattern: TypeScript errors**
```
Search logs for: "error TS" or "Cannot find module"
```

**Common causes:**
- Type mismatches after model changes
- Missing imports
- Circular dependencies

**Fix checklist:**
1. Run `npm run build` locally
2. Check for type sync between elohim-service and elohim-app
3. Verify all imports resolve

### Seeding Failures

**Pattern: Connection timeout**
```
Search logs for: "ETIMEDOUT" or "WebSocket" or "connection refused"
```

**Common causes:**
- Doorway not ready
- Wrong admin URL
- Network policy blocking

**Fix checklist:**
1. Check doorway health endpoint
2. Verify HOLOCHAIN_ADMIN_URL environment variable
3. Check K8s pod status

**Pattern: Schema validation**
```
Search logs for: "missing required" or "validation failed"
```

**Fix:**
1. Run `npm run validate` in genesis/seeder
2. Check content files for missing id/title fields

### Docker Build Failures

**Pattern: Image build error**
```
Search logs for: "COPY failed" or "RUN failed" or "denied"
```

**Common causes:**
- Missing build artifacts from previous stage
- Harbor registry auth issues
- Dockerfile syntax

## Environment Mapping

| Branch Pattern | Environment | Doorway URL |
|---------------|-------------|-------------|
| `dev`, `feat-*`, `claude-*` | Alpha | doorway-alpha.elohim.host |
| `staging-*` | Staging | doorway-staging.elohim.host |
| `main` | Production | doorway.elohim.host |

## Diagnostic Workflow

### 1. Identify Failed Build
```
Use mcp__jenkins__getJobs to list recent jobs
Use mcp__jenkins__getBuild to get failure details
```

### 2. Get Error Context
```
Use mcp__jenkins__searchBuildLog with pattern matching:
- "error" (case insensitive)
- "failed"
- "Exception"
- "panic"
```

### 3. Analyze Stage
Look at the stage name to determine which pipeline component failed:
- "Build DNAs" → Rust/WASM issues
- "Build App" → Angular/TypeScript issues
- "Seed Content" → Doorway/connection issues
- "Deploy" → K8s/Docker issues

### 4. Check Dependencies
```
Use mcp__jenkins__getBuildChangeSets to see what changed
Use mcp__jenkins__getBuildScm for commit info
```

## Triggering Builds

### Retry Failed Build
```
Use mcp__jenkins__triggerBuild with jobFullName="elohim-holochain"
```

### Trigger with Parameters
```
Use mcp__jenkins__triggerBuild with:
  jobFullName="elohim-genesis"
  parameters={"SKIP_SEEDING": "false", "ENVIRONMENT": "dev"}
```

## Key Jenkinsfile Locations

| File | Purpose |
|------|---------|
| `/projects/elohim/Jenkinsfile` | Root orchestrator |
| `/projects/elohim/orchestrator/Jenkinsfile` | Pipeline controller |
| `/projects/elohim/holochain/Jenkinsfile` | DNA/hApp builds |
| `/projects/elohim/genesis/Jenkinsfile` | Seeding pipeline |
| `/projects/elohim/steward/Jenkinsfile` | Desktop app |

## Artifact Flow

```
elohim-holochain
    ↓ elohim.happ
elohim-edge
    ↓ doorway:tag, storage:tag
elohim-app
    ↓ dist/elohim-app
elohim-genesis
    ↓ seed verification
```

Each pipeline fetches artifacts from upstream jobs. Check artifact availability if builds fail at fetch stages.

## Quick Diagnostics Commands

### Check all pipeline health
```
1. mcp__jenkins__getStatus (overall Jenkins)
2. mcp__jenkins__getJobs (list jobs)
3. For each job: mcp__jenkins__getBuild (latest status)
```

### Investigate specific failure
```
1. mcp__jenkins__getBuild with jobFullName + buildNumber
2. mcp__jenkins__getBuildLog with limit=-200 (tail)
3. mcp__jenkins__searchBuildLog with error patterns
4. mcp__jenkins__getTestResults if tests failed
```

### Check deployment health
```
After genesis completes, verify via:
- stats:dev / stats:prod commands
- Doorway health endpoints
- Application smoke tests
```
