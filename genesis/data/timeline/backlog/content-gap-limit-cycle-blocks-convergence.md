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

## RCA outcome (2026-08-01, follow-up shift — root-caused and fixed)

Deep-dive (rust-architect code trace + live Prometheus/Loki evidence) landed on a
different mechanism than the suspected shapes above — the acquisition/pull-queue
family was NOT the seat of the cycle:

- **Root cause: the adopt-before-author head-adoption arm — the ONLY path that
  can genuinely converge a peer-divergent/undeclared content head — timed out
  uniformly fleet-wide.** Two responder-side defects compounded:
  (i) `build_content_head_record_payload` awaited `call_get_record_for_action`
  UNBOUNDED (`HcClient::call_zome` has no timeout; effective bound ≈ the
  conductor's ~60s WS deadline) while the requester's budget is 10s
  (`head_record_client::HEAD_RECORD_TIMEOUT`), so a loaded responder could never
  answer in time; (ii) the view-federation inbound arm awaited the slice-build
  INLINE on the libp2p swarm event loop, so one inbound head-record ask wedged
  the responder's entire P2P loop (the custody-reconcile arm had already learned
  and fixed this exact lesson; this arm never got the cure). Symmetric across
  the fleet → every adoption fetch timed out → nothing ever converged.
- **The largest heal-outcome class ("missing", ~14k/h fleet-wide) never reached
  adoption at all**: conductor-missing rows with NULL `dht_anchor_hash` (the
  dominant `AnchorGap` class) were filtered out of the ghost-witness pre-flight
  and fell to `witness_bootstrap`'s self-authoring path — the exact self-election
  adopt-before-author exists to prevent.
- **The invisibility (why it read as a mystery)**: the `gaps` gauge is a
  per-sweep recount over a rotating 2000-row inventory page (6 peers × 2000 =
  the 12k discovery peak; ~6 sweeps/rotation × 300s = the ~30-min waveform —
  one cycling scope window, not two alternating sources); `exhausted` is a
  page-scoped recount reset by advertiser-flip evidence churn (the live
  0→252→752→0 sawtooth); and `converged` was gated on the REA arm only — the
  content arm was structurally invisible to the gauge.

**Fix landed (this shift): F-A(i)** responder conductor call bounded at 5s with
honest-absence (hash-only) fallback; **F-A(ii)** view-federation inbound
handling spawned off the event loop, response delivered via
`P2PCommand::SendViewFederationResponse`; **missing-class routing** —
conductor-missing rows with a peer-advertised declaration now join
`adopt_candidates`; **F-D** — content+collectives post-heal `GapCounts` folded
into the published sweep (`converged`/`caught_up` AND-folded, strictly
stricter).

**Deliberately deferred (still open, this doc carries them):**
- **F-B** — adoption throughput fan-out (today: sequential, 200/tick, 25ms
  spacing, 120s budget). Only needed if the post-fix drain proves too slow.
- **F-C** — MissLedger terminality backstop: order-independent evidence (digest
  of the advertised anchor SET, not first-advertiser-wins) + heal-outcome
  feedback (`Refreshed`/`SkippedDeclared`/`SkippedStale` count as misses).
  Stops the exhausted sawtooth from disguising the true backlog.
- **RC-3** — inventory paging is OFFSET over `updated_at DESC` while every
  stamp (incl. no-op `Refreshed`) bumps `updated_at`; window advances by CAP
  even when the responder byte-trims the page (latent skip hole). Fix shape:
  keyset pagination on a stable key; advance by rows served.
- **RC-4** — `classify_content_gap` presence/anchor reach-filter asymmetry:
  anchored-but-scoped-reach rows classify `AnchorGap` forever; heal can PUSH a
  row into the trap by overwriting local `reach` from the conductor answer.
- **Converged softness** — `mark_failed` drains `pending`, so persistently
  failing rows do not block `converged` (kept deliberately: a dead conductor
  must not wedge the fleet gauge; failures stay visible in `failed` + the
  pre-heal `gaps` gauge). Revisit together with F-C.
- **RC-5 / pull-stream** — metadata-only inventory re-admission of retired
  acquisition pins against byte-less advertisers is a REAL but SEPARATE churn
  loop (ch11's re-admission arm); gate re-admission on byte evidence or bound
  re-admissions per window. Watch `elohim_acquisition_pin_retirements_total{reason="readmitted"}`.
