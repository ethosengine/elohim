#!/usr/bin/env bash
# The orchestrator runs this before it dispatches any downstream pipeline.
set -euo pipefail
style_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$style_root"
if [[ -n "${CHANGE_TARGET:-}" ]]; then
  style_base="refs/remotes/origin/$CHANGE_TARGET"
elif [[ "${BRANCH_NAME:-}" != dev && "${BRANCH_NAME:-}" != main ]]; then
  # Feature branches owe paydown once over their integration change, not per build.
  style_base="refs/remotes/origin/dev"
else
  # Integration jobs compare with their last accepted build. A new job without
  # that evidence must refuse; HEAD^ would bless earlier untested commits.
  style_base="${GIT_PREVIOUS_SUCCESSFUL_COMMIT:-}"
  if [[ -z "$style_base" ]]; then
    echo 'Rust style UNMEASURED: integration job needs GIT_PREVIOUS_SUCCESSFUL_COMMIT from a trusted successful build.' >&2
    exit 2
  fi
fi
if ! style_base="$(git rev-parse --verify "$style_base^{commit}")"; then
  echo 'Rust style UNMEASURED: trusted baseline object missing; restore integration history in the checkout.' >&2
  exit 2
fi
if [[ "$style_base" == "$(git rev-parse HEAD)" && "${GIT_PREVIOUS_SUCCESSFUL_COMMIT:-}" == "$style_base" ]]; then
  # Only a rebuild of an already accepted commit can use its parent. This is
  # never the fallback for a first build or missing history.
  style_base="$(git rev-parse --verify 'HEAD~1^{commit}')"
fi
if [[ "${1:-}" == --print-base ]]; then
  printf '%s\n' "$style_base"
  exit 0
fi
export CARGO_TARGET_DIR="$style_root/.rust-style-target" RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_BUILD_JOBS=1
# The official Rust image uses the minimal profile.
rustup component add rustfmt clippy
cargo build --locked --manifest-path genesis/orchestrator/rust-style/Cargo.toml
cargo fmt --manifest-path genesis/orchestrator/rust-style/Cargo.toml --check
cargo clippy --locked --manifest-path genesis/orchestrator/rust-style/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path genesis/orchestrator/rust-style/Cargo.toml
"$CARGO_TARGET_DIR/debug/epr-rust-style" --root "$style_root" --base "$style_base" --head HEAD
