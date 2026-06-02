---
title: "History/ADR: Seed-row shape must satisfy the view's SQL predicates"
type: history-gotcha
status: Accepted
tier: history
created: 2026-06-02
topic: [seeder, projection, view, sql-predicate, silent-filter, idempotency]
# DISTILLS the recurring "silent-filter" failure mode across topology-substrate-completion
# (2026-05-07) and seeder-registry-coherence (2026-04-18). Distinct from the two existing
# seed memories. Raw bodies retire to git.
distills:
  - .claude/archive/2026-05-15/genesis/docs/specs/2026-05-07-topology-substrate-completion-design.md
  - .claude/archive/2026-05-15/genesis/docs/specs/2026-04-18-seeder-registry-coherence-design.md
# Bidirectional: the canonical surface this gotcha points back to.
canonical:
  - ../../../../../elohim/sdk/schemas/v1/views/CONVENTIONS.md   # view wire-contract conventions
memory_anchors:
  - feedback_seed_lock_means_schema_drift
  - project_placement_signals_are_shefa_inputs
  - project_seed_whoever_is_ready
  - feedback_schema_first_ioc
---

# Seed-row shape must satisfy the view's SQL predicates (2026-05-07, topology-substrate-completion)

> **Hot-context pointer (the one sentence to remember):**
> A projection view's `WHERE` clause is an **implicit contract on seed-data shape.** When the seeder
> and the view drift, you get *silent* zero-row surfaces that look like "feature not built" rather than
> "data filtered." If a surface is dark, check the view predicate against the seeded rows *before*
> blaming the aggregation code.

The `light-up-the-topology` sprint shipped substrate code for all six topology surfaces, but only 2 of
6 lit up — and *wiring the existing seeders as-is would still not have lit them*, because the seeders
wrote the wrong **shape**: `seed-commitments.ts` wrote `action="provide"` /
`provider="human-matthew-manager"` / `receiver="network"`, while the `reciprocity_view` /
`cluster_view` / `distribution_view` SQL predicates filter on `action="custody-blob"` AND
`provider IN (peer_id_set_from_AgentPeerBindings)`. The rows inserted fine and were then **silently
filtered out** by the view — no error, no warning, just an empty surface.

## The recurring failure mode

This is a class, not a one-off. Sibling instance (2026-04-18 seeder-registry-coherence): the seeder
iterated all 33 account packages but only 6 had StatefulSets, so 27 hash-routed to nonexistent pods and
502'd — same class (**seeder shape ≠ deployed reality**), fixed by making `deployments.json` the single
source of truth all three consumers read. Note this is distinct from the two existing seed memories:
`feedback_seed_lock_means_schema_drift` is about a DB-locked seeder meaning schema drift (not
concurrency), and `project_deployments_json_seed_or_skip_truth` is the suspended-flag gate. Neither
captures the **silent-filter** mode: well-formed rows written with the *wrong column values* that view
predicates discard with zero error.

## The defense

Treat the view predicate as part of the wire contract. When a surface is dark, *first* check whether
seeded rows satisfy the view's `WHERE` clause before assuming the aggregation code is wrong. Idempotency
belongs here too: bidirectional authorship (Adam↔Eve spouse relationship) must create ONE row, and
reruns must converge — the directional `UNIQUE` index `(h_app_id, party_a_id, party_b_id,
relationship_type)` is correct (per-party custody/consent legitimately distinguishes A→B from B→A), so
idempotency is the seeder's job, not the index's.

## Watch-out for future planners

The view's `WHERE` is a contract the seeder must honor — `schema-first-is-IoC` extends down to seed
shape, not just wire types. Before declaring a surface "not built," grep the view predicate and confirm
the seeded `action`/`provider`/`receiver` (and any join key) actually satisfy it.

## Bidirectional links

- **This record → canonical:** [view-schema CONVENTIONS](../../../../../elohim/sdk/schemas/v1/views/CONVENTIONS.md) (the HTTP wire-contract rules; the predicate is part of that contract).
- **Distilled-from (raw bodies in git history):** topology-substrate-completion design + seeder-registry-coherence design (linked in frontmatter).
