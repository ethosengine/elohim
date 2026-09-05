---
name: package-first-projection-drift-direction
title: "Projection-drift gate: read the diff before projecting"
description: "elohim-agent drift gate — project --write-runtime OVERWRITES runtime CLAUDE.md/SKILL.md from the package; if the runtime side is newer, fold runtime→package first or content is destroyed."
metadata: 
  node_type: memory
  index: true
  title: "Projection-drift gate: decide the direction by reading the diff"
  type: feedback
  originSessionId: 77071821-7182-463a-ae84-0c496dd5f84e
  modified: 2026-08-22T06:26:26.804Z
---

The elohim-agent package gate (pre-push, `package-projections.mjs`) treats `.epr-meta/elohim/packages/` as canonical for package-first surfaces (CLAUDE.md via the `elohim-root-gospel` agentdoc, `.claude/skills/*/SKILL.md`, agents, .codex). Sessions habitually edit the RUNTIME side directly, so at push time the gate reds with "projection drift".

**Why:** On 2026-08-22 the suggested cure (`project --write-runtime`) would have deleted the `just test mesh`/`just mesh prologue` lines from CLAUDE.md and stripped 157 lines of accumulated mesh-orchestration content from hc-dev-orchestrator's SKILL.md — the package was the stale side, not the runtime. `packages:import` did NOT capture the runtime bodies for already-package-first packages (it imports new source packages only).

**How to apply:** When this gate fires, `git diff` the projector's writes BEFORE committing them. If the runtime holds real newer content: restore runtime files (`git checkout --`), fold the runtime body into the package JSON yourself (`instructions.body` for skills = SKILL.md minus frontmatter; `source.body` for agentdocs = the whole doc), then `project --write-runtime` (should now be byte-stable) + `pnpm run elohim-agent:packages:write` (projection fixtures) + verify 0 FAIL, and commit packages+fixtures+AGENTS.md together. Better: edit the package first and project, instead of editing runtime surfaces of package-first packages. Related: [[feedback_verify_the_measure_before_the_ranking]], [[feedback_managed_surface_edit_discipline]].

**2026-09-02 addendum (cost: two refused pushes for the integrator):** `.claude/skills/*/SKILL.md` and
the root `CLAUDE.md` are PROJECTIONS of `.epr-meta/elohim/packages/*` — editing the runtime file directly
makes the pre-push `package-projections.mjs verify` leg report it stale and refuse the whole batch. Edit the
package JSON body, then `pnpm run elohim-agent:packages:project`; never hand-edit the runtime SKILL.md /
gospel line. The `dev-lifecycle-script-sync` hook nag on hc-mesh.sh routes to the PACKAGE, not to SKILL.md.

