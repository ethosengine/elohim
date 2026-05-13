---
name: /shift has two modes — bring-up and integration-iteration
description: A shift's iteration loop adapts based on whether the cluster is broken (bring-up, single objective) or stable (integration-iteration, multi-candidate fix-as-many-classes-as-possible across 5 loops)
type: feedback
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
`/shift` runs in one of two modes, decided at kickoff from the last orchestrator build's status:

- **Bring-up mode** when CI is broken (last orchestrator FAILURE/ABORTED/NOT_BUILT/cold). Single Objective, single failing dimension being driven down. Build waits are idle. Done = stable delivery (two consecutive passing measurements, fresh trigger).
- **Integration-iteration mode** when CI is already stable enough (last orchestrator UNSTABLE or SUCCESS — deployed, scenarios running). Plural objective: drive down the failure surface across **multiple classes** (test failures, missing implementations, console errors, CI dispatch efficiency). 5-loop budget. Build waits are filled with parallel candidate investigation. Multi-candidate observer dispatch (today via multiple ci-observer calls scoped to different artifacts; eventually via a `failure_candidates` schema extension).

**Why:** User design call — "on every shift, the orchestrator should try to fix as many 'things' as it reasonably can in that build window... iterate to reduce console errors, finish implementation, and try to increase valid feature delivery, should be a mode of the shift, that can happen AFTER we reach a stable delivery." The ci-* agents shouldn't stop at the first thing they find; they should support multi-candidate composition that the orchestrator works through in parallel during build-wait windows.

**How to apply:**
- Full design lives in `.claude/skills/agentic-developer/SKILL.md` → "Shift modes" + "Iteration loop adaptations in integration mode."
- When discussing /shift behavior, observer/investigator output shape, or sprint planning outside a shift session, reach for the two-mode framing.
- Schema follow-up: `.claude/schemas/haiku-output.schema.json` needs an optional `failure_candidates: array` (each entry shaped like `primary_failure`) to formalize what's currently composed by multiple observer dispatches.
- Don't conflate bring-up's sequential discipline with integration-mode's parallel-during-wait pattern — the modes have different correctness invariants (bring-up needs unambiguous attribution; integration trades some attribution clarity for breadth).
