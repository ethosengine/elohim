---
name: Plan/spec locations in elohim repo
description: Three plan locations based on scope — elohim domain work, meta/automation work, and submodule-scoped work. Corrected 2026-04-17.
type: reference
originSessionId: cba4ccfa-49ce-49fd-8896-afd15e101e73
---
**Location convention for this project:**

| Scope | Plans location | Specs location | Examples |
|-------|---------------|----------------|----------|
| **Elohim domain / cross-cutting design** | `genesis/docs/plans/` | `genesis/docs/specs/` | EPR navigation boxes, trust architecture, steward economy |
| **Meta / generalizable engineering automation** | `genesis/docs/superpowers/plans/` | `genesis/docs/superpowers/specs/` | Agentic developer loop, quality pipeline, CI automation |
| **Submodule-scoped** (sophia, brit, rakia) | Inside the submodule itself | Inside the submodule | sophia assessment engine changes, brit protocol work |

**Why:** User clarified 2026-04-17. `genesis/docs/plans/` is for the elohim protocol's functional and design work. `genesis/docs/superpowers/plans/` is for meta work that's generalizable (engineering automation, tooling, agent skills). Plans scoped entirely to a git submodule live in that submodule so they travel with it.

**How to apply:**
- Ask: "Is this elohim domain work, meta/tooling work, or submodule-scoped?" — route accordingly.
- Never create a top-level `/projects/elohim/docs/` directory.
- Retrospectives: `genesis/docs/retrospectives/`.

**Cross-reference convention (rakia cutover, added 2026-04-18):** Plans touching the build/test/attestation surface — the Jenkins→rakia cutover trajectory — live in `rakia/docs/plans/` with a one-paragraph stub at `genesis/docs/superpowers/plans/<same-filename>.md` pointing at the rakia path. The stub keeps the meta/automation index complete without duplicating content. Example: `rakia/docs/plans/2026-04-18-experience-story-discernment-gate.md` has its stub at `genesis/docs/superpowers/plans/2026-04-18-experience-story-discernment-gate.md`.

**Audit-hook cooldown:** The p2p-plan-audit hook supports a cooldown file at `/projects/elohim/.claude/hooks/.p2p-audit-cooldown.json` with `{until, reason, sprint}` keys. Set it for the duration of a sprint when JSON Schema content in a plan repeatedly false-positives and antidote inline notes can't satisfy the 10-line window — as long as the spec has completed the p2p-design-gate separately.

**p2p-plan-audit hook note:** Any file with `/plans/` in its path is audited by `.claude/hooks/p2p-plan-audit.py`. Avoid trigger keywords (`schema`, `CREATE TABLE`, `PRIMARY KEY`, bare `UUID`, `REST route`, `GET /…`) unless an antidote keyword (`source.of.truth`, `DHT`, `projection`, `operational`, `notarized`, `agent.scoped`) appears within 10 lines.
