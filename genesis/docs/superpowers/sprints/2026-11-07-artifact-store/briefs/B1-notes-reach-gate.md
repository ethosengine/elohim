---
id: artifact-store-brief-b1-notes-reach-gate
status: active
---

# Brief B1 — brit receipts travel: the style reach gate skips notes refs (Sonnet 5.5)

Worktree `as-b1`, branch `artifact-store/b1`. Read `common.md`.

**Problem (evidence):** pushing `refs/notes/brit/*` is refused by pre-push because the rust-style reach gate treats each notes ref as a commit to style-check (regenesis-1 `STATE.md` 22:50Z, `push-notes-brit.log`). The gospel therefore carries an exception: "Until that gate skips `refs/notes/*`, the receipts stay local."

**Write-set:** `.husky/pre-push.bash` (the ref loop around `:683-701` and the style leg around `:843-845`), a new or existing hook test under `genesis/orchestrator/*.test.mjs`, and, after the fix is verified by a real push, the gospel package body `.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json` re-projected with `replant.mjs --compose elohim-root-gospel` (never edit CLAUDE.md or AGENTS.md by hand).

**Deliverables:**
1. A pushed `refs/notes/**` ref is skipped by every commit-tree leg (style first; check every leg that iterates pushed refs) with one printed line `notes: <ref> skipped (not a commit tree)`; branch refs are unchanged.
2. A test that drives the ref loop with a notes ref and a branch ref in a scratch decoy repository (never `GIT_INDEX_FILE=/nonexistent`-style paths that get created: use a temp dir) and asserts the notes ref is skipped and the branch ref still gated.
3. The orchestrator then runs `git push origin 'refs/notes/brit/*'` from shem with `EXIT=` logged (Git fallback: brit has no notes push); only after that passes does the gospel exception text change.

**Gate:** `just gate orchestrator`. **Reviewer:** Haiku runs the test and the real push; orchestrator reads the diff.
