#!/usr/bin/env bash
# Typed local gate executor. Arguments come only from validated build manifests.
set -euo pipefail

repo_root="$1"
project_name="$2"
project_dir="$3"
kind="$4"
recipe="$5"
workspace="${6:-}"
target_dir="${7:-}"
profile="${8:-dev}"
rustflags="${9:-__inherit__}"

if [[ -n "$workspace" && -n "$target_dir" ]]; then
  echo "gate $project_name: cargo workspace and targetDir are mutually exclusive" >&2
  exit 2
fi

if [[ -n "$workspace" ]]; then
  # Use the cargo-pool's own family/slot functions. Do not infer a workspace by
  # walking Cargo.toml: storage and crates/* have deliberate explicit keys.
  source "$repo_root/genesis/agentic/bin/pool-lib.sh"
  family="$(detect_family "$repo_root")"
  target_dir="$(slot_path "$family" "$workspace" "$profile")"
fi

if [[ -n "$target_dir" ]]; then
  [[ "$target_dir" = /* ]] || target_dir="$repo_root/$target_dir"
  backing="$(readlink -m "$target_dir")"
  mkdir -p "$backing"
  export CARGO_TARGET_DIR="$target_dir"
  echo "  [$project_name] cargo target: $CARGO_TARGET_DIR"
fi

if [[ "$rustflags" != "__inherit__" ]]; then
  export RUSTFLAGS="$rustflags"
fi
export RUSTC_WRAPPER=""

case "$kind" in
  just)
    exec just --justfile "$repo_root/$project_dir/justfile" \
      --working-directory "$repo_root/$project_dir" "$recipe"
    ;;
  root-just)
    exec just --justfile "$repo_root/justfile" \
      --working-directory "$repo_root" "$recipe"
    ;;
  *)
    echo "gate $project_name: unsupported run kind '$kind'" >&2
    exit 2
    ;;
esac
