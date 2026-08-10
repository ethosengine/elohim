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

## 2026-08-07 — re-confirmed as the PRIMARY converged pin; operator decision surfaced

Same-day red-team review plus a live probe re-confirm this doc, not
`miss-ledger-exhausted-ids-veto-converged-forever` (closed not-a-defect the same
session — that doc's premise was falsified: `MissLedger`-exhausted ids never enter a
`GapTracker` and per-sweep trackers structurally cannot exhaust), as the standing dam
on `converged`. Live alpha-A reading: pending 2894, healed 0-1/sweep, against a
`CONTENT_LEG_BUDGET` of 120s wall-clock per sweep — the plateau this doc's RCA v3
predicted survives unchanged.

**Open operator decision surfaced by the review:** while this plateau drains, should
the CI fleet-quiesce gate poll for **"quiesced"** (`caughtUp && divergent_actionable
== 0`, sustained) instead of **"perfect"** (`converged == 1`)? The gate today waits on
`converged`, which this doc's own math shows will not go true until the content-gap
backlog fully drains — potentially the wrong bar for what the gate actually needs to
guarantee (no outstanding actionable divergence, not zero outstanding gap-fill work).
This is a decision only the operator can make (it trades gate strictness for gate
liveness); it is not resolved by this note.

## 2026-08-07 — DECIDED: quiesced (integrator ruling, same session)

The gate is re-pointed at **quiesced**: `pull.caughtUp` on A AND
`elohim_projection_reconcile_converged_blocked_by{term="divergent_actionable"} == 0`
AND `…{term="unmeasured"} == 0`, sustained across an advancing sweep — implemented in
`scripts/ci/fleet-quiesce-gate.sh` (fail-closed when the blocked_by series is absent,
i.e. a pre-honesty-floor image can never pass). Rationale: the gate's charter is
churn detection ("does a measurement mean anything?"), not drain completion; with
pending ~2.9k healing 0-1/sweep, `converged==1` cannot be reached inside any gate
deadline, so the perfect predicate had become an indefinite measurement embargo on
ch04/ch06/ch11. Honesty is not traded away: `converged` remains published, honest,
and printed by the gate as telemetry; the blocked_by gauge names the pinning term
from the fleet; this doc remains the named red until the plateau drains. The
`pending`/`failed` terms deliberately do not gate. Verified locally with a 5-case
fixture harness (plateau-doesn't-block / actionable-blocks / unmeasured-blocks /
absent-series-fail-closed / exact-name-boundary).

Sprint objective set the same session: the gap-plateau drain — F-B adoption/contest
throughput fan-out + known-no-chain backoff + peer-probe source widening (the
`PeerHeadRecordFetcher`/`alternates` direction shared with
`rea-stream-no-divergence-adjudication-drain-path`), attacking the 0-1/sweep healed
rate directly.

## 2026-08-07 — post-deploy finding (edge #1319): the cure cannot land — alpha storage image is lane-frozen

Edge #1319 (first deploy carrying the honesty floor + drain levers) went UNSTABLE
as an honest DID-NOT-MEASURE: the re-pointed quiesce gate read
`actionable=None, unmeasured=None` for its whole window — the `blocked_by` series
was absent because **alpha's storage container is hard-pinned to
`elohim-storage-iroh:hc-elohim-0.6.3-iroh`** (`resolveStorageImage`, edge
Jenkinsfile ~682), the one-time Wave-2 artifact built 2026-08-06. Pods rolled
(deploy stamp moved) but re-pulled the frozen digest `f09bc2a7…` — verified via
`kube_pod_container_info` on matthew. Every storage-side dev merge since the
iroh flip has been silently inert on alpha; the fail-closed gate is what finally
surfaced it. Cure in flight this session: per-commit iroh-lane storage build in
the edge pipeline (`elohim-storage-iroh:${STORAGE_TAG}`), alpha render re-pointed
to it, frozen tag retained in Harbor as the ratified rollback line; plus the
validate-path quiesce deadline raised 900s→2700s (900s < the ~20min restart
churn, so it systematically no-measures). Until that deploys, the drain-rate
predictions above are UNTESTED live — do not read #1319's fleet behavior as
evidence about the levers.

**Post-review watch item (independent review of `da752307f`, same session):** the
interaction of the pre-existing adopt fan-out (8) with the widened alternates ladder
(now up to 3, was 1) raises worst-case concurrent fetches against a small courier
pool from ~16 to ~32 (all 8 resolutions simultaneously mid-ladder). Bounded — each
fetch capped at 5s (F-A(i)), ladder sequential per resolution, first-Carried
short-circuit — but it is a real amplification of per-courier pressure on 2-3-courier
household meshes, exactly the F-A "loaded responder" shape. Watch
`adopt_evidence_fallback{attempted}` vs `{carried}` alongside per-peer conductor
latency in the first post-deploy hour; if attempted climbs while carried stays flat
and responder latency rises, halve `ELOHIM_EVIDENCE_FALLBACK_MAX_ALTERNATES` before
touching the fan-out.

