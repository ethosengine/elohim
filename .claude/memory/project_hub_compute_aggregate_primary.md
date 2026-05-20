---
name: project-hub-compute-aggregate-primary
description: Hub compute is a storage pool aggregating member-device capacities; hub aggregate is the primary UX with progressive disclosure by capability; per-device triptych is drill-down substrate
metadata: 
  node_type: memory
  type: project
  originSessionId: b08da6e2-3dac-4d8d-809f-aa002a0fd200
---

When a human opens their shefa dashboard, the compute they see is the **hub aggregate**, not a per-device breakdown. The hub is a storage pool aggregating capacities from all member devices — sliding a blade into the rack at home jumps the hub from "5GB / 15GB available" to "5GB / 100GB available" without changing the human's experience of "my hub."

**Why:** Hub is a role, dial-up-by-capability ([[project_hub_archetype_abstraction]], [[project_hub_optional_floor]]) — humans participate in scale through their hub, not through individual devices. The substrate truth lives per-device (system_metrics probes + rea_commitments), but the human-relatable surface is the aggregate.

**Progressive disclosure by capability:**
- **Kids / grandma surface:** "5GB / 15GB available" — no stewardship math exposed
- **Default human surface:** "5GB free of 15GB" — clean two-tuple
- **Power-user / developer surface:** "5GB free of 15GB (12GB stewarded to others, 3GB allocated to self)" — full triptych with the math revealed and drill-down to per-device tiles
- **Drill-down:** per-device compute tiles in `/shefa/cluster/<peer_id>` style routes

**How to apply:** When designing or implementing a compute/capacity visual surface, default to hub aggregate with capability-tier rendering; expose per-device truth as drill-down. Substrate work (per-device probes, rea_commitments aggregation) is the input layer — build it without hub-aggregate-coupling, but design the projection layer to roll up cleanly. Operator was running a parallel sprint (2026-05-20) to formalize the progressive-UX capability axis when this was captured.

Related: [[project_household_horizontal_scaling]] (more blades = more capacity, same hub identity), [[project_hub_optional_floor]] (single device = degenerate hub of one).
