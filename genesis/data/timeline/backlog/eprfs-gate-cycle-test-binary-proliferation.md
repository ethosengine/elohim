---
id: "backlog-eprfs-gate-cycle-test-binary-proliferation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs gate is over its 1200 s cycle ceiling: 59 epr-cli integration-test binaries are the first cut"
slug: "eprfs-gate-cycle-test-binary-proliferation"
written: "2026-10-24"
author: "claude-opus-5-5 (device-runtime slice A), report by gpt-6-astra"
status: "open"
priority: "medium"
tags: [gate-cycle, eprfs, epr-cli, test-binaries, link-time, gate-selection, performance]
cites:
  - elohim/eprfs/build-manifest.json
  - elohim/eprfs/Cargo.toml
---

# eprfs gate cycle: 1228 s against a 1200 s ceiling

## Measured (2026-10-24, `just gate eprfs` on `feat/device-runtime-a` e2b960074, jobs and test threads capped at one)

| Phase | Time | Share |
|---|---:|---:|
| clippy `--workspace --all-targets`, including dependency builds | 501 s | 41% |
| test compile and link, before execution | 531 s | 43% |
| test execution (sum of reported times; epr-cli ≈ 157 s) | 190 s | 15% |

1,518 tests passed across 96 test targets plus 11 doctest summaries. `epr-cli/tests/` holds **59**
top-level files, and each one is a separate integration-test binary that links the CLI library.
clippy's `Checking` artifacts do not replace the test build's codegen, so aligning profiles alone
saves nothing.

The log cannot separate compile time from link time or establish how warm the cache was, so the
saving from consolidation is unmeasured.

## First bounded step

1. Write a design spec that consolidates the 59 `epr-cli` integration targets into a few
   module-based suites. It keeps process-isolation and peer boundaries, every assertion and fixture,
   and the `--test` selectors that justfile recipes name (`flow_memory_footprint`, `session`, …).
2. Implement it, then compare matched cold, warm and one-file-change runs with `cargo build --timings`.
3. Then declare per-crate gate selection in `build-manifest.json` through the existing gate runner,
   including reverse dependents and dev-dependencies, so a leaf change skips unrelated CLI and peer
   work. Full-workspace runs stay for lockfile and shared-config changes.

Keep the 1200 s ceiling until the experiment reports. One run does not justify raising it.
Relief that comes sooner: the gate receipt (device-runtime slice A.4) lets pre-push accept an
already-green tree instead of re-running the gate.
