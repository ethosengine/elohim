---
name: feedback_pipe_tail_masks_exit_code
description: Piping cargo/git/test commands to tail/head/grep masks the real exit code — a failing gate reports green.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 21dcbb18-990e-405a-93d6-2beb9577827a
---

`cargo test ... 2>&1 | tail -60 && echo OK` reports success even when cargo FAILS,
because a shell pipeline's exit status is the LAST command's (tail), not cargo's.

**Why:** Without `set -o pipefail`, `cmd | tail` → exit = tail's (0). The `&& echo OK`
then runs, and a `run_in_background` Bash reports exit 0. Bit me 3× in one session:
a storage gate reported "GATE_DONE_OK" while `cargo test` had actually failed to
compile a test target, and two `git push` runs reported PUSH_EXIT-ish success while
the pre-push hook had FAILED (origin never moved — only a later `git rev-parse
origin/dev` comparison caught it).

**How to apply:** For any piped gate/push where the exit code matters, either
`set -o pipefail` at the top of the command, OR check the real result explicitly
(e.g. `git push ...; echo "EXIT=$?"` without a pipe; then verify `git rev-parse
origin/dev == HEAD`). Don't trust a trailing `&& echo OK` after a `| tail`.
Related: [[feedback_spare_no_expense_intelligence]].
