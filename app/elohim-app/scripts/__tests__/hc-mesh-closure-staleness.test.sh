#!/usr/bin/env bash
# Closure-key freshness and projection coverage for hc-mesh.sh. No real build, fetch, or pool slot
# is touched: EPR_BIN and cargo are fixtures rooted under the required persistent scratch area.
set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }

export TMPDIR=/var/tmp/b3-codex
mkdir -p "$TMPDIR"
tmp="$(mktemp -d "$TMPDIR/hc-mesh-closure-staleness.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT

stub="$tmp/epr"
cat > "$stub" <<'STUB'
#!/usr/bin/env bash
set -u
if [ "${1:-}" != artifact ]; then
  echo 'usage: epr <command>' >&2
  exit 2
fi
verb="${2:-}"
shift 2
case "$verb" in
  describe)
    out=""
    while [ $# -gt 0 ]; do
      case "$1" in --out) out="$2"; shift 2 ;; *) shift ;; esac
    done
    mkdir -p "$(dirname "$out")"
    printf '{"fixture":true}\n' > "$out"
    ;;
  key)
    case "${EPR_STUB_MODE:-equal}" in
      noartifact)
        echo 'usage: epr flow <command>' >&2
        exit 2
        ;;
      dirty)
        printf '{"method":"method-a","input":"input-a","keyCid":"key-a","head":"head","dirty":[" M source"]}\n'
        exit 3
        ;;
      nokey)
        echo 'epr artifact: no fixture key' >&2
        exit 1
        ;;
    esac
    if [[ " $* " == *' --print input '* ]]; then
      echo input-a
      exit 0
    fi
    case "${EPR_STUB_MODE:-equal}" in
      method) method=method-b; input=input-a; key=key-method ;;
      input) method=method-a; input=input-b; key=key-input ;;
      *) method=method-a; input=input-a; key=key-a ;;
    esac
    printf '{"method":"%s","input":"%s","keyCid":"%s","head":"head","dirty":[]}\n' \
      "$method" "$input" "$key"
    ;;
  fetch)
    case "${EPR_STUB_MODE:-equal}" in
      fetchgap|putfail) echo 'fetch gap: no-record: fixture' >&2; exit 3 ;;
    esac
    dest=""
    while [ $# -gt 0 ]; do
      case "$1" in --dest) dest="$2"; shift 2 ;; *) shift ;; esac
    done
    mkdir -p "$(dirname "$dest")"
    printf '#!/usr/bin/env bash\nexit 0\n' > "$dest"
    echo 'fixture fetch'
    ;;
  put)
    if [ "${EPR_STUB_MODE:-equal}" = putfail ]; then
      echo 'unrecorded: the view moved during the build' >&2
      exit 3
    fi
    echo 'fixture put'
    ;;
  *) echo "usage: epr artifact <command>" >&2; exit 2 ;;
esac
STUB
chmod 0755 "$stub"

export EPR_BIN="$stub"
export MESH_DIR="$tmp/mesh"
export MESH_PEERS=unit
export MESH_TRANSPORT_BACKEND=libp2p
export MESH_DOORWAYS=0
export CARGO_HOME="$tmp/cargo-home"
mkdir -p "$CARGO_HOME/bin"
cat > "$CARGO_HOME/bin/cargo" <<'CARGO'
#!/usr/bin/env bash
set -u
bin=""
while [ $# -gt 0 ]; do
  case "$1" in --bin) bin="$2"; shift 2 ;; *) shift ;; esac
done
mkdir -p "$CARGO_TARGET_DIR/debug"
printf '#!/usr/bin/env bash\nexit 0\n' > "$CARGO_TARGET_DIR/debug/$bin"
chmod 0755 "$CARGO_TARGET_DIR/debug/$bin"
CARGO
chmod 0755 "$CARGO_HOME/bin/cargo"

# shellcheck source=../hc-mesh.sh
source "$here/../hc-mesh.sh" >/dev/null
POOL="$tmp/pool"
_storage_mesh="$POOL/elohim__elohim-storage/mesh-bin/elohim-storage"
STORAGE_BIN="$_storage_mesh"
MESH_DOORWAYS_EFFECTIVE=0
mkdir -p "$(dirname "$STORAGE_BIN")"
printf '#!/usr/bin/env bash\nexit 0\n' > "$STORAGE_BIN"
chmod 0755 "$STORAGE_BIN"
method="$(dirname "$STORAGE_BIN")/.method-elohim-storage.json"
record="$(dirname "$STORAGE_BIN")/.derivation-elohim-storage.json"
printf '{"fixture":true}\n' > "$method"

write_record() {
  printf '{"key":"key-a","method":"method-a","input":"input-a","head":"head","via":"built","at":"2026-11-09T00:00:00Z"}\n' > "$record"
}
run_freshness() {
  local out rc
  out="$(preflight_binary_freshness 2>&1)"; rc=$?
  printf '%s\n__RC__=%s\n' "$out" "$rc"
}

