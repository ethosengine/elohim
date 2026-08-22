---
name: project_head_reach_freshness_semantics
title: Reach / head / freshness semantics (umbrella)
description: "Reach (audience) ≠ content_head (version) ≠ replication (custody); Reach vocabulary has a canonical spec — plan against it; head is a DECLARED dependency, not recency; staleness graded by stakes."
metadata:
  node_type: memory
  type: project
---

# Reach / head / freshness semantics (umbrella)

Folds the reach/head/version/freshness semantic cluster — the vocabulary that keeps these planes from collapsing into each other. Members:

- [[feedback_reach_head_replication_distinct_planes]] — Reach (audience, earned) ≠ content_head (version, declared) ≠ replication (availability, custody) — three orthogonal planes; landing-page divergence is a replication bug, not head election.
- [[project_reach_enum_drift_reconciliation]] — Reach 5-way vocabulary drift now has a canonical guiding-principles spec (2026-07-22) — sprints plan AGAINST it, never re-derive; schema-8 canonical, geographic-8→locality, Part-V-5→custody, two die.
- [[project_versioned_entity_head_is_declared_dependency]] — Which version applies is a DECLARED dependency (cid-pin=lockfile), not recency; versions are a DAG (fork/revert/merge); binding picks head, not the query layer.
- [[project_track4_projected_head_coherence]] — Declared-head rails already existed (Content head + StampMode + authorHeadOnce); real gap was boot-frozen doorway SSR + no served-vs-declared probe; T4-1/T4-2 landed 2026-07-22.
- [[project_freshness_graded_by_declared_stakes]] — Operator decision 2026-08-21 — doorway staleness tolerance is graded by the EPR's declared stakes (kind×reach×coupling×NetworkStage), not uniform honest-shed; being behind is an amber trust signal, not a 503.
