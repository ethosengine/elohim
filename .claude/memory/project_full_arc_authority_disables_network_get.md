---
index: false
name: project_full_arc_authority_disables_network_get
title: Full-arc authority disables every network get
description: "On a full-arc fleet every zome get/get_links is local-only — a link miss means gossip failed, not that the data is absent."
metadata: 
  node_type: memory
  title: Full-arc authority disables every network get
  type: project
  originSessionId: d50e36ac-198e-48f3-9faf-3317c687c102
  modified: 2026-07-25T03:10:23.226Z
---

Holochain's `GetStrategy::Network` (the `Default`) carries this in its own doc
comment, verbatim (`holochain_zome_types-0.6.0/src/entry.rs:98-99`):

> "If the current agent is an authority for this hash, this call will not go to
> the network."

Every alpha conductor runs **full arc** (`target_arc_factor` defaults to 1;
`conductor-diagnostics` showed 32 arcs at `[0,4294967295]`). A full-arc agent is
an authority for *every* hash. **So every `get` / `get_links` in every zome on
this fleet is effectively local-only.** Fleet-wide, "Network" buys nothing.

Consequence: a link miss is NOT evidence the data does not exist. It means
gossip has not delivered the op to this node. Diagnosed 2026-07-25 as one root
cause behind two symptoms long treated as separate:

- `declare_canonical_head: no content found for id 'elohim-host-landing'`
  (HTTP 502 on the DECLARE_ONLY leg of every app deploy) → the two-headed
  landing, `elohim.host` and `doorway-alpha` each serving their own
  `trust: notarized` head. Chain: `declare_canonical_head_inner`
  (`content_store/src/lib.rs:3220`) → `gather_content_chain` (`:2652`) →
  `get_links(IdToContent, GetStrategy::default())` → empty → `Ok(None)`.
- `conductor_missing=62, rea local_total=0` on adam → empty REA commitments
  projection. `get_rea_commitment` uses the identical `get_links` idiom.

**Why content looked healthier than REA, and why that comparison misled for a
day:** the asymmetry is in the FALLBACKS, not the base computation. Both use the
same `StringAnchor` base — content-derived, identical on every peer; write and
read are literally the same three lines, and neither path commits the anchor
entry (`hash_entry` only, which is legal). But `get_content_by_id` routes through
`healing_integration` and manufactures the entry+link locally on a miss, and
`resolve_content_head` degrades to root-author election. Content's heal counters
therefore advance whether or not the remote path is healthy. `get_rea_commitment`
has no fallback: link miss → `Ok(None)`, forever. REA was never more broken than
content — it was the only one telling the truth.

Corollary for diagnosis: **never read a content-vs-REA heal-counter gap as
evidence about the fetch path.** The backlog
`genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md` already
flagged the two candidate sets as structurally non-comparable; that caveat is
load-bearing, not hedging.

Any cure lives in `content_store` (coordinator zome); `IdToCommitment` /
`IdToContent` already exist in `content_store_integrity`, so it is
**hash-neutral and hot-swappable** via `sync_coordinators` — no reinstall, no
re-key. See [[project_dna_hash_blind_to_coordinator_zomes]].

**Sweettest caveat (2026-07-25).** Do not treat local sweettest runs as evidence
about this substrate. With `await_consistency` restored, `rea_commitment_replication`
still fails its read — but so does a CONTENT test (`earned_beats_newer_staging_at_resolve`)
whose own `await_consistency` passed first, and `elohim-holochain` has a green
streak of 12 in CI. Consistency-reached-then-read-fails appears for BOTH streams
here, so the container is a confound and cannot isolate a REA-specific defect.
The trustworthy evidence is the Holochain source above plus live alpha logs. Let
the CI DNA shard speak.

Open design question: a `call_remote` fallback makes the node ASK instead of
waiting for gossip. That is either eager reconciliation (the house style — see
[[project_principle_p1_reconciliation_controller]]) or a mask over a gossip
failure, which is exactly what made content's signal untrustworthy. If built, it
must emit a counter on every fallback fire, so a fleet healing by fallback can
never read as a fleet whose gossip works.

Related: [[project_alpha_topology_bootstrap_pair]],
[[project_per_node_memory_is_conductor_authority_arc]] (the same
`target_arc_factor = 1` that makes RAM ∝ corpus is what makes every node an
authority), [[project_inventory_exchange_not_byte_replication]].
