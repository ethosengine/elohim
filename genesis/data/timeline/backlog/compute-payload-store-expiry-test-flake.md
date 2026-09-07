---
id: "backlog-compute-payload-store-expiry-test-flake"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "compute_payload_store::tests::expired_owner_stays_expired_when_identical_bytes_are_reused failed once under the full storage suite (NotFound on get after put) and passed 1/1 on rerun and 15/15 isolated — flake with an unexplained mechanism, not yet a race"
slug: "compute-payload-store-expiry-test-flake"
written: "2026-09-07"
author: "overnight shift 2026-09-07"
status: "open"
priority: "low"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
tags: [flake, elohim-storage, delegated-compute, test]
---

**Evidence (2026-09-07 08:0xZ):** first full `just gate elohim-storage` on the merged tree (63a93bc58): `test result: FAILED. 3635 passed; 1 failed` — the one failure panicked at `src/compute_payload_store.rs:260:48` with `NotFound("bafkreife2…")` on the final `get` after `put("new")`. Second full run: green. Isolated module loop: 15/15 green. Logs: `$CLAUDE_JOB_DIR/tmp/gate-storage-merged{,-2}.log` (job dec5f3eb, ephemeral).

**Reading:** every write path holds the global `WRITES` tokio mutex; `get`/`contains` read lock-free but only after `put` completed. Not explained by the module alone. Candidates: cross-test interference on `ELOHIM_COMPUTE_PAYLOAD_BYTES` (read via `std::env::var` per put) or on `TMPDIR`; clock second-boundary in `now()` (owner expiry compares `> now()` at second granularity — a `put` at second N with ttl 60 cannot expire in ms, so unlikely).

**Done when:** the mechanism is named with a reproducer (e.g. run the lib suite 10× and correlate with the concurrently running test), or the test is made hermetic against env (`ELOHIM_COMPUTE_PAYLOAD_BYTES`) and re-observed green over 10 full runs.
