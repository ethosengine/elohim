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
# `${9-...}` (unset-only), NOT `${9:-...}` (unset-or-EMPTY). `rustflags: ""` is
# the single most common declaration in the manifests — it is how a native crate
# clears the ambient WASM `--cfg getrandom_backend="custom"`. With `:-` the empty
# string collapsed to __inherit__, RUSTFLAGS was left alone, and every native gate
# died at link time with `undefined symbol: __getrandom_v03_custom`.
rustflags="${9-__inherit__}"

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

# Manifest-declared `run.cargo.env`, serialized as JSON by gate-runner.mjs into ONE
# variable rather than added to the positional contract (which is fixed at four cargo
# args and depended on by every caller). This is where a project declares its own
# resource cap — CARGO_BUILD_JOBS / RUST_TEST_THREADS — so a heavy crate's gate peaks
# below the workspace RAM guard's shed line instead of being killed at 80%.
if [[ -n "${GATE_CARGO_ENV:-}" && "${GATE_CARGO_ENV}" != "{}" ]]; then
  gate_cargo_exports="$(node <<'NODE'
const map = JSON.parse(process.env.GATE_CARGO_ENV || "{}");
const SQ = String.fromCharCode(39);
const shellQuote = (value) => SQ + String(value).split(SQ).join(SQ + "\\" + SQ + SQ) + SQ;
for (const [key, value] of Object.entries(map)) {
  if (!/^[A-Z][A-Z0-9_]*$/.test(key)) {
    console.error("gate: refusing malformed cargo.env key: " + key);
    process.exit(2);
  }
  console.log("export " + key + "=" + shellQuote(value));
}
NODE
  )"
  eval "$gate_cargo_exports"
  while IFS= read -r line; do
    [[ -n "$line" ]] && echo "  [$project_name] cargo env: ${line#export }"
  done <<< "$gate_cargo_exports"
  unset GATE_CARGO_ENV
fi

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
