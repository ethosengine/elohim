#!/usr/bin/env bash
# Sourced-mode tests: every storage peer the household mesh launches (fresh start,
# late join, restart) gets an ABSOLUTE pillar-manifest directory. A peer runs from
# its own state directory, so the storage's historical bare default
# `elohim/sdk/domains` named nothing there and layer-1 write-through stayed empty.
# Sourcing never starts anything (dispatch guard at the bottom of hc-mesh.sh).
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../../.." && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

# 1. default: the repo's own pillar manifests, absolute, for every peer's restart overlay
( set +e; unset ELOHIM_PILLAR_MANIFEST_DIR
  MESH_PEERS=matthew,jessica,james source "$here/../hc-mesh.sh" >/dev/null 2>&1
  want="ELOHIM_PILLAR_MANIFEST_DIR=$repo/elohim/sdk/domains"
  for p in matthew jessica james; do
    t "overlay: $p carries the absolute manifest dir" \
      'restart_env_overlay /dev/null '"$p"' 2>/dev/null | grep -qxF "$want"'
  done
  t "default dir is absolute" '[ "${want#ELOHIM_PILLAR_MANIFEST_DIR=/}" != "$want" ]'
  t "default dir holds the shefa manifest" '[ -f "$(pillar_manifest_dir)/shefa/manifest.json" ]'
  exit $fail ) || fail=1

# 2. fresh start / late join launch the storage with the same variable
( set +e; MESH_PEERS=matthew,jessica source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "start_storage_peer exports ELOHIM_PILLAR_MANIFEST_DIR from pillar_manifest_dir" \
    'declare -f start_storage_peer | grep -q "ELOHIM_PILLAR_MANIFEST_DIR=\"\$(pillar_manifest_dir)\""'
  exit $fail ) || fail=1

# 3. an operator's explicit directory wins
( set +e; export ELOHIM_PILLAR_MANIFEST_DIR=/srv/manifests
  MESH_PEERS=matthew source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "explicit ELOHIM_PILLAR_MANIFEST_DIR wins" \
    'restart_env_overlay /dev/null matthew 2>/dev/null | grep -qx "ELOHIM_PILLAR_MANIFEST_DIR=/srv/manifests"'
  exit $fail ) || fail=1

exit $fail
