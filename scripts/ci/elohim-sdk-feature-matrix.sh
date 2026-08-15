#!/usr/bin/env bash
# Direct quality gate for crates/elohim-sdk.
#
# This is a build-only script. It runs in the edge CI builder and from the
# local pre-push gate; it is never invoked by a deploy container.

set -euo pipefail

SDK_MATRIX_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK_MANIFEST="${SDK_MATRIX_ROOT}/crates/elohim-sdk/Cargo.toml"

# Native Rust builds must not inherit the Holochain WASM getrandom cfg. The
# pre-push runner supplies a cargo-pool slot; direct and CI invocations use a
# bounded /tmp target instead of growing an in-tree target directory.
export RUSTFLAGS=""
export RUSTC_WRAPPER=""
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/elohim-sdk-feature-matrix-target}"

run_sdk_test() {
  local label="$1"
  shift

  echo "[elohim-sdk] cargo test: ${label}"
  cargo test --locked --manifest-path "${SDK_MANIFEST}" "$@"
}

cargo fmt --manifest-path "${SDK_MANIFEST}" --check

run_sdk_test "no default features" --no-default-features
run_sdk_test "default features"
run_sdk_test "client" --no-default-features --features client
run_sdk_test "native" --no-default-features --features native
run_sdk_test "wasm feature on native target" --no-default-features --features wasm
run_sdk_test "sync" --no-default-features --features sync
run_sdk_test "full" --no-default-features --features full
run_sdk_test "all features" --all-features

echo "[elohim-sdk] cargo clippy: all features and targets"
cargo clippy --locked --manifest-path "${SDK_MANIFEST}" --all-features --all-targets -- -D warnings
