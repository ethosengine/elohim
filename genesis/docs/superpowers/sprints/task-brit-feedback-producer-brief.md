---
id: task-brit-feedback-producer-20261111
gap: plans__2026-11-11-brit-feedback-operation#6
actor: agent:orchestrator@gpt-6-astra
status: In Progress
---

# Deliver the producer and bounded-history repair

Port the existing feedback producer and bounded-log repair from dirty `/projects/elohim/elohim/brit` onto the clean worktree `/projects/elohim/.worktrees/brit-feedback-release`, based on installed published `f8ef2fb6b40d3678fce36f114259d80e2bf331ee`. Preserve all source work; do not edit the dirty source checkout. Select only feedback and bounded-history changes, preserving complete retained-helper and CPU compatibility behavior in the newer release. Do not copy unrelated CPU work wholesale. The original feedback occurrence is `bafyreieytypsusld4whxkx7af7vfqg2hsofsyuv2ikftu2unm7ozawoe3e`; the orchestrator has proposed its repair through native feedback.

Bounded history supports `brit log -5`, `-n 5` and `--max-count 5`, including zero and option terminators. Inspect whether path-filtered history honors limits: the dirty candidate currently calls log_file without the bound, a likely gap. Add meaningful CLI coverage for limits, history exhaustion and path semantics. Preserve successful/refused operation capture and command exit status if recorder fails; no raw argv or output retained. Agent participation must not pretend to be Matthew. Installed replay and experienced recovery are integrator-owned after publication.

Read the worktree governance and installed Brit help before repository operations. Use Brit for changes and explicit fallback only for a proven missing capability. Coordinate cargo berth/builds with root. Build the complete helper closure from clean committed source when root requests; initially report the exact changes and focused tests. Do not commit/push until root has reviewed source isolation. Root owns native promise, final integration and terminal flow; do not replace its actor claim.
