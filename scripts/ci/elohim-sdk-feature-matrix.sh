#!/usr/bin/env bash
# Direct quality gate for crates/elohim-sdk.
#
# This is a build-only script. It runs in the edge CI builder and from the
# local pre-push gate; it is never invoked by a deploy container.

set -euo pipefail

SDK_MATRIX_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SDK_MANIFEST="${SDK_MATRIX_ROOT}/crates/elohim-sdk/Cargo.toml"

# The edge CI builder carries NO Rust toolchain by design (see
# storage-quality-gate.sh — cargo lives inside BuildKit targets there). When
# cargo is absent, re-enter this same script through the storage Dockerfile's
# `sdk-check` target, which COPYs it in and runs it where cargo exists. Local
# pre-push always has cargo and takes the direct path below; edge #1352 is the
# exit-127 this guard cures.
if ! command -v cargo >/dev/null 2>&1; then
  echo "[elohim-sdk] no cargo on PATH — running the matrix inside the sdk-check image target"
  buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null
  BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
      nerdctl -n k8s.io build \
      --target sdk-check \
      -t elohim-sdk-check:ci \
      -f "${SDK_MATRIX_ROOT}/elohim/elohim-storage/Dockerfile" "${SDK_MATRIX_ROOT}"
  exit 0
fi

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
