---
name: deployments.json is the single source of truth for "is this human exercise-able"
description: When a node fails (shem 2026-05-04), suspending humans in deployments.json gates BOTH deploy and seed; seeder + a2o framework + isHumanDeployed all read the same flag
type: project
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
`genesis/orchestrator/data/deployments.json` is the per-human deployment registry. Each human entry has `nodeTypes: [...]` (which k8s node-type labels their conductor StatefulSet targets) and an optional `suspended: true` flag with a `$suspendedComment` explaining why.

When a node fails catastrophically (canonical case: shem decommissioned 2026-05-04, PSU failure, PVCs lost), humans pinned to that node-type get `suspended: true`. Three code paths must respect this flag for the system to behave correctly:

1. **Deploy** — orchestrator/Jenkinsfile already skips suspended humans when rendering manifests.
2. **Seed** — `genesis/seeder/src/seed-humans.ts` filters via `loadSuspendedNames()` reading deployments.json (added 2026-05-05). Without this, doorway's node/device registration branches hit Pending pods and cascade 502/503/WebSocket-closed errors.
3. **Test** — `genesis/a2o/src/framework/fixtures/humans.ts:234` `isHumanDeployed()` reads the same file; step definitions return `'pending'` for suspended humans, auto-skipping scenarios.

**Why:** Three layers gating on one flag means re-enabling humans (when shem returns or replacement remote hardware comes online) is a single deployments.json flip — no code change. The flag is the architectural lever; code that mirrors the a2o pattern is correct, code that bypasses it cascades.

**How to apply:** When adding a new code path that exercises humans (a new seeder, a new test runner, a new health check), check whether it should respect deployments.json suspended state. If it talks to a per-human conductor pod (node/device branches), it MUST. If it only goes through doorway's hosted-pool path (which uses matthew/jessica/terrance regardless), it can skip the check. The pattern: `loadSuspendedNames()` returning a `Set<string>`, fail-open if file unreadable.

Companion to `project_compute_commitments_bounded.md` and `compute-commitment-bounds.feature` — the suspension flag is how the bounded-compute design surfaces operationally.
