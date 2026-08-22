---
name: project_ci_build_infra
title: CI build infra — images, caches, registries (umbrella)
description: "CI substrate: ci-playwright layers on ci-builder; cache PVCs are node-pinned hostpath; devspace node 22 vs CI 24 is structural; pull policy Always; Nexus cargo needs a read token."
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
