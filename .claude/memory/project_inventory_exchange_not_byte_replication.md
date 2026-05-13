---
name: P2P inventory exchange ≠ byte replication
description: When alpha logs show "Received content inventory from peer count=N" but blobs only exist on the genesis peer, sync is metadata-only — byte-fetch is a separate step that must be diagnosed independently
type: project
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
Elohim-storage's P2P layer has two distinct activities that are easy to conflate:

1. **Inventory exchange** — peers gossip "I have content IDs [...]" lists. Logged as `Received content inventory from peer count=3430 total=3430`. Runs every 60s. Light, metadata-only, does NOT move blob bytes.

2. **Byte replication** — actually transferring shard/blob data between peers. This is what `distribute_shards` (Plan 1) does at ingest, what reconstruction (Plan 3) does on gap detection, and what verification (Plan 2) checks for cross-peer have-vs-get consistency.

**The critical observation (2026-04-30 debugging on alpha):** All 6 peers exchange inventory of 3430 items every minute, but after 36h the only peer with any blob bytes on disk is Adam (the genesis peer). Inventory says "we agree what content exists." Byte-fetch says "we have any of it locally." These are different things.

**How to apply:**
- When asked "is P2P sync working?", check both:
  - Inventory: `kubectl logs ... | grep "content inventory"` (gossip is alive)
  - Byte-fetch: count files in `/data/blobs/blobs/` on each peer (mobility is alive)
- If inventory works but byte-fetch is dead, do not conclude "P2P sync works." The diagnosis to chase is: are Plan 1's prerequisites met? (humans.household_id populated, REA commitments active, peer-policy configmaps configured, peer_statuses=accepting). The fan-out can be wired correctly but silently no-op if peer_selection returns empty due to unmet preconditions.
- Future debug message proposal: log "outbound shard X to peer Y" or "inbound shard X from peer Y" so byte-fetch becomes visible without filesystem inspection.
