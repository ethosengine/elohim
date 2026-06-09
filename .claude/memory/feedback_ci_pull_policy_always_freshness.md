---
name: ci-pull-policy-always-freshness
description: Operator policy — CI tooling pods keep imagePullPolicy Always; outage mitigations on pull policy are temporary and must be reverted at recovery
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 5202e82d-a14d-4ad3-b13a-966c13b597d3
---

CI tooling pod templates (`ci-builder`/`ci-playwright`/`ci-builder-nix:latest` in the five Jenkinsfiles) run `imagePullPolicy: Always` by operator decision (2026-06-07, commit b23c86c26): "prevent successful deployments that bury the drift." The 2026-06-06 `IfNotPresent` Harbor-outage mitigation was deliberately reverted once hp-micro10 was repaired.

**Why:** `:latest` is a moving tag — `IfNotPresent` lets a cached node silently serve a stale toolchain (the documented #1218 incident shape in `elohim/holochain/dna/Jenkinsfile`), and per-node cache heterogeneity is the worst flake class. A green build on an unverified cached toolchain is buried drift.

**How to apply:** During a registry outage, `IfNotPresent` may be re-applied as an explicit, commented *incident mitigation* — but it must be reverted at recovery, never left as steady state. If outage-resilience is wanted permanently, the approved path is digest-pinning (immutable `@sha256` refs + `IfNotPresent`, bump via reviewed commit) — proposed and acknowledged but not adopted as of 2026-06-07. Deploy manifests are unaffected: app images use per-commit tags `{version}-{branch}-{git8}` + `Always` (on `main` the tag is bare baseVersion, so `Always` is load-bearing there — never weaken it). See [[concurrent-push-mutual-abort]] for push sequencing around the dispatches this policy affects.
