---
name: project_conductor_fork_verification_traps
description: "How to verify a conductor-fork change locally — patch-crate tests in-dir, one build at a time, timing the startup window from storage, metrics have no exporter"
id: project-conductor-fork-verification-traps
metadata:
  node_type: memory
  title: Conductor-fork verification traps and order
  type: project
  originSessionId: c72bf075-22be-4d33-8f6f-920025236870
  modified: 2026-09-23T20:33:18.854Z
cites:
  - elohim/holochain-conductor/
  - app/elohim-app/scripts/hc-mesh.sh
---

Verifying `elohim/holochain-conductor` changes locally (learned 2026-09-23 integrating the 09-21 diagnostics WIP + throttle + two perf branches as `int/2026-09-23-diagnostics-throttle-perf`, tip 61565f320):

- `cargo check -p kitsune2_transport_iroh` from the fork root PANICS cargo's feature resolver — the patch crate is a `[patch.crates-io]` path target excluded from the workspace. Test it from its own dir: `cd patches/kitsune2_transport_iroh && cargo test --lib --features test-utils` with its own `CARGO_TARGET_DIR`.
- A PreToolUse hook enforces ONE cargo build at a time per workspace (worn NVMe mirror); a second cargo is refused, not queued. Sequence test runs and release builds; `setsid nohup` launches still count.
- NEVER share one `CARGO_TARGET_DIR` between two checkouts/worktrees of the fork: cargo keys workspace-crate fingerprints on the path RELATIVE to the workspace, so both checkouts read and overwrite each other's artifacts (edited files read as fresh; a cached build script keeps the other worktree's absolute path baked in and fails after that worktree is removed). Give each worktree its own slot, or after removing a worktree run `git ls-files crates patches | xargs touch` in the survivor before trusting a build.
- The fork pins rust 1.96.1 (auto-installed on first use). Production-feature release build: `--no-default-features --features encryption,wasmer-sys-cranelift,jemalloc --bin holochain`, ~10.5 min on 24 cores from a cold slot.
- Household conductors run at `RUST_LOG=warn,…,kitsune2_gossip=warn`, so `DHT model initialised in Ns` and `Conductor startup: apps enabled.` (INFO) are NOT in `conductors/.sandbox_run_log` (one multiplexed file for all three). Time the CellDisabled startup window from storage: `logs/<peer>.log` first `ABSENT/STRANDED` line, then `/health` per-role `lastZomeCallAgeSecs`.
- Install a verified build as `/projects/.claude-config/tools/hc-fork-<pin12>/bin/{holochain,hc}` (copy, never a symlink into a cargo-pool slot) so `just mesh start` auto-detects it once the submodule pin moves; explicit `HOLOCHAIN_BIN=<dir>` skips the pin check; `MESH_ALLOW_STALE_HAPP=1` when only the conductor is under test.
- The conductor emits ~28 OpenTelemetry instruments but `holochain_metrics` only has InfluxDB exporters and the fleet sets no `HOLOCHAIN_INFLUXIVE_*`, so Prometheus has zero `hc_*` series. `opentelemetry-prometheus` 0.31 is deprecated (unmaintained protobuf dep); a small text exporter over `opentelemetry_sdk`'s `ManualReader` (`experimental_metrics_custom_reader`) or OTLP is the honest path.
- Pushing the fork branch to `ethosengine/holochain` was refused by the auto-mode classifier; the operator pushes it. The conductor image job builds only from the committed superproject gitlink, so the fork push must land before the pin-move commit is pushed.

**Why:** the fork WIP sat uncommitted for two days because every cargo pool slot for it had been reclaimed and nobody could see whether it compiled; each trap above cost a retry.
**How to apply:** patch crate tests in-dir → workspace crate tests → release build → tools-dir install → household on `HOLOCHAIN_BIN` → records → pin move. See [[project_conductor_arc_resources]] and [[feedback_local_mesh_first_cadence]].
