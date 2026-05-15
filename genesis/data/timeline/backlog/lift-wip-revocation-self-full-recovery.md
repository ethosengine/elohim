# Backlog — Lift @wip on revocation-self full-recovery scenario

**Filed:** 2026-05-15
**Origin:** Recovery M4 T25 (deferred honestly — substrate ready, BDD orchestration not yet)
**Owner:** M5 sprint (UX + multi-agent test world)
**Scope:** ~200 lines of cucumber step defs + multi-agent world wiring

## The scenario

`genesis/a2o/features/auth/recovery/revocation-self.feature:50-56`:

```gherkin
@wip
Scenario: Matthew can initiate full recovery after revoking his only key
  Given Matthew has revoked all of his keys
  And Matthew is effectively locked out of his identity
  When Matthew's emergency contacts help him through the M3 recovery flow
  Then Matthew obtains a new agent key
  And his identity and history are restored to his new key
```

## Why it's still @wip after T25

T25's plan called for: "Run the scenario; if steps are missing, wire to recovery-coordinator service; lift @wip when green."

Step-def search found **zero implementations** for the five distinctive phrases:
- "Matthew has revoked all of his keys"
- "Matthew is effectively locked out of his identity"
- "Matthew's emergency contacts help him through the M3 recovery flow"
- "Matthew obtains a new agent key"
- "his identity and history are restored to his new key"

Wiring them properly requires:
1. **Multi-agent test world** — at minimum Matthew + 3 emergency contacts running concurrent conductor cells, with witness-submission orchestrated across them.
2. **Pre-state setup** — driving the test world to "all of Matthew's keys revoked" requires either seeding revoked state via the storage projection OR running the actual revocation flow first (which itself is a multi-step ceremony).
3. **Recovery-coordinator UI binding** — `recovery-coordinator.service.ts` (481 lines) has `initiateRecovery` / `completeRecovery` surface, but the UI components that drive them through the M3 flow (`recovery-request/`, `recovery-interview/`) are the M5 surface; the full happy-path browser flow may not yet be wired.
4. **Continuity assertion** — "his identity and history are restored" requires reading post-rotation state from a separate cell (Matthew's new key) and comparing to pre-rotation state (the old key's history).

The substrate primitives are all in place (M3 quorum, M4 producers/bridges, T18 EPR signal, T19-T21 Shamir transport, T22 custody manifest, T23-T24 audit). What's missing is the **multi-agent cucumber orchestration layer** that runs these as a coordinated test.

## The honest call

T25 explicitly says: "**Do not invent UI flows** — wire the existing recovery-coordinator service if a step needs to talk to the backend."

Writing the step defs as orchestrated multi-agent flows requires either:
- A new multi-agent test-world helper (~200 lines, needs design)
- Faking the orchestration with assertions that pass on partial integration (which DOES invent UI flows by omission)

Both options exceed T25's intent. The substrate readiness is real; the BDD orchestration is genuinely M5 work.

## What M5 should add

1. **Step-def file** at `genesis/a2o/steps/recovery-flow.steps.ts` (or extend `recovery-cross-stack.steps.ts`).
2. **Multi-agent helper** in `app/elohim-app/src/framework/` for spinning up Matthew + N emergency contacts in the cucumber world.
3. **Pre-state helper**: `await reset_to_locked_out_state(world, "Matthew")` — seed revocation rows directly into storage projection.
4. **M3 flow driver**: orchestrate witness submissions across emergency contacts, await threshold, drive key rotation.
5. **Continuity assertion**: query storage from Matthew's new agent context, verify identity record + relationship records are reachable.

Once those land, lift @wip on line 50 and re-run cucumber to confirm green.

## Current substrate readiness

Substrate-side, the scenario could pass today if the step defs existed:

- Revocation primitive: ✅ T14 `submit_specialist_revocation` / `create_self_revocation`
- Lockout detection: ✅ M4 `query_effective_revocation_for_key` gate
- M3 emergency-contact recovery flow: ✅ T13 `create_recovery_request` + T3 `submit_intimate_witness`
- Key rotation: ✅ M4 `commit_key_rotation`
- History continuity: ✅ identity records persist across rotation (rotation is a key-replacement, not an identity replacement)

The bar for green is the BDD orchestration, not the substrate.

## Recommendation

Leave `@wip` on line 50. Reference this backlog item in the M5 sprint's UX work. Don't lift @wip until the orchestration steps are real.

This is the same honest-deferral pattern as T23/T24's audit docs: substrate ready, UX/BDD layer needs follow-up, name the gap explicitly so it doesn't accumulate as silent debt.
