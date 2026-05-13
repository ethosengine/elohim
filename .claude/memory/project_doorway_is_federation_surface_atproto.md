---
name: AT Protocol interop lives at doorway, not elohim-storage
description: Federation-flavor interop (AT Proto, ActivityPub, etc.) belongs at doorway as a projection surface; peer-native primitives stay clean of lexicon-level concerns
type: project
originSessionId: 22f7d697-5b2e-4099-a324-99589cb67e40
---
AT Protocol (and DDS-WG-style lexicons) interop belongs at the doorway federation layer, not in elohim-storage or DHT primitives.

**Why:** Doorway is already the web2 projection layer (three-layer-truth-model memory) with federation responsibilities — DNS, OAuth-RP, federated login, federated discovery, bootstrap server. AT Protocol is structurally another federation flavor (server-as-truth, lexicon-driven, Firehose-stream); it lives where federation lives. Pulling it into elohim-storage would pollute peer-native primitives with lexicon-level concerns AND hard-bind the protocol to AT Proto's substrate assumptions (PDS-as-server-of-record, did:plc, ETH commitment).

**How to apply:**
- Any AT Proto / DDS / ActivityPub interop spec is a *doorway* spec, not a storage spec. Two-doc layering: (1) peer-native primitive in elohim-storage, (2) projection adapter in doorway.
- Doorway operators choose interop posture per-instance — different doorways can speak different federation flavors. Protocol stays uniform; federation surface varies.
- Outbound projection: doorway translates peer-native EPRs into `org.dds.module.*` / `org.dds.result.*` records. Signing question: doorway-as-relying-party signs "faithfully projected from peer X's EPR Y" claim (peers don't have did:plc).
- Inbound ingest: doorway subscribes to Firehose, translates incoming AT Proto records as elohim-protocol-ingest-signal EPRs. Peers decide whether to consume. Same pattern as content federation already does.
- Optional graduation path: opportunistic did:plc per peer at registration time with doorway co-signing rotation events. Higher interop, parallel identity surface — defer until needed.
- Cross-chain commitment (ETH/Arweave/Filecoin/Logos) sits at the same layer: peer-native primitive uses the existing chain-agnostic SettlementBridge trait; doorway-mediated bridge to specific chains is operator-opt-in.
