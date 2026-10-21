#!/usr/bin/env bash
# This is the pinned justBin for the first provider CI profile, not a general shell.
set -euo pipefail
here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
source "$here/mesh-env.sh"
if [[ "$#" -ne 3 || "$1" != test || "$2" != mesh || "$3" != features/dataplane/peer-mesh.feature ]]; then
  echo 'REFUSED provider mesh: this no-cast profile permits only test mesh features/dataplane/peer-mesh.feature; other stories need a separately reviewed fixture/prologue profile' >&2
  exit 2
fi
repo="$(pwd -P)"
case "$repo" in
  "$compute_state_root"/repos/*) ;;
  *) echo 'REFUSED provider mesh: stage must run in a pinned provider checkout' >&2; exit 2 ;;
esac
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
