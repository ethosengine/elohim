#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
declaration="${1:-}"
if [[ -z "$declaration" || "$declaration" == /* || "/$declaration/" == */../* || ! -f "${LOCAL_VIEW_ROOT:-$root}/$declaration" ]]; then
  echo "Provide an existing repository-relative view declaration (for example elohim/elohim-storage/.epr-meta/idle-is-free.view.json)." >&2
  exit 1
fi
if [[ "${LOCAL_VIEW_RENEW:-0}" == "1" ]]; then
  exec node "$root/app/scripts/local-view-launcher.mjs" "$@"
fi
cd "$root/steward/node"
# Use the owning pool slot, never an in-tree native target.
slot=$("$root/genesis/agentic/bin/cargo-pool" key | sed -n 's/^slot_dev=//p')
export CARGO_TARGET_DIR="$slot" RUSTFLAGS='' CARGO_BUILD_JOBS=1
echo "Building the optimized local reader (first launch compiles dependencies; later launches reuse them)." >&2
cargo +1.96.1 build --profile local-view --no-default-features --features local-view-only --bin elohim-node
export ELOHIM_NODE_BIN="$slot/local-view/elohim-node"
cd "$root/app/elohim-app"
if [[ "${LOCAL_VIEW_SKIP_BUILD:-0}" != "1" ]]; then
  pnpm exec ng build --configuration production
fi
cd "$root"
exec node app/scripts/local-view-launcher.mjs "$@"
