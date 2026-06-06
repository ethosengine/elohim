---
id: "backlog-ci-genesis-projectionspec-ts2739"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis Validate Constants fails — projections-substrate.test.ts ProjectionSpec literal missing routeClaims/redirectTemplates (TS2739)"
slug: "ci-genesis-projectionspec-ts2739"
written: "2026-06-06"
author: "ci-failure-triage"
status: "wip"
priority: "high"
ci_status: in-progress
fingerprints: [0a93d2d79477, 9f60eb44561d]
jobs: [elohim-genesis]
relatedNodeIds: []
tags: [ci, elohim-genesis, typescript, ts2739, validate-constants, seeder, already-fixed]
cites:
  - https://jenkins.ethosengine.com/job/elohim-genesis/job/dev/1101/
  - genesis/seeder/src/__tests__/integration/projections-substrate.test.ts
  - genesis/seeder/src/seed-projections.ts
---

# Genesis `Validate Constants` aborts — test ProjectionSpec literal missing two required fields

## The failure

`elohim-genesis` build #1101, stage **Validate Constants** (TypeScript
type-check, the gate that runs before any seeding):

```
src/__tests__/integration/projections-substrate.test.ts(127,13): error TS2739:
  Type '{ stewardHumanId: string; stewardArchetype: "desktop"; doorwayId: string;
  eprId: string; urlPath: string; mode: "cached"; reach: string; baseHref: string;
  entryFile: string; redirectsFrom: never[]; previewEprRef: null; gateHints: never[];
  deadEnd: false; stewardDirectEndpoint: null; }' is missing the following
  properties from type 'ProjectionSpec': routeClaims, redirectTemplates
❌ TypeScript type errors found — fix before seeding
```

Build result FAILURE (this is the ONE genesis FAILURE in the window — 1091–1100
are all UNSTABLE; the type-check gate `exit 1`s, the catchError-wrapped E2E
stages never run). Fingerprint `9f60eb44561d` ("❌ GENESIS PIPELINE FAILED") is
this same build's terminal banner — **same concern, not a second one**.

Occurrence evidence: `0a93d2d79477` seen 1 (first=last=1101); `9f60eb44561d`
seen 1 (first=last=1101).

## Verdict

**real — a TypeScript type-completeness error**, already fixed on `dev`. Not a
flake, not infra. The `ProjectionSpec` interface gained two required fields
(`routeClaims: RouteClaimGrant | null`, `redirectTemplates: RedirectTemplate[]`,
`seed-projections.ts:93–94`) in the spec §3.2/§4 routeClaims work
(`dc72b333f`/`3777bd185`); a test-fixture object literal in
`projections-substrate.test.ts:127` was not updated in lockstep, so the literal
no longer satisfies the interface.

## Root cause

Build #1101 was built from commit `31779cb2e`
(`feat(omni-resilience): tooltip folds down …`). At that commit the test literal
at lines 127–144 lacked `routeClaims`/`redirectTemplates` (verified via
`git show 31779cb2e:…/projections-substrate.test.ts`). The interface widened in
the routeClaims grant work; the test object was the lone unmigrated consumer.
Standard "interface gained a required field, one literal lagged" drift — caught
exactly where it should be (the pre-seed type-check gate), not at runtime.

## Current decision

**Already fixed on `dev` (commit `39c3c8b6b`), landed AFTER the harvest captured
build #1101 — awaiting CI disappearance confirmation.** The fix added the two
missing fields to the literal:

```
+        routeClaims: null,
+        redirectTemplates: [],
```

`39c3c8b6b` ("fix(seeder): unblock genesis Install Seeder — integration-spec
ProjectionSpec fields + connectedPeers camelCase", 2026-06-06 16:06Z) is an
ancestor of `dev` HEAD and a descendant of build #1101's commit `31779cb2e`
(`git merge-base --is-ancestor 31779cb2e 39c3c8b6b` → true). No new code change
needed from triage. The ledger stamp (`triaged_at_build: 1101`) lets the
harvester confirm by green streak once a genesis build > 1101 runs the Validate
Constants stage clean.

Note: a genesis build > 1101 will still go UNSTABLE while the alpha substrate is
degraded (see `ci-alpha-cluster-degraded-substrate.md`) — but it will pass
Validate Constants and reach E2E, so this TS2739 fingerprint disappears
independently of the substrate condition. The two concerns close on different
signals: this one on Validate-Constants-clean, the other on substrate-stable.

## Fix trail

- Fix commit (pre-existing on `dev`): `39c3c8b6b`.
- `genesis/seeder/src/__tests__/integration/projections-substrate.test.ts:138–139`
  — added `routeClaims: null` + `redirectTemplates: []` to the gated-reach spec
  literal.
- Verification: the file at `dev` HEAD now type-checks (literal satisfies
  `ProjectionSpec`); at build commit `31779cb2e` it does not. No integrator
  action needed beyond the next genesis run, which confirms by the stage going
  green.
