---
name: project_hub_identity_cid_canonical_slug_alias
description: "hub identity is dual — Collective CID (collective:{action_hash}) is the canonical notarized identity; slug (family-dowell) is a first-class steward-configurable human-readable alias resolving to it (for names + SEO in the elohim network, not just the seeder). Wave 2 of the prioritizer epic reconciles the slug-keyed SQL onto the canonical CID + adds the DNA MembershipCommitted signal + storage projector."
metadata: 
  node_type: memory
  type: project
  originSessionId: 22de0299-7a43-43b5-a84a-e497e9397bbe
cites:
  - genesis/docs/superpowers/plans/2026-05-29-prioritizer-end-state-wire-hub-fetch.md
---

**Decision (2026-05-29, operator, sprint/cross-pillar-cleanup):** a hub (household/collective) has TWO first-class identifiers:

- **Collective CID `collective:{action_hash}` = canonical identity** — the notarized A2 truth (derived from the imagodei `Collective` DHT entry; `Membership` entries link members). This is what `recipient_hub_id` / device→hub aggregation key on.
- **slug (`family-dowell`, `household-matthew`) = a first-class steward-configurable alias** — human-readable, resolves to the canonical CID. NOT merely seed convenience: stewards will configure slugs for names + SEO as the elohim network matures.

**The collision this resolves:** local SQL today is entirely *slug-keyed* — `collectives.id` holds the slug (not a `collective:` CID), there's no `action_hash`/`dht_anchor_hash` column on `collectives`, `humans.household_id` + `stewarded_nodes.household_id` point at slugs, and the imagodei DNA emits NO Membership/Collective post-commit signal (so `collective_participations`, though born annotated "A2 DHT-derived", is only seed/HTTP-populated). The Wave-3 derivation chain `blob_hash → content → content.created_by(=agent_cid) → hub` is otherwise already built (`peer_topology_view::compute_resilience_cliffs` is the prototype; it stops at agent_cid for exactly this missing `agent_cid → hub` hop).

**Chosen path: hybrid + correct-now** (not modeled-now-defer). Add `action_hash`/canonical-CID + `slug` columns to `collectives`; add `member_cid` + `member_kind` + `dht_anchor_hash` to `collective_participations`; reconcile `humans.household_id`/`stewarded_nodes.household_id` onto the canonical CID with slug-alias resolution; add the DNA `MembershipCommitted` (+ likely `CollectiveCommitted`) post-commit signal + a storage projector arm (`reconcile/holochain_app_signal.rs` `translate_imagodei`). Zero new DHT entry types — `Collective`/`Membership` entries already exist; this completes their A2 projection. Slug = Category-C operational alias.

**Why:** captured during Wave 2 design of the prioritizer epic (see `genesis/docs/superpowers/plans/2026-05-29-prioritizer-end-state-wire-hub-fetch.md`). Relates to [[project_storage_tiering_placement_intelligence]] (the agent→hub derivation is the shared scalability primitive), [[project_hub_archetype_abstraction]], [[project_node_metrics_vs_hub_aggregation_boundary]], [[project_dna_changes_dont_redeploy_without_forced_reinstall]] (the DNA-signal change carries the redeploy gotcha).

**How to apply:** treat CID as the join key going forward; slug resolves to CID via a collectives alias lookup. Land the read-side resolver + hub_capacity rewrite FIRST (works on seed data via the slug→CID backfill), then isolate the DNA signal + projector (carries deploy risk). The prioritizer's hint↔commitment match only needs both sides to use the SAME representation — converge them on the canonical CID.
