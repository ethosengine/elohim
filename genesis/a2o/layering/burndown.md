# Act I burn-down — the benchmark and the waves

**Done (Act I lane)** = on a fresh `just mesh start` + `just mesh prologue`, `just test mesh` reports every
non-`@wip` Act I scenario GREEN (0 failed, 0 undefined, 0 pending), the saga in order reads 11/11, and the
`@wip` Act I pool has been drained to the stories that genuinely need a form the mesh cannot provide. Three
counters, measured every wave from the cucumber JSON report:

| counter | definition | direction |
|---|---|---|
| **eligible** | non-`@wip`, non-browser Act I scenarios the lane executes | grows as `@wip` drains |
| **green rate** | passed ÷ eligible | → 100 % |
| **debt** | `@wip` Act I/host scenarios (bound / partial / none) | → the honest floor |

A wave = one mesh measurement (saga + `just test mesh`), after a batch of disjoint fixes. Jenkins never
appears here: the fleet confirms a batch; it does not move this table.

| wave | when | eligible | passed | failed | undefined | pending | green rate | @wip Act I/host | notes |
|---|---|---|---|---|---|---|---|---|---|
| −1 | 2026-08-21 13:43 | 194 | 101 | 39 | 1 | 26 | 52 % | ~516 `@wip` total | first inventory, pre-layering, alpha-shaped tags |
| 0 | 2026-08-21 19:58 | 363 | 106 | 55 | 159 | 35 | 29 % | 482 (101 bound / 305 partial / 76 none) | post-layering: un-parking + `@e2e` lifted eligibility; the 159 undefined were placeholders → `@wip`-swept (103) |
| 1b | 2026-08-22 01:32 | 227 | 134 | 58 | 0 | 35 | 59 % | census re-run pending | wave-1 was unmeasurable (container restart killed the lane); regen6 full chain: corpus fix landed `/lamad` on both doorways, boot-order restart fixed empty `servedBundleHeads`; saga scoped 15/6/1 — 4 of 6 saga fails share one cause-chain (ch05 no co-steward pin → ch09/10/11), expected to collapse when the chunk-aware storage binary (c04c2b423) rolls |
| 2 | 2026-08-22 02:21 | saga-scoped 22 | 15 | 4 | 0 | 3 | 68 % | — | binary roll (chunk-aware producers, RS tolerance, real httpPort, warmup re-warm, shell pre-warm, dev signal subscriber): ch09 green, ch10 zeros→1-vs-2 divergence, root `/` 1.4 ms cold; 3 pendings from servedBundleHeads re-warm gap (head declared after productive warmup never materializes) |
| 2b | 2026-08-22 02:26 | saga-scoped 22 | 17 | 4 | 0 | 1 | 77 % | — | boot-order doorway restart materialized heads (re-warm gap confirmed); ch03+ch06a green; remaining reds all on the consent-pin chain: ch05 station A (no household pin), ch11 pull-state (downstream of pin), ch10 1-vs-2 convergence, ch11b alpha-shaped worked example |
| 2c | 2026-08-22 02:40 | saga-scoped 22 | 19 | 2 | 0 | 1 | 86 % | — | consent-pin keystone (ca5f3618e + live pin on james): ch05 both scenarios green, ch11a green (pull state caughtUp on matthew AND james — cross-peer acquisition proven); remaining: ch10 doorway-truth divergence (1 vs 2, per-peer distribution not reconciled), ch11b alpha-shaped worked example, ch06b pending on served-head reconcile (fix in flight). NEW defect surfaced: T21 blob requests with `sha256-`-prefixed CID (double-wrapped address) rejected peer-side — blocks evolution-of-trust bytes |
| 3 | 2026-08-22 03:54 | saga-scoped 22 | 18→20 | 2→0 | 0 | 1→0 | 91→95 % | — | binary roll with T21 cure (833ba4c58), SyncGate (c309a3a5e), served-head adoption (e6f4bf714), household canonicalization (2cb043387): ch05/10/11 all green at first run; the 2 ch06 "fails" were publish-time (re-ran green after settle) and the ch06 pending flipped via the per-peer serverBlobHash stamp leg (0e8057cf9). T21 runtime-proven: bytes on all 3 peers, zero rejections, invalid_markers:0 |
| 3c | 2026-08-22 05:00 | saga-scoped 22 | 21 | 0 | 0 | 0 | **100 % of implementable** | — | clean re-measure: 21 passed + 1 `@requires:owned-substrate` skip (honest env hold) — the saga lane is COMPLETE on the household mesh; fleet confirmation rides the next edge roll. Full Act-I lane same wave: 154/225 eligible = 68 % (up from 59 %), remaining fails triaged into families (sync-control step-precondition class fixed 746b035cd; p2p-validation live-fabric re-scope landed bc215f632 — 4 passed/3 honest skips/0 failed; qahal participations arm + federation/resilience triage in flight). Census re-run vs wave-1b: FIXTURE 74→52, IMPLEMENT-DESIGN 5→0 (all implemented), DEFECT-STALE 17→21, WIRE 311 stable, population 517→548 |
