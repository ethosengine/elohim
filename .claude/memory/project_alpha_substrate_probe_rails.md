---
name: alpha-substrate-probe-rails
description: "What IS and ISN'T reachable read-only from the dev session for live-alpha substrate diagnosis (doorway proxy scope, Loki caveats)"
metadata: 
  node_type: memory
  type: project
  originSessionId: ff42f03d-630b-4a73-9282-9b8ad2b78e57
---

Read-only probe rails against live alpha from the dev session (verified 2026-06-11, EPR-arc Phase 0):

- `https://doorway-alpha.elohim.host/api/v1/commitments?...` (and `/db/rea_commitments?...`) WORK — but the EprRouter routes storage-proxy reads to **matthew's pod only**, so this is a window into ONE projection, never jessica/adam.
- `/p2p/status` is NOT proxied — it falls through to the SPA (HTTP 200 + HTML). Pod p2p counters (`kicksFiredTotal`, `reconcilePassesTotal`) are pipeline-internal; from dev they come only via Loki or CI artifacts.
- substrate-verify.sh artifacts do NOT capture the p2p counters (projection stage extracts only caughtUp flags; mesh only peerId/connectedPeers).
- Loki: adam's pod logged 26GB/day (~20× siblings) and saturated Loki into blanket 502s mid-investigation — treat Loki zero-results during 502 storms as "query failed", not "absent" (see [[backlog ops-adam-pod-log-volume-saturates-loki]], genesis/data/timeline/backlog/).
- Storage container label in Loki is `container="elohim-node"` / `instance="<name>-alpha"` — NOT `elohim-storage`.

**Why:** Phase-0 of the EPR durability arc burned a workflow round discovering these; the matthew-only doorway read was the decisive probe (16 anchored custody rows visible while jessica's reconcile discovered 0 ids).

**How to apply:** for cross-pod projection questions, curl doorway for matthew's view + Loki (post-502-check) for the others; for counters, wait for a CI run or add them to substrate-verify artifacts.
