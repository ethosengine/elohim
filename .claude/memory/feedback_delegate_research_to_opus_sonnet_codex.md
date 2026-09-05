---
name: feedback_delegate_research_to_opus_sonnet_codex
title: Delegate research legwork to Opus/Sonnet/Codex
description: "Operator (2026-09-03, reaffirmed 2026-09-05) — research AND plan-implementation legwork fan out to Opus and Codex (Sonnet for code grounding); the top model spends only on decisions, coherence, judgement, delegation, vision-alignment — bites at every grounding fan-out and every plan execution"
metadata: 
  node_type: memory
  title: "Delegate research to Opus, Sonnet and Codex"
  type: feedback
  originSessionId: bf90213f-876c-4014-807d-504fb20fefd3
  modified: 2026-09-03T22:48:34.021Z
---

When a design pass needs grounding or research, fan it out: Sonnet for code
grounding (file:line), Opus for adversarial/upstream/web research, and Codex
(`/usr/local/bin/codex`; `codex exec --sandbox read-only -C /projects/elohim -o <outfile> "<prompt>" </dev/null`,
run in the background with the prompt kept in a scratchpad file) for a DISJOINT
read-only question. Keep the fleet ≤3 concurrent ([[feedback_agent_fleet_and_harness]]).

**Why:** the operator said so twice on 2026-09-03 ("remember to delegate to opus
and sonnet if you need help with the research" … "and codex") while I was
grounding the rung-6 hApp-migration design; premium reasoning belongs on the
synthesis, not the reading ([[feedback_role_coherence_orchestration_operator_executes]]).
**How to apply:** at the atlas-grounding fan-out give each seam group to a
reader; give Codex the seam whose read-set is disjoint from the others; write
the prompt to the scratchpad so it is reviewable; synthesize only after all
returns. Codex output lands in the `-o` file; check `CODEX_EXIT` in the log.

**Reaffirmed 2026-09-05 (mid valueflow-tooling design):** "Use opus and codex to help you
research, and then when writing the plan, use opus and codex to do the legwork of
implementation of the plan.. I want you to save your tokens at the decision making level,
coherence, judgement, delegation, vision-alignment and orchestration." So: implementation
tasks from a plan go to Opus agents and `codex exec` (full-auto in a worktree when it must
write), not to me. I read their evidence and decide. See [[orchestrate]] skill tier ladder.

**2026-09-05 (integration session):** the local `codex` CLI (0.153.x) now runs **GPT-6 "astra"** — the
operator rates it as technically proficient as the top model, so it can take real implementation and
verification legwork (it ran the epr-rea/eprfs gates and audited task-report test claims cleanly), but
"I wouldn't necessarily trust it if it needs to make a moral judgement" — keep governance/ethics/
framing calls with the top model. Practical: `--sandbox danger-full-access` is needed when the task
runs cargo (the target pool lives outside the workspace); keep the prompt in the scratchpad.
