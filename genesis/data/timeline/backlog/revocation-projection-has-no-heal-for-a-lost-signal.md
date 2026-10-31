---
id: "backlog-revocation-projection-has-no-heal-for-a-lost-signal"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "An accepted revocation whose post-commit signal is lost leaves its commitment reading live here, and nothing heals the projection"
slug: "revocation-projection-has-no-heal-for-a-lost-signal"
written: "2026-10-29"
author: "claude (provide revoke renewal lane, Codex review round 3)"
status: "open"
priority: "low"
severity: low
nodes: [alpha, household]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [elohim-storage, provide-reconcile, mishpat, commitments, projection, reconcile, lane-background]
cites:
  - elohim/elohim-storage/src/db/mishpat_revocations.rs
  - elohim/elohim-storage/src/services/provide_reconcile.rs
  - elohim/elohim-storage/src/services/mishpat_mirror_backfill.rs
  - genesis/data/timeline/backlog/provide-revoke-arm-dormant-behind-namespace-mismatch.md
---

**What.** A `revokes-commitment` reaches this peer's projection only through its post-commit signal
(`signals::handle_mishpat_signal` -> `mishpat_revocations::record_projected`). If the conductor accepts the
revocation (`mishpat_revocations.issue_outcome = 'issued'`) and the signal never arrives (the storage signal stream
was down when the action committed), the target row in `mishpat_commitments` keeps `revoked_at = NULL`. The
prioritizer keeps serving it and the bounds validator keeps accepting it, although the DHT holds the revocation.

**What exists now.** The removed-pin arm reports such a target on every pass in `RevokeSweep::unprojected`, with one
warning a process, once its last attempt is older than `RETRY_SETTLE` (15 minutes). It does not submit again. The
outcome is known, and a second `create_commitment` would be one more action on the source chain for an entry that is
already committed. What is missing is the projection, so the cure belongs to the projection side, not the submitter.

**What is missing.** A level-triggered heal for the mishpat revocation projection, in the shape of
`services::mishpat_mirror_backfill`: read this agent's own `revokes-commitment` actions from the mishpat cell and
project any the local table lacks. No such sweep was found in `elohim-storage` on 2026-10-29. It would also cover
revocations made by the HTTP un-pin route while the arm is off, which nothing reports today.

**Retire when.** A heal projects a revocation committed while the signal stream was down (a test that drops the
signal and asserts `revoked_at` after the sweep), or the arm's `unprojected` count is measured at zero across the
household for a week with the arm on.
