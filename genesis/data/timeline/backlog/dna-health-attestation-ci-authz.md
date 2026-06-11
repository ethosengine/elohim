---
id: "backlog-dna-health-attestation-ci-authz"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "record_health_attestation rejects the CI seeding agent — 'Only the doorway operator can record attestations'"
slug: "dna-health-attestation-ci-authz"
written: "2026-06-11"
author: "agentic-developer (EPR durability shift, live-seeding Grafana read)"
status: "backlog"
priority: "high"
jobs: [elohim-genesis]
tags: [dna, infrastructure-zome, authorization, ci-seeding, delegates-compute]
cites:
  - genesis/data/timeline/backlog/security-ci-substrate-authorization-grant-coherence.md
---

# Health-attestation authz vs CI agent key

Repeated WASM guest error on matthew's conductor during the genesis
window: zomes/infrastructure/src/lib.rs:566 "Only the doorway operator can
record attestations" — fn record_health_attestation, 4x in 2h. The CI
seeding path calls under an agent key the zome guard does not recognize as
the doorway operator. Either the caller should not be invoking it (drop
the call from the seed path), or this is precisely a delegates-compute
consumer (CI holds a bounded commitment authorizing attestation writes) —
which folds into the standing authorization-grant-coherence concern. Do
NOT loosen the zome guard; it is correct.

shift_objective: |
  Decide caller-side: remove the CI call or route it through a
  delegates-compute grant; verify zero authz rejections in a full genesis
  run.

## UPGRADED TO CENTRAL 2026-06-11 ~20:00Z (operator on-host diagnosis)

The caller is NOT the CI agent — it is the DOORWAY ITSELF
(doorway::services::zome_caller, signed, every 5 min, both replicas).
Mechanism (verified in code): `record_health_attestation` compares the
DoorwayRegistration record's frozen `operator_agent` (set to the
registering agent at register time) against the live caller. The live
doorway's cell key no longer matches (non-prod reinstalls mint new agent
keys), and the lockout is total: `register_doorway` rejects duplicates,
`update_doorway` carries the same operator guard — a re-keyed doorway can
never reclaim its own id. Attestations never land → the
content_attestations projection table is never created → matthew's
/api/v1/attestations 500s → the E2E custody check reds. This single root
explains the remaining red gate. Fix in flight (this shift): allow loud
re-registration (latest-wins lookup, churn trail preserved) as scoped
bootstrap debt, with the commitment-gated reclaim (operate-doorway REA /
delegates-compute family) as the end-state — routed to
security-ci-substrate-authorization-grant-coherence.md.

## FIX LANDED (this shift) + verification path

Zome-side: `register_doorway` now ALLOWS re-registration (each
registration = new entry + link; operator churn WARN'd and readable on
the DHT trail) and `get_doorway_by_id` resolves latest-wins. The lockout
breaks: a re-keyed doorway reclaims its id on its next boot
self-registration, the guard then matches its live key, attestations
land, content_attestations materializes, /api/v1/attestations stops
500ing. WASM check clean; the two clippy warnings in the integrity zome
are pre-existing (untouched). zome-sweettest-sync note: a two-conductor
sweettest should pin (a) re-registration latest-wins lookup, (b) a
re-keyed agent attesting successfully after reclaim, (c) the churn trail
remaining readable via get_doorways_by_operator.

DEPLOYMENT: DNA-content change → new hash → reaches alpha via the
non-prod ALLOW_DNA_REINSTALL path on the next DNA deploy; the reinstall
mints new agent keys, and the new re-registration semantics are exactly
what lets the doorways heal on that same boot. Genesis pair caveat still
applies (both pods take the reinstall together).
