---
index: false
name: feedback-partition-compile-and-stale-dist
title: Commit partitions respect compile deps
description: Two integration anti-patterns from 2026-07-24 overnight — commit partitions must respect COMPILE deps, and local dist/ presence proves nothing about CI stage coverage
metadata:
  type: feedback
---

Two CI-break classes from one overnight integration (2026-07-24):

1. **REVIEW-PARTITION-COMPILE-COUPLING** — a reviewed commit partition verified for *runtime* deploy-order independence still broke the build: pushed doorway code consumed `infrastructure_types::DoorwayEndpoint` from the sdk types file held back in the deferred DNA commit (edge E0425, 15 errors).
2. **LOCAL-GREEN-VIA-STALE-ARTIFACT** — full local AOT `ng build` passed for both apps because `elohim-qahal/dist/` existed from earlier element work; CI's Build Elohim Core stage built only core+imagodei, so CI failed on `Could not resolve 'elohim-qahal/register'`.

**Why:** partitions and local greens both *look* verified; the missing check is the build graph — compile-time deps across commit groups, and CI-stage build coverage for newly-imported workspace packages.

**How to apply:** before splitting a batch into ordered pushes, walk the compile deps of each group against what's already public (not just runtime fallbacks). When a change adds the FIRST import of a workspace package to an app, grep the Jenkinsfile stage list for that package's build before trusting any local green ([[feedback_swarm_composition_fresh_tree_build]] is the cargo-side sibling).
