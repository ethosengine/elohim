#!/usr/bin/env bash
# Preflight refuses a DEFAULTED storage binary in the gate's slot (dev/debug), which the next
# `just gate elohim-storage` rewrites with default features under the running peers; an
# explicit STORAGE_BIN and the mesh's own copy pass. No binary is executed.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
export MESH_TRANSPORT_BACKEND=libp2p

# Each case sources the script in its own shell, as each `just mesh` invocation does.
in_gate_slot() { # <explicit STORAGE_BIN or ''> <slot to resolve to: debug|mesh>
  (
    if [ -n "$1" ]; then export STORAGE_BIN="$1"; else unset STORAGE_BIN; fi
    source "$here/../hc-mesh.sh" >/dev/null
    case "$2" in
      debug) STORAGE_BIN="$_storage_debug" ;;
      mesh) STORAGE_BIN="$_storage_mesh" ;;
    esac
    storage_bin_in_gate_slot
  )
}

debug_slot="/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev/debug/elohim-storage"
t "a defaulted dev/debug binary is the gate's slot" "in_gate_slot '' debug"
t "the mesh's own copy is not" "! in_gate_slot '' mesh"
t "an explicit STORAGE_BIN naming dev/debug is the caller's choice" "! in_gate_slot '$debug_slot' debug"

# The refusal line names its cure.
line="$(
  unset STORAGE_BIN
  source "$here/../hc-mesh.sh" >/dev/null
  STORAGE_BIN="$_storage_debug"
  storage_bin_in_gate_slot && echo "REFUSED elohim-storage binary: $STORAGE_BIN is the gate's slot"
)"
t "the refusal names the slot" "[[ \"\$line\" == *dev/debug/elohim-storage* ]]"
t "preflight carries the cure" "grep -q 'is the gate.s slot .*just mesh build storage' '$here/../hc-mesh.sh'"

exit "$fail"
