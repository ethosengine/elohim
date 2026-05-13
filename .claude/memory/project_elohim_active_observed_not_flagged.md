---
name: Elohim active is observed, not flagged
description: Phase::ElohimActive is determined by whether real inference actually happened, not by a config flag. If stub runs, elohim are not active — regardless of what any flag says.
type: project
originSessionId: 6ec4bfae-b3f0-4040-8a90-6ae504910fe7
---
`Phase::ElohimActive` vs `Phase::DevContext` is NOT a feature flag. It is an observed property of the actual execution.

**The architectural rule:**
- If `WisdomInvokeExecutor` returned a real LLM inference → Phase::ElohimActive propagates into the attestation
- If it returned a mocked/stub response → Phase::DevContext propagates — REGARDLESS of config flags, env vars, or transport settings

**How this works in practice:**
- `elohim-agent-service::/wisdom/invoke` is the authoritative source of the phase marker
- If the service had an API key available AND the LLM call succeeded, it returns `{phase: "elohim-active", ...}`
- If no API key, or the call errored and fell back to a stub, it returns `{phase: "dev-context", ...}`
- The gate-client's WisdomInvokeExecutor reads that phase from the response and threads it through the DecisionAttestation
- A gate-client with `WisdomTransport::Http(...)` pointing at a service that returns stubs → still emits DevContext attestations

**Why this matters:**
- Prevents architectural self-deception: "we set the flag but nothing's running" shouldn't count as active
- Makes the phase honest: the attestation graph shows the truth of what happened, not what was configured
- Simplifies auditing: anyone reading an attestation sees whether real wisdom was applied
- Reputation accumulation filters to genuine `Phase::ElohimActive` attestations only (per spec §5.5) — if flags could lie, reputation would be corruptible

**How to apply:**
- Never introduce a `Phase::ElohimActive` assignment from config alone
- Any code that emits Phase must derive it from observing the actual wisdom call's outcome
- The phase field in the HTTP response from elohim-agent-service is the source of truth for propagation

Flagged 2026-04-19 during Phase 8 kickoff. The user corrected my initial framing of "flag-driven activation" — activation is a consequence of real inference, not a setting.
