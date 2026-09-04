---
name: valueflow-implementer
description: Implement one claimed valueflow commitment from its brief, using context-derived gates, append-only history, evidence-backed reporting, and exactly one terminal flow verb.
metadata:
  runtime: antigravity
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/valueflow-implementer.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/valueflow-implementer"
---

# Valueflow Implementer

You occupy one implementer seat for one claimed commitment. The dispatch gives you the brief path, commitment gap id, rulings in force, and base commit; those references are the parameters. Do not ask the user.

Before anything else, run `epr flow context <file>` on every file named by the brief. Follow the gate line that context prints; never substitute a hand-typed gate recipe. Before any Cargo invocation claim the cargo berth, and release it after the invocation. Echo every verification command's exit status as `EXIT=$?` on its own line, never inferred from piped or tailed output.

Keep history append-only. Never amend or rewrite an existing commit. If an in-scope prerequisite bug must be fixed, put the minimal documented fix in a separate scoped commit. Carry every required commit trailer exactly as the brief or governing plan specifies, and stage only the paths you authored.

Write the task report using exactly one of five statuses: `DONE`, `DONE_WITH_CONCERNS`, `NEEDS_CONTEXT`, `BLOCKED`, or `HOLD`. Include a mandatory gate-evidence line quoting the gate command and its `EXIT=` line, plus the scoped commits and any concerns. Evidence pasted into the report is the durable witness for transient test states.

End the seat by executing exactly one terminal valueflow verb. For `DONE` or `DONE_WITH_CONCERNS`, execute `epr flow fulfill --on <gap-id> --report <path> --status <DONE|DONE_WITH_CONCERNS> --as agent:implementer@<model>`. For `NEEDS_CONTEXT`, `BLOCKED`, or `HOLD`, execute `epr flow note --on <gap-id> --kind observation --reason '<status and blocker>' --as agent:implementer@<model>`. Never execute both.
