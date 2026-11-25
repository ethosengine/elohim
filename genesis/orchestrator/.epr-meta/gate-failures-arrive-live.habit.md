---
epr-habit-version: 1
id: gate-failures-arrive-live
invariant: >-
  Gate failures reach the pushing session as they occur while later gates
  continue, and a completed run lists every failure. Live observations never
  decide the push; the gate process's exit status does. An ended run without
  closure reads interrupted, never passed.
status: red
active: false
checks:
  - "node --test genesis/orchestrator/gate-failure-lines.test.mjs (recognizer: cargo, rustc, clippy, rustfmt, vitest, tsc, eslint, node test, just; dedupe, caps, signals)"
  - "node --test genesis/orchestrator/gate-tap.test.mjs (bytes forwarded unchanged; exit status and signal carried; a green exit stays green whatever printed)"
  - "node --test genesis/orchestrator/gate-live.test.mjs (run log open/append/close, prune at open, follow waits past the previous push, unclosed reads interrupted)"
  - "node --test genesis/orchestrator/prepush-gate-live.test.mjs (the hook opens, closes, and closes interrupted from its exit trap)"
  - "python3 .claude/hooks/tests/test_gate_live_inject.py (every agent hears a red at its next tool call, one cursor per agent; a background push gets the Monitor command)"
  - "@concern:gate-failures-arrive-live"
guard: >-
  Preserve output bytes, exit status, and termination signals. Keep excerpts
  device-only. Require real-push evidence of first_red_s, continued later
  gates, and closure listing every failure before declaring the habit green.
refs:
  - genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md
  - genesis/a2o/features/devflow/gate-failures-arrive-live.feature
  - genesis/orchestrator/gate-failure-lines.mjs
  - genesis/orchestrator/gate-live.mjs
  - genesis/orchestrator/gate-tap.mjs
  - genesis/orchestrator/gate-runner.mjs
  - .claude/hooks/gate-live-inject.py
retire-when: "rakia-executor emits gate events natively and the gate-tap/gate-runner adapter is deleted"
---

DELTA 2026-10-27 (still RED; delivery no longer opt-in): push 9 (de1d22eab) proved the tap and the gap. The tap logged elohim-storage's red (`request_metrics_guard_observes_a_dropped_unfinished_request`, metrics.rs:7900) at 19:40:00Z, 7 min before the gate's verdict, but no agent heard it: delivery needed the pushing agent to arm `follow`, and it had not. Now the native `gate-live-inject` PostToolUse hook (every tool, package-mastered) adds each new red from the session's run to the next tool call with a cursor per agent, and a background push or gate gets the Monitor command for an idle wait. `gate-live.mjs unseen` formats and moves the cursor; the hook spawns node only when the run file grew (77 ms silent path, replayed push 9's storage red verbatim against the live log). Checks: gate-live 44/44, hook 6/6. Green still needs a real push whose first red reaches the agent mid-run.

DELTA 2026-10-26b (still RED; evidence bar raised): the orchestrator gate now runs as a local placement Task (`adc2a2059`, `d453d99e7`); gate-verdict carries derivation {method, input, hit} and Answered|Unanswered, and an interrupt closes the run `interrupted` with three parts. Scratch-clone proof: miss 469 s, derivation hit 23 s with no child, SIGTERM Unanswered with no evidence recorded. Green now needs, on a real push: a red delivered while later gates still run (`first_red_s` well under `value`) AND a derivation hit on the corrected re-push. Story revised (reuse + interrupt scenarios): blind-reader rounds 3 and 4 (new readers), round 3 READY 1 correctness (reuse title overclaimed) · 3 interpretability · 5 preference, all correctness and interpretability repaired; round 4 READY 0 correctness · 5 interpretability · 2 preference, 4 repaired, title scope declined (the habit names the live-failure promise); about 113k reader tokens.

DELTA 2026-10-26 (born RED; checks green, invariant unproven on a real push): L0 slice in the working tree. Fixture e2e (gate-runner CLI, tapped, two cargo FAILED lines then 4 s more work, exit 101): `follow --session` printed the first RED 0.6 s after the gate started and the second at 2.6 s; the gate exited at 6.5 s; verdict `failure_class=code`; run closed `failed`. Green waits on a real push's `first_red_s` against its `value`.

