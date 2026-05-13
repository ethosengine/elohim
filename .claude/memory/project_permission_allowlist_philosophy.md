---
name: Permission allowlist philosophy — broader is better, within safety rails
description: In this project, the .claude/settings.local.json allowlist is the trusted authority surface for agent commands. Make it as broad as safely possible, not more numerous.
type: project
originSessionId: 9a934a92-144d-4415-9d43-14fcb046e2db
---
When managing Claude Code bash/tool permissions in this project:

**The allowlist is trusted.** Everything in `.claude/settings.local.json` allow list is considered pre-approved by the user. The user said: "everything in the settings.local.json allow list should be allowed, we allow things... all the time."

**The pain is specificity, not trust.** Each new parameter variant (e.g. `RUSTFLAGS="" cargo clippy -p foo` vs `RUSTFLAGS="" cargo clippy -p bar`) creates a new prompt even when semantically equivalent. Hundreds of near-duplicate entries accumulate.

**The fix is generalization within safety rails.** Use broader patterns (wildcards, glob) with awareness of which command families are safe to wildcard:

- **Broadly safe to wildcard:** cargo, pnpm, npm, vitest, jest, pytest, eslint, stylelint, prettier, tsc, rustc, rustup
- **Partial wildcard (subcommand-scoped):** git (add/commit/diff/status/log/show/fetch/pull/rebase-in-progress — safe; push, reset --hard, branch -D, checkout --, force — prompt or deny), kubectl (get/describe — safe; apply/delete/rollout — prompt)
- **Never wildcard:** rm, sudo, curl, ssh, aws, gcloud, gh delete, npm publish, cargo publish

**How to apply:**
- When adding permission entries, prefer the broadest safe pattern over the most specific literal command.
- Before agentic-developer shifts, run a pre-shift generalization pass on the allowlist: cluster near-duplicates, propose generalizations, user approves once.
- Treat the `less-permission-prompts` skill as the starting point; extend its patterns with write-verb patterns scoped by safety taxonomy.
- Never silently widen the allowlist. Every generalization requires explicit user approval, but the proposal surface should be bulk (10 entries collapse to 1) rather than per-command.
