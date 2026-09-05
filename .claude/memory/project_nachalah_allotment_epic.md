---
name: project_nachalah_allotment_epic
title: Nachalah allotment epic
description: "Companion to the crossing epic, planted 2026-09-05: owns HOLDING — gold/deeds/paper tiers on DNA seams, earned arcs, block governance, conductor as a rung-5 artifact; start at its §7 hub"
metadata: 
  node_type: memory
  title: Nachalah — the Allotment Epic (companion to the crossing epic)
  type: project
  originSessionId: bf90213f-876c-4014-807d-504fb20fefd3
  modified: 2026-09-05T13:09:37.231Z
---

Hub: `genesis/docs/superpowers/specs/2026-09-05-nachalah-allotment-epic-design.md` (id
`nachalah-allotment-epic`, brand Nachalah, name Allotment, status ACTIVE — graduated 2026-09-05 on the operator's acceptance; next artifact = the Slice 1 plan). Born from the
operator's sidebar the morning the sunset partitioned the household (see
[[project_household_space_partition_blocks_and_round_deadline]]).

**Boundary (recorded on both hubs):** the Holochain Evolution Epic owns the CROSSING (a hApp
version carried by the network); Nachalah owns the HOLDING — who holds, validates and serves a
record, at what floor, by what evidence — plus the conductor as our own artifact. A holding rule a
crossing needs (never write on a closed chain, Task 32) is minted THERE and cited HERE; a crossing a
holding rule needs (an arc policy changing under running peers) is minted HERE and rehearsed with
THEIR vehicle.

**Premise the operator set:** today everything on the DHT is treated as digital gold (held,
validated, served by all, forever); a record needs a critical floor (≈7 diverse hubs), not
gossip among all; discriminate paper from golden commandments and let EPR governance + social
compute decide who holds what. Holochain shipped a sharded design with the shard off (0.7 arcs
are Empty→Full only) and a stranger-model punishment (permanent block, no unblock).

**Steering (2026-09-05):** allotment is only the performance slice — the REA + VSM primitives (inherited from elohim-epr / epr-rea / elohim-compute) make the DHT a living ecology: an arc is an REA commitment (resource = holding capacity, events = holding/validating/serving, priced by the trust gradient); the five VSM seats are named in §3b; Ashby's variety is the design rule; EPR coupling: every holding claim/event/commitment/trust judgement is a content-addressed witnessed EPR atom — the minimal unit that keeps the network honest and what the elohim deliberate over to scale trust (G3b.4). Never design an arc or block policy as a bare tuning knob.

**Gevul (boundary) — the fractal:** every holon (device → household → neighbourhood → collective → commons → orchestra) has a gevul and a nachalah inside it; gold at one level is paper at the level above; the global covenant is core at every level but each level's agreed scope is constructed by the consensus underneath — values are ONE system held at different arcs, immutability is EMERGENT from arc width (planetary deliberation vs supper-table); the Republic ceiling (§2c): one space's carrying capacity is the real protocol limit (we are the IPv4 generation), the orchestra is a republic of republics, diversity is carried by depth not width; DETERMINISTIC FLOOR, ELOHIM CEILING: the ceiling (planetary carrying capacity, limitarian cap) is reached only at the full-arc global tier — the everyday goal is the most performant representation TRUST allows (trust lets fewer hold and validate viably; a trust-full architecture is MORE viable; full-arc-everywhere = the trustless crypto anti-pattern in a Holochain costume); a record's reach is a path up the holarchy with a floor per level, crossing upward is a stewarded ceremony (earned reach), downward is projection. §2b.

**Slices, in measurement order:** (1) gold/deeds/paper tiers + DNA seams (lever exists today);
(2) our 0.7 fork — `list_blocks`/`unblock` first, then sharding on with an arc policy hook driven
by the valueflow trust gradient, tier-graded blocks under Mishpat; (3) the conductor as the sixth
rung-5 artifact class adopted by the ark, the fork inside the primary repo's watch, Jenkins builds
only. Graduated Active 2026-09-05; Active→Canonical needs the three measured legs in the graduation-trigger.

**How to apply:** any arc/sharding/block/tiering/conductor-pipeline question routes to this hub,
never to the crossing epic; the fork to start from is stock 0.7.0 (the 0.6.3 fork
`elohim/holochain-conductor` is a re-port, not a rebase). See
[[project_holochain_evolution_epic]], [[feedback_upgrade_propagation_north_star_wall_clock]].
