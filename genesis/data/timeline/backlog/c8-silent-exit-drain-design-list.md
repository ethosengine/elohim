---
id: "backlog-c8-silent-exit-drain-design-list"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "C8 silent-exit drain — the design-tier findings from the 2026-08-03 seam-wide audit (mechanical tier shipped same shift)"
slug: "c8-silent-exit-drain-design-list"
written: "2026-08-03"
author: "pipeline-landing shift (integrator; audit by seam-wide C8/C14 sweep)"
status: "backlog"
priority: "high"
tags: [observability, concern-c8, concern-c14, dataplane, doorway, signals, silent-exit, debug-unreachable]
cites:
  - elohim/elohim-storage/src/services/head_adoption.rs
  - elohim/elohim-storage/src/signals.rs
  - elohim/elohim-storage/src/main.rs
  - doorway/doorway-service/src/render/registry.rs
  - genesis/data/timeline/backlog/resolve-canonical-election-get-links-deadline.md
  - genesis/data/timeline/backlog/identity-head-projection-catchup-signal-gap.md
---

# C8 silent-exit drain — design tier

Framing fact that governs every cure here: `elohim-storage/src/main.rs:256`
applies `add_directive("elohim_storage=info")` AFTER the env read, so
`RUST_LOG=elohim_storage=debug` is overridden — **the debug tier is
unreachable by configuration** for the whole storage crate (only per-module
targets escape). Promote-to-warn/info or count; never park a failure at debug.
The mechanical tier (7 log promotions, pre-touch of every bounded counter
vocabulary, 2 no-behavior counter adds) shipped in the same shift that ran the
audit; this entry carries what needs design:

1. **StampOutcome witnessing (TOP — the purer instance of the obey wall).**
   `adopt_local` (~head_adoption.rs:778) collapses Refreshed/SkippedDeclared/
   SkippedStale/NoRow into a silent `Held`; `try_obey_visible_election`'s
   `stamp_refused` label merges db-error, stamp-error, and four Ok(other)
   states (the b19f12014 correct-refusal-reads-as-failure recurrence). Cure:
   `StampOutcome: ReasonLabel` + per-call-site counting (the HealCanonical
   refusal arm at content_diesel.rs:~1432 is the model); split stamp_refused
   into stamp_conflict / stamp_db / stamp_error.
2. **decide_head_action decision-distribution counter** — the five-way rule at
   the center of convergence has no counter at all; Hold's only witness is a
   dropped debug. One counter at the match head covers all arms. Add C8 rows
   for decide_head_action + canonical_move_verdict in the seam registry (both
   currently C8-ABSENT — 13/16 storage points and 10/11 doorway points lack a
   C8 row; the census already encodes the gap, drain it).
3. **Quiescence-gate counter family** (declaration_would_move,
   claim_self_candidacy, declare-storm gate): the quiescence-to-moves ratio is
   the limit-cycle diagnostic and is unmeasurable today. Counters, never logs.
4. **Subscriber supervision**: four of five hc_client subscribers
   (Infrastructure/Rea/ElohimContent/Mishpat) are spawn-and-forget — no
   reconnect loop, no liveness gauge; a mid-stream WS drop kills projection
   silently (the fifth, ReconcileController, has the forever-retry model).
   Plus identity_heads cursor/lag (already ledgered, cross-cited).
5. **Three dead pipelines** — wire or delete-with-supersession-note:
   handle_recovery_v2_signal (no production caller ⇒ pending_recovery_requests
   permanently empty), handle_imagodei_dna_signal (module doc claims a
   "Step 2c" wiring that does not exist — the doc is actively false today),
   ReconcileController::{on_key_rotation, on_revocation_attestation} Ok(())
   stubs on a live dispatch path.
6. **Doorway SSR-reconcile outcome counter** (the one doorway subsystem with
   none) + the tick summary now counts `unreachable` (mechanical half shipped;
   the counter is design).
7. **Not-reached remainder** (the audit's honest cap): content_diesel.rs
   non-stamp paths, view_federation.rs, provide_reconcile.rs (~196: fails OPEN
   on DB error, self-documented as unfixed), reach_authorization.rs (zero
   counters on any decision; `_ => NoStanding` catch-all would silently
   swallow a topic-vocabulary drift).

Status: open, unowned. The census's C8-ABSENT rows are the drain queue.
