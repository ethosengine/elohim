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
retire-when: "rakia-executor emits gate events natively and the gate-tap/gate-runner adapter is deleted"
---

DELTA 2026-10-26b (still RED; evidence bar raised): the orchestrator gate now runs as a local placement Task (`adc2a2059`, `d453d99e7`); gate-verdict carries derivation {method, input, hit} and Answered|Unanswered, and an interrupt closes the run `interrupted` with three parts. Scratch-clone proof: miss 469 s, derivation hit 23 s with no child, SIGTERM Unanswered with no evidence recorded. Green now needs, on a real push: a red delivered while later gates still run (`first_red_s` well under `value`) AND a derivation hit on the corrected re-push. Story revised (reuse + interrupt scenarios): blind-reader rounds 3 and 4 (new readers), round 3 READY 1 correctness (reuse title overclaimed) · 3 interpretability · 5 preference, all correctness and interpretability repaired; round 4 READY 0 correctness · 5 interpretability · 2 preference, 4 repaired, title scope declined (the habit names the live-failure promise); about 113k reader tokens.

DELTA 2026-10-26 (born RED; checks green, invariant unproven on a real push): L0 slice in the working tree. Fixture e2e (gate-runner CLI, tapped, two cargo FAILED lines then 4 s more work, exit 101): `follow --session` printed the first RED 0.6 s after the gate started and the second at 2.6 s; the gate exited at 6.5 s; verdict `failure_class=code`; run closed `failed`. Green waits on a real push's `first_red_s` against its `value`.
