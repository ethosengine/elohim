---
name: cargo-nextest-installed
description: "cargo-nextest 0.9.135 is installed at /opt/rust/cargo/bin/cargo-nextest — prefer it over `cargo test` for unit/integration test runs because it parallelizes and is significantly faster on warm caches."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: ca911629-dfdd-46f5-8bb1-e936364bea8e
---

`cargo nextest run` is the preferred way to run Rust tests in this repo. Installed 2026-05-15 at `/opt/rust/cargo/bin/cargo-nextest`. Version 0.9.135.

**Why:** parallel test runner; faster than `cargo test` on warm caches; same `--lib` / `--test <name>` / filter-pattern syntax.

**How to apply:**
- Substitute `cargo nextest run` for `cargo test` in subagent dispatches.
- Filter syntax: `cargo nextest run --lib services::recovery_flow_projector` (substring match — same shape as `cargo test`).
- Single test: `cargo nextest run --lib services::recovery_flow_projector::tests::recovery_request_opens_a_flow`.
- Integration tests: `cargo nextest run --test schema_contract` (works the same).
- `cargo nextest run --no-run` to build only.
- `--success-output never --failure-output immediate` for tighter logs in CI-style dispatch.

**Doesn't replace** `cargo test export_bindings` (ts-rs export) — that's a separate harness and stays as `cargo test`.

**Caveats:**
- nextest needs `--release` flag explicitly if you want release tests.
- nextest does NOT honor `#[ignore]` attributes by default — same as cargo test; use `--run-ignored only` or `all` if you need them.
- For sweettests, the `#[ignore]` gate still applies — nextest by default skips them, which is what we want for local runs.

Linked: `feedback_shells_need_timeouts.md` (timeouts still apply); `feedback_subagent_dep_conflict_supervision.md` (still relevant for cargo invocations).
