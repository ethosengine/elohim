---
id: "backlog-session-envelope-resume-after-restart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A resumed harness session can never reopen its envelope after a workspace restart"
slug: "session-envelope-resume-after-restart"
written: "2026-10-26"
author: "claude-opus-5-5 (device-runtime, slice M landing)"
status: "open"
priority: "high"
tags: [session-envelope, device-runtime, resume, berth, security]
cites:
  - elohim/eprfs/device-runtime/src/envelope.rs
  - elohim/eprfs/epr-cli/src/session_envelope.rs
---

# A resumed harness session can never reopen its envelope after a workspace restart

## Observed

Found on shem on 2026-10-25, after its workspace restarted at 22:19:51Z. Claude Code reuses a
session id when it resumes a conversation in a new process. For that id, `epr session open` refuses
`envelope-owner-mismatch` (`Envelope::validate_owner`, `envelope.rs`). That holds even after
`epr session gc` has closed the dead owner's envelope: `closedAt` is set, the owner pid is gone, and
the boot id differs. So a resumed session can never reopen its envelope, and every berth claim it
makes is refused. The session on shem worked around it by declaring
`ELOHIM_SESSION_ID=<id>-r2` with its own process as owner. That workaround breaks agreement with
anything keyed by the runtime's session id, such as the head-notice hook's
`sessions/<payload session_id>` pin.

## Why this is not a one-line fix

The refusal follows a deliberate rule: an envelope is "bound once and never reopened after
closure". Replacing a closed envelope under the same id without isolating its facets could inherit
covenant clauses, berth leases and moorings. The cure must preserve history and inherit no authority.

## The cure (R3, review round 1 ruling)

A resumed session opens a successor generation of the same id. It is admitted when the predecessor
is closed and its owner proven dead; the runtime-ancestor requirement guards against accidental
takeover by a stray declaration; a successor inherits no authority (clauses re-offered, no facet
transfer), which is what makes a same-principal takeover harmless.

- Closure means `closedAt` or the durable seal checkpoint is present. `owner_dead()` must prove death
  by an absent pid or a different boot. Unknown liveness is never death.
- `Source::RuntimeAncestor` is an accident guard, not authentication. Claude Code assigns its id
  after exec and exports it to children; its exec-time environment does not name that id. A child
  can export the same variable and resolve as `RuntimeAncestor`. Runtime and children run as the
  same user and can read the same device key: they are one principal on this device.
- A stray outer-shell `ELOHIM_SESSION_ID` declaration must not silently become a successor.
  Successor admission proves owner death through real `/proc`; only `EPR_UNDER_TEST=1` permits
  a caller-supplied proc root for that proof. `EPR_PROC_ROOT` and `EPR_AGENT_RUNTIMES` remain
  production isolation inputs everywhere else, including ordinary envelope liveness and GC.
  This switch prevents accidents; it is not a same-user security boundary.
- Predecessor records and seals remain immutable history. The successor records the exact
  predecessor bytes' raw CID and seal reference.
- Covenant clauses are keyed by generation, never inherited, and must be re-offered.
- Berth leases and moorings are not transferred. Dead-session leases remain governed by berth's
  existing expiry and takeover rules, which already let any session take over an expired lease.
  Validate the requested store before successor publication.

Regression at the native seam: open X under P1, prove P1 dead, and close through GC. A child of P2
exports X while P2's exec environment remains empty. Generation 2 opens with no predecessor clause
or transferred berth facet; archived records, seals and old berth bytes remain unchanged. Bare
declarations, live or unknown owners, and invalid berth destinations refuse without publishing.
The successor's pin overwrites the old pin at the same id path consumed by the head-notice hook.
