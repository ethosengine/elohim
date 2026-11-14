#!/usr/bin/env bash
# Public epr/ark contracts only: no real store, build, or household mutation.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export TMPDIR="${TMPDIR:-/var/tmp/lane-a}"
mkdir -p "$TMPDIR"
tmp="$(mktemp -d "$TMPDIR/hc-mesh-compatibility.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
fail=0
count=0
t() {
  count=$((count + 1))
  if eval "$2"; then echo "ok $count - $1"; else echo "FAIL $count - $1: $out"; fail=1; fi
}
mkdir -p "$tmp/bin"
cat > "$tmp/bin/epr" <<'STUB'
#!/usr/bin/env bash
set -eu
[ "$1" = artifact ]
verb="$2"
shift 2
case "$verb" in
  holders)
    [ "$*" = '--key key-a --json' ]
    if [ "$MODE" = no-own ]; then
      echo '{"holders":[{"device":"sibling","output":"foreign","self":false}]}'
    else
      echo '{"holders":[{"device":"sibling","output":"foreign","self":false},{"device":"local","output":"output-a","self":true}]}'
    fi ;;
  requires)
    [ "$1" = --output ] && [ "$2" = output-a ]
    [ "$3" = --write ] && [ "$5" = --json ]
    case "$MODE" in
      unavailable) printf 'no recorded requirement\ndocument unavailable locally\n' >&2; exit 3 ;;
      unknown) echo 'unknown output output-a' >&2; exit 2 ;;
      missing) echo "error: unrecognized subcommand 'requires'" >&2; exit 2 ;;
      error) echo 'reader failed' >&2; exit 1 ;;
    esac
    printf 'exact requirement bytes\n' > "$4"
    case "$MODE" in
      own) echo '{"output":"output-a","requires":"requires-a","self":true,"produce":"produce-a"}' ;;
      null) echo '{"self":true,"produce":null}' ;;
      sibling) echo '{"self":false,"produce":"publisher-claim"}' ;;
      *) echo '{"output":"output-a","requires":"requires-a","self":true}' ;;
    esac ;;
  *) exit 99 ;;
esac
STUB
cat > "$tmp/bin/ark" <<'STUB'
#!/usr/bin/env bash
set -eu
[ "$1 $2 $3" = 'host admit --requires' ]
[ "$(cat "$4")" = 'exact requirement bytes' ]
[ "$5" = --binary ] && [ -f "$6" ]
[ "$7 $8 $9" = '--output output-a --name' ]
name="${10}"
if [ "$MODE" = own ]; then
  [ "$#" = 12 ] && [ "${11} ${12}" = '--own-produce produce-a' ]
  echo "ok compatibility $name: admitted under the own-build rule (waived: isa.x86 avx512f)"
else
  [ "$#" = 10 ]
  if [ "$MODE" = refused ]; then
    echo "REFUSED compatibility $name: isa.x86 requires avx512f"
    exit 1
  fi
  echo "ok compatibility $name: admitted (2 clauses)"
fi
STUB
chmod +x "$tmp/bin/epr" "$tmp/bin/ark"
export PATH="$tmp/bin:$PATH"
export EPR_BIN=epr ARK_BIN="$tmp/bin/ark"
export MESH_DIR="$tmp/mesh" MESH_PEERS=unit MESH_TRANSPORT_BACKEND=libp2p MESH_DOORWAYS=0
source "$here/../hc-mesh.sh" >/dev/null
STORAGE_BIN="$tmp/elohim-storage"
DOORWAY_BIN="$tmp/doorway"
BEACON_BIN="$tmp/relay-addr-beacon"
MESH_DOORWAYS_EFFECTIVE=0
MESH_MEMBERSHIP=1
for bin in elohim-storage doorway relay-addr-beacon; do
  touch "$tmp/$bin"
  printf '{"key":"key-a","method":"method-a","input":"input-a","head":"head"}\n' > "$tmp/.derivation-$bin.json"
done
export MODE=admitted
run() { out="$(preflight_binary_compatibility 2>&1)"; rc=$?; }
run
t 'admitted from own holder, absent produce key accepted' '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: admitted (2 clauses)" ]'
MODE=own
run
t 'own-build claim forwarded and waived clause printed' '[ "$rc" = 0 ] && [[ "$out" == *"admitted under the own-build rule (waived: isa.x86 avx512f)" ]]'
for MODE in null sibling; do
  run
  t "$MODE Produce never forwarded as an own-build claim" '[ "$rc" = 0 ] && [[ "$out" == *"admitted (2 clauses)" ]]'
