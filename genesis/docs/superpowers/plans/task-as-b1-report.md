---
id: task-as-b1-report
cites: []
status: landed
landed_commit: 3b266e41d
verified_by: "orchestrator (Claude Opus 5.5): diff read + `just gate orchestrator` EXIT=0 at dc0a8b2f4 with EPR_BIN at bafyreif5zxb6… (evidence/gate-B1-orchestrator.log, 2026-11-07T23:23Z); the real notes push and the gospel change follow at landing"
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
`just gate orchestrator`, log `.../evidence/gate-B1-orchestrator.log`: **EXIT=0** (2026-11-07T23:23Z, run by the orchestrator with `EPR_BIN` at the shem-built epr `bafyreif5zxb6…`). The earlier reds were environmental and are kept as `gate-B1-orchestrator-run1-dirty.log` and `-run2-envred.log`: the worktree lacked node_modules (`.husky/_/h` ENOENT in six commit-msg cases), and shem's installed epr predated abd66bdb1's byte-profile closure CID (the "dirty external dependency" case).

## Deviations
None to the fix. Gate not green here; see above.