### Sprint delta (2026-08-07, same session) — the levers landed one layer down

The three named levers were re-scoped on contact with the code: **F-B's fan-out and
known-no-chain backoff had ALREADY landed on the adopt/contest arm** (`571b0e704`,
`7b2fceb66` — `adopt_contest_fanout` default 8, `contest_backoff` with its
`no_local_chain` / `evidence_absent` classes, `PeerHeadHint.alternates` cap 3). What
had NOT received them was the **content heal leg that SUPPLIES that arm**. All three
levers therefore landed at the layer where they were actually missing:

- **Lever 1 — heal-leg fan-out** (`resolve_pipeline`, `config::heal_resolve_fanout`,
  default 8, `1` = the old sequential leg). `heal_content` awaited one
  `resolve_content_head_local` at a time inside the 120s `CONTENT_LEG_BUDGET`. At the
  live ~0.75s conductor latency that is **~160 of ~2 894 pending ids touched per
  sweep** — and that single number caps BOTH `healed` and the adopt arm's candidate
  supply, because every adopt candidate is produced by an outcome of this leg.
  `buffered(fanout)` yields answers **in input order**, so the apply half stays the
  byte-identical sequential body (one task, one order, no lock); only the I/O is
  concurrent. Bounded overshoot on budget/circuit trip: ≤ `fanout - 1` already-issued
  resolves, cancelled by dropping the stream.
- **Lever 2 — heal-leg known-no-chain replay** (`services::heal_backoff`,
  `config::heal_missing_backoff_seconds`, default 600s = 2 sweeps, `0` = off). The
  leg's dominant outcome is `Ok(None)` (~24.9k `missing` per 2h fleet-wide against 59
  `healed`) and it is a predictable repeat: only the author/obey arms, which run later
  in the same tick on their own budget, can change it. A replay reproduces the
  `Ok(None)` arm's effect **exactly** — same ghost candidate, same adopt-candidate
  routing (so the `try_obey_visible_election` probe still runs every sweep), same
  `mark_failed` — and skips only the round-trip. Deliberately **not** the F-C
  exhausted-sawtooth shape: nothing is exhausted, tombstoned, or written off; it is a
  cached answer with an expiry and three automated exits (window, any non-`Ok(None)`
  answer, chain arrival via the same author-path hooks as
  `contest_backoff::note_local_chain_arrived`). New heal-outcome label
  `missing_deferred` keeps `missing` meaning "the conductor answered nothing".
- **Lever 3 — widened advertiser probe** (`advertiser_health::select_alternatives` /
  `rank`, `config::evidence_fallback_max_alternates`, default 2, hard-clamped to
  `ALTERNATE_ADVERTISER_CAP` = 3, `0` = single-ask). The harvest was already retaining
  up to 3 couriers per id and the resolution asked exactly **one**, so two thirds of
  the plurality the sweep had paid to collect were never used. The probe now walks the
  ranked couriers healthiest-first and stops at the first that carries.
  `inc_adopt_evidence` still fires exactly once per resolution; `fallback{attempted}`
  becomes the round-trip denominator while `carried`/`degraded`/`no_alternative` stay
  one-per-resolution.

**Expected drain-rate math** (per pod, per 300s sweep, 120s content leg):

```text
  pre-lever   : 120s ÷ 0.75s              = ~160 ids touched/sweep of ~2 894 pending
                                          → ~18 sweeps (~90 min) to touch each id ONCE
  lever 1 only: 120s ÷ (0.75s ÷ 8)        = ~1 280 ids touched/sweep
                                          → ~2.3 sweeps (~12 min) for full coverage
  levers 1+2  : replay share p ≈ 0.85     → ~2 460 replayed free + ~1 280 resolved
                                          → the WHOLE pending set every sweep, with
                                            ~1 280 slots of real conductor work left
                                            over for ids whose answer is unknown
```

Fleet-wide that is 7 pods × ~1 280 real resolves/sweep × 12 sweeps/hour ≈ **107k
conductor answers/hour against ~11k contested ids**, versus ~13.4k/hour before —
roughly **8×**, and the coverage horizon per id falls from ~90 min to one sweep.
Lever 3 multiplies the *conversion* of that supply rather than its volume: each
adopt-evidence resolution that previously gave up after one starved courier now
reaches up to three, so the share of `no_local_chain` candidates that convert to
`minted{source="adopt_before_author"}` rises with the courier plurality that actually
exists on the mesh (2-3 on a household mesh).