write_record
export EPR_STUB_MODE=equal
out="$(run_freshness)"
t "equal key is fresh" '[[ "$out" == *"ok elohim-storage binary matches its closure (key key-a)"* ]] && [[ "$out" == *"__RC__=0"* ]]'

EPR_STUB_MODE=method
out="$(run_freshness)"
t "method change refuses and names the method slot" '[[ "$out" == *"REFUSED elohim-storage binary is STALE"* ]] && [[ "$out" == *"toolchain/flags/env (method) changed"* ]] && [[ "$out" == *"__RC__=1"* ]]'

EPR_STUB_MODE=input
out="$(run_freshness)"
t "input change refuses and names the input slot" '[[ "$out" == *"REFUSED elohim-storage binary is STALE"* ]] && [[ "$out" == *"source view (input) changed"* ]] && [[ "$out" == *"__RC__=1"* ]]'

rm -f "$record"
EPR_STUB_MODE=equal
out="$(run_freshness)"
t "missing record uses the announced mtime fallback" '[[ "$out" == *"ok elohim-storage binary is not older than elohim/elohim-storage/src (mtime rule: no derivation record)"* ]] && [[ "$out" == *"__RC__=0"* ]]'

write_record
EPR_STUB_MODE=noartifact
out="$(run_freshness)"
t "missing artifact verb uses the announced mtime fallback" '[[ "$out" == *"mtime rule: artifact verb unavailable"* ]] && [[ "$out" == *"__RC__=0"* ]]'

EPR_STUB_MODE=dirty
out="$(run_freshness)"
t "dirty closure uses the announced mtime fallback" '[[ "$out" == *"mtime rule: dirty closure"* ]] && [[ "$out" == *"__RC__=0"* ]]'

MESH_ALLOW_STALE_BINARY=1
out="$(run_freshness)"
unset MESH_ALLOW_STALE_BINARY
t "stale override skips closure inspection" '[[ "$out" == *"ok binary freshness: skipped (MESH_ALLOW_STALE_BINARY=1)"* ]] && [[ "$out" == *"__RC__=0"* ]]'

# A fetched install writes the projection from the same canned key JSON.
rm -f "$record" "$method" "$STORAGE_BIN"
EPR_STUB_MODE=fetch mesh_build storage >/dev/null 2>&1; build_rc=$?
t "fetched install writes a derivation projection" \
  '[ "$build_rc" -eq 0 ] && [ -f "$record" ] && grep -q '"'"'"via":"fetched"'"'"' "$record" && grep -q '"'"'"key":"key-a"'"'"' "$record"'

# A successful local install with no key must erase a projection for prior bytes.
printf '{"key":"stale","method":"stale","input":"stale"}\n' > "$record"
EPR_STUB_MODE=nokey mesh_build storage >/dev/null 2>&1; build_rc=$?
t "successful unkeyed install removes a stale derivation projection" \
  '[ "$build_rc" -eq 0 ] && [ -x "$STORAGE_BIN" ] && [ ! -e "$record" ]'

# Review M1: an explicit STORAGE_BIN naming the canonical mesh-slot path keeps the mtime rule, and is
# not refused for a changed closure while its mtime check passes.
write_record
EPR_STUB_MODE=input
STORAGE_BIN_EXPLICIT=1
out="$(run_freshness)"
STORAGE_BIN_EXPLICIT=0
t "explicit canonical-slot STORAGE_BIN keeps the mtime rule" '[[ "$out" == *"ok elohim-storage binary is not older than elohim/elohim-storage/src (mtime rule: explicit binary)"* ]] && [[ "$out" == *"__RC__=0"* ]]'

# No projection = the old mtime rule, including its refusal of a binary older than tracked src.
rm -f "$record"
EPR_STUB_MODE=equal
touch -d '2000-01-01T00:00:00Z' "$STORAGE_BIN"
out="$(run_freshness)"
t "no projection refuses an old binary under the announced mtime rule" '[[ "$out" == *"REFUSED elohim-storage binary is STALE"* ]] && [[ "$out" == *"(mtime rule: no derivation record)"* ]] && [[ "$out" == *"__RC__=1"* ]]'

# A locally built install with a recorded key writes a "built" projection.
rm -f "$record" "$STORAGE_BIN"
EPR_STUB_MODE=fetchgap mesh_build storage >/dev/null 2>&1; build_rc=$?
t "built install with a record writes a built projection" \
  '[ "$build_rc" -eq 0 ] && [ -f "$record" ] && grep -q '"'"'"via":"built"'"'"' "$record"'

# Review M2: when artifact put refuses the build, the pre-build key is not projected.
printf '{"key":"stale","method":"stale","input":"stale"}\n' > "$record"
EPR_STUB_MODE=putfail mesh_build storage >/dev/null 2>&1; build_rc=$?
t "refused record leaves no projection beside the installed bytes" \
  '[ "$build_rc" -eq 0 ] && [ -x "$STORAGE_BIN" ] && [ ! -e "$record" ]'

exit "$fail"
