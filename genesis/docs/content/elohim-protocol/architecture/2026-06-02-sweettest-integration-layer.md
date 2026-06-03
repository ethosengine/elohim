---
title: Sweettest — the DNA-Level Integration Test Layer
id: sweettest-integration-layer
tier: architecture
status: Landed (mechanical floor live in CI; single-DNA ~10-15min target HELD — not yet demonstrated)
created: 2026-06-02
pillar coupling: elohim (DNA substrate), infrastructure (CI runtime)
# Born-linked: this seed compacts a settled multi-thread design+plan cluster. Raw bodies retire to git history.
compacted_from:
  - genesis/docs/superpowers/specs/2026-04-22-sweettest-integration-layer-design.md
  - genesis/docs/superpowers/plans/2026-04-22-sweettest-integration-layer-plan.md
  - genesis/docs/superpowers/specs/2026-05-24-sweettest-stage-efficiency-design.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md (the DNA substrate these tests exercise)
informs:
  - All future DNA-level test authoring (sweettest crate placement + scope markers)
  - All future CI sweettest-stage efficiency work (sharding, nextest archive, RUSTFLAGS hygiene)
memory_anchors:
  - project_sweettest_cost_anatomy
  - feedback_cargo_nextest_installed
  - feedback_sccache_spawn_enoent_rca
  - feedback_sweettest_cross_agent_consistency
  - feedback_sweettest_ignore_is_ci_noop
  - feedback_sweettest_native_build_env
defers:
  - Gherkin-driving the sweettest layer (cucumber-rs parked)
  - Per-DNA Jenkinsfiles
  - W4b sccache 0.14 downgrade (one-line follow-up thread, not a kept body)
---

# Sweettest — the DNA-Level Integration Test Layer

> **Canon status:** Architecture seed for the protocol's DNA-level integration tier.
> The CLAUDE.md CI table and the R&O cross-wave guidance point here for "what sweettest is."

---

## What sweettest is

Sweettest is the **DNA-level integration layer** of the test pyramid: in-process Holochain conductors running real DHT, real validation, and real cross-agent propagation against our *packed* DNAs. It sits between the two layers that bracket it — below the browser-tier a2o BDD scenarios (human experience), above the in-zome unit tests (pure logic) — and is the only layer that exercises the **integrity↔coordinator seam, link traversal, and multi-agent DHT consistency** as the live network would.

The crate lives at `elohim/holochain/tests/sweettest/` (28 files), each carrying a `//! @dna-scope:` marker so the CI filter can select which suites a changeset must run. Coverage is **hybrid**: a mechanical floor (the `zome-sweettest-sync` generator + the `sweettest-check` push gate keep a baseline of round-trip and cross-agent tests honest) plus per-Wave focal scenarios authored where a substrate landing needs integration proof.

## How it runs in CI

The DNA Integration stage builds a `nextest archive` once, then fans the suites across **4 shards** selected by `build-nextest-filter.sh` from the changeset's touched DNA scopes. When `CHANGED_PATHS` is empty (or a broad substrate change is detected) the filter falls through to the **full suite** — sweettest never silently under-runs on an ambiguous changeset.

CI deliberately runs **`cargo nextest run --release --run-ignored all`** — every sweettest carries `#[ignore]` so a developer's local `cargo test` skips the expensive suite, and CI overrides that to run them all. (See the `#[ignore]`-is-a-CI-no-op watch-out below.)

## Non-goals (settled)

Sweettest does **not** replace either neighbor in the pyramid: it is not a substitute for a2o browser scenarios (which prove human experience) nor for in-zome unit tests (which prove logic). It is **Rust-native** — cucumber-rs Gherkin-driving was evaluated and **parked**. Per-DNA Jenkinsfiles were considered and **deferred**.

---

## Watch-outs (carry these forward — each cost real CI time)

### 1. Sweettest is a NATIVE build — it needs the native-build env, NOT the WASM env

This is the load-bearing gotcha and the reason this seed exists. The CLAUDE.md "RUSTFLAGS Override Required" rule (custom getrandom backend for the WASM DNA build) is **exactly wrong for the sweettest compile**, which is a native in-process build. Three stacked failures, each revealing the next:

1. **`datachannel-sys` panics `is cmake not installed?`** — the Nix devShell driving the stage must provide `cmake, pkg-config, clang, libclang.lib, openssl, zlib, libsodium` + `LIBCLANG_PATH`. (Fixed in the DNA `flake.nix`.)
2. **After cmake, link fails `undefined reference to __getrandom_v03_custom`** — the WASM `RUSTFLAGS=--cfg getrandom_backend="custom"` leaked into the native sweettest compile. The DNA Integration stage **must clear `RUSTFLAGS`** (`RUSTFLAGS=""`).
3. **After it links, the cold sweettest compile alone exceeds a 30-min stage budget** — bump the DNA Integration stage timeout to **~90min** and the pipeline to **~150min**.

Parameter-bearing: cmake/clang/LIBCLANG_PATH present, `RUSTFLAGS` cleared in the stage, ~90/150-min budget. (Full RCA: memory `feedback_sweettest_native_build_env`.)

### 2. `#[ignore]` is a no-op as a CI silencer

Because CI runs `--run-ignored all`, **annotating a broken sweettest with `#[ignore]` does nothing in CI** — the test still runs and still fails. To remove a sweettest from the CI run you must **delete it** or change the Jenkinsfile invocation, never annotate it. (Cost a full ~75-min holochain cycle when `#[ignore]` on a flaky round-trip test was a no-op and the test had to be deleted instead. Memory: `feedback_sweettest_ignore_is_ci_noop`.)

### 3. `CARGO_TARGET_DIR` scope and sccache are operational landmines, not test bugs

- `CARGO_TARGET_DIR=/cargo-target` MUST stay **scoped to the sweettest `sh` block** — exporting it pod-wide broke `hc dna pack` (the DNA pack and the sweettest build want different target dirs).
- **sccache stays DISABLED** for this stage. The ~1.7% spawn-ENOENT rate is a spawn problem, NOT a cache-substrate problem (RCA: `feedback_sccache_spawn_enoent_rca`); enabling it here buys flake, not speed.

### 4. Cross-agent suites need explicit DHT consistency

A `two_agent_conductors` sweettest is not consistent by default — it needs `exchange_peer_info` + `await_consistency` before asserting cross-agent readback, or it races. (Memory: `feedback_sweettest_cross_agent_consistency`.)

### 5. `@dna-scope` defaults to always-run; compose with the quarantine

`@dna-scope` markers default to **always-run** (fail-safe: an unmarked or ambiguous suite is included, never dropped). Compose new suites *with* the standing 4-test quarantine rather than re-litigating it.

---

## Excluded (genuinely live — left in the pile, not folded here)

- `genesis/docs/superpowers/sprints/2026-05-24-sweettest-stage-efficiency-w1-w2-w3-w5.md` — the landing-record for the efficiency sprint; gets its own curation pass.
- `2026-04-21-rno-lessons-cross-wave-guidance.md` — points here, isn't a body of this cluster.
- The ~6 specs that merely *mention* sweettest stay where they are.

The only un-landed thread from the efficiency design is the **W4b sccache 0.14 downgrade** — a one-line follow-up pointer, not a body worth keeping.
