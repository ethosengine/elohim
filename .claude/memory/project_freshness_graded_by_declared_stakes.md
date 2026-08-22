---
index: false
name: project_freshness_graded_by_declared_stakes
title: Freshness graded by declared stakes
description: Operator decision 2026-08-21 — doorway staleness tolerance is graded by the EPR's declared stakes (kind×reach×coupling×NetworkStage), not uniform honest-shed; being behind is an amber trust signal, not a 503.
metadata:
  type: project
---

Decided 2026-08-21 (operator, during the doorway-failover / resiliency-saga work): the doorway's
"honest-shed" contract (spec 2026-07-19, data reads 503 `catching-up` when the upstream circuit is
open) must NOT be uniform. Freshness requirement is priced against the EPR's **declared stakes**,
the read-side sibling of `elohim-storage/src/trust/pricer.rs` (stage×floor×reach×standing):

- blog / landing / Knowledge-only Content at Public/Commons reach → serve the last reconciled head, marked amber
- Value-coupled reads (e.g. rewards-program form data) → serve amber WITH warning
- authority reads (auth/session/identity-credential, governance tallies, head-declare) → strict projection, shed — the floor, never stage-priced
- dev/staging (`NetworkStage::Simulacra`/`Bootstrap`) → serve stale freely (amber = DHT-trust signal)

Being behind because of conductor/projector lag becomes a **trust signal on the wire**
(`x-elohim-freshness: amber`, generalizing the existing `x-elohim-bundle: last-reconciled`), rendered
by the client (saga ch10 card-tells-truth) — not a blocker to resiliency.

**Why:** build 1374's correlated pair shed made the saga read far redder than the substrate
(`divergent_actionable` on matthew ≈1). Root cause was four doorway bugs (9be1f84a7, 18a65fd0d —
endpoint-keyed breaker tripped by a 10s SSR shell fetch, NOT projector lag), but the open design
question Opus left — "should the breaker be route-class aware?" — is this decision: route class = stakes class.

**How to apply:** no new entity — it is a pure decision predicate (p2p-design-gate Step 4, C13 graduated
authority / C4 honest absence) derived from `Envelope{kind,reach,coupling}` + `NetworkStage` + head
status; amber is DERIVED from `dht_anchor_hash`/head-status presence, never written. Circuit open +
last-good bytes present + verdict allows amber → serve amber; else shed. Register it in the doorway's
`seam-registry.yaml`; the a2o scenario in `doorway-failover.feature` is the flip authority. Do not
canonize reach tier names (vocabulary in declared drift) — take `Reach` as the input type only.
Related: [[feedback_reach_head_replication_distinct_planes]], [[project_doorway_ops_incidents]].
