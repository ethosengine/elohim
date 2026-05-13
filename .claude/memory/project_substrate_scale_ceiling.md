---
name: Substrate scales by federated Tier 3 nodes, not hyperscale
description: Substrate is designed for ~100M Tier 3 nodes carrying billions of humans (most via hub-and-spoke, custodial keys, hosted accounts). Per-node load bounded by trust-network membership; doorway absorbs web2 mass-readership; substrate-level care is what enables honest entry-tier participation, not a side-effect for hardware owners.
type: project
originSessionId: 909de5de-3db0-4c88-af4b-12f47dd2762c
---
The substrate is **not** designed to handle FB/YT-shape hyperscale load. It's designed for a federated topology where Tier 3 family nodes (per `genesis/docs/content/elohim-protocol/hardware-spec.md`) are the substrate participants. Closest analogy: email or Mastodon's federation layer — but each "instance" is a household, church basement, or community-center Tier 3 serving its trust-network members deeply, not one operator serving thousands shallowly.

**Reach math: 100M Tier 3 nodes × tens-to-hundreds of humans served each = billions of participants, most without owning hardware.** A single Tier 3 carries: its own household (4–10 members) + spokes (laptops/phones syncing to the hub, including extended family) + custodial key hosting for less-technical relatives + relational backup for the trust network. A church basement Tier 3 can be the substrate participation point for a hundred-person congregation. The framing "100M households participate" is wrong — it's "100M Tier 3 nodes, billions of humans reached."

**Inclusion is the point, not a side effect.** The substrate-level care (narrow integrity layer, content-addressed identity, migration-preserves-everything) is what makes entry-tier participation honest rather than extractive. Stage 1 visitors and Stage 2 hosted users are first-class participants whose substrate rights are guaranteed by the same constitutional contracts the Tier 3 operators run on — they are not second-class citizens of the protocol. A hyperscale equivalent forces everyone onto the same surveillance platform; the protocol's careful peer-to-peer work is precisely what enables billions of people to participate meaningfully without owning infrastructure or losing data, identity, reputation, or relationships across stage transitions. **Never frame the protocol as "for the rich who can afford Tier 3" — frame it as "Tier 3 nodes are the substrate participants who carry billions through trusted hosting and hub-and-spoke."**

**Tier 3 changes the per-peer baseline.** Substrate-level performance reasoning must assume Tier 3 specs, not consumer-device specs:
- 16+ cores, 64–128GB RAM, RTX 4070-class GPU
- 2TB NVMe + 10TB+ bulk storage
- 10GbE local, multi-WAN (fiber + cable + 5G + LoRaWAN + satellite)
- Always-on, UPS-backed, designed for 24/7 operation

A bloom filter for millions of blobs at 1% FPR is ~1MB — Tier 3 absorbs it trivially. Parallel fetch of 3–5 candidates is free. GPU is available for things beyond bytes (validation, ranking, semantic indexing). Performance worries that assumed Raspberry-Pi-class peers mostly evaporate.

**Substrate participation is Tier 3-to-Tier 3, not consumer-to-consumer:**
- Stage 1 visitors and Stage 2 hosted users → served by **doorway** (web2 CDN scale, traditional infra; substrate never sees them)
- Stage 3 spokes (laptops, phones) → sync to **their household's Tier 3 hub**; never on substrate hot path
- Stage 4 nodes (Tier 3) → are the actual substrate participants
- Realistic per-Tier-3 connection count: 100–500 (extended family + close community + neighborhood mesh + bootstrap), not thousands

**The three "extensions" I previously called scale-ceiling work are actually topology-expression, not new architecture:**

| Previously framed as | Reality given hardware + social topology |
|---|---|
| Bloom inventory for FB-scale gossip | Cheap on Tier 3; just engineering. Needed for cross-collective gossip but not load-bearing for performance. |
| Hierarchical aggregation as scale extension | **Already the deployment topology** — household hub aggregates spokes; families aggregate into trust networks; neighborhoods form collectives. Substrate routing just needs to express what's socially true. |
| Tiered storage hierarchy as scale extension | **Already in the hardware spec** — NVMe (hot) / bulk SSD (warm) / encrypted shards to trusted institutions (cold). Substrate routes by tier; tiering is a hardware fact. |

**T17 fetch racing (3 candidates parallel vs sequential per-peer 5s timeout) still stands** as a small in-scope refinement — easy on Tier 3, much better tail latency.

**What stays a real concern:**
1. **Cross-collective discovery** — Kademlia at the household-aggregate handle level (not per-blob); search space is "which household has it" not "which blob in N billion."
2. **Reach asymmetry** — viral content steward gets hammered, but by other Tier 3s (capable, always-on); natural CDN-like behavior emerges as fetchers re-serve to their spokes and neighbors. Reach class + tiered storage handle this organically.

**Why this matters:** The architecture isn't "right for alpha and needs scaling work for prod." It's "right for the deployment topology the protocol is designed around." The deployment topology assumes Tier 3 hardware as the substrate participant, which means most so-called "scale work" is really "make routing match the hardware + social topology."

**How to apply:**
- When asked "does this scale?", lead with the federation paradigm — not "yes/no for FB-scale," but "scales with Tier 3 nodes × trust-network members carried, reaching billions without each person owning hardware."
- When discussing reach math, count humans carried (billions), not nodes (100M). Spokes, custodial-keyed relatives, and hosted users are first-class.
- When framing the protocol's audience, push back on "this is for the rich" reads. Substrate care exists to make entry-tier participation honest — it's the load-bearing layer for inclusion, not optional plumbing for hardware owners.
- Don't propose hyperscale-style optimizations (sharded controllers, leader election, partition assignment) unless the workload genuinely exceeds federated-Tier-3 capacity.
- Distinguish substrate concerns (Tier 3 mesh) from doorway concerns (web2 projection, CDN) — see `project_doorway_is_federation_surface_atproto` and `project_doorway_single_target_no_fanout`.
- When designing routing, first-class the household-as-aggregation-unit (`project_household_is_resilience_unit`); per-peer is drilldown.
- The integrity layer (DHT) stays narrow — ~one row per agreement, not per byte served. Performance work lives in libp2p data plane where Tier 3 hardware absorbs it.
- T17 fetch racing is in-scope; flag during T17 implementation.
