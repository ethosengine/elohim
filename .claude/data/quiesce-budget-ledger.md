## PREDICT @ 2026-08-17T01:04:00.069830Z

- source: live scrape (localhost:8090-8092)
- rows: 3439
- peers: 3 (reachable: matthew, jessica, james)
- pending_total (live): 0.0
- drain_backlog_total (live): 0.0

| atom | expected count | unit cost ms | parallelism | contribution s | count source | cost source |
|---|---:|---:|---:|---:|---|---|
| put_record | 0.0 | 0.00 | 3 | 0.00 | live:drain_backlog | observed (n=3) |
| inventory_page | 0.0 | 0.00 | 6 | 0.00 | live:pending/page | observed (n=3) |
| head_batch_per_id | 0.0 | 0.00 | 6 | 0.00 | live:pending | observed (n=3) |
| head_record_verify | 0.0 | 0.00 | 12 | 0.00 | live:pending*frac | observed (n=3) |
| adopt_declare | 0.0 | 0.00 | 3 | 0.00 | live:pending | observed (n=3) |
| shard_fetch | 687.8 | 0.00 | 9 | 0.00 | rows-model | observed (n=3) |
| manifest_persist | 1031.7 | 0.00 | 3 | 0.00 | rows-model | observed (n=3) |
| digest_fold | 0.0 | 0.00 | 3 | 0.00 | live:pending | observed (n=3) |
| **TOTAL** |  |  |  | **0.00** |  |  |
**Predicted quiesce time: 0.00s (0.00min)**


## PREDICT @ 2026-08-17T01:04:39.355927Z

- source: live scrape (localhost:8090-8092)
- rows: 3439
- peers: 3 (reachable: matthew, jessica, james)
- pending_total (live): 0.0
- drain_backlog_total (live): 0.0

| atom | expected count | unit cost ms | parallelism | contribution s | count source | cost source |
|---|---:|---:|---:|---:|---|---|
| put_record | 3439.0 | 50.00 | 3 | 57.32 | rows-model | fallback |
| inventory_page | 24.0 | 75.06 | 6 | 0.30 | rows-model | observed (n=21) |
| head_batch_per_id | 6878.0 | 20.00 | 6 | 22.93 | rows-model | fallback |
| head_record_verify | 2063.4 | 80.00 | 12 | 13.76 | rows-model | fallback |
| adopt_declare | 6878.0 | 40.00 | 3 | 91.71 | rows-model | fallback |
| shard_fetch | 687.8 | 120.00 | 9 | 9.17 | rows-model | fallback |
| manifest_persist | 1031.7 | 15.00 | 3 | 5.16 | rows-model | fallback |
| digest_fold | 10317.0 | 0.00 | 3 | 0.01 | rows-model | observed (n=12) |
| **TOTAL** |  |  |  | **200.34** |  |  |
**Predicted quiesce time: 200.34s (3.34min)**


## SETTLE @ 2026-08-17T01:07:45.699019Z

- predicted (from PREDICT @ 2026-08-17T01:04:39.356203Z): 200.34s
- measured: 94.00s
- residual: +106.34s (+113.1%) — OVER-predicted
- **residual = named design concern**: state which atom's count model or parallelism constant is the leading suspect (see the per-atom breakdown below, ranked by predicted contribution) and file it, don't just log the number.

per-atom breakdown (largest predicted contribution first):
  - adopt_declare: predicted 91.71s (count=6878.0 [rows-model], cost=40.00ms [fallback], parallelism=3)
  - put_record: predicted 57.32s (count=3439.0 [rows-model], cost=50.00ms [fallback], parallelism=3)
  - head_batch_per_id: predicted 22.93s (count=6878.0 [rows-model], cost=20.00ms [fallback], parallelism=6)
  - head_record_verify: predicted 13.76s (count=2063.4 [rows-model], cost=80.00ms [fallback], parallelism=12)
  - shard_fetch: predicted 9.17s (count=687.8 [rows-model], cost=120.00ms [fallback], parallelism=9)
  - manifest_persist: predicted 5.16s (count=1031.7 [rows-model], cost=15.00ms [fallback], parallelism=3)
  - inventory_page: predicted 0.30s (count=24.0 [rows-model], cost=75.06ms [observed], parallelism=6)
  - digest_fold: predicted 0.01s (count=10317.0 [rows-model], cost=0.00ms [observed], parallelism=3)



**Named concern for the +113% residual (2026-08-17 run):** the rows-model
conflates BOOTSTRAP with REPAIR. On a fresh seed under a declared Simulacra
grant, adopt_declare ran 516 times, not rows×2=6,878 — the Q7
accept-with-provenance ceremony compression working as designed — and
head_batch_per_id ran 24 times, not 6,878: the heal batch plane is a repair
term with no backlog to drain at bootstrap. Unit-cost fallbacks were also
uncalibrated: put_record actual 0.61ms vs 50ms assumed (80× fat),
adopt_declare actual 89ms vs 40ms (2× thin). Next-run fix: model counts per
trust stage (declared-grant bootstrap ⇒ compressed ceremony counts; repair
runs keep the rows-model), and prefer observed means now that a calibrated
histogram exists (mesh scrape 2026-08-17: put_record 0.61ms n=4172,
adopt_declare 89.17ms n=516, inventory_page 112.62ms n=147,
head_batch_per_id 25.26ms n=24).
