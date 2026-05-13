---
name: Cadences are archetype-tunable with dev-mode overrides
description: Any scheduled/periodic loop must have archetype-aware defaults, policy-file overrides, env/CLI dev overrides, and a synchronous admin trigger for testing
type: project
originSessionId: 17546f03-3ee8-4704-bdf9-18d0d64baf9b
---
Any scheduled or periodic operation (verification scans, distribution retries, reconstruction polls, PeerStatus cadence) must support **four layers of control**:

1. **Archetype-aware defaults** — a Level-0/edge device runs a slower cadence than a Level-5 archival node. Defaults baked into code as a lookup by archetype, not a single global constant.
2. **Policy-file overrides** — `peer-policy.toml` (or equivalent) lets the operator tune cadence per deployment. Overrides archetype default.
3. **Env/CLI dev overrides** — `ELOHIM_VERIFY_INTERVAL=30s` or `--verify-interval=30s`. Overrides policy. Intended for tests and local dev.
4. **Synchronous admin triggers** — `POST /api/v1/admin/<op>/run-now` (authenticated) to force the operation immediately without waiting. Essential for chaos tests, regression a2o, operator investigation.

**Why:** User framing (2026-04-19): "be sure you provide developer-mode params or something so we can isolate types of peer interactions so we can test and not wait 6hrs to prove something." Also: different archetypes have different resource budgets; one cadence does not fit all.

**How to apply:**
- When designing any timed/scheduled loop, specify all four layers in the spec.
- Never ship a hard-coded interval. Never ship without a synchronous trigger.
- Document the default table (archetype → interval) in the spec.
- Chaos/a2o tests use the synchronous trigger, never the cadence (tests should be deterministic, not flaky on timing).
