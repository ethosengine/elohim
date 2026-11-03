#!/usr/bin/env bash
# This is the pinned justBin for the first provider CI profile, not a general shell.
set -euo pipefail
here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$here/mesh-env.sh"
unset MESH_API_KEY_ADMIN MESH_COMPUTE_LOCAL_TOKEN
if [[ "$#" -ne 3 || "$1" != test || "$2" != mesh ]]; then
  echo 'REFUSED provider mesh: expected test mesh <feature> from provider-stages.json' >&2
  exit 2
fi
repo="$(pwd -P)"
case "$repo" in
  "$compute_state_root"/repos/*) ;;
  *) echo 'REFUSED provider mesh: stage must run in a pinned provider checkout' >&2; exit 2 ;;
esac
allowlist="$repo/genesis/agentic/compute/provider-stages.json"
if ! /usr/local/bin/node -e '
const fs = require("node:fs");
const [file, feature] = process.argv.slice(1);
const parsed = JSON.parse(fs.readFileSync(file, "utf8"));
process.exit(Array.isArray(parsed.features) && parsed.features.includes(feature) ? 0 : 2);
' "$allowlist" "$3"; then
  echo "REFUSED provider mesh: $3 is not listed in provider-stages.json" >&2
  exit 2
fi
[[ -f "$repo/.compute-source-commit" && -f "$MESH_DIR/.provider-mesh-ready" ]] || {
  echo 'REFUSED provider mesh: pinned checkout or owned mesh readiness is absent' >&2; exit 2;
}
export MESH_ALLOW_NO_PROLOGUE=1
# There is no workspace lease here: the sole worker serializes admission, and the executor
# owns this process tree's hard deadline. Declare that authority rather than relying on the
# workspace berth's permission-denied fallback. The wrapper exposes no mesh control verb.
export BERTH_FENCED=1
echo 'provider mesh: sequential worker admission and executor deadline own this read-only stage' >&2
exec "${COMPUTE_JUST_BIN:-/usr/local/bin/just}" --justfile "$repo/justfile" "$@"
