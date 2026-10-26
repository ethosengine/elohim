---
id: "backlog-security-session-successor-admitted-before-facet-release"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A successor session is admitted at the seal checkpoint, before its predecessor's berth facets are released, so it can revive the dead session's leases"
slug: "security-session-successor-admitted-before-facet-release"
written: "2026-10-26"
author: "claude-opus-5-5 (device-runtime R3 follow-ups; steward: human:matthew)"
status: "backlog"
priority: "medium"
area: "device-runtime/session-envelope"
domain: "protocol"
review_trigger: "Re-review R3's threat model when this lands, or before any change that lets a session carry authority across generations"
relatedNodeIds:
  - "backlog-session-envelope-resume-after-restart"
tags: [security, hardening, session-envelope, device-runtime, berth, threat-model]
cites:
  - elohim/eprfs/device-runtime/src/envelope.rs
  - elohim/eprfs/epr-cli/src/session_envelope.rs
  - genesis/agentic/bin/berth
---

# A successor session is admitted before its predecessor's berth facets are released

## Threat model (R3, as landed in 66e8ea380)

On one device the harness runtime, its children and any other process of the same user are one
principal. `Source::RuntimeAncestor` is an accident guard, not authentication: any same-user process
can claim it. Taking over a dead session's id is acceptable only because **a successor inherits no
authority**. Covenant clauses are keyed by generation and re-offered, the predecessor's record and
seal stay as history, and no berth lease or mooring transfers.

Operator ruling 2026-10-26: the model is acceptable for now. Affirmation is held until this item
lands and the model is re-reviewed. Raising the bar to real authentication (a key held outside the
runtime user) is a separate, unscheduled decision.

## The gap

Closure is two steps (`epr-cli/src/session_envelope.rs`, `close`):

1. checkpoint the covenant seal and save the record. `Envelope::is_live()` is false from here;
2. `berth unmoor`: release every lease the session holds, then delete its mooring.

Successor admission (`device-runtime/src/envelope.rs`, `Store::existing`) needs only
`!old.is_live()`, which step 1 already satisfies. If closure is interrupted between the two steps,
the mooring and leases remain under the session id. Examples of an interruption: a workspace restart
during `epr session gc`, a killed process, or an I/O or lock error inside `_release`. When the
successor then runs `berth moor`, `_moor_locked` reads the old mooring file and refreshes its pid.
Every lease naming that id is live again and held by the successor. That includes daemon-class leases
with no TTL, such as `mesh`. The no-transfer guarantee the threat model rests on is broken.

A failure to fold the release out to other devices does **not** open the gap. `_unmoor` releases
local leases and removes the mooring before `take_release_fold_failures` reports.

Exposure: narrow. It needs an interrupted closure, and only same-user code can act on the result,
which that code could already do by opening its own session. It matters because it falsifies the
claim the threat model depends on, not because it opens a new principal.

## Fix direction

Pick one, test the half-closed state either way:

- **Admit only on completed release:** require `closed_at` (set after unmoor) for successor
  admission. A half-closed predecessor refuses a successor until a retried close finishes. Close
  retries are idempotent and resume at the checkpoint. The cost is that a predecessor whose unmoor
  keeps failing locally wedges its id until someone repairs it.
- **Finish release inside admission:** when the predecessor is sealed but not closed, the exclusive
  opener runs the predecessor's local unmoor under the same lifecycle guard before it archives and
  publishes generation n+1. This never wedges, and never revives a facet.
- **Belt and braces either way:** `berth moor` refuses to adopt a mooring file whose envelope
  generation differs from the caller's, or each mooring records its generation.

Regression: open X under P1, claim a no-TTL lease, seal the envelope, kill before unmoor, prove P1
dead, open X as P2. Assert that P2 holds no lease of P1's, and that `berth status` shows the lease
as stale, never held by X.

## Re-review

When this lands, re-read R3's threat model against the code: the admission predicate, every
facet store keyed by session id rather than by generation (berth moorings and leases, the
`sessions/<id>` pin, the flows sidecar), and the `EPR_UNDER_TEST` proc-root switch. Record the
operator's affirmation in `session-envelope-resume-after-restart.md`.
