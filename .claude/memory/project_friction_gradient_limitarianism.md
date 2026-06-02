---
name: friction-gradient-limitarianism
description: "Protocol-level anti-concentration design — friction to acquiring more power/centralization increases as accumulation increases, making existential power structures mechanically expensive to form."
metadata: 
  node_type: memory
  type: project
  originSessionId: 0deb6177-e250-4b18-9ca2-dd7a2b971a5d
cites:
  - genesis/docs/content/elohim-protocol/governance-layers-architecture.md
---

# Friction-gradient limitarian design

The political principle that makes Qahal-scale architecture safe to scale. Friction to acquiring more power/centralization is **not constant** — it **increases as accumulation increases**.

- Small Qahals grow easily (low friction, low coordination cost)
- Mid-sized Qahals grow with effort
- Approaching "existential power structure" scale, the protocol mechanically resists further concentration

## Why: design intent

This is anti-concentration **as substrate**, not as policy that can be removed by future governance. Substrate-level expression of [[project_no_sovereignty_stewardship_over_ownership]] — sovereignty isn't forbidden, it's mechanically expensive.

## Where it lives

Two enforcement layers (likely both):

1. **Soft / elohim-discernment**: accruing standing in a Qahal that has already reached threshold yields diminishing returns. Reach into oversized collectives costs more. Standing curve flattens at scale.
2. **Hard / protocol-floor**: substrate refuses certain operations as the collective approaches concentration thresholds (e.g., Agreement clauses giving one agent > X% of cascade beyond Y total members; rubric updates that would centralize authority).

## Relationship to other patterns

- Constrains [[project_qahal_graduated_capability_surface]] — standing can rise within a Qahal, but the Qahal itself can't become too consequential without protocol resistance
- Implemented through [[project_social_reach_nervous_system]] preference guards + [[project_signal_kind_extensible_protocol_class]] (feedback debits accelerate at concentration)
- Compatible with [[project_redeploy_the_substrate]] — even on uncontrolled hardware, the substrate prevents concentration shapes from forming
- Supports [[project_elohim_vision_fruit_back_on_tree]] — designs against the weaponization shape

## Open

Threshold values, friction curves, and the exact mix of soft/hard enforcement are TBD. The principle is foundational; the parameters are tunable per archetype.
