---
id: "backlog-household-chaos-restore-window-shorter-than-mdns-requery"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "chaos-peer-churn's @chaos After hook waits 120 s for every restarted peer to see 2 neighbours, but two peers restarted in the same second only find each other at the next mDNS re-query (~5 min): LAN redial covers only mDNS-discovered peers and the household has no bootstrap list"
slug: "household-chaos-restore-window-shorter-than-mdns-requery"
written: "2026-11-08"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "open"
priority: "medium"
jobs: [household-mesh]
cluster: "dataplane-peer-fallback-and-blob-replication"
tags: [a2o, household, chaos, mdns, peering, timing]
---

**Observed 2026-11-08 18:13–18:18Z** on the shem household (storage + doorway from `artifact-store/h`
86e1395e0, conductor fork 993cf597a19b), run `sprint-report-household-20261108T180942Z-86e1395e`:
`chaos-peer-churn.feature` 34/34 steps passed; scenario "Simultaneous loss of two peers leaves the
survivor degrading honestly" went red only in `After # steps/mesh/household-chaos.steps.ts:1927`:
`jessica sees 1 neighbours; a joined peer must see 2`.

**Mechanism (logs, `genesis/local-dev/household-dowell/logs/{jessica,james}.log`):** the restore
restarted james (P2P up 18:13:16.598, listening 16.875) and jessica (18:13:16.786) together. Each
peer's startup mDNS pass ran before the other listened, so jessica discovered only matthew (18:13:17);
the startup dial to james failed (`F-T19 … DialFailure`). `redial_disconnected_lan_peers`
(`elohim-storage/src/p2p/mod.rs` ~:3972, every `LAN_REDIAL_INTERVAL_SECS`=10) dials only peers in
`delivery_peers` with `network == "lan"`, i.e. mDNS-discovered ones; `bootstrapNodes` is empty on the
household. jessica rediscovered james at the next mDNS query, 18:18:05, and both reached 2 neighbours
by 18:18:09. The hook's budget is `RESYNC_WINDOW_MS` = 120 s (`E2E_PEER_RESYNC_WINDOW_MS`).

**Not a regression of the storage closure:** its diff touches no dial, redial or mDNS code (checked
`git diff 9faf54076 86e1395e0 -- elohim-storage/src/p2p/mod.rs`). Timing-dependent: it reds only when
the two restarted peers race within the mDNS startup pass.

**Cure options (pick one):** (a) a peer that sees an inbound connection or an iroh/inventory message
from an unknown-to-libp2p LAN peer records its addresses for the redial arm; (b) re-run an mDNS query
shortly after start (e.g. at +15 s) instead of waiting the library's default interval; (c) the hook
restarts peers one at a time, or waits for listen before the next. (a) or (b) fixes the product, not
only the test.
