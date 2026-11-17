#!/usr/bin/env bash
# The same native checker runs locally, before push, in CI and behind agent adapters.
set -euo pipefail
style_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$style_root/genesis/orchestrator/rust-style-build-env.sh"
rust_style_build_environment "$style_root" || exit $?
cd "$style_root"
# Ownership failures must be diagnosed before pool discovery or any Cargo invocation.
if ! style_git="$(git -C "$style_root" rev-parse --show-toplevel 2>&1)"; then
  echo "Rust style UNMEASURED: Git cannot read $style_root: $style_git" >&2
  if [[ "$style_git" == *"dubious ownership"* ]]; then
    echo "Cure in this Che workspace: git config --global --add safe.directory '*' (or rerun devfile setup-git)." >&2
  fi
  exit 2
fi
source "$style_root/genesis/agentic/bin/pool-lib.sh"
style_family="$(detect_family "$style_root")"
style_target="$(slot_path "$style_family" genesis__orchestrator__rust-style dev)"
style_toolchain="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["cargo_env_overrides"]["*"]["RUSTUP_TOOLCHAIN"])' "$style_root/genesis/agentic/pool-policy.json")"
export CARGO_TARGET_DIR="$style_target" RUSTFLAGS="" CARGO_BUILD_JOBS=1
export RUSTUP_TOOLCHAIN="$style_toolchain"
style_binary="$style_target/debug/epr-rust-style"
if [[ "${1:-}" == "--hook" ]]; then
  shift
  if [[ ! -x "$style_binary" ]]; then
    echo "Rust style UNMEASURED: run just gate rust-style to build the native checker." >&2
    exit 2
  fi
  shopt -s globstar nullglob
  for style_input in "$style_root"/.cargo/config.toml "$style_root"/genesis/orchestrator/rust-style/{Cargo.toml,Cargo.lock,src/**/*.rs}; do
    if [[ "$style_input" -nt "$style_binary" ]]; then
      echo "Rust style UNMEASURED: checker inputs changed; run just gate rust-style." >&2
      exit 2
    fi
  done
  exec "$style_binary" --root "$style_root" "$@"
fi
# Cargo's fingerprint checks are the freshness authority. Never use an arbitrary PATH binary.
if [[ "${1:-}" != --verify* ]]; then
  cargo build --quiet --locked --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml"
fi
if [[ "${1:-}" == "--acceptance" ]]; then
  shift
  [[ $# == 1 ]] || { echo '--acceptance requires one fixture name' >&2; exit 2; }
  exec cargo test --quiet --locked --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml" --test reach "$1" -- --exact
fi
# --verify-checker: the checker's own legs, a function of its sources, the root
# .cargo/config.toml and the toolchain alone (gate project rust-style-checker, reused by key).
# --verify-scan: the repository scan against the moving baseline plus the adapter test, whose
# reads lie outside the checker (gate project rust-style, run on every push that selects it).
# --verify: both, in the original order, for a direct full run.
style_manifest="$style_root/genesis/orchestrator/rust-style/Cargo.toml"
style_checker_legs=(
  fmt -- cargo fmt --manifest-path "$style_manifest" --check
  ::: lint -- node "$style_root/genesis/orchestrator/cargo-lint.mjs" --quiet --locked --manifest-path "$style_manifest" --all-targets -- -D warnings
  ::: test -- cargo test --quiet --locked --manifest-path "$style_manifest"
)
style_distribution_leg=(
  distribution -- python3 -m unittest discover -s "$style_root/genesis/orchestrator/rust-style" -p test_source_distribution.py
)
style_hooks_leg=(
  hooks -- python3 "$style_root/genesis/orchestrator/rust-style/hook_test.py"
)
style_check_leg=(
  check -- bash -c 'cargo build --quiet --locked --manifest-path "$1/genesis/orchestrator/rust-style/Cargo.toml" && "$CARGO_TARGET_DIR/debug/epr-rust-style" --root "$1" "${@:2}"' _ "$style_root"
)
case "${1:-}" in
  --verify-checker)
    shift
    [[ $# == 0 ]] || { echo '--verify-checker takes no arguments' >&2; exit 2; }
    bash "$style_root/genesis/orchestrator/gate-steps.sh" "${style_checker_legs[@]}" ::: "${style_distribution_leg[@]}"
    exit $?
    ;;
  --verify-scan)
    shift
    bash "$style_root/genesis/orchestrator/gate-steps.sh" "${style_hooks_leg[@]}" ::: "${style_check_leg[@]}" "$@"
    exit $?
    ;;
  --verify)
    shift
    bash "$style_root/genesis/orchestrator/gate-steps.sh" "${style_checker_legs[@]}" ::: "${style_hooks_leg[@]}" \
      ::: "${style_distribution_leg[@]}" ::: "${style_check_leg[@]}" "$@"
    exit $?
    ;;
esac
exec "$style_target/debug/epr-rust-style" --root "$style_root" "$@"
