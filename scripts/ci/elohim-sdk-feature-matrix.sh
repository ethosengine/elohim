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

bash "${SDK_MATRIX_ROOT}/genesis/orchestrator/gate-steps.sh" \
  fmt -- cargo fmt --manifest-path "${SDK_MANIFEST}" --check \
  ::: lint -- node "${SDK_MATRIX_ROOT}/genesis/orchestrator/cargo-lint.mjs" --locked --manifest-path "${SDK_MANIFEST}" --all-features --all-targets -- -D warnings \
  ::: test-no-default -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --no-default-features \
  ::: test-default -- cargo test --locked --manifest-path "${SDK_MANIFEST}" \
  ::: test-client -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --no-default-features --features client \
  ::: test-native -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --no-default-features --features native \
  ::: test-wasm -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --no-default-features --features wasm \
  ::: test-sync -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --no-default-features --features sync \
  ::: test-full -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --no-default-features --features full \
  ::: test-all -- cargo test --locked --manifest-path "${SDK_MANIFEST}" --all-features
