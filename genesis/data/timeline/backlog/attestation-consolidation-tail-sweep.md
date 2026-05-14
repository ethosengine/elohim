---
id: "backlog-attestation-consolidation-tail-sweep"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Tail-end sweep: residual lamad_event_type / DoorwayHeartbeat / old verbs across app + seeder + a2o"
slug: "attestation-consolidation-tail-sweep"
written: "2026-05-14"
author: "cartographer"
status: "proposed"
priority: "medium"
relatedNodeIds:
  - "memory:project_principle_p1_reconciliation_controller"
  - "memory:feedback_cascade_hidden_test_surface"
  - "memory:feedback_signature_changes_grep_callers"
tags: [attestation, consolidation, tail-sweep, projections]
shift_objective: |
  The attestation Stages A→G consolidation landed last week — DoorwayHeartbeat retired,
  peer_blob_inventory + system_metrics reclassified as observation projections, 4 DNAs +
  Angular consumers migrated. Residual drift remains in projection layers that aren't
  part of the cross-crate compile gate: prose references in app/elohim-app/src/, the
  seeder, and a2o feature files. Sweep grep for `lamad_event_type`, `DoorwayHeartbeat`,
  and any pre-consolidation verbs the carrier memory entry enumerates. Each hit: either
  rename per the migration table, or annotate with a comment explaining why the legacy
  name is intentional (forensic, e.g. version-compat scenarios). Expect cascade-hidden
  surfaces (per feedback_cascade_hidden_test_surface): each fix may reveal more. Budget
  2-3 cycles, not 1. Done when grep is clean OR every remaining hit has an explanatory
  comment AND a passing test pins it.
---

# Attestation A→G tail-end sweep

## Why this matters

Substrate-rename moments are P1 reconciliation moments. The DHT (notary) is now
post-consolidation; if the projection layer (app/seeder/a2o) lags, the protocol is
two-faced. Worse: a2o feature files describing learner experience in the OLD verb-set
will mislead future Story-First implementers.

## What's blocking

- Confirm the cross-crate compile gate at 09901d27f doesn't hide the projection drift
  (it operates on Rust crates; app/ and a2o are not in its scope)
- feedback_cascade_hidden_test_surface warns: fixing one will reveal more

## What's ready

- A→G migration table exists in the carrier memory entry (per storyteller Wave 2 HOLD)
- Grep targets are concrete: `lamad_event_type`, `DoorwayHeartbeat`, plus verbs from
  the migration table
- Story-First default applies: any a2o file changed must keep its scenario passing

## Convergence

- Historian Wave 1: precedent 1 (wave-0 consolidation absorbs preexisting drift)
- Storyteller Wave 2 HOLD: sprint-state entry held pending this sweep

## Definition of done

1. grep -r over app/elohim-app/src + seeder/ + genesis/a2o/features clean OR comment-pinned
2. Any renamed a2o scenarios pass
3. Carrier memory entry can flip from HOLD to graduate/memorialize at next ceremony
