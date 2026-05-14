---
name: self-reinforcing-path-bug-class
description: A walk-up-to-find-marker resolution can satisfy its own broken output and lock the bug in. Co-anchor on two independent markers (e.g., .claude/ AND .git) to break self-reinforcement.
type: feedback
---

Three memory-kit audit scripts were writing reports to `.claude/.claude/memory-kit/<date>/` instead of `.claude/memory-kit/<date>/`. Root cause: a `repo_root_from_file` walk-up looking only for `.claude/` would find `.claude/` as the result of its own buggy output (the doubled-path artifact contains `.claude/` as a child directory, which the next walk-up satisfies). The bug bootstrapped itself.

**Why:** Single-marker walk-up resolution is fragile when the marker is itself something the code may create incorrectly. Once the wrong location is written to, the next invocation finds the wrong location as a "valid" root.

**How to apply:**

- For any walk-up-to-marker resolution helper (repo root, project root, workspace root), **co-anchor on two independent markers**. Examples: require BOTH `.claude/` AND `.git` (strict mode), or `.claude/` AND `pyproject.toml`, or `package.json` AND `.git`. Single-marker is acceptable only when the marker is read-only/external to the resolution logic.
- After fixing the walk-up, **verify the fix by inspecting prior buggy artifacts**: in this case, leaving `/projects/elohim/.claude/.claude/memory-kit/` intact for operator inspection confirmed the cycle was real.
- This bug class is **invisible to tests that mock the filesystem** — the cycle only manifests across invocations that share a real filesystem.
- Bug-class signal: if you ever see "x.x" / "x/x" doubled in a resolved path (filename, directory, URL), suspect a self-reinforcing walk-up resolution.

Pairs with [[project_shared_lib_pattern]], [[feedback_cascade_hidden_test_surface]].
