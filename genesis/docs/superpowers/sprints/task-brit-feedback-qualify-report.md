---
id: task-brit-feedback-qualify-report-20261111
gap: plans__2026-11-11-brit-feedback-operation#4
actor: agent:orchestrator@gpt-6-astra
status: HOLD
commits: []
---

# Native feedback qualification implemented; combined gate pending

`epr feedback qualify --brit PATH --root ROOT --json` now exercises the invoked recorder and explicit producer in a disposable store. It applies the resolved contextual policy snapshot, checks both outcomes and the Brit producer identity, reopening, private argument exclusion, exact native Intent scope, no admission or recovery, and recorder failure preserving producer exits. `--recorder-only` performs the recorder half and explicitly reports `pairQualified: false`.

The existing a2o probe now selects installed current or an explicit candidate pair and consumes the native CheckWitness. It rejects inconsistent witnesses and recorder-only success for a pair check. A native launcher is invoked by its own name with `resolve`; older launchers remain unmeasured with a direct-resolver remedy. No session-pinning alias is used for inspection.

The shared bounded subprocess executor accepts a prepared Command, preserving the existing process API. Its deadline now includes inherited pipe drain and kills the child's process group, preventing descendants from holding qualification open. No new subprocess framework or telemetry store was introduced.

## Verification

Evidence is in `genesis/a2o/reports/recovery/brit-feedback-20261111/`.

- Native qualification: `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test feedback_qualify` — **7 passed; EXIT=0**, `qualify-native-tests.log`.
- Native scope regression: `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --lib feedback::qualify::tests` — **1 passed; EXIT=0**, `qualify-scope-tests.log`.
- Shared executor regressions: `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-epr-cli --test flow_memory_recall bounded` — **3 passed; EXIT=0**, `qualify-bounded-process-tests.log`.
- Native landing qualification, after root's source freeze: `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-device-runtime --lib feedback_health` — **5 passed; EXIT=0**, `qualify-health-tests.log`.
- Native launcher: `cargo test --manifest-path elohim/eprfs/Cargo.toml -p elohim-device-runtime --test launcher` — **7 passed; EXIT=0**, `qualify-launcher-tests.log`.
- Adapter tests with explicit real candidates: `node --import tsx --test scripts/__tests__/verify-brit-feedback.test.ts` from a2o — **10 passed, zero skipped; EXIT=0**, `qualify-adapter-tests.log`.
- Scoped ESLint and rustfmt checks — **EXIT=0**.
- Installed-current probe still reports **failed; EXIT=1**, `qualify-installed-before.json`; explicit candidate recorder-only and pair checks both pass. Those successes establish fixture behavior, not installed delivery.

Native tests used `RUSTFLAGS=`, `RUSTUP_TOOLCHAIN=1.96.1`, `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__eprfs/dev`, and one build/test thread under the retained orchestrator cargo berth.

Gate-evidence: **`just gate eprfs` and `just gate genesis-a2o` remain pending the integrator's frozen combined source.** This report therefore holds the station open and records no terminal fulfilment. The root integrator owns publication, installed replay, appointed acceptance and the terminal native act.

## Changed surfaces

- `elohim/eprfs/epr-cli/src/feedback/{mod,qualify}.rs` and `tests/feedback_qualify.rs`.
- `elohim/eprfs/epr-cli/src/flow/memory/recall/{mod,providers}.rs` for shared bounded execution.
- The qualifier verdict entry in `elohim/eprfs/epr-cli/seam-registry.yaml`; the source reconciliation entry belongs to the other implementation task.
- `genesis/a2o/scripts/verify-brit-feedback.ts` and its existing test file.

The task did not commit, promote releases, alter actor claims, or write to the real feedback store. Root-owned landing and launcher changes are verified separately after their final source freeze.
