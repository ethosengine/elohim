---
name: project-steward-publishing-and-device-ceremony
title: "Steward publishing & device ceremony (umbrella)"
id: project-steward-publishing-and-device-ceremony
description: "Publish from the steward's own device via root-author delegation; Che = live alpha lineage, pipeline packs differ; device-join proof recipe (i1006 approves, sandbox asks); reach widening is a steward act."
metadata:
  node_type: memory
  type: project
  originSessionId: 08597c1b-3ceb-4809-8b33-55c0b88bb4d8
  modified: 2026-10-08T01:00:00.000Z
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/head_delegation.rs
  - "one-command-device-join-design | One command on the new device, one yes on a device that is already yours | sha256:f0cebf3702df05b1 | path: genesis/docs/superpowers/specs/2026-10-07-one-command-device-join-design.md"
---

# Steward publishing & device ceremony (umbrella)

Folds the campaign to master the steward's own agency over content — publishing from
their own runtime, the lineage and grantor facts that gate it, and the live device-join
proof. Members (each carried in full in its own file):

- [[project-steward-agency-publish-from-own-device]] — Operator 2026-09-29: publish via root-author delegation from his own conductor; pipeline grading is interim; ruling 2026-10-02: operator runtime first, devices under it.
- [[project_campaign14_leg2_lineage_and_root_author]] — Che carries the live alpha DNA lineage; CI and worktree packs differ; no non-Che Matthew root on alpha, so the delegation had no grantor until the operator root of 2026-10-03.
- [[project-device-join-live-proof-recipe]] — how `epr device join` / `approve` was proven live 2026-10-07 (i1006 approves, a generated sandbox asks); stale coordinators and no-network are the two non-code failures.
- 2026-10-08 finding: 220 of 256 FCT atoms on alpha sit at `private` reach from an older seeder's default while the repo declares `commons`; the seeder leaves rows it cannot read alone (`unverified`), and widening is the steward's act (steward-grade / steward-publish), never a pipeline leg — see the dataplane-convergence habit delta of that date.
