---
id: "backlog-archetype-resource-conformance-validation-gap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "No validation keeps deployments.json ↔ device-archetypes ↔ manifests in resource-sync — the archetype k8s budget isn't even machine-readable, and adam's sole explicit manifest is the drift seam (proven by the adam-saturation incident)"
slug: "archetype-resource-conformance-validation-gap"
written: "2026-07-07"
author: "pipeline-shakeout shift (operator root-cause question)"
status: "proposed"
priority: "high"
area: "recovery/k8s-deploy-validation"
domain: "operator"
tags: [deployments, device-archetypes, validation, drift, resource-conformance, adam, sync-gate, ungated-validator]
cites:
  - genesis/data/timeline/backlog/adam-genesis-anchor-sustained-saturation-post-storm.md
  - genesis/data/timeline/backlog/seeder-validate-deployments-stale-validator-plus-human-gap.md
  - genesis/orchestrator/data/deployments.json
  - genesis/plans/2026-04-13-device-archetypes-design.md
  - genesis/seeder/src/validate-deployments.ts
  - genesis/a2o/features/deployment/human-device-mapping.feature
---

# Archetype ↔ deployment ↔ manifest resource-conformance is unvalidated — the adam under-provisioning was drift, not a mismatch

## The question (operator, 2026-07-07)

After the adam CPU under-provisioning (`adam-genesis-anchor-sustained-saturation-post-storm.md`):
was this a deployment *mistake*, or *drift* — and what validation keeps deployments.json,
manifests, and device-archetypes in sync? Verdict below: **drift from a validation/sync gap, NOT an
archetype mismatch.** adam and matthew are provably the same archetype (`device-family-node-base`).

## Why the gap is deeper than "a missing gate"

1. **The archetype k8s resource budget does not exist machine-readably.** `device-archetypes-design.md`
   defines only PHYSICAL hardware specs per archetype (`memory_gb`, `cpu_cores`, GPU, lifespan) — never a
   k8s request/limit. `genesis/data/devices/devices.schema.json` likewise has only physical-hardware fields.
   The "operational envelope derivation" (a function hardware-spec → protocol/resource params) is listed in
   the design doc's Implementation Path as **not-yet-built**. So the only "contract" is unenforced PROSE in
   `deployments.json`'s top `$comment` ("edgenode* fields are THE source of truth … tied to a deviceArchetype
   … the archetype's validated minimum") pointing at a doc that doesn't define the numbers. matthew's comment
   even *invents* a floor ("device-family-node-base is 1Gi/4Gi + 500m/2000m") that appears nowhere else —
   self-asserted, per-entry, unshared.

2. **The validation that exists is dead.**
   - `genesis/seeder/src/validate-deployments.ts` (`pnpm run validate:deployments`) checks existence/enum/
     pattern/sed-hazards — but NOT resource-value conformance — and is **not wired** into `.husky/pre-push`
     or either Jenkinsfile. Already flagged stale+ungated by `seeder-validate-deployments-stale-validator-
     plus-human-gap.md` (2026-06-15).
   - `human-device-mapping.feature` has a scenario literally named *"Pod resources fit within the device
     archetype envelope"* — but it's `@wip` (a2o runs `not @wip`, so it never executes) AND scoped to the
     retired `legacy` pattern (every human is `consolidated` now). Doubly dead.

3. **adam being the ONLY explicit-`manifest:` human is the structural drift seam.** All 13 other humans use
   `template:` → their resources flow deterministically from `deployments.json` `edgenode{Cpu,Memory}{Request,
   Limit}` via the Jenkinsfile sed into `_edgenode-consolidated.template.yaml`. adam uses
   `manifest: adam-firstman.yaml` (a "historical reference impl") with resources HARDCODED in the YAML, and
   its deployments.json record has **no edgenode\* fields at all** — so the declared source-of-truth budget
   has no mechanical path into adam. Two disconnected copies, no gate comparing them.

## Proof it was drift (not a point-in-time mistake)

Commit `4bc407072` (2026-06-15) bumped matthew's `edgenodeCpuLimit` 3000m→4000m + `edgenodeDbPoolSize` 10→20
in deployments.json + template — `git show 4bc407072 --name-only` shows it **never touched adam-firstman.yaml**
(and git log confirms adam's manifest was untouched 2026-05-27→2026-06-24). The bump was correctly applied to
the source of truth; it simply had no path to adam, and nothing noticed adam falling ~3 weeks behind — until it
saturated under load. No one mis-edited; the system lacked the conformance gate.

## The fix — three layers (small → structural)

1. **Wire + extend the gate (bounded, high-value).** Add `validate:deployments` to `.husky/pre-push` +
   the deploy pipeline (closes the ungated half of the 2026-06-15 backlog), and EXTEND it with a
   resource-conformance check: for every human, compute *effective* resources (template humans: from
   edgenode\* fields; explicit-manifest humans: parse the YAML's `resources:` block) and assert they meet
   the archetype's declared budget floor and don't diverge from archetype-siblings. Fails the push on drift.
2. **Make the archetype budget a real contract.** Add a k8s-resource-budget (request/limit floor per
   archetype) to the device/archetype schema — or build the design doc's own not-yet-built "operational
   envelope derivation" (hardware-spec → k8s budget). Removes the ad-hoc prose/self-asserted floors so the
   gate in (1) has something authoritative to check against.
3. **Retire adam's special-case manifest (deepest, removes the seam).** Fold adam onto the shared template
   (deployments.json already calls adam-firstman.yaml "the historical reference impl"); express any
   anchor-specific bits (peer-policy, genesis self-heal, ALLOW_DNA_REINSTALL) as template hooks/placeholders.
   Then EVERY node — anchors included — is single-source-of-truth driven, scaling is one deployments.json
   edit, and the gate in (1) covers all uniformly with no exception to drift.

## Vision fit

This is the missing third leg of the operator's infra-tracking vision. The **scale-stories-up/down** levers
already exist (deployments.json human-activate/suspend = coordination-ladder cast; `target_arc_factor`;
scope-reconcile). What's absent is the **coherence validation** that keeps archetype ↔ deployment ↔ manifest
in sync as those levers move. See also [[k8s_is_not_the_architecture]] — this lives entirely in the
compute/hardware-modeling layer (deployments.json/manifests/archetypes), not the protocol plane.
