# elohim-hub

Runtime composition primitive for the protocol's **hub layer** — the home-node cluster that stewards a family's or collective's compute, federates horizontally with peer hubs, and absorbs the aggregate-scale concerns hyperscalers traditionally centralize.

> **Status:** scaffold-stage. The hub trait is sketched in `2026-05-02-elohim-hub-boundaries-design.md` and the responsibility surface is mapped in `2026-05-08-doorway-hub-edge-design.md`. This directory is bootstrapped now to reserve the namespace; code lives in `elohim-node` until a second consumer (operator UI, fixtures crate) needs the trait independently.

## What a hub is for

The substrate has three layers (memory `project_three_layer_truth_model`):

```
DHT             — notarization, expensive, narrow (Holochain DNAs)
libp2p          — operational state, per-peer (elohim-storage)
hub             — runtime composition + federation (elohim-hub)
   ↓
doorway         — optional web2 projection surface
```

The hub composes elohim-storage, elohim-agent, elohim-bitswap, elohim-compute, and elohim-render into a single runtime that:

- federates horizontally with peer hubs (the substrate's only scaling dimension; memory `project_substrate_scale_ceiling`)
- applies **reach-earning** at four aggregate-scale surfaces (compute / distribution / defense / AI-coordination)
- coordinates discernment via **elohim-operators** (the household-fabric-manager role, memory `project_household_fabric`)
- absorbs Cloudflare-class concerns (DDoS coordination, traffic-shape recognition) and FANG-class concerns (mobile inference, observer streams, workload migration) without recreating their centralization

A pattern shaped like a DDoS attack is structurally just unearned-reach compute or distribution. The hub fabric simply doesn't engage with it — defense is a side-effect of earning, not a bolt-on firewall. This is the same reach-earning principle the social-reach nervous system already applies to messages (memories `project_reach_earned_at_authoring`, `project_social_reach_nervous_system`), extended to aggregate-scale surfaces.

See the design spec for the full thesis: [`genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md`](../../genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md).

## Archetypes

The hub has two archetypes that share a trait surface but differ in **design attitude**:

### DwellingHub (primary)

The home-node cluster sized to one family / one dwelling. "Dwelling" grounds the concept at its natural physical limit — the place where humans live, walls and a roof, a known set of inhabitants who steward it together. Multi-blade cluster (memory `project_household_horizontal_scaling`); horizontal scaling means more dwellings, not bigger dwellings.

**Attitude — co-presence.** Humans and their elohim-operators are co-present in the dwelling's fabric. The fabric is visible; family members can intervene; the elohim-operator is a fabric-helper alongside the humans, not a fabric-owner. Memory `project_intelligence_revolution_scales_to_humans` is most directly true at this archetype.

DwellingHub is the canonical-case archetype. It supersedes the previous `HouseholdHub` term in `2026-05-02-elohim-hub-boundaries-design.md` and memory `project_hub_archetype_abstraction`; existing code/docs may transition incrementally.

### CollectiveHub

The runtime composition primitive for a collective — church, co-op, patron circle, DAO, mutual aid network (memory `project_social_compute_collective_is_stewardship_unit`).

**Attitude — delegated stewardship.** The collective expects elohim-operators to carry more day-to-day fabric work autonomously, with humans designating stewardship roles rather than every member operating fabric directly. Not a hard taxonomic split from DwellingHub — both are accountable, both are bound by the substrate-floor / elohim-ceiling pattern (memory `project_substrate_floor_elohim_ceiling`) — but the *attitude* of who is most active in operation differs. This is the technical tier where the human-vs-elohim separation is more visible.

CollectiveHub is sized to a collective's stewardship contract; may span multiple physical sites; doorways are typical (collectives usually have a public face) but not mandatory.

## Doorway optionality

A hub may or may not host a doorway (memory `project_p2p_is_hosting`). A dwelling that publishes content to the public web hosts a doorway; a dwelling that participates only privately does not. Humans inside a dwelling may register with multiple doorways for resilience (memory `project_multi_doorway_human_registration`) — their own dwelling's (if any) and others'.

The doorway is one role a hub can take, not a mandatory layer. See `doorway/doorway-service/EDGE-DESIGN.md` for the doorway-side framing.

## What goes here, eventually

When the hub trait moves from sketch to crate, this directory will hold:

- `Cargo.toml` — crate definition
- `src/lib.rs` — `Hub` trait and core types
- `src/dwelling/` — `DwellingHub` impl
- `src/collective/` — `CollectiveHub` impl
- `src/federation/` — peer-hub federation manifest, gossip, contracts
- `src/operator/` — elohim-operator role manifests for hub scope
- `src/reach/` — the four reach-earning surfaces (compute, distribution, defense, AI-coord)

Until then, the architectural truth lives in:

- `genesis/docs/superpowers/specs/2026-05-08-doorway-hub-edge-design.md` (this spec — the canonical map)
- `genesis/docs/superpowers/specs/2026-05-02-elohim-hub-boundaries-design.md` (the predecessor — Hub trait sketch)
- `elohim/elohim-node/src/` (current implementation site for hub composition)

## Memory anchors

Constitutional commitments this crate inherits:

- `project_substrate_scale_ceiling` — Tier 3 federation paradigm
- `project_elohim_hub_elevation` — hub-to-hub federation scaling story
- `project_hub_archetype_abstraction` — DwellingHub + CollectiveHub
- `project_household_fabric` — elohim-operator as fabric-manager
- `project_substrate_floor_elohim_ceiling` — substrate determines deterministic; elohim adds discernment
- `project_reach_earned_at_authoring` — reach is earned at every node
- `project_household_is_resilience_unit` — resilience is hub-to-hub
- `project_three_layer_truth_model` — DHT / libp2p / doorway, with hub composing libp2p+storage
- `project_intelligence_revolution_scales_to_humans` — protocol scales TO human complexity
