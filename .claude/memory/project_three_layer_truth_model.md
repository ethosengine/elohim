---
name: Three-layer truth model — DHT / libp2p / doorway
description: The Elohim Protocol separates concerns into three layers, each with a distinct role; code placement must match the layer's role
type: project
originSessionId: 253292ea-69ea-4e76-86e3-6d87ebdac46c
---
The protocol's architecture has three layers, each with one job. When designing any feature, placement decisions should start by asking which layer the work belongs to.

- **Holochain DHT — notary / authoritative.** Integrity-only concerns. Entry types, validators, signed commitments, provenance. Expensive to sign; keep narrow.
- **libp2p — performant + resilient data-ops.** Content distribution, gossip, bulk replication, sharded backups, discovery, peer-to-peer operations. This is where "data movement" lives. Elohim-storage is the canonical libp2p participant per deployment.
- **Doorway — web2 projection.** HTTP routing, proxy, CDN, load-balancing, convenience hosted-conductor facilitation for browser clients. Doorway is NOT a P2P participant; it does not run a libp2p swarm. The doorway-steward runs a colocated elohim-storage pod that IS the P2P participant.

**Why:** conflating layers (e.g., putting libp2p code in doorway, or pushing operational state into the DHT) produces brittle architecture. DHT churn explodes if operational data lives there; doorway becomes a second identity on the P2P mesh if it carries its own swarm; web2 conveniences leak into P2P code paths and vice versa.

**Applied to recovery flows specifically:** routine re-login after device loss is a doorway/web2 concern — facilitate a hosted conductor, done. Identity recovery (lost credentials, lost account, elevated security events) is where the human + elohim witness ceremony happens, gated by DHT-notarized quorum. The two paths are separable; design for them separately.

**How to apply:**
- When a new feature touches data movement, first instinct is elohim-storage + libp2p, not doorway.
- When a feature needs attestation / integrity, DHT is the home.
- When a feature is HTTP UX convenience, doorway is the home.
- If code seems like it wants to live in two layers at once, you're probably missing a layer boundary — split it.
- Reject "doorway libp2p subsystem" framings; they point at the wrong layer. The doorway-steward's P2P pod (elohim-storage) is what you actually mean.
