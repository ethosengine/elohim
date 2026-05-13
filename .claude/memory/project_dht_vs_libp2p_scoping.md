---
name: DHT vs libp2p scoping — keep DHT narrow and authoritative
description: When designing new data flows, keep DHT usage narrow; push operational/ephemeral state onto libp2p. DHT signing is expensive and authoritative.
type: project
originSessionId: 17546f03-3ee8-4704-bdf9-18d0d64baf9b
---
DHT signing is expensive and authoritative. Keep DHT usage narrow to protect the integrity concern Holochain actually provides; push operational/ephemeral state (gossip, census, shard placement state) onto libp2p.

**Why:** User framing (2026-04-19, self-healing dataplane spec): DHT signing costs real bandwidth + entry budget (~3000 entries per DNA before degradation). Noise on the DHT erodes the signal — the things that ARE notarized should be the things that matter for protocol integrity.

**How to apply:**
- Default: new state goes on libp2p gossip + local SQLite projection, not DHT.
- Only reach for a DHT entry type when the claim is protocol-integrity-load-bearing (attestations, governance, economic events, cross-peer trust anchors).
- For dataplane ops (shard placement census, verification results, reconstruction events): libp2p + local tables. Notarize outcomes (holding attestations) only when cross-peer trust demands it (a later "rung e" layer).
- Matches the inside-out pattern: Holochain = notary; libp2p = dataplane; doorway = projection.
