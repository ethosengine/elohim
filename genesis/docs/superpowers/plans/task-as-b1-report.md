---
id: task-as-b1-report
cites: []
status: claimed-not-verified
landed_commit:
verified_by:
concerns: gate orchestrator red in this worktree for environmental reasons (see body)
actor: agent:b1@claude-sonnet-5-5
---

# Task B1 report: notes refs skip the pre-push ref legs

## What changed
- `.husky/pre-push.bash`: new `is_notes_ref()` (line ~107). The change-detection loop (~832) prints `notes: <ref> skipped (not a commit tree)` and `continue`s for `refs/notes/*` before the style reach gate, PUSH_TARGETS accumulation, DEV/LAST sha tracking and the `git diff` legs. The LIVE_SHA/LIVE_TARGETS loop (~748) skips notes silently (the message is printed once by the other loop). The berth loop (:177) and Phase L loop (:695) already only act on refs/heads/dev|main and are unchanged. Branch refs are untouched.
- `genesis/orchestrator/prepush-notes-skip.test.mjs`: new test.

## Tests
- `notes_ref_is_skipped_and_branch_ref_still_gated_by_the_pushed_ref_loop`: PASS. It extracts the real ref loop from the hook, runs it in a mkdtemp scratch repo with a stub rust-style.sh, one notes ref and one branch ref; asserts one skip line, no style call on the notes sha, the branch still style-gated and diffed.

## Gate
`just gate orchestrator`, log `.../evidence/gate-B1-orchestrator.log` ends `EXIT=1` (earlier dirty-tree run kept as `gate-B1-orchestrator-run1-dirty.log`). The new test passed inside it. Seven failures, none in this change: six `.husky/commit-msg.test.mjs` cases fail with ENOENT `.husky/_/h` (the untracked husky install dir is absent in this worktree) and one `gate-runner.test.mjs` "dirty external dependency" case. The preflight also reported submodules not checked out. Orchestrator should run the gate from the main checkout/shem, where `.husky/_` exists.

## Deviations
None to the fix. Gate not green here; see above.
