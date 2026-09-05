---
id: concurrent-agent-attribution-task-1-brief
title: Preserve exact actor claims through existing authoring verbs
status: active
class: process-meta
gap: plans__2026-09-05-concurrent-agent-attribution-plan#1
actor: agent:implementer@gpt-6
cites:
  - "concurrent-agent-attribution-design | The exact claim-reference and compatibility contracts for this bounded implementation. | sha256:e1ba98b4f3ca4b47 | path: genesis/docs/superpowers/specs/2026-09-05-concurrent-agent-attribution-design.md"
  - "concurrent-agent-attribution-plan | Station 1 scope, gate, evidence requirements and deferred work. | sha256:af37cc0afeb713a8 | path: genesis/docs/superpowers/plans/2026-09-05-concurrent-agent-attribution-plan.md"
---

Implement station 1 of the governing plan and specification. Read both in full, especially the
primitive reuse, intentional-friction and station 1 compatibility contracts. This is the
existing attribution lookup preserving information it already receives; callers gain no new
registration or argument ritual. Add no EPR/REA model, atom, sidecar, CLI flag or dependency.

Files in scope:
- `elohim/eprfs/epr-cli/src/govern.rs`
- `elohim/eprfs/epr-cli/src/flow/note.rs`
- `elohim/eprfs/epr-cli/src/flow/claim.rs`
- `elohim/eprfs/epr-cli/src/flow/fulfill.rs`
- `elohim/eprfs/epr-cli/tests/actor_claim.rs`
- `elohim/eprfs/epr-cli/tests/flow_edges.rs`
- `elohim/eprfs/epr-cli/seam-registry.yaml`

Use existing registry rows for the changed attribution surfaces. Extend existing integration
tests; tests must inspect actual stored records and governance payloads, not compare the helper
to itself. Independently construct expected ActorClaim CIDs, cover same-role workers, model
supersession, all three flow verbs, direct override and missing/corrupt fallbacks. Do not change
concurrent storage semantics in this station or claim real three-harness lifecycle support.

Use `epr flow context` for each scoped file to get its gate. The manifest gate is `just gate eprfs`;
claim/release the cargo berth and report exact EXIT lines. No commits, staging, install or push:
the reviewer reads the scoped worktree diff from the base. Preserve unrelated worktree changes.
Write `task-1-report.md` in this directory with the skill's required frontmatter and evidence.
Execute the skill's single terminal flow verb. If tools or gate infrastructure fail, diagnose
safe local alternatives and report the remaining limitation accurately.
