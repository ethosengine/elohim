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

## Root cause + landed cure (2026-07-09, feat/frontend-eyes-sprint)

Loki 7-day sweep (2026-07-02→09, ~2,876 adam failures + fleet view) settled the open questions:

- **CORRECTION to the observation above:** "the request timeout is already 30s" was wrong —
  the view-federation behaviour pinned a HARDCODED 3s `with_request_timeout`
  (`behaviour.rs`), overriding the general 30s; the caller budgets (10s sweep / 3s HTTP)
  never applied. Links were *marginal at 3s*, not dead.
- **Failure level: response-level.** Variant distribution: ~88% Timeout, ~9.5%
  Io(UnexpectedEof), ~1.2% DialFailure, <1% ConnectionClosed. Requests arrive; responders
  (esp. adam: DB read pool "Util 1737%", PTxnGuard ~1.8s) exceed the 3s budget — requester
  tears the stream while the responder is still building (matches responder-side
  "F-T20: failed to send response" ~2s after each requester timeout).
- **Fleet-wide, not adam-outbound:** last-24h failures by requester: matthew ~1,515,
  james ~1,373, jessica ~1,318, adam 37 — adam barely asks because his sweep tick took
  hours (inline conductor heal leg; one tick: ids_discovered=6000, healed=2925,
  divergent_anchor=2833). Lamad-bridge late-connect (7.5–15.6 min) is real but only delays
  the first ask — it is a compounding factor, not the cause.

Landed cure (committed, awaiting deploy + live re-measure):
- `4389bb8a4` — removed the 3s pin (transport now uses the general 30s; caller budgets are
  the effective deadline), split reconcile into conductor-free discovery (runs every tick
  from boot, no lamad gate) + single-flight background heal (a multi-hour heal no longer
  blocks discovery), and added the per-protocol counters this item asked for
  (`elohim_view_federation_outbound_total{result}`, inbound-served).
- `fd22cc9b2` — panic-safe RAII release of the heal single-flight flag.
- Divergent-anchor churn feeding responder load is being cured at the ingest layer
  (`bulk-seed-witness-bootstrap-single-head.md`, same arc).

Still open after deploy: re-measure the variant distribution via the new counters
(expect Timeout share to collapse); adam's DB read saturation is a separate concern if
timeouts persist at the 10s caller budget.

## Next move

Fold into the `sync-scale-honesty` red when it gets written: the mesh-scale
health scenario should assert view-federation request success rates alongside
sync-protocol ones. Instrument: a per-protocol request success/failure counter
on /metrics would make this measurable instead of Loki-anecdotal.
