---
name: Doorway is single-target dispatch — no blob fan-out, ever
description: Architectural decision (2026-04-30) — doorway forwards each blob request to ONE storage target; peer-iteration belongs in the P2P substrate, never in the web2 projection layer
type: project
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
Doorway forwards each `/blob/<hash>` request to a single storage target (singular `STORAGE_URL` or a registry-routed peer). It does NOT iterate `STORAGE_URLS` looking for which peer holds a particular blob. This is durable architectural decision, not a temporary state.

**Why:** Maintains the three-layer truth model (DHT=notary, libp2p=data-ops, doorway=web2 projection). Adding peer-aware blob routing to doorway would re-introduce P2P logic into the web2 layer where it doesn't belong, creating two competing sources of "where does this blob live" truth. The substrate is responsible for byte mobility — if a blob isn't reachable from the routed peer, that's a P2P replication bug to fix in elohim-storage, never in doorway.

This decision was reached after a debugging session (2026-04-30) where a blob seeded on Adam wasn't accessible via doorway-alpha because doorway forwards to Matthew (singular `STORAGE_URL`). Investigation confirmed peer-fan-out at the doorway layer is the wrong fix — the right fix is making P2P substrate replication actually move bytes (currently inventory exchange runs but byte transfer doesn't).

**How to apply:**
- When debugging "where's my blob" symptoms, reject any proposal to add peer-iteration logic to doorway. Such proposals are a regression vector.
- The right paths are: (a) ensure substrate replication moves bytes to all commons-eligible peers; (b) ensure doorway's blob-tier cache writes-on-fetch so subsequent requests are local. Neither requires doorway to know about peer multiplicity.
- Documented in `doorway/CLAUDE.md` "CRITICAL: No Blob Fan-Out" section. If a future reader proposes fan-out, point them at that section.
