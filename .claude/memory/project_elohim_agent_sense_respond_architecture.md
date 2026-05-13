---
name: elohim-agent sense-and-respond architecture for gates and capabilities
description: Architectural layering — discernment/gates live in elohim-agent (Rust primitive), app manifests declare which gates apply, TS is sense-and-respond UX only, not evaluator
type: project
originSessionId: 94ce7c3c-076a-4eaa-bbc8-9ba6bf9b48eb
---
**Architectural layering (clarified 2026-04-18 during experience-story discernment work):**

Discernment, gates, and agent-level judgment are **core primitives of `@elohim/elohim-agent`**, not app concerns. Do NOT implement agent-level decision logic in `elohim-library` or `elohim-app` — even as "temporary" or "for testability."

**Three layers:**

| Layer | Where | Role |
|-------|-------|------|
| **Core gate-interface** | `elohim/elohim-agent/elohim-agent-service/` (Rust) | Defines the `Gate` trait primitive, gate registry, constitutional-reasoning coupling. Load-bearing protocol infrastructure. |
| **App-manifest gate declarations** | `elohim/sdk/domains/*/manifest.json` (per-domain coupling) | Declarative: "contentType X uses gate Y with these rules." Emergent app-domain dimension — not hardcoded. |
| **Sense-and-respond UX** | `.ts` surfaces (elohim-library, elohim-app) | Gathers relevant context (moment shape, observation memory), calls elohim-agent via SDK, renders responses. Does NOT evaluate. |

**Examples that all use the same gate-interface primitive (implying cross-cutting value):**
- Journal drafting (context gathering → agent evaluates → rendered draft)
- Comment reach check (user action → gate evaluates reach policy → allow/escalate/decline)
- Experience-story discernment gate (moment recorded → gate evaluates evidence → maybe attest)
- Imagination bounds (capability invocation → gate evaluates → fulfilled/declined/escalated)

**Why:** The user is explicit that discernment is cross-cutting in elohim-agent. Implementing it per-app duplicates the pattern and bypasses the constitutional reasoning / audit trail that elohim-agent-service already provides via `ElohimCapability` + `ConstitutionalReasoning` + `AuditLog`.

**How to apply:**
- Before implementing "judgment" logic (classify, discern, evaluate, gate), ask: is this an agent capability? If yes, it goes in elohim-agent-service as a Gate impl.
- Manifests declare which gates apply via a coupling field (exact schema TBD — see gate-interface spec when authored).
- TS callers invoke via `elohim-agent-sdk` (Fastify `/invoke` sidecar) — they don't re-implement the judgment.
- Anti-pattern: a pure-function judgment module in `elohim-library/*/discernment/` or `elohim-app/*/services/judge*.ts`. If I catch myself writing one, stop and put it in elohim-agent.

**Incident referenced:** 2026-04-18 I (Opus 4.7) wrote a TS pure-function discernment module in `elohim-library/projects/elohim-service/src/discernment/` for experience-story. User caught it after ~10 commits and redirected. Reverted in commit `dfadce0b`. The preserved rule-logic (7 valences, rule ordering, 20% duration threshold) is kept in the spec at `genesis/docs/superpowers/specs/2026-04-18-experience-story-epr-design.md` §5–§7 and will be the first Gate implementation once the gate-interface spec lands.
