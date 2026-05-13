---
name: Household horizontal scaling via blade fleet
description: Scaling within a household means more elohim-node instances across blades, not bigger single processes. Operator manages placement and purpose assignment.
type: project
originSessionId: 63499c63-1cde-41b5-a0b0-66503d4c008c
---
When a family's needs outgrow one node, they add blades to the rack. The elohim operator distributes purpose across the fleet — not by scaling up one process, but by running more elohim-node instances with different roles:

- Primary node: conductor + storage + doorway (the household's anchor)
- Content stewardship nodes: elohim-node instances stewarding community content
- Inference nodes: AI workloads, model serving
- Per-person nodes: each family member's conductor + source chain on dedicated hardware
- Guest nodes: grandma moves in, slides her blades in, operator figures out replication

The P2P mesh handles horizontal scaling — each elohim-node is a peer. The operator's job is placement (which blade runs what), local optimization (what's replicated locally for LAN speed vs. what rides the DHT), and lifecycle (blade joins/leaves the household).

**Why:** A person's entire internet experience mediated through this system means libraries, apps, content, relationships, AI — far more than one node can handle. The consolidated elohim-node binary is the unit of deployment; the operator manages the fleet.

**How to apply:** The elohim-node consolidation (single binary) is the prerequisite. Don't design the binary to "scale up" internally — design it to be lightweight enough to run many instances. The operator layer (not yet built) handles fleet orchestration.
