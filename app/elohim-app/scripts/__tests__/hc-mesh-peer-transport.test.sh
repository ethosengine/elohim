#!/usr/bin/env bash
# Sourced-mode unit tests for hc-mesh.sh's peer transport and footprint helpers.
# Sourcing never starts anything (dispatch guard at the bottom of hc-mesh.sh).
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

# 4. footprint is a function of what is RUNNING, formatted for grep
( set +e; MESH_PEERS=matthew,jessica source "$here/../hc-mesh.sh" >/dev/null 2>&1
  out="$(mesh_footprint 2>/dev/null)"
  t "footprint prints a total line" 'grep -q "^footprint total rss=[0-9]*MB" <<<"$out"'
  exit $fail ) || fail=1

exit $fail
