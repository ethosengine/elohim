---
name: P2P mesh is the hosting layer, doorway is web 2.0 projection
description: The peer-sharding P2P mesh IS the content hosting infrastructure. Doorway is an optional projection layer for web 2.0 browsers, not the hosting layer itself.
type: project
originSessionId: 63499c63-1cde-41b5-a0b0-66503d4c008c
---
The P2P mesh (libp2p in elohim-storage) provides human-scale content distribution. Peers who care about content shard and replicate it among themselves — 50 people interested in a learning path means 50 peers collectively hosting it. No central hosting needed.

Doorway is NOT the hosting layer. It's a web 2.0 projection layer. A peer contracts with doorway: "I steward this content on the mesh; please serve it to HTTP browsers for me." Doorway absorbs the thundering herd of browser traffic, shielding peers from scale they can't handle.

**Why this matters:**
- Holo Host's value prop (hosting for users who can't self-host) is already solved by the peer mesh
- Doorway's value prop is different: web 2.0 projection + HTTP scale absorption
- A peer can function with zero doorway involvement (layers 1+2 only)
- Doorway is optional, not architectural

**How to apply:**
- When designing node architecture: conductor + storage belong together (they're the node, layers 1+2). Doorway stays external (layer 3, different concern).
- When designing P2P features: the mesh is primary. Don't route through doorway what peers can serve directly.
- When explaining the system: "peers host for each other; doorway projects that back to the web" — not "doorway hosts content for users."
