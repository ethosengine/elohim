---
id: "backlog-storage-gate-skips-iroh-two-peer-test"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "just gate elohim-storage compiles tests/compute_payload_native.rs to 0 tests — the two-peer iroh payload transfer/expiry assertion only runs under --features p2p-iroh, so the standing gate never carries the delegated-compute claim that weighs most"
slug: "storage-gate-skips-iroh-two-peer-test"
written: "2026-09-07"
author: "overnight shift 2026-09-07 (integration of the Codex delegated-compute work)"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:operator-runtime-surface"
tags: [gate, elohim-storage, iroh, delegated-compute, ci-hygiene]
---

**Evidence (2026-09-07 05:1xZ):** `just gate elohim-storage` runs `cargo test` with default features; `elohim/elohim-storage/tests/compute_payload_native.rs` is gated on `p2p-iroh` and compiles to 0 tests there. Run out-of-band with `--features p2p-iroh --test compute_payload_native` it passes (1 test, EXIT=0, verified twice: Codex's `genesis/a2o/reports/compute/validation-2026-09-07/compute-storage-gate.log` and the shift's re-run).

**Cure options:** (a) add a typed gate step in `elohim/elohim-storage/build-manifest.json` that runs the iroh-featured integration tests (costs one extra link of the test binary, not a full rebuild since the dev slot already carries the feature for the mesh binary); (b) make `p2p-iroh` a default feature of the crate (broader — every consumer pays). Prefer (a).

**Done when:** the standing `just gate elohim-storage` output shows `compute_payload_native` with ≥1 test run, and pre-push carries it.
