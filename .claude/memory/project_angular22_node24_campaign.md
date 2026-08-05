---
name: project_angular22_node24_campaign
title: Angular 22 + Node 24 campaign — state and lessons
description: "Angular22+Node24 LANDED on dev 2026-07-30, wave 0-failed, alpha deployed; SSR follow-ups settled (shim sort() stall fix, trust-scoped cache); OnPush/vitest-blindness lessons."
metadata: 
  node_type: memory
  type: project
  originSessionId: d37bdfc0-4d71-497c-ab53-9452e457b095
  modified: 2026-07-30T18:34:44.363Z
---

Campaign LANDED on dev 2026-07-30 (push 9c7988542..5290e3f90, operator-authorized; orchestrator
#1571 wave: 0 failed, storybook green, elohim/edge/genesis unstable on PRE-EXISTING classes only —
elohim.host head-declare gap, docker-compose missing in P2P sim stage, alpha-cluster-6peer a2o
fixtures. Node-24 ci-builder fully green: all build/test/sonar stages). Alpha deployed + E2E passed. Four repos carry commits: monorepo
(~25), sophia `feat/node24` (Node 24 + security set incl. uuid bundle fix), che-devworkspaces
`main` (ci-builder → node:24-bookworm), brit `fix/nexus-first-party-registry-split`
(crates.io direct). Submodule pointers deliberately NOT bumped.

What landed: Angular 22.1.0 + TS 6.0.3 + `@angular/build` (build-angular dropped), zoneless
SSR (ɵ-hacks gone), animations + platform-browser-dynamic deleted, 133-net Eager stamps
removed (12 restored as OnPush-unsafe — see backlog-onpush-eager-debt-inventory), elohim-render
web_api polyfill layer (v22 FetchBackend needs AbortController etc. in the isolate), patch
re-cut @22.1.0 (defect NOT fixed upstream; re-cut every major until native vitest builder
adoption — see backlog-angular-native-vitest-builder-adoption).

Follow-ups settled 2026-07-30 (commits 4fd29a954/6a0e00f6a/f89c1c420 on the branch): SSR
stall FIXED — root cause was `URLSearchParams.sort()` missing from elohim-render's url.js
shim (Angular 22 transfer-cache interceptor throws between `pendingTasks.add()` and
`finalize` → unremovable PendingTask; every earlier interceptor/router theory was wrong).
Isolate-reuse settled: compile-enforced `DataFetcher::trust_scope()` (Ambient/Principal),
trust-gated render cache (`is_cache_shareable` lives in elohim-render, doorway = thin
adapter), default auth_modes now `["anonymous"]`; real fix = deno_core snapshot spike
(realms API verified absent at 0.339). zone.js split: NO standalone build lever — folds
into browser-zoneless migration.
Design canon: 2026-07-30-render-delivery-manifest-adapter-design.md (SSR = manifest
contract, doorway never a render dependency; .epr-meta guard enforces the framing). Still
open: jQuery 2.1.1 in sophia UMD.

**Lessons that bite again:** (1) `fixture.detectChanges()` forces a check → vitest is
structurally BLIND to OnPush subscribe-mutation staleness; suites-green proves nothing for
CD-strategy changes — eyes-on-render or signals conversion is the real gate. (2) Bulk
codemod-style edits must re-run prettier on the touched set or format:check + the
prettier/prettier eslint rule go red at a distance. (3) Node 24 toolchain for this container
lives at /home/user/.local/node-v24.18.1-linux-x64/bin (container-persistent; scratchpad
copies die with sessions); the node24 bin dir carries a corepack pnpm-11 shim that must not
write the lockfile (workspace pnpm is 10.30.3 under PNPM_HOME).