done
MODE=refused
run
t 'refusal fails preflight' '[ "$rc" = 1 ] && [[ "$out" == "REFUSED compatibility elohim-storage:"* ]]'
MESH_ALLOW_INCOMPATIBLE_BINARY=1
run
t 'override preserves refusal and announces its use' '[ "$rc" = 0 ] && [[ "$out" == "REFUSED compatibility elohim-storage:"* ]] && [[ "$out" == *"MESH_ALLOW_INCOMPATIBLE_BINARY=1 used" ]]'
unset MESH_ALLOW_INCOMPATIBLE_BINARY
MODE=unavailable
run
t 'exit 3 is unmeasured with a single-line reason' '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: unmeasured (no recorded requirement document unavailable locally)" ]'
MODE=unknown
run
t 'exit 2 is a provenance refusal with a runnable rebuild command' '[ "$rc" = 1 ] && [ "$out" = "REFUSED compatibility elohim-storage: epr does not know output output-a — rebuild with just mesh build storage" ]'
for MODE in missing error; do
  run
  t "$MODE reader is unmeasured" '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: unmeasured (epr has no requirement reader yet)" ]'
done
MODE=no-own
run
t 'sibling holder is not local Produce' '[ "$rc" = 0 ] && [[ "$out" == *"unmeasured (no Produce on this device)" ]]'
MODE=admitted
ARK_BIN="$tmp/absent"
run
t 'missing ark is unmeasured' '[ "$rc" = 0 ] && [[ "$out" == *"unmeasured (no ark binary)" ]]'
ARK_BIN="$tmp/bin/ark"
rm "$tmp/.derivation-elohim-storage.json"
MODE=unknown
run
t 'missing derivation record stops before epr' '[ "$rc" = 0 ] && [[ "$out" == *"unmeasured (no derivation record)" ]]'
MODE=admitted
MESH_DOORWAYS_EFFECTIVE=1
run
t 'storage doorway and staged beacon each get exactly one line' '[ "$rc" = 0 ] && [ "$(wc -l <<<"$out")" = 3 ] && [[ "$out" == *"ok compatibility doorway: admitted"* ]] && [[ "$out" == *"ok compatibility relay-addr-beacon: admitted"* ]]'
MESH_MEMBERSHIP=0
run
t 'unstaged beacon is omitted' '[ "$rc" = 0 ] && [ "$(wc -l <<<"$out")" = 2 ] && [[ "$out" != *relay-addr-beacon* ]]'
# Exercise the public preflight wiring without probing or modifying a household.
load_canonical_household_binding() { return 0; }
household_start_mode() { echo resume; }
preflight_binary_freshness() { echo freshness-marker; }
happ_bundle_freshness() { return 0; }
happ_bundle_identity() { return 0; }
storage_bin_in_gate_slot() { return 1; }
assert_storage_transport_capability() { return 0; }
mesh_owned_ports() { :; }
assert_launch_prerequisites() { return 0; }
MESH_DOORWAYS_EFFECTIVE=0
MESH_HOLOCHAIN_BIN_EXPLICIT=1
HOLOCHAIN_BIN="$tmp/bin/ark"
chmod +x "$STORAGE_BIN"
printf '{"key":"key-a"}\n' > "$tmp/.derivation-elohim-storage.json"
MODE=admitted
out="$(preflight 2>&1)"; rc=$?
t 'otherwise green preflight measures compatibility after freshness' '[ "$rc" = 0 ] && [[ "$out" == *$'"'"'freshness-marker\nok compatibility elohim-storage: admitted'"'"'* ]]'
MODE=refused
out="$(preflight 2>&1)"; rc=$?
t 'compatibility alone makes preflight refuse' '[ "$rc" = 1 ] && [[ "$out" == *"REFUSED compatibility elohim-storage:"* ]]'
MESH_ALLOW_INCOMPATIBLE_BINARY=1
out="$(preflight 2>&1)"; rc=$?
t 'declared override restores preflight success' '[ "$rc" = 0 ] && [[ "$out" == *"MESH_ALLOW_INCOMPATIBLE_BINARY=1 used"* ]]'
echo "$count assertions"
exit "$fail"
