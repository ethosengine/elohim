---
id: "backlog-view-federation-request-flakiness-mesh-wide"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Chronic F-T19 view-federation request failures mesh-wide (Timeout / UnexpectedEof to multiple peers) — pre-existing, now load-bearing for reconcile sweeps"
slug: "view-federation-request-flakiness-mesh-wide"
written: "2026-07-04"
author: "notary-authority-land shift"
status: "open"
priority: "medium"
ci_status: open
jobs: [elohim-edge]
tags: [p2p, view-federation, request-response, libp2p, reconcile, timeouts, sync-scale-honesty]
cites:
  - elohim/elohim-storage/src/p2p/view_federation.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - genesis/manifests/spine.yaml
---

# Chronic F-T19 view-federation request flakiness (mesh-wide)

## Observed (2026-07-04, Loki, elohim-alpha)

During the notary-authority landing: adam→matthew content-inventory requests
failed 4/4 (`Io(Kind(UnexpectedEof))` ×3, `Timeout` ×1, 17:09–17:15Z). The
UnexpectedEof class was root-caused and fixed (256KB `MAX_PAYLOAD` exceeded by
the content inventory — responder killed its own reply). BUT matthew's
pre-deploy logs (14:00–14:24Z) also show frequent `F-T19: outbound
view-federation request failed` **Timeout** entries to *other* peers —
a standing, mesh-wide request-response flakiness unexplained by the payload fix.
PROVENANCE CORRECTION (19:45Z): all observed F-T19 evidence dates from AFTER the
content arm first deployed (~13:4xZ) — whether this flakiness pre-dates the
content sweep's added request volume is OPEN, not established. Check historical
Loki (before 2026-07-04) before treating it as pre-existing. Also observed on
adam (18:47–19:27Z): outbound view-federation Timeouts to ~11 distinct peers
continuously, while a brief post-boot window succeeds (3/11 at 18:48) — the
request timeout is already 30s, so these links are effectively dead from adam's
side, not marginal. Related lead: adam's content heal-sweep is gated on the
lamad bridge (13-minute late connect on adam), which pushes the sweep's first
asks past the healthy boot window — decoupling discovery (no conductor needed)
from heal (conductor needed) would let discovery use the boot window.

## Why it matters

Reconcile sweeps (REA + content) treat a failed inventory request as
"peer not asked this sweep" — persistent flakiness slows convergence
linearly and silently. This is the view-federation sibling of the sync-plane
timeouts named in spine node `sync-scale-honesty` first_move (task #9,
persistent sync-request timeouts from 3 peers) — likely the same underlying
transport/relay condition, different protocol.

## Next move

Fold into the `sync-scale-honesty` red when it gets written: the mesh-scale
health scenario should assert view-federation request success rates alongside
sync-protocol ones. Instrument: a per-protocol request success/failure counter
on /metrics would make this measurable instead of Loki-anecdotal.
