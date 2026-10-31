---
id: "backlog-dialogue-stasis-live-adapter-native-seams"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "dialogue-stasis live: the adapter and native decide keep separate park records, and the adapter's rung-0 path lacks the once-per-class check"
slug: "dialogue-stasis-live-adapter-native-seams"
written: "2026-10-31"
author: "claude-opus-5-5 (dialogue-stasis session 4ec0c39f); both questions ruled by the stasis keeper ladder at C1 (park)"
status: "open"
priority: "medium"
tags: [dialogue-stasis, stasis-loop, rung-0, park, adapter, native, precondition]
cites:
  - genesis/agentic/stasis_control.py
  - elohim/eprfs/device-runtime/src/session_stasis/live.rs
  - genesis/docs/superpowers/specs/2026-10-30-dialogue-stasis-loop-design.md
---

# Two seams between the Python adapter and native `decide --live`

Both were left open by the live-decision leg (commit 81c7ba6bc). The stasis keeper ladder ruled on
each at C1 (haiku), and both were parked. The traces are in
`genesis/local-dev/dialogue-stasis/ladder-decisions-live/results.json` (local).

## D1: two park records, in two streams

Native `decide --live` (`live.rs:229-241`) appends the authoritative Permit/park consultation to
`events.jsonl`. The adapter (`stasis_control.py:387-390`) separately writes a delivery park row
(`delivery.jsonl`, key `park:<incidentId>`) and a stall-closed row under `delivery_lock`. Each stream
deduplicates only locally.

The ladder rejected "the adapter stops journaling": `Delivery.pending()`
(`stasis_control.py:142-153`) and the Stop batch logic depend on the delivery row, and native writes
no delivery row. **To close:** give the delivery vocabulary a native home, so one record serves the
fold and the delivery batch. Alternatively, derive the delivery row from the native consultation by
its seq. A replay test must show one park per PermissionRequest across both streams.

## D2: rung 0 at PreToolUse, a precondition for slice 11

Native rung 0 (`live.rs:249-298`) reshapes only when an open incident already exists with a matching
effect class. PreToolUse usually runs before the PermissionRequest that opens one, so native rung 0
rarely fires.

The ladder found the adapter's own PreToolUse path (`stasis_control.py:393-424`) emits the
learned-form deny directly:
- it never calls native `decide`;
- it writes no consultation record;
- it has **no once-per-effect-class-per-incident check**, so it can repeat a reshape.

That contradicts the spec's "one same-effect reshape per shape class". Today it carries no risk:
the policy holds no learned forms, so the path cannot fire.

**Precondition:** this must be closed before slice 11 (L1) promotes the first learned legible form.
Route PreToolUse shape-class calls through native `decide`, which opens or attaches a procedural
rung-0 incident keyed by effect class and enforces the once-per-class limit. Alternatively, keep the
adapter path disabled until it does. **To close:** a fixture with one active learned form shows
exactly one reshape per effect class per incident, across repeated PreToolUse calls and a restart,
with a Refer on the second attempt.
