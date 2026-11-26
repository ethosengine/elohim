#!/usr/bin/env bash
# recovery_write_record carries its source identity (F2): commit, dirty flag, binary
# identity — each "unmeasured" with a reason when it cannot be read. No mesh is touched.
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fail=0
t() { if eval "$2"; then echo "ok   $1"; else echo "FAIL $1"; fail=1; fi; }
tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
field() { python3 -c 'import json,sys;r=json.loads(open(sys.argv[1]).read().splitlines()[-1]);print(json.dumps(r.get(sys.argv[2])))' "$1" "$2"; }

(
  unset STORAGE_BIN
  set +e; RECOVERY_SOURCE_ONLY=1 source "$here/../hc-mesh-recovery.sh"; set +e
  out="$tmp/a.jsonl"
  recovery_write_record "$out" warm jessica matthew iroh libp2p 1 42 3 "" '{}' null null per-peer mesh-wide ""
  head="$(git -C "$REPO_ROOT" rev-parse HEAD)"
  t "source_commit is HEAD of REPO_ROOT" '[ "$(field "$out" source_commit)" = "\"$head\"" ]'
  t "source_dirty is a boolean" 'case "$(field "$out" source_dirty)" in true|false) true;; *) false;; esac'
  t "no STORAGE_BIN -> binary_identity unmeasured with reason" \
    '[ "$(field "$out" binary_identity)" = "\"unmeasured\"" ] && field "$out" binary_identity_reason | grep -q "STORAGE_BIN unset"'
  t "existing fields keep their names" '[ "$(field "$out" time_to_recover_s)" = 42 ] && [ "$(field "$out" zome_path)" = "\"unknown\"" ]'

  mkdir -p "$tmp/bin"; STORAGE_BIN="$tmp/bin/elohim-storage"
  recovery_write_record "$out" warm jessica matthew iroh libp2p 1 42 3 "" '{}' null null per-peer mesh-wide ""
  t "missing derivation record -> unmeasured naming the path" \
    'field "$out" binary_identity_reason | grep -q "no derivation record"'
  echo '{"key":"bafykey123","method":"m","input":"i"}' > "$tmp/bin/.derivation-elohim-storage.json"
  recovery_write_record "$out" warm jessica matthew iroh libp2p 1 42 3 "" '{}' null null per-peer mesh-wide ""
  t "derivation record -> binary_identity is its key" '[ "$(field "$out" binary_identity)" = "\"bafykey123\"" ]'

  REPO_ROOT="$tmp/not-a-repo"; mkdir -p "$REPO_ROOT"
  recovery_write_record "$out" warm jessica matthew iroh libp2p 1 42 3 "" '{}' null null per-peer mesh-wide ""
  t "no git -> source_commit and source_dirty unmeasured" \
    '[ "$(field "$out" source_commit)" = "\"unmeasured\"" ] && [ "$(field "$out" source_dirty)" = "\"unmeasured\"" ]'
  t "four records, one line each" '[ "$(wc -l < "$out")" -eq 4 ]'
  exit "$fail"
) || fail=1
exit "$fail"
