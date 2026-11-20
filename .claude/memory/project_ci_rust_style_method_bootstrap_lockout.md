---
name: project_ci_rust_style_method_bootstrap_lockout
title: "Rust-style method bootstrap locks the orchestrator until the baseline moves"
description: "elohim-orchestrator/dev #2220+ (2026-11-05/06): a discipline-method re-pin makes every CI run refuse 'discipline method differs from this installed checker' because the baseline (GIT_PREVIOUS_SUCCESSFUL_COMMIT) only advances on SUCCESS; cure = move the baseline to the bootstrap commit in genesis/orchestrator/rust-style-ci.sh (landed)."
metadata:
  node_type: memory
  type: project
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-06T11:26:27.301Z
---

**What happened (2026-11-05 02:21Z → 2026-11-06):** commit `dc01152d8` ("the checker stops compiling the tree it polices — one-time bootstrap") pinned a new discipline method in `.epr-meta/rust-style-policy.json`. From orchestrator/dev #2220 on, the Rust Style Gate failed on every build: `genesis/orchestrator/rust-style-ci.sh` measures from `GIT_PREVIOUS_SUCCESSFUL_COMMIT` (#2219 = `944c1c33c`, old method), `repository.rs` `before.policy()` verifies the BASE snapshot's pinned method against the checker built from HEAD, refuses, and because the baseline advances only on SUCCESS the job can never climb out. Local gates never saw it: `rust-style.sh` measures from merge-base(origin/dev, HEAD), which already carries the new method. Nothing dispatched for a day (no epr publish, no DNA, no edge) and the "expected red once" note on the bootstrap commit hid it.

**Cure (landed in genesis/orchestrator/rust-style-ci.sh; fixture in genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/ci-lockout-cure/):** when the trusted baseline pins a different method than HEAD, it moves the baseline to the earliest first-parent ancestor pinning HEAD's method (the bootstrap commit) and prints why. No tag, no re-pin, no bypass; the whole range since the bootstrap is still measured. The auto-mode classifier refuses an agent edit to that script as "CI Bypass" even when the operator delegated the call.

**How to apply:** any future method re-pin ships WITH this baseline move (or the orchestrator stays red until a human notices); when orchestrator/dev reds at the style gate with "discipline method differs", compare `git show <base>:.epr-meta/rust-style-policy.json` `.discipline.method.cid` against HEAD's before touching code. Related: [[feedback_big_crossing_working_code_first_verify_after_dust_settles]], [[feedback_push_branch_discipline]].
