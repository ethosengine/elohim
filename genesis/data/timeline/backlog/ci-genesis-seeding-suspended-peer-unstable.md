---
id: "backlog-ci-genesis-seeding-suspended-peer-unstable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis seed/verify stages go UNSTABLE waiting on suspended peers — getHumanStorageUrls includes suspended humans"
slug: "ci-genesis-seeding-suspended-peer-unstable"
written: "2026-06-08"
author: "agentic-developer (overnight shift)"
status: "wip"
priority: "medium"
ci_status: in-progress
jobs: [elohim-genesis]
tags: [ci, genesis, seeding, scope-reconcile, suspended, jenkinsfile, ready-to-apply]
cites:
  - genesis/Jenkinsfile
  - genesis/orchestrator/data/deployments.json
---

# Genesis seed/verify stages go UNSTABLE waiting on suspended peers

## Symptom
Two genesis SEED stages emit `unstable()` every build for a known-suspended peer:
- **Seed Database**: `WARNING: Seeded genesis peers: human-matthew-manager; skipped unreachable: human-adam-firstman — partial-cluster steady state` (Jenkinsfile `runContentSeedStage`, ~line 400).
- **Verify Seeding**: `WARNING: Verified …; skipped unreachable peers: human-adam-firstman` (Jenkinsfile `runVerifySeedingStage`, ~line 499).

adam is `suspended:true, suspendedBy:scope-reconcile:shem` in deployments.json (correctly — shem
is down). He should never be a seed/verify target, yet the seeder waits ~120s on his dead storage
then marks the stage UNSTABLE.

## Root cause
`getHumanStorageUrls('alpha')` returns **all** humans (from topology.json / the fallback hardcode),
**including suspended ones** — it does not consult deployments.json's authoritative `suspended` flag.
Both seed stages iterate that set, so adam (suspended, genesisPeer:true) is probed → timeout → UNSTABLE.

The existing remote-only-shem reconciliation (`runContentSeedStage` lines 354-361, via
`remoteOnlyHumanIds()`) does NOT hold adam, for two compounding reasons:
1. `remoteOnlyHumanRecords()` (Jenkinsfile:236) parses with `new groovy.json.JsonSlurper().parseText(readFile(path))`
   which is **CPS-unsafe** → throws → caught → returns `[]` (logged `WARN: remoteOnlyHumanRecords() parse failed`).
2. Even with a working parse, its filter is `!h.suspended && nt.every{=='remote'}` — and ALL 11
   remote-only humans ARE suspended, so it returns `[]` by design (the `!h.suspended` is intentional:
   line 226 doc — it mirrors `humans.ts isAnyNodePoolAvailable` for the reduced-scope BANNER, naming
   the not-yet-suspended set; do NOT just delete it or you break that banner/runner mirror).

## Ready-to-apply fix (Jenkinsfile — design-coherent, honors the suspension contract)
deployments.json `suspended` is the single source of truth for is-this-human-exercise-able. Honor it
directly in the seed/verify peer set, independent of the remote-only banner contract:

1. **CPS-safe parse** — Jenkinsfile:236, in `remoteOnlyHumanRecords()`:
   `new groovy.json.JsonSlurper().parseText(readFile(path))` → `readJSON(file: path)`
   (the in-file CPS-safe pattern, already used at lines 704, 1157). Stops the `parse failed` warning
   and makes `remoteOnlyPersonas()` report the real reduced-scope count in the banner.
2. **Add a `suspendedHumanIds()` helper** (readJSON deployments.json → humanIds where `suspended`).
3. **Filter the seed/verify peer set by it** at the top of BOTH `runContentSeedStage` (after
   `genesisPeers = humans.findAll{ it.genesisPeer }`) and `runVerifySeedingStage` (the `humans`
   loop) — drop any human whose humanId ∈ suspendedHumanIds(), ALWAYS (a suspended human is never
   seeded/verified, shem up or down). Emit an echo ("⏸️ Holding suspended peer(s) …") for visibility.

Net: adam is held from both stages → no 120s wait → those two UNSTABLEs disappear when shem is down.

## Status: LANDED 2026-06-08 (operator authorized the push)
Implemented in `genesis/Jenkinsfile`: `readJSON` CPS fix (line ~236) + `suspendedHumanIds()` helper +
the authoritative suspension filter on the seed set in BOTH `runContentSeedStage` and
`runVerifySeedingStage`. Brace-balanced (delta 0 vs HEAD); Groovy not locally runnable (Che) →
**CI is the validator** (the genesis build confirms; push is authorized so iterate if it errors).
`ci_status: in-progress` → confirm by disappearance of the "skipped unreachable: human-adam-firstman"
UNSTABLE in the next genesis build's Seed Database + Verify Seeding stages.

## Scope note
This removes only the 2 suspended-peer-skip UNSTABLEs. Genesis stays UNSTABLE from the DOMINANT cause
(conductor admin-WS unreachable → see `ci-genesis-conductor-adminws-unreachable`) and the intentional
shem-down Probe Substrate `unstable()` (a design choice — see the shift sprint-result's probe-marking
proposal). Flipping genesis fully to SUCCESS needs all three addressed.

## Follow-ups discovered (don't drop)
- Jenkinsfile lines 83 & 125 use the same CPS-unsafe `JsonSlurper` for topology parsing — verify they
  don't have the same latent failure (they did not log a parse-fail, so likely fine, but confirm).
- Assessment seed-data gap (latent, non-fatal): `assessment-personal-values` / `assessment-values-hierarchy`
  log `format=sophia, no widgets in content body` during Verify Seeding — a genesis/data/lamad data-shape
  issue worth a separate seed-data fix.
