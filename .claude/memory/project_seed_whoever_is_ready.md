---
name: Seed whoever is ready — partial readiness aligns with P2P architecture
description: Seeder must not be all-or-nothing on conductor health; per-peer seeding with partial reporting is the correct behaviour
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
When a conductor admin WS is down on one peer (e.g. terrance in alpha-cluster #1002), the seeder must continue seeding the peers that ARE ready (adam, jessica, etc.) and report the unready peer as partial — not abort the whole stage.

**Why:** This aligns with the P2P-native architecture target: peers join and leave fluidly, partial-cluster operation is the steady state, and downstream consumers (a2o tests, doorway projection, federation) should already tolerate one peer being absent. An all-or-nothing seeder pretends the substrate is monolithic, masks per-peer health information, and produces 40-minute "Seed Agent Peer Bindings FAILED" stages where only one of N pods was actually unhealthy. The other (N-1) peers were ready the whole time.

**How to apply:**
- Readiness probes for Holochain admin WS belong at the per-peer level, not gating the whole seed stage.
- The seeder should record a per-peer readiness snapshot at start, attempt seeds against ready peers only, and surface unready peers in the report — not retry-loop them.
- The orchestrator's reconciliation artifacts already accommodate this: actual-build-graph.json's `results` map can carry per-peer status; downstream advisories can decide whether partial-seed warrants UNSTABLE or just informational.
- Same principle extends to genesis E2E: tests that target a specific peer should skip-with-reason if that peer was unready, not fail-cascade.
- This is downstream of the larger "household is the resilience unit" memory — a household can tolerate one node being out; a single-node failure should not become a household-wide failure.
