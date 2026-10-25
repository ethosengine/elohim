---
id: "backlog-brit-covenant-session-segment-aliasing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit's covenant refs let a session label nest under another session's clauses"
slug: "brit-covenant-session-segment-aliasing"
written: "2026-10-25"
author: "claude-opus-5-5 (device-runtime, pre-push head landing)"
status: "open"
priority: "medium"
tags: [brit, covenant, session-envelope, security, refs]
cites:
  - elohim/brit/brit-epr/src/elohim/refs.rs
  - elohim/eprfs/epr-cli/src/session_envelope.rs
---

# brit's covenant refs let a session label nest under another session's clauses

## Observed

Covenant clauses live at `refs/notes/brit/covenant/<encode_segment(session)>/<cid>`, and
`list_covenant_clauses(session)` reads every ref under `refs/notes/brit/covenant/<encoded>/`.
`encode_segment` (`brit-epr/src/elohim/refs.rs:270`) escapes `:`, `@`, `?`, `*`, `[`, `\`, `^`, `~`,
space, control characters and `..`, but neither `/` nor `%`. So:

- a clause put under session `victim/x` lands at `covenant/victim/x/<cid>`, inside the prefix
  `covenant/victim/` that session `victim` lists as its own;
- `a%3Ab` and `a:b` encode to the same segment.

## Guarded in epr since 2026-10-25

`live_guard` (epr's facet guard for actor claims, covenant record/check and flow claims) refuses
any session label that is not a P1 envelope id (`envelope::is_envelope_id`: ASCII letters, digits,
`_`, `.`, `-`), on every device. Before that it refused such labels only on a device that had an
envelope directory, and not at all on a fresh device. A caller of `brit-build-ref covenant`
directly is still exposed.

## Fix shape

In brit: percent-encode `/` and `%` in `encode_segment` (and decode them), or refuse a session
segment outside the P1 shape; list clauses for exactly one segment rather than a prefix walk. A
test puts a clause under `victim/x` and asserts `list --session victim` does not return it.

## Done when

`brit-build-ref covenant list --session victim` never returns a clause put under any other label.