**What to watch, in order of trustworthiness** (the per-sweep `gaps` gauge remains a
rotating-page sample and proves nothing):

1. The new `projection-reconcile[content]: heal leg finished` line — `to_resolve` vs
   `replayed`, with `fanout`. `replayed` rising while
   `elohim_projection_reconcile_heal_outcome{stream="content",outcome="missing"}`
   falls and `…{outcome="missing_deferred"}` rises IS lever 2 working.
2. `elohim_content_canonical_links_minted_total{source}` — still the only series that
   proves contest supply, now fed ~8× more candidates per sweep.
3. `elohim_content_adopt_evidence_fallback_total{outcome="attempted"}` over
   `{carried}` — the per-round-trip hit rate of lever 3; `attempted / (carried +
   degraded)` above 1 is the widened ladder engaging.
4. `elohim_projection_reconcile_healed_total` and `head_adopted_total` — the outcome.
5. `elohim_projection_reconcile_converged_blocked_by{term}` — which term still pins.

**Side effect the post-deploy observer must expect — `caught_up` will start flipping
true.** A leg that never finished was masking this doc's own "converged softness":
with ~160 of ~2 894 ids touched per sweep, the untouched remainder held `pending`
non-empty for free, so `caught_up` was false by starvation rather than by judgement.
A finishing leg empties `pending` every sweep, and `caught_up` reading true is then
the HONEST statement that the sweep processed everything it discovered. From that
point the only thing between a fully conductor-missing corpus and a green `converged`
is the honesty floor's `failed` term (`ad4d5ed3f`) — which holds, because a replay
`mark_failed`s exactly as the live `Ok(None)` arm does. That invariant is now pinned
by a test (`a_replayed_id_is_never_marked_completed`) asserting both halves: the
replay never marks completed, AND `converged_gauge_value` stays 0 for a sweep whose
only outcome was replayed misses. Anyone reading a `caught_up` rise on the CI quiesce
gate should read it as the leg finishing, not as the plateau draining;
`healed_total` / `head_adopted_total` / `minted_total` remain the drain evidence.

**Honest bound on the claim:** lever 1 raises how many ids the leg can ASK about; it
does not by itself make an answer converge. If `missing` stays the dominant outcome at
8× volume, the residual is upstream (the both-sides-missing pairs and the
coordinator-zome no-chain-gate decision this doc already carries), not throughput —
and `adopt_sweep_total{outcome="budget_elapsed"}` staying dominant would say the
conductor, not the leg, is now the constraint. Both readings are available on the
first post-deploy hour.

