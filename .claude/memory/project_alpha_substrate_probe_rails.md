---
name: alpha-substrate-probe-rails
title: Alpha substrate probe rails
description: "Doorway reads are per-doorway single-target (A→matthew, B→adam); conductor projection fans the pool; Loki 502s = untrustworthy zeros."
metadata: 
  node_type: memory
  type: project
  originSessionId: ff42f03d-630b-4a73-9282-9b8ad2b78e57
---

Read-only probe rails against live alpha from the dev session (verified 2026-06-11, EPR-arc Phase 0):

- `https://doorway-alpha.elohim.host/api/v1/commitments?...` (and `/db/rea_commitments?...`) WORK. **CORRECTION 2026-07-07:** reads are per-doorway **single-target to that doorway's OWN storage backend** (doorway-A/doorway-alpha → matthew; doorway-B/elohim.host → adam) — NOT "matthew only" universally (the 2026-06-11 note read doorway-A alone). The conductor projection worker pool fans round-robin across the whole peer pool. To see adam's projection curl elohim.host; matthew's, curl doorway-alpha.
- `/p2p/status` — **CORRECTED 2026-07-07:** it IS now doorway-proxied (declared in storage `build_manifest`, commit 84aea07a7 / the amber single-head arc). Before that it fell through to the SPA (HTTP 200 + HTML), which false-flagged the lamad sync strip as "unreachable". Pod p2p counters (`kicksFiredTotal`, `reconcilePassesTotal`) are pipeline-internal; from dev they come via Loki or CI artifacts.
- substrate-verify.sh artifacts do NOT capture the p2p counters (projection stage extracts only caughtUp flags; mesh only peerId/connectedPeers).
- Loki: adam's pod logged 26GB/day (~20× siblings) and saturated Loki into blanket 502s mid-investigation — treat Loki zero-results during 502 storms as "query failed", not "absent" (see [[backlog ops-adam-pod-log-volume-saturates-loki]], genesis/data/timeline/backlog/).
- Storage container label in Loki is `container="elohim-node"` / `instance="<name>-alpha"` — NOT `elohim-storage`.

**Why:** Phase-0 of the EPR durability arc burned a workflow round discovering these; the matthew-only doorway read was the decisive probe (16 anchored custody rows visible while jessica's reconcile discovered 0 ids).

**How to apply:** for cross-pod projection questions, curl doorway for matthew's view + Loki (post-502-check) for the others; for counters, wait for a CI run or add them to substrate-verify artifacts.

**New rails (dht-unity arc, 2026-07-11 — deployed via edge #1172/#1173):**
- `GET {doorway}/db/p2p/conductor-diagnostics[?include=metrics]` — the routed (PRIMARY) conductor's own peer store (agent_info projection), live transport connections (dump_network_stats), and per-DNA fetch queue incl. peers_on_backoff + gossip rounds. B→adam's conductor, A→matthew's (same primary routing as above).
- `POST {doorway}/admin/steward-peers/refresh` — re-fetch every configured storage's /manifest and recompile routes in place (registration is otherwise BOOT-only; a storage deploy adding a route needs this or a doorway restart). Response lists per-peer route counts — doubles as a "which storages answer, at which manifest version" probe. Slow (~1-4 min): sequential fetches, unreachable peers eat timeouts.
- `GET {doorway}/admin/bootstrap-coherence` — kitsune2 shared-bootstrap read (spaces × agents held) — both doorways read the same mongo table.
- Outside-in SBD cross-relay probe: `doorway/doorway-service/tools/sbd-cross-relay-probe.py` (needs pynacl+websockets) — proves mongo signal-bus cross-pod delivery from the public internet, no cluster access.
- Trap: new storage routes under `/db/<new-namespace>/…` must ALSO be added to `extract_app_context`'s legacy_prefixes in storage http.rs or the segment is eaten as h_app_id and the handler is dead code (edge #1172 lesson; regression-tested now).
