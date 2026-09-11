---
name: project_ci_build_infra
title: CI build infra — images, caches, registries (umbrella)
id: project-ci-build-infra
description: "CI substrate: ci-playwright on ci-builder; cache PVCs node-pinned; devspace node22 vs CI24 is structural; pull policy Always; Nexus needs read token."
metadata:
  node_type: memory
  type: project
---

# CI build infra — images, caches, registries (umbrella)

Folds the CI build-infrastructure cluster — images, cache volumes, registries, toolchain skew. Members:

- [[project_ci_playwright_image]] — ci-playwright is a thin layer on ci-builder (node20+pnpm+CHROME_BIN) bundling playwright@1.59.1; root launch needs no --no-sandbox (auto-disabled in containers)
- [[project_ci_storage_topology]] — CI cache PVCs (nix/cargo/sweettest-target) are openebs-hostpath in jenkins ns; pin with kubernetes.io/hostname nodeAffinity or pods thrash on volume binding.
- [[project_devspace_node_ceiling_ubi10]] — udi-plus lineage gets node from UBI10 appstream (only 22.23.1, no 24 stream) — CI's ci-builder is node:24-bookworm, so dev/CI skew is structural, not drift.
- [[feedback_ci_pull_policy_always_freshness]] — CI pods keep imagePullPolicy Always: IfNotPresent on :latest buries toolchain drift (#1218); outage mitigations revert at recovery; digest-pin for permanence.
- [[project_brit_rakia_nexus_ci]] — rakia+brit crates publish to Nexus cargo-internal; hosted repo is auth-required so cargo needs a read token; committed paths override hard-errors in CI.
- [[project_serde_wall_escapable_via_hsb_057]] — Dep-advisory remediation is gated by TWO separate things — Nexus can't fetch uncached artifacts, and holo_hash =0.6.0 pins serde =1.0.219; don't conflate them.

**2026-08-25 — `readFile('/tmp/…')` after a container `sh` reads the WRONG filesystem.** In the kubernetes
agent, `sh` inside `container('…')` runs in that container, but `readFile` runs in the jnlp container — an
absolute `/tmp` path written by `sh` does not exist there (NoSuchFile → whatever `catch` wraps it). genesis
`resolveSeedDoorwayToken()` hid this behind a "credential not visible" echo for weeks; the seed upload ran
unauthenticated and only passed via the doorway dev_mode hole until 62b658784 closed it (genesis #1503 403 →
fixed 47fb60f58 with `sh(returnStdout:true)`). Rule: read container-side files back with `sh 'cat …'`, or write
them under `${WORKSPACE}` (shared volume) — never `readFile` an absolute `/tmp` path.
- **App builder agent needs a memory request (2026-09-04):** the root Jenkinsfile's builder pod had only ephemeral-storage
  resources, so the scheduler put the Angular build on a 7.6 GB ThinkPad (node-type: edge includes the dqlite voters
  .110/.111/.112); the node hit 0.13 GB free ~3 min into `Build App`, the JNLP channel closed and the controller's exec
  fallback got "Expected HTTP 101 but was 500" ×5 — reads as a control-plane fault but is node OOM. Fixed with
  requests 6Gi / limits 10Gi. Rule: any CI agent running a multi-GB build declares memory, or the ThinkPads eat it.

**2026-09-06 — `elohim/rakia` is PRIVATE to the Jenkins agent.** `git submodule update --init -- elohim/rakia` in the edge
Checkout stage died with `could not read Username for https://github.com` (edge #1433 → orchestrator #1821 FAILURE, no
deploy). An agent's "anonymous `ls-remote` works" from the devspace proves nothing about CI's network — the devspace holds
credential helpers. Rule: any CI fetch of an ethosengine repo other than the main checkout rides the `ee-bot-pat`
credential (process-local `git -c url.…insteadOf` in a `scripts/ci/*.sh`, token via withCredentials env, never argv);
and a step that only feeds an ADVISORY test (the rakia schema mirror) must be warn-only — print `RAKIA-UNAVAILABLE` and
exit 0 — so it can never take the deploy path down.

- [[feedback_sccache_failure_classes]] — folded (index: false); three distinct sccache failures, not one: cache corruption (null-byte / unclosed-delimiter in a cached object), spawn ENOENT, and AccessDenied against a dead Garage key — the last turns the DNA pipeline red in ~85s.