**Deliberately deferred:** the sweettest convergence scenario (still design-ready,
unbuilt — it lives in the DNA workspace, out of this sprint's path scope) and the REA
declared-head-equivalent record (`rea-stream-no-divergence-adjudication-drain-path`
remains the owner; lever 3 only widens the content arm's courier set, it does not give
REA the primitive it lacks).

**Bandwidth note:** the peer-probe/more-sources direction adopted for the MissLedger
and REA adjudication drain paths (`Answer::Absent`-graduated adjudication via
`PeerHeadRecordFetcher`, `PeerHeadHint.alternates` cap 3 — see
`rea-stream-no-divergence-adjudication-drain-path`) also increases fill bandwidth
here: more advertiser sources probed per gap id directly attacks the same
0-1/sweep healed rate that keeps this plateau from decaying. The two backlog items
are adjacent levers on the same underlying drain-rate problem, not independent
concerns.

## 2026-08-08 — edge #1320: lane cure PROVEN live; embargo narrowed to a ~3-row actionable residue

The per-commit iroh-lane build worked end-to-end on its first run: `✓
elohim-storage-iroh pushed: …:1.0.0-dev-88a4d622`, all 7 alpha humans rendered
it, and from the first completed sweep the honesty floor published live —
`blocked_by{divergent_actionable}=1, {unmeasured}=0` in the gate log. The 2700s
gate then did its designed job: five measured sweeps, fleet calm (caughtUp,
content 200s), refusal for ONE named reason — actionable divergence
outstanding. Not churn, not instrument absence. Prometheus sizes it:
matthew content actionable oscillates 1-3 (rea 0, collectives 0) — **the whole
measurement embargo is now ~3 nameable content rows.** Drain levers visibly
live: `missing_deferred` ticking (~146/h), refused_declared collapsed ~50×
(≈238/h vs ≈12.5k/h pre-fix). Note the old image's last half-hour read
actionable=0 while the new image holds 1-3 — either post-restart backlog
(<1h soak vs hours-class precedent) or a persistent class the widened probing
now surfaces honestly. Next: soak, re-probe; if 1-3 persists, sample the ids
from adopt-deferred Hold / adjudication logs — that class IS the Stage-2
peer-probe adjudication scope (`rea-stream-no-divergence-adjudication-drain-path`).
A validate-only recording (`[edge:validate-only] [build:edge]` empty commit)
fires the gated saga measurement once actionable holds 0.

**Follow-up (from the lane-cure review, 2026-08-07):** the normal-build path writes
`STORAGE_TAG` into the env file without a pushed-tag preflight — a manual `STEPS`
subset that skips `cargo-build-storage` while another component pushes can render
alpha's `elohim-storage-iroh:${STORAGE_TAG}` against a tag never pushed
(ImagePullBackOff). Pre-existing on the tx5 lane for staging/prod; alpha newly
exposed now its tag is per-commit. Narrow (deliberate operator override only);
fix shape: extend the DEPLOY_ONLY-style Harbor preflight to the normal-build
env-file write.

## Post-transport-cure evidence (2026-08-09, convergence-serve-path shift)

The 6h settle-clock after the relay/cross-relay cures (edge #1332,
T0=13:24Z) answers the "how long to converge" question with: NEVER under
current dynamics. With the transport fully open (both relay error classes
at zero fleet-wide), per-pod elohim_projection_reconcile_divergent
{stream="content"} OSCILLATES in bounded bands for 6h with no trend:
matthew 1622↔1858, jessica 777↔945, adam 451↔1750, eve 647↔1140,
gertrude 647↔1072, james 728↔1437; susan swings 0↔3129 (touching ZERO
twice, then re-spiking — the sweep can measure clean and then re-diverge
or re-measure a different subset). rea streams flat (6-12/peer),
collectives 0 everywhere. Two prior same-day "monotone drain" reads were
SAMPLING ARTIFACTS (pool-fanned /health across peers; sweep-phase
aliasing) — always read this per-pod from Prometheus, never from a
fanned endpoint.

Consequence for the head-plane §6.4 memo: the F2 quiesce window is
UNMEASURABLE until this objective lands — that is the decision input,
not a minutes-number. This item is now the confirmed gate in front of:
caughtUp=true fleet-wide, the doorway shed lifting, notary-authority
3/3, saga ch04/06/10, and the fork-deploy decision.

## 2026-08-10 sprint close — limit cycle BROKEN; residual threads (limit-cycle sprint, overnight)

The oscillation was proven an instrument artifact (rotating-window aliasing;
gauge now measured-gated with drain-readable known_gaps/known_divergent
series) wrapped around a real stall: the heal leg broke unconditionally on
the first failed batch call (fixed budget-bounded, b96861c1b+31f8a9e89) on
three CPU-starved conductors. Actives drained to single digits (james +
jessica reached known_divergent=0); notary-authority measured 3/3 locally
twice across independent deploys after the authorHeadOnce declare cycle
busted the elohim.host ghost anchor. Remaining threads, each owned:

- **Shem-trio conductor burn (OPEN, blocks the CI quiesce gate + trio
  drain):** k2Gossip datacenter profile (15000/10) deployed per-human
  (61118ace9) but the trio still saturates any CPU ceiling and
  arc-to-full remains unobserved on shem. Round-2 lead: relay-path gossip
  rounds time out fleet-wide (even matthew, arc=Full, ~4-6 WARNs/min,
  every dying session relay-mediated, peer_max_op_data_bytes:0) while
  relay ERROR classes are zero — the round transport works, rounds die
  anyway. Pyroscope not instrumented (wishlist) — next probe is the
  rendered-config confirmation + a round-lifecycle trace on one
  gertrude↔adam session.
- **Seed-pod unanchored backfill (batch-3 seed):** H3 CONFIRMED via
  /db/stats parity (matthew 4456 rows ≈ adam 4454; ~2000 rows present but
  dht_anchor_hash NULL on matthew/jessica) — H2 corpus-split and RC-4
  reach-narrowing both falsified (apply_content_patch_fields has no reach
  branch on UPDATE; the shipped guard is inert-but-armed). Cure direction:
  anchor backfill / AnchorGap stamping from peer-advertised anchors.
- **Validate-only edge mode (measurement-by-deploy cure):** every
  [build:edge] validate run rolls all 7 STSs — the measurement restarts
  the fleet it measures (ch04's structural cause; aborted/gate-skipped
  banking runs #1336-#1338). Local a2o runs are the iteration loop;
  the habit bank needs a deploy-decoupled CI measure.
- **Flip condition (notary-authority habit):** one edge run whose quiesce
  gate completes recording 3/3 — structurally gated on the trio drain,
  which is gated on the burn thread above.
