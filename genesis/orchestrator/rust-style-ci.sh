#!/usr/bin/env bash
# The orchestrator runs this before it dispatches any downstream pipeline.
set -euo pipefail
style_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$style_root/genesis/orchestrator/rust-style-build-env.sh"
rust_style_build_environment "$style_root" || exit $?
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
# A discipline-method bootstrap pins a new method at HEAD that the trusted
# baseline does not pin. The checker built from HEAD then refuses to read the
# baseline's policy ("discipline method differs from this installed checker"),
# and an integration job whose baseline advances only on success can never
# advance past it (orchestrator/dev #2220 onward). The earliest first-parent
# ancestor of HEAD that already pins HEAD's method is the oldest commit the
# installed checker can measure from: measure from it and say so. Commits
# before it were measured by the checker they pinned.
style_method_of() {
  git show "$1:.epr-meta/rust-style-policy.json" 2>/dev/null \
    | python3 -c 'import json,sys
try:
    print(json.load(sys.stdin)["discipline"]["method"]["cid"])
except Exception:
    print("")' 2>/dev/null || true
}
style_head_method="$(style_method_of HEAD)"
style_base_method="$(style_method_of "$style_base")"
if [[ -n "$style_head_method" && "$style_base_method" != "$style_head_method" ]]; then
  for style_candidate in $(git rev-list --first-parent --reverse "$style_base..HEAD"); do
    if [[ "$(style_method_of "$style_candidate")" == "$style_head_method" ]]; then
      echo "Rust style baseline moved: $style_base pins discipline method ${style_base_method:-(none)}; HEAD pins $style_head_method, first pinned at $style_candidate (a method bootstrap landed between). Measuring $style_candidate..HEAD." >&2
      style_base="$style_candidate"
      break
    fi
  done
fi
if [[ "${1:-}" == --print-base ]]; then
  printf '%s\n' "$style_base"
  exit 0
fi
export CARGO_TARGET_DIR="$style_root/.rust-style-target" RUSTFLAGS="" CARGO_BUILD_JOBS=1
# The official Rust image has no Node; the shared lint driver needs it in CI.
if ! command -v node >/dev/null 2>&1; then
  apt-get update
  apt-get install -y --no-install-recommends nodejs
fi
# The official Rust image uses the minimal profile.
rustup component add rustfmt clippy
bash "$style_root/genesis/orchestrator/gate-steps.sh" \
  fmt -- cargo fmt --manifest-path genesis/orchestrator/rust-style/Cargo.toml --check \
  ::: lint -- node genesis/orchestrator/cargo-lint.mjs --locked --manifest-path genesis/orchestrator/rust-style/Cargo.toml --all-targets -- -D warnings \
  ::: test -- cargo test --locked --manifest-path genesis/orchestrator/rust-style/Cargo.toml \
  ::: check -- bash -c 'cargo build --locked --manifest-path genesis/orchestrator/rust-style/Cargo.toml && "$CARGO_TARGET_DIR/debug/epr-rust-style" --root "$1" --base "$2" --head HEAD' _ "$style_root" "$style_base"
