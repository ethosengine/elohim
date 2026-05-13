---
name: dev branch is integration target — no PR needed for batch merges
description: dev IS the development branch in this project; merging feature branches to dev locally is the norm, not bypassing review
type: feedback
originSessionId: f89e70a8-12f4-4fbc-a7c5-589cce37e5b0
---
When merging feature branches (e.g. `feature/epr-phase-2b-batch-a`) into `dev`, do a local merge — do NOT push and create a PR. `dev` is the project's development/integration branch, not the protected default. PR review happens at `dev → main`, not at `feature → dev`.

**Why:** This project's branch model uses `dev` as the integration target. Feature work lands on `dev` continuously; release-grade review happens later when `dev` is promoted toward `main`. The "merging into default branch bypasses review" framing doesn't match this project — `dev` is explicitly the catch-all for batch work.

**How to apply:** When `superpowers:finishing-a-development-branch` Option 1 (merge locally) is chosen with target = `dev`, just do the merge. Do not redirect to Option 2 (push + PR) on size grounds. If a permission guard blocks the merge, surface the guard to the user and ask them to authorize the action (one-shot or via settings) — but the right answer is the merge, not the PR.
