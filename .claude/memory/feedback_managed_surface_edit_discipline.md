---
id: feedback-managed-surface-edit-discipline
name: feedback-managed-surface-edit-discipline
description: CLAUDE.md/spec/plan/doc edits flow through the memkit cite tooling — the managed-surface registry + PRE/POST hooks enforce it; never hand-write slug/fingerprint/path
metadata:
  type: feedback
cites:
  - managed-surface-edit-discipline-design | the durable home of this lesson — registry design, forensic, and the open gap-items | sha256:80a33cc48810e061 | path: genesis/docs/superpowers/specs/2026-06-05-managed-surface-edit-discipline-design.md
  - .claude/scripts/_lib/managed_surfaces.py
---

Editing any project-managed memory surface (gospel CLAUDE.mds, specs/plans, doc-roots, memory entries) means
using the cite tooling — `cite-gen --seal`, `cite-describe`, `cite-propagate`, `--refresh` — never hand-writing
a slug, fingerprint, or path. The 2026-06-05 episode: rails were hand-cited into 4 CLAUDE.mds and needed two
operator corrections, because every hook hardcoded its own scope (doc-roots only) and nothing fired pre-edit.

**Why:** strategy–scope drift — the operator's "the tooling exists" model was true at strategy level but each
enforcement surface privately re-encoded a narrower scope. Scope must live in ONE registry.

**How to apply:** trust the PreToolUse injection (`managed-surface-context.py`) — when it names a discipline +
tool, use that tool. When adding a new managed home, add it to `_lib/managed_surfaces.py` (the cross-check test
fails if it diverges from `subject-routing.yaml`), never to an individual hook. `status: stale` on a cite is a
re-verify queue → `cite-gen --refresh <doc>` after re-verifying, not a thing to hand-edit. [[project-memory-in-repo-two-tier]]
