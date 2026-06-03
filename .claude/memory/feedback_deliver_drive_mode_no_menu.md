---
id: feedback-deliver-drive-mode-no-menu
name: feedback-deliver-drive-mode-no-menu
description: "When running /deliver (or any iteration-loop skill), prefer 'just drive' mode over presenting AskUserQuestion menus. Self-direct the iteration; if waiting on CI, schedule a wakeup and do anticipatory debugging in the meantime."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: d8ac4ba5-8c70-42b9-a7eb-f98689dea358
cites:
  - .claude/skills/deliver/SKILL.md
---

When running `/deliver` (or any other iteration-loop skill that ships with "present options, wait for kickoff" patterns), DO NOT present multi-option AskUserQuestion menus at iteration kickoff. The operator prefers self-directed driving.

**Why:** Iteration loops in /deliver, /shift, and adjacent skills often include a "compose journal + present options, wait for kickoff" step. In practice this slows the operator down — they've already chosen the handle (e.g., "/deliver epr-app delivery"), and presenting a menu of 3-4 paths forces them to context-switch back into the decision space when they wanted the agent to be in motion. The agent should make the reasonable call and drive; the operator will redirect if needed (per AUTO_MODE bias).

**How to apply:** During /deliver iter-N (or any iteration kickoff):
1. Do the search-first audit, compose the FeaturePromise update, render the initial state, write the tier-3 verdict.
2. Pick the **highest-leverage next move** based on what just landed, what's gating delivery, and what's in flight. Just do it.
3. If waiting on something external (CI build, deploy, P2P propagation):
   - Set up a wakeup signal (`Bash run_in_background` with an `until <condition>; do sleep 60; done` poll, or `ScheduleWakeup` if in /loop dynamic mode)
   - Do **anticipatory work** in the meantime — curl every relevant endpoint to map the failure surface, pre-stage fixes for likely gaps, investigate adjacent issues that may compound, read code around the suspected gap so when CI lands you can act immediately.
4. When the wakeup fires (or external state changes), fold the new findings into the iteration and continue driving.

Anti-pattern: AskUserQuestion with "Which path would you like?" at iter-N kickoff when the search trail + render state already make the highest-leverage move obvious.

Anti-anti-pattern: still bail (per skill spec) when search_trail < 7 and design is genuinely uncharted, or when a destructive consent-required path is about to be touched. Those legitimate operator-decision moments STAY.

Related: [[feedback_agent_framing]] (autonomous loops are devs with full cycle), [[feedback_orchestrating_vs_implementing]] (orchestrating = decisions pending; implementing = file/test pointer), [[feedback_spare_no_expense_intelligence]] (read full context in parallel; auto-mode ≠ shortcut synthesis).
