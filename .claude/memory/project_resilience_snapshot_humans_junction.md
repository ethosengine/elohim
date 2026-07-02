---
index: false
id: project-resilience-snapshot-humans-junction
name: resilience-snapshot-humans-junction
title: Resilience snapshot humans-junction
description: Both snapshot() joins pass through substrate-owned humans.agent_pub_key+household_id (no HTTP create sets them); commitments-only seeding lights nothing.
metadata: 
  node_type: memory
  type: project
  originSessionId: 8dd8d2c6-91fb-499c-bd3e-cadbcb784c8b
cites:
  - elohim/elohim-storage/src/services/household_resilience.rs
  - elohim/elohim-storage/src/db/humans.rs
  - elohim/elohim-storage/src/api/rea_commitments.rs
---

`household_resilience::snapshot()` (elohim/elohim-storage/src/services/household_resilience.rs) lights up only through the **humans junction**, and both of its joins require fields no HTTP create surface can set:

- `stewardingCollectives`/`protectionStatus`: `shard_locations.peer_id = humans.agent_pub_key` + `humans.household_id IS NOT NULL`
- `commitmentBackedCollectives`: `rea_commitments.provider = humans.agent_pub_key` + `action='provide'` + `state='active'` + `resource_classified_as='content:<reach>'`

**Why naive seeding lights nothing (verified 2026-06-05):**
1. `CreateHumanInputView` has no `householdId`; doorway `/auth/register` writes neither `household_id` nor `agent_pub_key` — these are deliberately substrate-owned ("projection of collectives DHT entry with kind:household", humans.rs:31-33). Seeded humans = NULL on both → every join excludes them.
2. `POST /api/v1/commitments` inserts `state="proposed"` hardcoded (rea_commitments.rs:288); activation needs `PATCH /api/v1/commitments/{id}` (auth_required).
3. **(CORRECTED 2026-06-20)** The `commitmentBackedCollectives` leg is now LIT on CI/alpha: a dedicated runtime seeder `genesis/seeder/src/seed-provide-rows.ts` writes `action='provide'/state='active'/resource_classified_as='content:<reach>'` against live pods, keyed to `humans.agent_pub_key`, CI-wired at `genesis/Jenkinsfile:1496` (via `genesis/scripts/ci/seed-provide-rows.sh`). Verified `commitmentBackedCollectives=1` on live alpha (matthew's active commons provide row). So the older "only `test_util.rs` writes provide rows → 0 even on alpha" claim is STALE — provide rows are seeded outside test_util. The `commitment_backed` fold (lifted into the rea-economic facing lens) is therefore the one resilience leg that folds real data today. STILL TRUE: the `stewardingCollectives`/`protectionStatus` leg (shard_locations join) has no seed path and needs the substrate chain; local dev `hc:start:seed` still lights nothing (no live mesh).

**Honest states:** local dev `hc:start:seed` → at-risk/all-zeros (correct, not a bug — see [[local-stack-dht-anchor-gap]] for the sibling seeding gap); CI/alpha fixture cluster → placement-driven partial/protected via real shard_locations + peer heartbeats (the a2o `observable-distribution.feature` scenarios prove it).

**How to apply:** any plan that says "just seed commitments to demo resilience" is the terrance-drift dead-data shape (cf. seed-commitments.ts docstring: "Drift in any of these fields = views silently filter out the row"). Lighting local dev requires the substrate chain (household projection + peer registration + placement) first.
