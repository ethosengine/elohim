---
name: feedback_p2p_vs_federation_layer_vocabulary
title: p2p vs federation/fediverse layer vocabulary
description: "p2p = DHT conductor + iroh/libp2p blob stores; federation = doorways riding OVER p2p (DHT-gossiped DoorwayRegistration); route WAN-NAT gaps by layer."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 89b2cfb3-8192-4899-ad63-aac5d7e04cf3
---

The architect names the stack as two layers; use these terms precisely (2026-06-23):

- **p2p** = the **substrate**: the Holochain DHT (conductor) **and** the iroh/libp2p blob
  stores. Both byte-replication transports are "p2p," together with the DHT.
- **federation / fediverse** = the **doorways** — a web2-projection layer that sits **over**
  p2p. (doorway.elohim.host, alpha.elohim.host/threshold, the `/threshold/doorways` selector.)

**Why:** I lumped all three (conductor DHT + iroh/libp2p + doorway HTTP) as undifferentiated
"planes" in a WAN-NAT gap doc; the architect corrected the framing. Mixing them obscures the
key architectural fact below and mis-routes where a fix belongs.

**How to apply:**
- **Federation rides p2p.** Doorway peer-discovery is NOT a separate direct doorway-to-doorway
  mechanism — doorways discover each other through DHT-gossiped `DoorwayRegistration` entries on
  the Holochain conductor (that is exactly how the 2026-06-23 `get_all_doorways` fix made the
  selector list peers: it reads the p2p DHT, not a doorway-to-doorway HTTP call). The
  `FEDERATION_PEERS` HTTP path is only a bootstrap fallback.
- **Route WAN-NAT gaps by layer:** iroh/libp2p blob-store reachability = a **p2p** gap; the
  doorway selector / cross-doorway JWKS = a **federation/fediverse** gap. Don't call a doorway
  concern a "p2p" concern or vice-versa.
- The conductor DHT (kitsune2/tx5 + signal/bootstrap) is the WAN-native reference posture for
  the p2p layer; the iroh/libp2p blob stores are the lagging p2p sub-layer (pinned in-cluster).

Sibling framing-guards: [[feedback_k8s_is_not_the_architecture]] (k8s models compute/hardware,
not the peer-native home), [[feedback-identity-sovereignty-ontology-guard]] (apex-tier lexicon).
Canonical record this corrected: backlog `wan-nat-federation-dataplane-discovery-gap-2026-06-23`.
