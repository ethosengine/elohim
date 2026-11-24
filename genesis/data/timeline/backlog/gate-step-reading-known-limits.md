---
id: "backlog-gate-step-reading-known-limits"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The shadow gate-step reading has six known limits that keep its numbers from being accounting"
slug: "gate-step-reading-known-limits"
written: "2026-11-24"
author: "claude-opus-5-5 (ethosengine; CD-reading lane, integrator shem)"
status: "backlog"
priority: "low"
jobs: [pre-push]
cluster: "push-gate-pays-once"
tags: [gate, reading, change-detection, shadow, telemetry]
---

**Context.** `gate-step-reading@1` (station CD-reading; spec
`2026-11-23-change-detection-one-reading-design`) records, per gate step and pre-push leg, its
seconds, closure key, whether the key moved since the last green, outcome and wait. Readings live
in `.eprfs/status/readings/gate-step.jsonl`, written only by `epr flow reading append` and read by
`epr flow report reading`. It is shadow telemetry: it selects nothing and changes no verdict. The
integrator's third review (2026-11-24) accepted it with these limits stated rather than fixed; the
report prints them above every output and `elohim/eprfs/epr-cli/src/flow/reading.rs` carries them
in its module doc.

**Known limits.**
1. `key_changed` compares with the last *recorded* green reading of that step on this device
   (reading.rs `derive_key_changed`). A dropped batch (lock contention, a timed-out append, a device
   whose pinned `epr` predates the verb) can make it wrong either way; it is `unknown` on a first run
   and after retention trims the last green row. Cure: derive it from the gate evidence store (the
   attested green closure), not from the reading log.
2. `ran` seconds exclude the cost lookup, the input shadow and final evidence recording, while
   `reused` seconds include them (gate-runner.mjs, the reuse and record paths). Cure: one timer
   around the same span for both outcomes.
3. Pre-push leg seconds are whole seconds (bash `SECONDS`). Cure: `EPOCHREALTIME`.
4. A torn tail (a write cut off before its newline) is joined by the next append and the joined
   line fails to parse, so the next row is lost with it. Cure: the append verb repairs a tail
   without a newline before writing.
5. An ordinary skipped row records near-zero time and can still be named the step that set the
   finish; only rows the hook synthesizes at exit are excluded. Cure: exclude skips from the
   finish-setter, or name it only among ran rows.
6. The on/off decision-equivalence test covers one non-reusable fixture. Cure: add a reusable
   (hit) fixture and a carry fixture.

**Probe.** `epr flow report reading` prints the limits header; a fixture for each cure turns its
limit into a passing test. Retire-when: limits 1, 2 and 4 are cured with tests (3, 5 and 6 are
cheap follow-ons), and the header lists only what remains.
