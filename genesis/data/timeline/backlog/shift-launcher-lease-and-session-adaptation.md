---
id: "backlog-shift-launcher-lease-and-session-adaptation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Bounded shift launcher is dormant until it speaks dev's steward-scoped lease protocol and native session envelope"
slug: "shift-launcher-lease-and-session-adaptation"
written: "2026-10-28"
author: "port/b-autonomous pre-landing review (steward of record: human:matthew)"
status: "backlog"
priority: "high"
jobs: []
relatedNodeIds: []
tags: [agentic, shift, lease, session, dormant]
---

# Shift launcher: what remains before it may run unattended

The launcher ported from `feat/autonomous-delivery-20261022` (`genesis/agentic/shift*.py`, `SHIFT.md`)
is landed but **dormant**: the devfile `postStart` entry `recover-authorized-shifts` is removed (the command
stays defined, run by hand), and nothing else registers it. Pre-landing review (Codex, three lenses, each
finding refuted-then-grepped) found three majors that the port did not adapt; each must be fixed and proven
before it is re-armed.

1. **No `dev-integration` lease for publication.** `genesis/agentic/shift_lease.py:37` delegates only
   `{cargo, mesh, disk-heavy}`; `.husky/pre-push.bash:261` needs `dev-integration` (plus tag-dependent
   `alpha-fleet` / `jenkins-dispatch`). The push worker inherits the guardian proxy, so pre-push refuses.
   Needed: delegate the full steward-scoped lease set dev's pre-push claims.
2. **Synthetic session conflicts with native session ownership.** `shift_supervisor.py:154` moors a synthetic
   `shift-<id>` session; `elohim/eprfs/epr-cli/src/session_envelope.rs:353` refuses with
   `envelope-owner-mismatch`. Needed: bind readiness and guardian processes through the current native session
   lifecycle instead of minting a session id.
3. **Readiness baseline lacks host lease delegation.** `shift.py:117` and the mocked `shift_lease_test.py` hide
   that nested berth checks read namespace-local PIDs as dead. Needed: delegate through the host lease and
   replace the mock with a real nested-namespace check.

Re-arm only when all three have tests that do not mock the lease/session layer, then restore the `postStart`
entry. Habit context: `measure-runs-on-a-peer`, `dataplane-convergence` evidence 2026-10-22/23 describe the
unfinished publication/CI/rehearsal obligations.
