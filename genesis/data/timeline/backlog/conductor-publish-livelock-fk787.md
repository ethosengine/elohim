---
id: "backlog-conductor-publish-livelock-fk787"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor publish-queue livelock (SQLite FK 787) — upstream Holochain 0.7.0 get_ops_to_publish LEFT JOIN + batch-voiding record_published_op_hashes; confirmed locally, fleet risk unconfirmed tonight"
slug: "conductor-publish-livelock-fk787"
written: "2026-09-17"
author: "doorway-overnight-20260914 shift, from the publish-storm RCA (genesis/a2o/reports/recovery/doorway-pickup-20260917/)"
status: "backlog"
priority: "high"
tags: [risk, incident, conductor, holochain, upstream, sqlite, publish-queue, livelock, fleet-risk, 0.7.0]
cites:
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/publish-storm-rca.md
  - genesis/a2o/reports/recovery/doorway-pickup-20260917/cell-debug-notes.md
  - genesis/local-dev/preserved/pre-fresh-household-20260917T1515Z-publish-storm/
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
  - genesis/data/timeline/backlog/conductor-source-chain-unwrap-panic-db-timeout.md
---

# Conductor publish-queue livelock (SQLite FK 787) — upstream defect, locally confirmed

## 1. The defect — upstream Holochain 0.7.0 (`e52fa68ac`), not a fork commit

Chain, file:line from the RCA:

- **Selection** — `get_ops_to_publish`, `crates/holochain_data/src/dht/inner/chain_op_publish.rs:115-152`.
  A **LEFT JOIN** to `ChainOpPublish`: a `ChainOp` row with no matching `ChainOpPublish` row reads
  NULL on all three publish columns, so it satisfies `withhold_publish IS NULL`,
  `last_publish_time IS NULL`, `receipts_complete IS NULL` — **selected every loop, forever**. No
  `locally_validated` filter.
- **Recording** — `record_published_op_hashes`, `crates/holochain_state/src/dht_store.rs:891-909`.
  One transaction per whole batch. `set_chain_op_last_publish_time` is a plain UPDATE (not an
  upsert); zero rows updated is read as "must be a warrant" and routed to
  `insert_warrant_publish`, whose FK references `Warrant(hash)` — a chain-op hash has none, so
  **SQLite 787 FOREIGN KEY constraint failed**, aborting before `tx.commit()` at `:908`. **No op in
  the batch gets `last_publish_time`.**
- **No bail** — `queue_consumer.rs:797-805` `handle_workflow_error` logs and returns `Ok(())`,
  short-circuiting before the `pause_loop()` decision at `publish_dht_ops_workflow.rs:86-95`. The
  trigger never pauses; the identical batch republishes forever.

**Suspected producer of poison rows (medium-high confidence, inferred):** `cache_chain_ops`
(`crates/holochain_state/src/dht_store/cache.rs:32-72`) writes ops fetched from peer authorities
into the same per-DNA DHT db with `locally_validated = false` and deliberately no `ChainOpPublish`
row. Any such op whose `Action.author` is a locally-hosted agent becomes a permanently-unpublishable,
permanently-reselected poison hash.

## 2. Measured local evidence (household, 2026-09-17)

- FK-787 rate flat: **~82/min at 04:56Z vs ~76/min at 15:03Z** — zero decay across 10 hours, the
  signature of a livelock (a real backlog decays as ops gain `last_publish_time`; this did not).
- Refusal noise (`kitsune2_transport_iroh::connection_context.rs:638 recv_data … Full(..)`) was
  **98.8% of conductor log volume**, ~2.5 MB/s, 6.3 GB total log file.
- Conductors at 230–460% CPU each, sustained 1h+; household unusable for acceptance lanes.
- State preserved (non-destructively) at
  `genesis/local-dev/preserved/pre-fresh-household-20260917T1515Z-publish-storm/`.

## 3. Related trap, separately actionable — post-kill boot looks ready but isn't

After an unclean kill, `create_cells_and_startup` (all-or-nothing) takes 5–17 minutes during which
admin/app websockets are live but `running_cells` is empty and every zome call is `CellDisabled`.
`just mesh wait` reports ready (TCP-only, blind to this). Restarting mid-boot discards the in-flight
DHT-model rebuild and resets the clock. Proposed fix (not applied): a `list-cells > 0` rung in
`hc-mesh.sh` `wait_all()` and `preflight`; also note storage-restart's return window is shorter than
the post-restart catch-up it's meant to wait for.

## 4. Fleet status — stated precisely

A Loki check on 2026-09-17 ~15:20Z found **no `FOREIGN KEY`/787 lines in `elohim-alpha` conductor
logs over the prior 48h** — the livelock is **not confirmed on the fleet right now**. The
operator-reported 787 on alpha-Matthew was **2026-09-14**, three days earlier; the two are not the
same measurement.

What **is** live on the fleet: `elohim-adam-alpha-conductor-0` logged **~15k `recv_data … Full(..)`
publish-queue refusals in one hour**, all from a single remote peer `a493501af339…` — an
un-throttled publish flood against adam, independent of the livelock question.

**Correction to the RCA's own earlier read:** the fork's receive-throttle patch
(`patches/kitsune2_transport_iroh/src/recv_throttle.rs`, untracked; `connection_context.rs`,
modified-uncommitted in the submodule working tree) is **not in the running binaries** — the
committed source still logs `error!` at line 638 with no throttling, matching what both the household
and the fleet actually ran.

## 5. Fix path (not applied)

- (a) Route warrant vs. chain-op by the op's actual type, and record per-hash (or per-chunk) rather
  than per-whole-batch, so one bad hash cannot void the batch; skip-and-warn unknown hashes.
- (b) Add `locally_validated = 1` to the publish selection queries — excludes exactly the poisoned
  cache-path rows, no wire/validation change.
- (c) Commit the recv-throttle patch and rate-limit the per-refusal error log.
- Report upstream (verdict: their defect, not ours — commit `e52fa68ac`).

**No config mitigation exists**: `tuning_params.min_publish_interval` is bypassed by the
`last_publish_time IS NULL` arm once the livelock starts — raising it does nothing.

**Cost of landing:** fork commit → conductor pin move → `[build:conductor]` image → edge roll. Not a
same-night change.

**Verification path:** fork unit tests at the locations the RCA names (`dht_store/tests.rs`,
`chain_op_publish.rs`, `publish_dht_ops_workflow/unit_tests.rs`); locally, FK-787 rate should go to
zero on the preserved household; on the fleet, the Loki query shape from the RCA (`count_over_time`
ratio on `787`, flat-vs-decaying discriminator).

## 6. Current decision

**Captured, not started.** Owner: next conductor-fork shift. Blocks nothing tonight — a fresh
household sidesteps the amplitude (no poisoned rows in a new store) rather than curing the defect.
