---
id: "backlog-content-gap-limit-cycle-blocks-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet-wide content-gap limit cycle blocks convergence — ~2.8k discovered-never-fetched ids per pod, divergent oscillating with zero decay 3h+"
slug: "content-gap-limit-cycle-blocks-convergence"
written: "2026-08-01"
author: "quiescence-gated-saga-recording shift (post-#1285 fleet observation)"
status: "backlog"
priority: "high"
jobs: [elohim-edge]
---

# Fleet-wide content-gap limit cycle blocks convergence

## Finding (evidence, 2026-08-01 ~14:00-18:20Z, post edge #1285 deploy)

The fleet does not drain to convergence after the #1285 restart — it enters a
**stable limit cycle**:

- `elohim_projection_reconcile_divergent{pod=adam,stream=content}` repeats an
  identical ~30-min waveform for 3+ hours with ZERO amplitude decay:
  ~1245 → ~2500/2740 → ~3055 → ~1245 … (Prometheus range 15:58-18:58Z).
- `elohim_projection_reconcile_gaps{stream=content}` sits at a stable plateau
  on EVERY pod: matthew 2771, adam 2844, jessica 2872, james 2099,
  gertrude 2518, eve 2453, susan 1075 — discovered-but-never-fetched ids that
  cycle instead of filling.
- matthew's discovery-complete lines show per-sweep ids_discovered swinging
  1.7k→12k→10.5k with gaps climbing 1583→2954→3308 inside an hour; content
  divergence adjudication keeps pace (refused ≈ divergent), so the actionable
  count stays 0-3 — divergence is NOT the blocker, the unfetched gaps are
  (`converged` needs `pending==0 && exhausted==0`).
- doorway /health p2p rollup on A oscillates converged true↔false with raw
  divergentAnchor cycling ~810/~1381/~1757 in phase with the sweeps.
- rea gaps: matthew cleared to 0; james 7, jessica 7, susan 67 (susan's dead
  conductor WS is a known incident).

Consequence: the resiliency-saga recording (needs A converged sustained) is
blocked indefinitely; two gated validate-only runs (#1283, #1286) honestly
no-measured against this state. The floor held at 8/11 — the quiesce gate
prevented the false-red recordings this state would previously have produced.

## Suspected shapes (verify, don't assume)

- Inventory-vs-bytes split (inventory gossip is metadata-only; per-peer blob
  custody may never satisfy these ids) — the ~2.8k ids may be advertised by
  peers that cannot serve the bytes (shem pods / susan's WS), so every fetch
  attempt fails and the gap re-cycles instead of going exhausted-terminal.
- Per-sweep discovery scope changing (ids_discovered swings 1.7k↔12k) makes
  gap/divergent counts oscillate rather than converge — possibly two
  discovery sources alternating (inventory vs anchor walk).
- The 2026-08-01 runbook row for divergentAnchor>200 hours-class assumed a
  DRAIN; this is the third regime: a NON-draining cycle. Runbook needs the
  distinction once root-caused.

## Next-shift shape

rust-architect deep-dive on elohim-storage reconcile/acquisition: what are
the ~2.8k gap ids (sample them via the discovery ledger), who advertises
them, why fetches never succeed, and why they never reach exhausted-terminal
(or why exhausted resets). The fix likely belongs in the acquisition
pull-queue / MissLedger family (ch11 territory), not adjudication.
