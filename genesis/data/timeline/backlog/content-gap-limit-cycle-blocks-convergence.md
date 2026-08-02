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

## Post-deploy verdict (2026-08-02 ~01:40Z, edge #1287 + ~4h live observation)

**The transport cure is proven live; the fleet still does not converge — the
residue is a DIFFERENT illness, now precisely characterized.**

Cure confirmed (all on the new image, all 7 pods):
- "federation timeout" head-adoption lines: **zero** in sampled windows (was
  the dominant per-request class, thousands/hour).
- Responder budget works: `elohim_content_head_record_degraded_total{cause=
  "budget_elapsed"}` ticks modestly (10 in the first hour) instead of wedging
  the swarm loop.
- The adopt-deferred arm RUNS every sweep (was: never completed a fetch).
- The honest fold works: `/health p2p.caughtUp/converged` read false while
  content work is outstanding — the gauge can no longer lie.

Residue (the remaining blocker, NOT the cycle this doc originally named):
- Fleet gaps-sum still oscillates 9k-19.8k with no trend over 4h. Adoption
  total after 4h: **15**. Every adopt sweep logs
  `adopt-deferred: candidates ~300, adopted 0, held 200` (200 = per-tick cap)
  — `decide_head_action` Holds because the local row is ALREADY DECLARED with
  a different head than the peer advertises: **two-way declared divergence,
  ~11.4k divergent / ~10.7k refused fleet-wide** — consistent with adam's
  re-anchoring storm having minted competing declared heads corpus-wide.
- Heal outcomes (2h): refused_declared 25k · missing 24.9k · failed 18.2k ·
  healed 59. The projection plane spends ~35k ops/hour honestly re-refusing
  the same divergence.
- Rust-side newest-wins is DELIBERATELY absent (`head_adoption.rs` module doc:
  `declared_head_at` not globally comparable; carried-record branch substitutes
  receiving conductor sys_time; presence-based rule prevents head-flapping).
  The `HealCanonical` forward-ordering arm IS implemented
  (`content_diesel.rs` stamp guard) but canonical answers lack provably-newer
  `declared_at` for this class.
- **The intended arbiter is in-zome: `select_canonical_winner` on the
  conductor/DHT plane.** For ~11k rows it is not converging the fleet — either
  the competing declaration links never propagated cross-conductor (gossip /
  witness gap), or the in-zome election cannot order them either.

## Next-shift shape (v2 — conductor-plane RCA)

Why doesn't zome-level canonical arbitration converge the two-way declared
class? Sample competing pairs (pick ids from adopt-deferred Hold logs on two
pods), inspect both conductors' views of the declaration links
(`select_canonical_winner` inputs), and determine: propagation gap vs
election tie vs elected-but-projection-never-informed. ch06 heads-converge
territory; touches the substrate trust contract (canonical channels alone
move declared heads). The projection plane is now honest and instrumented —
`refused_declared`, `divergent_refused`, and the Hold counts are the
progress meters for whatever the conductor-plane fix is.

Measurement lesson for anyone watching the drain: the per-sweep `gaps` gauge
remains a rotating-page sample — fleet-sum oscillates ±5k by construction and
a 30-60min decline proves nothing (this shift briefly mistook a downswing for
a drain). Trend-trustworthy signals: `healedTotal`, `head_adopted_total`,
`divergent_refused`, and the converged/caughtUp holds.

## RCA v3 — conductor-plane arbitration CURED in structure (2026-08-02 shift, five waves)

