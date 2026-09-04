---
id: "backlog-2026-09-04-rung5-c3-mesh-findings"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Rung-5 c3 overnight shift — a2o/CI findings from the long-lived-channel run on the holochain-0.7 household mesh (2026-09-04)"
slug: "2026-09-04-rung5-c3-mesh-findings"
written: "2026-09-04"
author: "shift 2026-09-04T05-20-rung5-long-lived-channel-0-7-mesh"
status: "open"
priority: "medium"
jobs: [genesis]
tags: [a2o, ci-quality, release-ceremony, holochain-0.7, household-mesh, rung-5, codex-claimable]
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "backlog-task-release-channel-ceremony-driver"
cites:
  - genesis/a2o/steps/delivery/runtime-upgrade-propagation.steps.ts
  - genesis/a2o/scripts/release-ceremony.ts
  - elohim/rakia/schemas/v1/release-manifest.schema.json
---

# c3 shift findings — a2o rig + CI quality, not a substrate defect

Measured overnight 2026-09-04 on the local household mesh (holochain-0.7, 3 stock conductors),
during the long-lived-channel rung-5 exercise (journal gitignored; evidence under
`genesis/a2o/reports/release-ceremony/2026-09-04/`). Each item is a separate, bounded a2o/CI fix;
none touches the DNA or the coordinator zomes.

## 1. `runtime-upgrade-propagation.steps.ts` sleeps 315 s twice per run — a2o-only, bounded

Station 6 sleeps `INSTALLED_REALITY_TTL_SECS` (300) + 15 s buffer waiting for the
installed-reality snapshot to age out, and the post-run teardown repeats the same 315 s wait —
~10 min of pure sleep per run (measured: Station 6 alone was 429 s of a run, 315 s of which was
this sleep, in run `shift-it1-20260904T061043Z`). Unnecessary since storage commit `6ae703bd2`: a
node's own `apply` now invalidates its own installed-reality snapshot immediately, so the fixture
no longer needs to wait out the TTL to see a fresh read. **Fix:** remove both sleeps, re-measure
Station 6 and the teardown against the invalidation path.

## 2. `release-ceremony.ts channel create` authors an empty root before validating the id — a2o-only, bounded

The release-manifest schema constrains a channel id to
`^runtime:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*:[a-z0-9][a-z0-9-]*$`
(`elohim/rakia/schemas/v1/release-manifest.schema.json:81`) — lowercase only, no `T`/`Z` stamp
separators. `release-ceremony.ts`'s `channel create` (`warnUnlessChannelIdConvention`,
`genesis/a2o/scripts/release-ceremony.ts:346`) only **warns** on a non-conforming id; it does not
refuse, so the driver authors an empty channel root to the DHT before the packager later rejects
the same id against the schema. A stray root `runtime:coordinators:elohim:c3-20260904T062743Z`
now exists on the mesh with no releases. **Fix:** validate the channel id against the schema
pattern before the `channel create` extern call, refusing (not warning) on mismatch.

## 3. genesis/a2o gate is red on dev, independent of this shift — standing quality debt

Measured 2026-09-04 before any shift edit: `pnpm exec tsc --noEmit` in `genesis/a2o` returns 10
errors; tree-wide eslint returns 136 errors / 348 warnings. Two of the tsc/eslint errors
(`sonarjs/function-return-type` in `epr-release-package.ts`) date to that file's first commit
(`86fbf2a2d`), so this is not new drift from the rung-5 work — it is standing debt the pre-push
gate already carries. Pre-push runs `genesis/a2o`'s gate, so a push from this tree currently needs
the CI backstop rather than a clean local gate. No dedicated quality-debt cluster was found for
`genesis/a2o`; capturing here until one exists or this item graduates a fix shift.

## 4. A row never answers "has this staged candidate been attested" — controller observability gap

With a candidate staged BENEATH an earned head (story Station 9; controller `86445bd08`), no `/admin/adoption`
row carries the candidate's attestation evidence: apply-mode rows resolve the earned WINNER and take the
idempotence exit (no threshold read), and the canary's row — which does resolve the candidate — goes quiet the
moment it has applied it (same exit, zero conductor calls), so its `attestations` stays `null` even after its own
controller authored the soak attestation (james, 09:31:36Z, run `shift-it6-20260904T091619Z`). On a fresh channel
Station 4 was covered only as a side effect: the candidate IS the winner there, and matthew's waiting row verifies
it every sweep. The fixture now reads the evidence by cid through the conductor (`release-ceremony.ts attestations
<cid>`, `312d4f05d`, mirroring `release_attestation::classify/tally`). Cure candidates: (a) the canary's row keeps a
`candidateEvidence` block refreshed on each sweep while its applied release is a staging candidate (one bounded
threshold read per sweep); (b) a storage route `GET /admin/adoption/evidence/{releaseCid}` composing the same read.
Storage-only, coordinator-hash-neutral. Source: shift 2026-09-04T05-20-rung5-long-lived-channel-0-7-mesh, iteration 6.
