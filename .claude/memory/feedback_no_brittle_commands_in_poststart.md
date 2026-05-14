---
name: feedback-no-brittle-commands-in-poststart
description: "Devfile postStart failures abort whole-workspace startup; only add a command to postStart if it is idempotent, fast, and cannot reasonably fail"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: f5a2d454-b452-40db-b4fa-5bf6f18e9117
---

Do not add new entries to `devfile.yaml` `events.postStart` unless the command is idempotent, fast, and effectively cannot fail. Commands can still be defined under `commands:` (invokable on demand via `exec`) without being auto-run.

**Why:** A failure in any postStart command aborts workspace startup entirely — the whole DevSpace becomes unusable until the operator can intervene out-of-band. The cost of a flaky/new postStart entry is workspace-killing, not a degraded experience. Operator burned by this before; treats postStart as a high-risk surface.

**How to apply:**
- New tooling (mempalace setup, future MCP bootstraps, etc.) goes in `commands:` only.
- Operator (or a dedicated skill) decides when to invoke. Document the manual invocation step in the related agent/skill instead of auto-running.
- Existing postStart entries (`setup-pnpm`, `setup-vscode-cli`, `setup-claude-mcp`) are grandfathered because they have proven idempotence and harmless failure modes.
- If a new command really needs to run at session start, the right move is a session-start hook (`.claude/hooks/`) that fails soft, not a devfile postStart entry that fails hard.

See also: [[project_three_temporal_perspectives]] (mempalace is historian's substrate; setup is operator-driven now).
