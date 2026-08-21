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
