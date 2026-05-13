---
name: Shem is the live P2P modeling canvas (household + shem topology)
description: shem runs most personas; Matthew/Jessica/Timothy run on the household cluster on separate nodes (Matthew is a doorway operator); cross-node topology is the real P2P test environment
type: project
originSessionId: 9c3a2266-4f19-410d-b4d4-30d06366e38d
---

The live P2P test environment is a **multi-node topology**, not a single-node simulation:

- **Household cluster** — Matthew, Jessica, and Timothy live on the household cluster on separate nodes. This represents a real household with real per-member device archetypes. **Matthew is a doorway operator** — his node runs a doorway for the household's peers.
- **Shem** — every other persona (Susan, Adam, Eve, Pete, Nancy, Gertrude, Maria...) runs on shem exclusively. Shem has the compute headroom to deploy all of them as real, independent peers with their own conductors + storage + doorway projections.

The two form a cross-node P2P environment: household peers federating with Matthew's doorway, shem personas federating through shem's doorway, and the DHT gossip + libp2p traffic crossing between them. This is what makes it a real P2P test — not everyone is on one box.

**Why the topology matters:** a2o scenarios prove logic correctness; the cross-node topology proves emergent P2P behavior (cross-doorway federation, real DHT gossip, real libp2p latency, real churn as nodes go offline and come back). Features claiming "P2P resilience" must run in this topology.

**Matthew-as-doorway-operator edge cases** (notable for recovery and identity features):
- Matthew's personal identity and his doorway service run on the same hardware but are distinct concerns — his personal cell is not the same as the doorway service.
- When Matthew loses his device, his doorway goes down separately from the identity recovery. Identity recovery uses a peer doorway (e.g., shem's). Doorway continuity is an operational concern (Phase 3+ migration story), not identity.
- Multi-doorway federation is tested naturally: household peers default to Matthew's doorway, shem peers default to shem's, cross-federation recovery and gossip exercise both.

**How to apply:**
- For P2P sprints, include a "cross-node activation" deliverable — deploy personas per the correct placement (Matthew/Jessica/Timothy on household, everyone else on shem).
- Acceptance test topology must include cross-node flows, not single-box simulation.
- Dashboards (/shefa/devices, /shefa/dashboard, /shefa/resources, doorway admin) on BOTH Matthew's doorway and shem's doorway are the observability surface; both must light up.
- Recovery demo specifically: Matthew loses his device (and his doorway temporarily); he recovers via shem's doorway; Jessica + Timothy (household, on their own nodes) + Pete + another shem persona (on shem) authorize; Matthew lands in a hosted cell at shem's doorway until he restores his own doorway operation.
