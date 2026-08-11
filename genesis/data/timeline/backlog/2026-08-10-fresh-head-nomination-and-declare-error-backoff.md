---
id: "backlog-fresh-head-nomination-and-declare-error-backoff"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Post-decay convergence residuals: (a) no contest arm nominates the fresh authored head (phantom-candidate loop, design decision needed); (b) declare_error lacks per-id backoff (bounded code fix)"
slug: "fresh-head-nomination-and-declare-error-backoff"
written: "2026-08-10"
author: "batch-3 integration session (follow-up from cascade trace)"
status: "partially-resolved"
priority: "medium"
tags: [dataplane, head-adoption, contest, ghost-decay, design-capture, codex-claimable-partial]
cites:
  - genesis/data/timeline/backlog/2026-08-10-post-decay-adjudication-cascade-trace.md
---

# Two residuals the cascade trace proved — one design, one bounded fix

Source-trace verdict (cited item): decay-authored rows contribute ZERO to
`divergent_actionable` (declaration stays phantom → row stays refused), so
the banking gate does not need these — but full canonical convergence stalls
in a bounded **phantom-candidate loop**: contest Arm 1 nominates the peer's
phantom, Arm 2 nominates the local phantom, and NEITHER can name the fresh
authored head the decay minted.

## (a) Fresh-head nomination — DESIGN DECISION, do not build unilaterally

Letting a node nominate its own freshly-authored root is adjacent to the
self-election C1 boundary (it goes through the DHT arbiter, so it is a
candidacy, not a crown — but the deliberation belongs in a spec note with the
same evidence discipline the decay arm carries: e.g. only for rows whose
declaration is decay-proven phantom, same dwell evidence). Deliverable here:
a one-page design note + concern answers, reviewed before any code.

## (b) declare_error per-id backoff — bounded code fix, claimable

The cascade trace found a generic Arm-1 `declare_error` is retried every
sweep with NO per-id contest-backoff write (only fanout + sweep budget bound
it). Add the missing `contest_backoff::note` (ordinary contest class) on that
branch, mirroring the `no_local_chain` and self-candidacy branches, with a
unit test. Sequencing: head_adoption.rs is also queued for clippy batch C —
coordinate or fold into one pass.

## Resolution — 2026-08-10

Part (a) is delivered for operator review, with **no nomination code**:
`genesis/docs/superpowers/specs/2026-08-10-fresh-head-nomination-design.md`.
The sealed one-page note classifies the proposal as an A2 relationship on the
existing canonical-head DHT link, answers C0-C14, preserves the C1 evidence
floor at every deployment stage, and requires cohort measurement plus explicit
operator sign-off above 500 ids. It remains `Draft`; review is the unresolved
half of this item.

Part (b) is implemented. Generic Arm-1 declaration failures now write a
per-id `DeclareErrorBackoff` entry. The additive typed reason reports
`declare_error_backoff` while `BackoffWindows::for_class` assigns it the
ordinary finite contest window, keeping C8 truthful without inventing a new
retry policy. The seam-registry contract and stable-label vocabulary were
updated with the code.

Clippy batch C is folded into the same pass: all seven async advertiser-health
tests retain their process-global exclusion guard across the await and carry a
narrow, documented `clippy::await_holding_lock` allowance. The subsequent
all-target `-D warnings` run reported zero `head_adoption.rs` findings; its
remaining failures are outside batch C (nine await-lock findings in
`capacity_reporter.rs` / `identity_fill.rs`, plus an unused import in
`tests/db_humans_http_route.rs`).

### Verification

- `a_generic_declare_error_enters_the_ordinary_per_id_backoff`: green.
- `contest_skip_labels_are_stable`: green.
- `each_class_expires_on_its_own_window`: green.
- Full `cargo test`: 2,625 passed, 1 failed, 2 ignored. The sole failure is
  unrelated and environmental: `forwarder_pipes_bytes_bidirectionally` cannot
  bind its localhost listener in the managed sandbox (`EPERM`). A locked,
  offline outside-sandbox rerun was attempted but could not acquire the shared
  Cargo build lock before this session closed.
- `cite-gen --seal` on the design note: green.

### Story-harvest handoff

`chain: dataplane/resiliency-saga/06-heads-converge`  
`between: convergence excludes only adjudicated divergence -> elohim.host adopts alpha-A's declared head after a restart`  
`missing node: a generic canonical-head declare refusal is counted and defers that id for the ordinary one-hour contest window; probe elohim_content_contest_skipped_total{reason="declare_error_backoff"} beside declare_error failures`  
`current state: green locally by the real-ledger unit contract; not yet minted into a2o because a new feature edit requires the context-isolated blind-reader loop`
