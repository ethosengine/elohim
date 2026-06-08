---
id: "backlog-ci-storage-workspace-tests-uncovered"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "No CI stage runs cargo test/nextest on elohim-storage — storage lib + integration e2e (e.g. mishpat_bounds_gate_chain) verified locally only"
slug: "ci-storage-workspace-tests-uncovered"
written: "2026-06-08"
author: "claude-opus-slice2b-planning"
status: "documented"
priority: "medium"
# Surfaced by the Slice 2b planning ci-investigator pass (high confidence): the
# elohim-edge `cargo-build-storage` stage is a Docker IMAGE build (cargo build
# --release in elohim/elohim-storage/Dockerfile), NOT a test run. No Jenkins
# pipeline invokes `cargo test`/`cargo nextest run` on the elohim-storage
# workspace. Consequence: the 2a behavioral proofs — tests/mishpat_bounds_gate_chain.rs
# and the storage `cargo test --lib` suite (1406 tests) — gate LOCALLY ONLY.
# A storage-layer regression that compiles can ship CI-green. The Mishpat
# post_commit CommitmentCommitted signal also has zero sweettest coverage
# (only bootstrap_steward_is_configured exists).
# Mitigation in Slice 2b: behavioral proof lands as DNA sweettests (which DO run
# in CI with --run-ignored all) + local storage e2e. The durable fix is a
# dedicated CI stage.
ci_status: documented
jobs: [elohim-edge]
relatedNodeIds:
  - "backlog-ci-alpha-cluster-degraded-substrate"
tags: [ci, elohim-edge, elohim-storage, test-coverage, cargo-nextest, sweettest, slice-2b, coverage-gap]
cites:
  - https://jenkins.ethosengine.com/job/elohim-edge/job/dev/1051/
  - https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1314/
  - elohim/elohim-storage/Dockerfile
  - elohim/elohim-storage/tests/mishpat_bounds_gate_chain.rs
  - elohim/holochain/Jenkinsfile
  - genesis/docs/superpowers/specs/2026-06-08-epr-acquisition-slice2b-provide-loop-design.md
---

# CI coverage gap: `elohim-storage` workspace tests have no CI gate

## What

The `elohim-edge` pipeline's `cargo-build-storage` stage runs
`nerdctl build -f elohim/elohim-storage/Dockerfile`, whose only Rust step is
`RUN cargo build --release`. **No CI pipeline runs `cargo test` or
`cargo nextest run` on the `elohim-storage` workspace.** The storage unit suite
(`cargo test --lib`, ~1406 tests) and the integration e2e
(`tests/mishpat_bounds_gate_chain.rs`, the Slice 2a REA-bounds-gate composition
proof) are verified **on the developer host only**. The DNA sweettest gate
(`elohim-holochain`) is green and does run with `--run-ignored all`, but it
exercises the DNA layer, not the storage services.

## Why it matters

A storage-layer regression that still compiles (a broken bounds check, a
projection parser that drops a field, a scorer that mis-tiers) can land
CI-green. "host-green ≠ CI-green" cuts both ways — here CI is structurally
blind to the storage behavioral layer.

## Proposed fix

Add a `cargo nextest run -p elohim-storage` (or `cargo test --lib` + the named
integration tests) stage to the edge pipeline, building into a
`CARGO_TARGET_DIR` pool slot, PVC-pressure-aware per `pool-policy.json`. Until
then, Slice 2b (and any storage work) must rely on DNA sweettests for
CI-visible behavioral coverage + local storage e2e for the rest.

## Status

`documented` — not yet actioned. Deliberately deferred out of Slice 2b scope
(operator decision 2026-06-08; see the slice-2b spec §2.1/§14). Pick up as a
standalone CI task.
