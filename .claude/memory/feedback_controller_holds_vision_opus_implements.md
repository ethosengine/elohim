---
name: feedback_controller_holds_vision_opus_implements
description: "Operator 2026-09-23: on a multi-task plan the top model focuses on fidelity to the vision, design rulings and alignment review; Opus 5.5 absorbs every implementation task."
metadata:
  node_type: memory
  title: Controller holds vision and reviews alignment; Opus implements
  type: feedback
  originSessionId: 4264e76d-be32-47a9-939b-6b00665dcd45
  modified: 2026-09-24T17:30:00.000Z
---

Operator, 2026-09-23, while picking up the governed-discovery station-4 plan: "opus 5.5 is
very capable, make sure you orchestrate most of the heavy lifting there … you just focus on
the fidelity to the vision, and design of what we're trying to accomplish here, and review
what's built for its alignment."

**Why:** premium reasoning spent on transcription is waste, and a controller that also
implements loses the fresh-context review seat the process buys. On station 4 the split
worked: eight tasks, one Opus implementer each, one review + at most one fix round; the
controller's own alignment read caught the design defects the task reviews could not
(string CIDs in a governed declaration, a whole-tree fold that made the lag bound
unreachable, a self-run window, a fused passage taken from the wrong producer).

**How to apply:** run plans subagent-driven with `model: opus` implementers and reviewers;
keep the controller to: reading the spec's invariants, pre-flight conflict rulings, a
short alignment read of each landed diff against the vision, rulings on reviewer/implementer
disagreements (with cost-if-wrong in the ledger), and the whole-branch review dispatch. Never
fix in the controller session. Give evidence runs a PRIVATE cargo target dir — other
sessions overwrite the shared gate target's binary mid-run. See also
[[feedback_role_coherence_orchestration_operator_executes]] and
[[feedback_agent_fleet_and_harness]].


**Restated 2026-09-24** (rejecting a plan whose execution section had the controller running the
integration merge): "use opus5.5 to drive the bulk of the work.. I want you only as orchestrator,
architect, and planner.. helping as advisor and guide to the plan." Scope is total: Lane-0 merges,
habit deltas, plan landing, fleet-step prep are all Opus work; the controller's outputs are rulings,
review verdicts, dispatch/sequencing and the operator summary. Sprint plan:
`genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md`.
