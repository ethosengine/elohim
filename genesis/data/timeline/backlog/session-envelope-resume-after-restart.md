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
closure". `Store::open` lets neither another owner nor a closed session take over an id. That rule
is what keeps a dead session's id from being picked up by any process that declares it. Session-keyed
records include covenant clauses, berth leases and moorings, and a successor that simply replaced a
closed envelope would inherit all of them. Shem proposed letting `open` replace a closed envelope
whose owner `owner_dead()` proves dead, keeping the old seal. That cure is sound only if the
successor's claim to the id is as strong as the original's.

## The cure (to be designed and reviewed)

A resumed session opens a successor generation of the same id, never an overwrite:

- It is admitted only when the old envelope is closed (seal checkpoint present) and
  `owner_dead()` proves its owner dead, by an absent pid or a different boot.
- The successor's identity must come from the runtime itself: source `runtimeAncestor`, where the
  runtime process's own environment names the id. A bare `ELOHIM_SESSION_ID` declaration is not
  enough, since anyone can write one.
- The predecessor's record and seal are kept, as a generation under the id rather than replaced in
  place. The successor records the predecessor's record CID and seal.
- Facet writers (covenant, berth) see the successor. Whether its covenant clauses carry over or
  must be re-offered is the open design question. Default to re-offer, so nothing is inherited
  silently.

Regression, at the native seam: open as runtime session X under owner P1, close it and GC it with
P1 dead. A new owner P2 whose runtime environment names X opens a successor, the old record and seal
survive, and a bare declaration of X from an unrelated process is still refused.
