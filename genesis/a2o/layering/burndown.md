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
