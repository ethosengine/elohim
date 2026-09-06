---
id: "backlog-runtime-steward-adoption-of-fleet-cut-release"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Steward adoption of a release cut FOR the fleet — publish-over-earned wedges the workspace"
slug: "runtime-steward-adoption-of-fleet-cut-release"
written: "2026-09-06"
author: "story-harvest"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "genesis/a2o/features/delivery/workspace-to-fleet-release.feature"
  - "elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md"
tags: [runtime-upgrade-propagation, release-ceremony, packager, steward, adoption]
---

**Chain:** runtime-upgrade-propagation / between "a canary applies + attests" → "the steward promotes and publishes the next release" / **missing node:** the steward's own peer can ADOPT a release that was cut for a fleet it does not match.

**Assertion.** `release-ceremony.ts publish` over an EARNED head refuses `not_adopted` unless the acting peer's `GET /admin/adoption` reports `appliedRelease == earned head` ("a steward cannot push what they have not themselves adopted"). A release cut FOR the fleet (`--applies-to-from-adoption`, 2026-09-06) binds roles to the FLEET's running coordinators; the steward's workspace peer runs the builder's (mishpat differed on 2026-09-06), so its own controller refuses the release `coordinator_lineage_mismatch` and — in `observe` — records no `appliedRelease` even when it already runs the target bytes (`already_runs_target` → `Observed` → `Ok`, not `Applied`). Promote it8 today and the workspace's NEXT publish is wedged.

**Probe.** `curl :8090/admin/adoption` on the workspace after `promote`: `appliedRelease` null → `publish` of it9 refuses `not_adopted`.

**Current state.** it8 applied + attested on james (canary); promotion deliberately not declared (no apply-mode peer to move; nothing lost). Two candidate cures, pick by design not by convenience: (a) the by-bytes exit in `canary`/`apply` mode records `appliedRelease` with `VEHICLE_ALREADY_INSTALLED` — so the steward flips its own channel to `canary` and adopts by bytes (config-epr flip, seconds; but binds "adopted" to a mode flip); (b) `publish`'s adopted check accepts "runs the target bytes" (`already_runs_target`) as adoption for a release whose `appliesTo` was cut for other peers — the honest reading of "I have adopted" is "I run it", not "my controller wrote a row". (b) is the semantic fix; it needs the ceremony to read installed reality (now on `/admin/adoption`) and compare to the release's target hashes (the bundle's, not the manifest's — `happ_manager::bundle_coordinator_wasm_hashes`).