The v2 question ("why doesn't zome-level canonical arbitration converge the
two-way declared class?") is answered and the cure is live. The answer was
FOUR stacked walls, each a correct safety rule, discovered serially because
each was only observable after the previous was cured:

1. **Supply deadlock** — the only automated canonical-link minter (AdoptPeer)
   is unreachable when the local row is declared; the canonical_head anchor
   was EMPTY for the whole class; select_canonical_winner never ran. (The
   projection's "declared head" and the zome's canonical-head link are
   different authority planes sharing one phrase.)
2. **Consumption wall** — the HealCanonical stamp guard compared wall-clock
   declared_at (NULLed by the deploy PATCH path); the election's own clock was
   computed in-zome then DISCARDED; AdoptLocal was structurally unreachable.
3. **Declare wall** — contest declares fail on conductor-missing ids
   (target-independent no-chain gate); two plausible hypotheses (reach gate,
   unwired fetcher) were DISCONFIRMED by code before instrumentation named it.
4. **Admission wall** — chain-HOLDING pods never contested: declared+divergent
   rows died at the SkippedDeclared refusal without entering the candidate
   list (gapfill route requires local-undeclared). Neither side ever supplied
   the election.

**Landed (commits 496a4aba8, 134331c83, da8975176; DNA #1388/#1390 SUCCESS;
edge #1289/#1291/#1292/#1293 deployed):** the election clock + tier travel in
head answers (canonical_declared_at/canonical_earned, additive); the stamp
guard keys on election ordering (earned > elected > un-elected; both-elected
compares notarized link clocks — never wall-clock); AdoptLocal reachable;
ContestPeer mints canonical candidates (carried-record verified, self-head
fallback when chain exists, (id,target) idempotence, never stamps);
declared-divergence rows ADMITTED to candidates at the refusal site;
election-obey for conductor-missing rows (resolve_canonical_election reads
the election WITHOUT target retrieval; peer-couriered bytes zome-validated
against the elected target). Telemetry that makes every layer visible:
canonical_answers_total{tier}, contest_failed_total{class},
canonical_links_minted_total{source}, election_obeyed/obey_failed{class},
refused_stale{reason}.

**Live verdict at shift close (~15:15Z):** every layer PROVEN live — CONTESTED
lines minting on formerly-stuck pods (carried and non-carried shapes),
staging-tier canonical answers climbing (0 → 605), divergent fleet-sum band
down ~33% (19,252 → ~12,9xx). The fleet converges in STRUCTURE but not yet in
RATE: ~90 elections minted against ~11k contested ids in the first hour.

**Open residuals (this doc still carries them):**
- **F-B (throughput) — now PROVEN necessary:** sweep budgets (200/tick, 120s
  wall-clock, 300s cadence) are burned by no_local_chain candidates
  (predictable failures) crowding out productive contests; add fan-out and/or
  a known-no-chain backoff so contest supply reaches the full corpus in hours
  not days. This is the next code lever.
- **Both-sides-missing pairs** — unreachable by any storage arm; approximate
  post-hoc via obey-fetch failures across all advertisers; if it survives as
  the remaining divergence, the coordinator-zome no-chain-gate decision
  (declare with validated carried record when no local chain) is the
  operator-facing follow-up (adopt-before-author semantics).
- **Recording lever** — once `/health p2p.converged` holds on A: one empty
  commit `[edge:validate-only] [build:edge]` fires the gated saga recording
  (2700s quiesce deadline); ch04 (stale-regressed) + ch06 (heads-converge) +
  ch11 (never-measured) all ride it; a second recording confirms stability.
- **Convergence sweettest (design ready, unbuilt):** two-agent conductors, one
  id, independent roots, each declares the OTHER's head (contest shape);
  await_consistency; assert both resolve the SAME head_action_hash AND the
  SAME canonical_declared_at (equal heads with different clocks would still
  leapfrog). Second scenario: earned-vs-newer-staging tier precedence across
  gossip order. Lives in elohim/holochain/tests/sweettest (out of the closing
  shift's path scope); `#[ignore]` is a CI no-op there.
- **Self-mint trailing** (4 mints vs 603 fetch_none) — verify the (id,target)
  ledger is deduping repeats as designed vs a second admission gap; one Loki
  hour post-F-B answers it.
