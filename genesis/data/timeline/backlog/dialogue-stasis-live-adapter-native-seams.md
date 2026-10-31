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

## D2: rung 0 at PreToolUse — adapter closed; native frontier remains

**Adapter closed (2026-10-31, review M3, C1 "fix as proposed").** The previous path called only
`classify` and built a learned-form deny itself, bypassing native's incident and once-per-class
guards; a probe produced two denies with no incidents. `stasis_control.py` now routes PreToolUse
through `epr session stasis decide --live --session … --event -`, using the same process-group
budget (2 seconds) as PermissionRequest. It delivers native's deny message unchanged. Any other
result, nonzero exit (including 3), or timeout emits nothing. The adapter-direct learned-form
reshape is removed; ladder-off output remains byte-identical and shadow emits no rung-0 output.

**Evidence:** `python3 -m unittest discover -s genesis/agentic -p 'stasis_*test.py'` passed 147
tests, including no-allow, the off baseline, native-deny delivery followed by native `none`, two
silent calls with an active learned form and native `none`, and process-group timeout cleanup.
`python3 -m unittest discover -s .claude/hooks/tests -p 'test_dialogue_stasis_sense.py'` passed 51
tests. The sense-hook replant changed no files; package projection verification passed 2,350 checks.

**Remaining frontier (safe):** native rung 0 still reshapes only when an open incident already
exists with a matching effect class. PreToolUse usually precedes the PermissionRequest that opens
one. Without an incident native returns `none`, and the adapter stays silent even with an active
learned form. Opening or attaching a procedural rung-0 incident keyed by effect class remains
native work for slice 11 (L1). Its completion fixture must prove one reshape per effect class per
incident across repeated PreToolUse calls and restart, with a Refer on the second attempt. This
change closes the adapter bypass, not that native frontier.

## D3: the lock-ordering test relies on a fixed wait

`a_policy_edited_while_decide_waits_for_the_lock_is_inert_with_the_post_edit_cid` (epr-cli
`session_stasis.rs`) holds `journal.lock`, starts `decide --live` in a thread, waits 700 ms, edits the
policy and releases. The fifth review asked for a deterministic hook. The keeper ladder parked it at C1:
the blocking lock is taken before any policy read (`live.rs:161-171`), so there is no ordering defect,
and a contended-lock hook would restructure the production lock path just to test it. A regression that
read the policy before the lock would make the test flaky, not falsely green. **To close:** replace the
fixed wait with a bounded poll until the worker is observably blocked on `journal.lock` (for example
`/proc/<tid>/wchan` or a lock-holder probe), with no production change.
