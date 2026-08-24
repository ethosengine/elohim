#!/usr/bin/env bash
# Sourced-mode unit tests for hc-mesh.sh's peer transport and footprint helpers.
# Sourcing never starts anything (dispatch guard at the bottom of hc-mesh.sh).
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

# 1. unset knob → every peer inherits MESH_TRANSPORT_BACKEND
( set +e; MESH_PEERS=matthew,jessica MESH_TRANSPORT_BACKEND=dual MESH_PEER_TRANSPORTS= \
  source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "inherit: matthew=dual"  '[ "$(peer_transport matthew)" = dual ]'
  t "inherit: jessica=dual"  '[ "$(peer_transport jessica)" = dual ]'
  exit $fail ) || fail=1

# 2. partial map → named peer gets its own, the other inherits
( set +e; MESH_PEERS=matthew,jessica MESH_TRANSPORT_BACKEND=dual MESH_PEER_TRANSPORTS="jessica=iroh" \
  source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "map: matthew inherits dual" '[ "$(peer_transport matthew)" = dual ]'
  t "map: jessica=iroh"          '[ "$(peer_transport jessica)" = iroh ]'
  t "overlay carries the PEER mode, not the global" \
    'restart_env_overlay /dev/null jessica | grep -qx "ELOHIM_TRANSPORT_BACKEND=iroh"'
  exit $fail ) || fail=1

# 3. invalid mode is refused at source time
( set +e; MESH_PEERS=matthew,jessica MESH_PEER_TRANSPORTS="jessica=quic" \
  source "$here/../hc-mesh.sh" >/dev/null 2>&1; rc=$?
  t "invalid per-peer mode refused (rc=$rc)" '[ "$rc" -ne 0 ]'
  exit $fail ) || fail=1

# 4. footprint is a function of what is RUNNING, formatted for grep
( set +e; MESH_PEERS=matthew,jessica source "$here/../hc-mesh.sh" >/dev/null 2>&1
  out="$(mesh_footprint 2>/dev/null)"
  t "footprint prints a total line" 'grep -q "^footprint total rss=[0-9]*MB" <<<"$out"'
  exit $fail ) || fail=1

# 5. doorway-less overlay omits the doorway URL
( set +e; MESH_PEERS=matthew,jessica MESH_DOORWAYS=0 source "$here/../hc-mesh.sh" >/dev/null 2>&1
  t "MESH_DOORWAYS=0 overlay has no ELOHIM_DOORWAY_URL" '! restart_env_overlay /dev/null matthew | grep -q ELOHIM_DOORWAY_URL'
  exit $fail ) || fail=1

exit $fail