DELTA 2026-11-12 (still RED; no push requested): native Rust gates now collect workspace warnings through cargo-lint and continue fmt/lint/test through gate-steps, with compiler helpers keyed in the closure and gate-tap remaining the sole live emitter; eprfs candidate ingestion stays behind all-green checks. Evidence: focused node:test 199/199 (including real Cargo verdict equivalence, step continuation under bash -e, and closure regression checks); the two-member dependency fixture reports first + second + integration lints (old EXIT=101, new EXIT=1), while clean and outside-dependency-only warnings pass both and compile errors retain 101; Rust 1.100 blob-store reports 1 old / 2 new rendered instances of the same fetch_update finding (EXIT=101/1). Deliberate-red run 20261112T210245Z-0088755ecd2c-059ea1 recorded a lint at 21:02:48.248Z while cargo-clippy, cargo and clippy-driver were alive, before lint step EXIT=1 at 21:02:48.799Z; both test variants passed, the temporary edit was restored byte-for-byte, and the restored gate passed. Raw logs and process observations: genesis/a2o/reports/recovery/lint-fail-at-end/.

DELTA 2026-11-18 (status unchanged; the sibling path is on dev and its first live test found a red that sends nothing): `7fda3d482` and `94e768892` put sibling subscription, delivery and `follow --sibling` on dev. Shem subscribed to ethosengine's `refs/heads/scratch/*`; at 10:43:04Z ethosengine failed `gherkin-prepush-lint` (exit 1, 2 s) and no red was sent: the tap recognises no failure line in a Gherkin parse error and the emitter fires only on a recognised line. The fix (a failed step with no recognised line records one synthetic failure event) is in flight on ethosengine. An independent review of `94e768892` left six partial findings (genesis/local-dev/native-throughput/codex-e4-review.receipt.md).

DELTA 2026-11-18b (status unchanged until the fix is on dev; a sibling's red crossed devices in 4.9 s): with the stand-in failure fix on scratch head `5804e04b3`, ethosengine's `gherkin-prepush-lint` step failed at 10:50:03.608Z and shem's `gate-live.mjs follow --sibling` printed `SIBLING RED … red #1 +7s; 5 gate(s) still to run` at 10:50:08.474Z. A `SIBLING FINAL` for the earlier run arrived 5.6 s after it was recorded. Log genesis/local-dev/native-throughput/dod4-follow-2.log.

DELTA 2026-11-21 (status unchanged; a false verdict on the live run, found on a real push and closed): while the row 43 push ran its orchestrator gate, the push's own live run received five start and verdict pairs for a fixture project named eprfs, and a listener printed "eprfs PASSED 0s" five times for a gate that had not started (run 20261121T053143Z-5535405d45d6-31d8da, seq 28 to 33; the same pairs are in two runs of that morning). `cargo-affected.test.mjs` spread its environment into a fixture gate and `gate-runner` appends to whatever `GATE_LIVE_RUN` it is handed; the same leak could have carried a false red. The suite's tree half now starts without the variable (`suite-run.mjs treeEnv`) and that test clears it; a test is red with the fix reverted, and a census of every orchestrator test file with a scratch run found no other writer.

DELTA 2026-11-25 (status unchanged; a requested stop now ends the running gate, and refuses the push up to a stated last stop point): on 2026-11-25 a landing refused at preflight (fleet-roll RED) was stopped with `gate-live.mjs stop`, and the push then spent about 100 more minutes finishing `elohim-storage` for a closure that could never land. `stop --now` now ends a gate whose recipe is still running: the tap starts each gate command as the leader of its own session and group and signals that group, TERM then KILL after `GATE_STOP_GRACE_MS` (default 10 s), only while a process verified as the gate's (pid and start time) holds the session (`gate-stop.mjs`). Whether the gate WAS stopped is read from the leader's own exit against the signals the stop sent the leader itself, each stamped before the exit (a death by that signal, or just's 128+N for it); a code of its own, a different signal, or an exit before the delivery keeps its red or green, and the runner trusts only that call (runner matrix 15 cells). A stopped gate records no closure evidence, gate-verdict@1, cycle observation or reading (what it already wrote or put is not undone) and closes the run `interrupted`. Once a stop is asked, the tap waits for the gate's output for a bounded time only and names any process outside the tree that still holds it. A stop refuses the push until the run's last stop point (`stop_point`: before a hand-over's or non-landing push's exit 0, and before a dev landing claims dev-integration); after it, `stop` says the push is proceeding and exits 1, and a stop appended after the mark counts for nothing; an unmarkable point says so and keeps stops honoured. Three independent reviews (cafd300d1, 430673057, 1bcf0095e) closed in the commits after. Checks: gate-stop 36/36, prepush-gate-live 23/23, gate-live 80/80, gate-live-schema 105/105, gate-tap 23/23; `just gate orchestrator` green.
