---
id: feedback-overnight-shift-jenkins-is-workhorse
name: feedback-overnight-shift-jenkins-is-workhorse
description: "For overnight pipeline-iteration shifts, Jenkins does the heavy lifting. Run only targeted local tests for diagnosis; never kick off huge local builds. Husky guards may be skipped freely. Long ScheduleWakeup intervals (1200-1800s) between observations are the norm."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 5ed7452d-de73-43b1-814f-3b1742a3b1b8
cites:
  - .claude/commands/shift.md
  - genesis/orchestrator/Jenkinsfile
---

For overnight shifts where the operator says "you own the pipeline tonight" / "use Jenkins to do the heavy lifting":

**Jenkins is the workhorse.** Local builds compete with sprint workspace builds for the shared cargo-target pool and burn iteration budget. Push small targeted commits with `[build:<pipeline>]` tags and let Jenkins run the matrix. Never run `cargo build --release` for the whole workspace, `pnpm --filter all run build`, or `hc dna pack` locally during an overnight shift unless a single specific test requires it.

**Targeted local tests are fine.** A single `cargo test --test schema_contract`, `pnpm --filter <one-package> exec vitest run <pattern>`, or `pnpm --filter <one-package> exec eslint <file>` runs in seconds and is the right way to validate a single fix before pushing. The line between targeted and huge is: does it touch one crate/package and finish in <60s? If yes, run it. If it's whole-workspace, push and let Jenkins.

**HUSKY=0 is the expected pattern.** The pre-push gate runs the whole workspace's quality check locally — exactly the kind of huge build the operator wants to avoid. For pipeline-iteration shifts, CI is the gate by explicit design. `HUSKY=0 git push` is the standard form, not a bypass. Memory entry `feedback_husky_bypass_for_ci_only_changes.md` originally narrowed this to "CI-only commits" — overnight pipeline-iteration shifts broaden it: any commit on a `fix/<shift-slug>` branch may use HUSKY=0.

**ScheduleWakeup pacing.** Between iteration observations during an overnight shift, sleep 1200-1800s (20-30 min). Each pipeline run takes 15-30 min; polling every 5 min burns cache windows for no signal gain. Long sleeps are correct for overnight pacing — the cache miss is one-time per wake; the savings are linear in sleep duration. Exceptions: when a specific build's ETA is known (e.g. orchestrator typically ~10 min), sleep 600s (10 min, in-cache).

**Scope of "own the pipeline".** During an overnight shift with this framing, you have authority to push fixes that affect:
- Jenkinsfiles (any pipeline)
- Orchestrator dispatch logic
- Test fixtures (with scope restraint)
- CI-affecting config (clippy.toml, eslint.config.js, vitest.config.ts, etc.)
- Manifest infra under `genesis/orchestrator/manifests/**`

You do NOT have authority to:
- Touch the sprint workspace (operator's in-flight work) — see `feedback_pipeline_work_separate_from_sprint`.
- Modify the Objective measure or oracle files mid-shift.
- Force-push, rebase, or amend.

**Delivery framing.** When the shift's named outcome is a delivery (e.g. "EPR-app delivery working"), don't stop at pipeline-green count. After CI stabilizes, drive the deliverable's user-visible state — does alpha.elohim.host actually serve the landing EPR? Does `/lamad/` serve its own bundle? Pipeline-green is necessary but not sufficient.

Related: [[feedback_pipeline_work_separate_from_sprint]] (workspace separation), [[feedback_husky_bypass_for_ci_only_changes]] (narrower scope), [[feedback_multi_agent_pvc_pacing]] (cargo-target pool contention).
