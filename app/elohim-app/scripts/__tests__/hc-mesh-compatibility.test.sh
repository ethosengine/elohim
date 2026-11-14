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
[ "$1 $2 $3 $4 $5" = 'host admit --output output-a --binary' ]
[ -f "$6" ] && [ "$7" = --name ] && [ "$#" = 8 ]
name="$8"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
rc=0
reply="$(epr artifact requires --output "$4" --write "$work/requires" --json 2>"$work/why")" || rc=$?
case "$MODE" in
  unavailable) [ "$rc" = 3 ]; echo "ok compatibility $name: unmeasured (no recorded requirement or document unavailable locally)"; exit 0 ;;
  unknown) [ "$rc" = 2 ]; echo "REFUSED compatibility $name: epr does not know output output-a — rebuild or restore the recorded binary"; exit 1 ;;
  missing) [ "$rc" = 2 ]; echo "ok compatibility $name: unmeasured (epr has no requirement reader yet)"; exit 0 ;;
  error) [ "$rc" = 1 ]; echo "ok compatibility $name: unmeasured (malformed requirement record: artifact reader failed)"; exit 0 ;;
esac
[ "$rc" = 0 ] && [ "$(cat "$work/requires")" = 'exact requirement bytes' ]
if [ "$MODE" = own ]; then
  [ "$(jq -r '.self' <<<"$reply")" = true ]
  [ "$(jq -r '.produce' <<<"$reply")" = produce-a ]
  echo "ok compatibility $name: admitted under the own-build rule (waived: isa.x86 avx512f)"
else
  if [ "$MODE" = refused ]; then
    echo "REFUSED compatibility $name: isa.x86 requires avx512f"
    exit 1
  fi
  echo "ok compatibility $name: admitted"
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
t 'admitted from own holder, absent produce key accepted' '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: admitted" ]'
MODE=own
run
t 'ark resolves own-build eligibility and prints waived clause' '[ "$rc" = 0 ] && [[ "$out" == *"admitted under the own-build rule (waived: isa.x86 avx512f)" ]]'
for MODE in null sibling; do
  run
  t "$MODE record does not supply own-build eligibility" '[ "$rc" = 0 ] && [[ "$out" == *"admitted" ]]'
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
t 'exit 3 is unmeasured with a single-line reason' '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: unmeasured (no recorded requirement or document unavailable locally)" ]'
MODE=unknown
run
t 'exit 2 is a provenance refusal printed unchanged' '[ "$rc" = 1 ] && [ "$out" = "REFUSED compatibility elohim-storage: epr does not know output output-a — rebuild or restore the recorded binary" ]'
MODE=missing
run
t 'missing reader is unmeasured' '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: unmeasured (epr has no requirement reader yet)" ]'
MODE=error
run
t 'failed reader is unmeasured' '[ "$rc" = 0 ] && [ "$out" = "ok compatibility elohim-storage: unmeasured (malformed requirement record: artifact reader failed)" ]'
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
