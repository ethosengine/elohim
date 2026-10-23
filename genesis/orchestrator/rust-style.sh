#!/usr/bin/env bash
# The same native checker runs locally, before push, in CI and behind agent adapters.
set -euo pipefail
style_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$style_root/genesis/agentic/bin/pool-lib.sh"
style_family="$(detect_family "$style_root")"
style_target="$(slot_path "$style_family" genesis__orchestrator__rust-style dev)"
style_toolchain="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["cargo_env_overrides"]["*"]["RUSTUP_TOOLCHAIN"])' "$style_root/genesis/agentic/pool-policy.json")"
export CARGO_TARGET_DIR="$style_target" RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_BUILD_JOBS=1
export RUSTUP_TOOLCHAIN="$style_toolchain"
style_binary="$style_target/debug/epr-rust-style"
if [[ "${1:-}" == "--hook" ]]; then
  shift
  if [[ ! -x "$style_binary" ]]; then
    echo "Rust style UNMEASURED: run just gate rust-style to build the native checker." >&2
    exit 2
  fi
  shopt -s globstar nullglob
  for style_input in "$style_root"/genesis/orchestrator/rust-style/{Cargo.toml,Cargo.lock,src/**/*.rs} "$style_root"/elohim/eprfs/{eprfs-meta,eprfs-core}/{Cargo.toml,src/**/*.rs} "$style_root"/crates/seam-contracts/{Cargo.toml,src/**/*.rs}; do
    if [[ "$style_input" -nt "$style_binary" ]]; then
      echo "Rust style UNMEASURED: checker inputs changed; run just gate rust-style." >&2
      exit 2
    fi
  done
  exec "$style_binary" --root "$style_root" "$@"
fi
# Cargo's fingerprint checks are the freshness authority. Never use an arbitrary PATH binary.
cargo build --quiet --locked --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml"
if [[ "${1:-}" == "--acceptance" ]]; then
  shift
  [[ $# == 1 ]] || { echo '--acceptance requires one fixture name' >&2; exit 2; }
  exec cargo test --quiet --locked --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml" --test reach "$1" -- --exact
fi
if [[ "${1:-}" == "--verify" ]]; then
  shift
  cargo fmt --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml" --check
  cargo clippy --quiet --locked --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml" --all-targets -- -D warnings
  cargo test --quiet --locked --manifest-path "$style_root/genesis/orchestrator/rust-style/Cargo.toml"
  python3 "$style_root/genesis/orchestrator/rust-style/hook_test.py"
fi
exec "$style_target/debug/epr-rust-style" --root "$style_root" "$@"
