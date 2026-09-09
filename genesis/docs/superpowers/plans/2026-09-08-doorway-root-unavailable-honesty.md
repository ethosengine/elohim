---
id: doorway-root-unavailable-honesty
title: Doorway delivery diagnostics and recoverable local verification
status: Active
class: substrate
serves: doorway-failover
date: 2026-09-08
cites:
  - "doorway-federated-continuity-roadmap | Doorway-federated continuity | sha256:6e79cd43ecef2594 | path: genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md"
---

The delivery sweep found a healthy local household with no canonical landing app staged.
Its root returned a 503 claiming the conductor could not connect. The fallback only knows
projection counts and federation-peer counts; no connectivity observation supports that claim.

- [ ] A visitor reaching a doorway with no root projection sees that the requested site is unavailable, without an invented conductor outage; observed projection counts remain available, and the existing status page supplies deeper diagnosis.

Change the existing fallback copy/comments and its focused regressions in
`doorway/doorway-service/src/server/http.rs`. Keep routing, status, Retry-After and no-store
behavior. Add no probe, entity, route or decision surface. Verify both empty and populated
projection caches, run the owning gate, and inspect the resulting page through `pnpm look`.
A passing check narrows doorway-failover; it cannot certify the full federated CDN epic.

The implementation brief/report live in `genesis/a2o/reports/delivery-20260908/`.
Use the native claim, verdict and fulfillment verbs; do not add an active habit or require
commits in this shared worktree. Preserve in-flight mesh validation by never replacing its
binaries or restarting its peers during the run.

- [ ] A developer with a configured Jenkins URL can invoke the served-shell probe locally without package installation; explicit CI=false suppresses bootstrap, while real CI still installs the pinned browser dependencies.

Measured prerequisite: `scripts/ci/verify-served-shell.sh` treated the workspace's Jenkins
connection URL as evidence of execution inside Jenkins and attempted an unavailable apt-get.
Constrain the existing bootstrap condition to actual build context, honoring explicit CI=false.
Exercise local URL-only, explicit opt-out, and actual CI cases with a stub package manager;
retain real browser positive/negative controls in the existing served-shell regression suite.

- [ ] A household fault command refuses to stop a peer when its restart path no longer contains the executable bytes running in that peer; the refusal preserves the live process and its previous restart capture, and a compatible unchanged executable remains stoppable and recoverable.

The full deliverability run found that `resolve_exe` strips procfs's deleted suffix and
accepts a rebuilt cargo target as though it were the loaded binary. The stopped peers had
been using a dual-capable binary, but its path now held a build without iroh; cleanup refused
and downstream scenarios failed. Validate the restart candidate against the kernel-observed
executable before destructive signals and reject a transport-incompatible candidate. Cover
changed, deleted, compatible and failed-capture cases in the existing mesh script tests.
Do not introduce a new binary cache or weaken the transport guard. This bounds the observed
pre-fault hazard; concurrent replacement after the check still needs an immutable runtime
artifact and must not be claimed solved. Native gate plus scoped regression and local
refusal/recovery evidence own verification. Keep fault tests serialized on the mesh lock.

Verification: all three implementation commitments are fulfilled and independently approved in the native valueflow. The owning gates passed. After explicit user approval, all five local services were upgraded to the verified current-source binaries. The complete deliverability run passed 5/5 scenarios and 102/102 steps with no skips or failures; all outage cleanup completed. See `genesis/a2o/reports/delivery-20260908/summary.md` and run `20260909-codex-current-deliverability` (executed 2026-09-08). Whole-feature graduation still requires fleet serving and actual apex routing-through-shed evidence.
