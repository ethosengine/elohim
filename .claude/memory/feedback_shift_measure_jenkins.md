---
name: Shift measures live in Jenkins, not locally
description: Eclipse Che dev env has no docker/holochain/k8s — shift Objective measures MUST use Jenkins MCP (triggerBuild, getBuild, getTestResults) rather than local build/test commands
type: feedback
originSessionId: 5284bee7-715e-4461-9a99-f9a170474791
---
When authoring `/shift` Objectives in this project, the `measure.run` command almost always needs to be a Jenkins MCP call, not a local shell command.

**Why:** The dev environment is Eclipse Che (cloud IDE). Docker is NOT installed. Holochain conductor is NOT running. k8s is NOT available. Any local `docker build` / `docker run` / `kubectl` / full-stack test will silently return 0 and give a misleading baseline. The readiness check's "measure.ok = true, baseline = 0" signal is worthless when the command itself can't execute meaningfully.

**How to apply:**
- Default measure shape: `mcp__jenkins__getBuild` of the relevant job, parsing `result` to a number (SUCCESS=1, FAILURE=0, etc.). Or `getTestResults` returning pass count.
- Acceptable local measures: anything that only needs Node/Rust toolchain that IS in Che (pnpm, cargo, tsx, node:test). NOT docker, NOT k8s, NOT holochain.
- Fresh-trigger stability: `mcp__jenkins__triggerBuild` produces a new build id — that's the fresh trigger. `git push` also triggers orchestrator webhook and qualifies.
- If you catch yourself proposing a `docker build` / `docker run` / `kubectl` / `hc` measure, stop — replace with the equivalent Jenkins job check.

**Reason:** Matthew called this out on 2026-04-20 after I bailed a shift at readiness because `docker` wasn't available locally. The shift loop was designed for Jenkins-as-oracle from the start; I defaulted to local because that's what I knew, and it cost an iteration. Don't repeat.
