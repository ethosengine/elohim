#!/usr/bin/env bash
# The eprfs gate's producer leg: offer the release the gate just built and ingested to this
# person's other devices, so a sibling at the same head runs `epr bin follow --tool epr` with no
# local build (backlog steward-device-arrival row 1; native-throughput DoD 1).
#
#   eprfs-produce.sh <tested-head> <tree-clean> <ingest-json>
#
# Called by the root justfile's `_eprfs-ingest-around` in the same shell environment its
# `cargo build` ran in, right after the ingest. For each of epr and bin-launch: `epr artifact
# describe` (the bin-build view as sources, the environment recorded as it is), `epr artifact key`,
# then `epr artifact put --release <ingested release> --release-bin <bin> --cross` of the gate's
# own bytes in $CARGO_TARGET_DIR/debug. No second build: the method records the environment the
# gate's build ran under, and `put` re-checks that this process's environment is that record.
#
# A put never fails the gate. Every refusal prints one line
#   [gate] epr producer put skipped (<reason>; gate remains green)
# and exits 0. EPRFS_PRODUCER=off turns the leg off. EPR_PRODUCER_BIN overrides the driving epr
# (default: the epr this gate built).
set -uo pipefail

tested_head="${1:-}"
tree_clean="${2:-}"
ingest_json="${3:-}"
repo="$(pwd -P)"
target="${CARGO_TARGET_DIR:-}"
epr="${EPR_PRODUCER_BIN:-$target/debug/epr}"
sources="elohim/eprfs/bin-build-view.json#bin-build"
bins=(epr bin-launch)
declare -A crate=([epr]=elohim/eprfs/epr-cli [bin-launch]=elohim/eprfs/device-runtime)

skip() {
  echo "[gate] epr producer put skipped ($1; gate remains green)"
  exit 0
}

# The first line of a refusal that says something: the not-enforced gap notes come first.
reason_of() {
  grep -v -e ': pending, not enforced$' -e '^reach: ' -e '^[[:space:]]*$' <<<"$1" | head -n 1 | cut -c1-240
}

json_field() { # json_field <json> <python expression over `d`>
  python3 -c 'import json,sys
try:
    d = json.loads(sys.argv[1])
    v = eval(sys.argv[2], {}, {"d": d})
except Exception:
    v = None
print(v if isinstance(v, str) else "")' "$1" "$2"
}

[[ "${EPRFS_PRODUCER:-on}" == off ]] && skip "EPRFS_PRODUCER=off"
[[ -n "$target" ]] || skip "CARGO_TARGET_DIR is not set"
[[ "$tree_clean" == true ]] || skip "epr's source closure was not clean across the build"
head="$(git -C "$repo" rev-parse HEAD 2>/dev/null)" || skip "HEAD does not resolve"
[[ -n "$tested_head" && "$head" == "$tested_head" ]] \
  || skip "HEAD ${head:0:12} is not the tested head ${tested_head:0:12}"
release="$(json_field "$ingest_json" 'd.get("cid") or d.get("rcid")')"
[[ -n "$release" ]] || skip "no release was ingested"
[[ -x "$epr" ]] || skip "$epr is not executable"
[[ -f "$repo/${sources%%#*}" ]] || skip "${sources%%#*} is not in the checkout"

# The environment as the build saw it. `describe` records PATH, RUSTFLAGS (given) and the native
# toolchain variables itself; every other build variable set here is recorded as is, so the
# method states what the bytes were built under and `put`'s environment check holds by
# construction. The benign set mirrors epr-cli's BENIGN_BUILD_ENV.
recorded=()
while IFS= read -r name; do
  case "$name" in
    CARGO_HOME | RUSTUP_HOME | CARGO_TARGET_DIR | CARGO_BUILD_JOBS | RUST_TEST_THREADS | RUST_BACKTRACE | RUST_LOG) ;;
    RUSTFLAGS) ;;
    CARGO_* | RUST*) recorded+=(--env "$name=${!name}") ;;
  esac
done < <(compgen -e | LC_ALL=C sort)

out="$target/producer"
mkdir -p "$out" || skip "$out cannot be created"
declare -A input
for bin in "${bins[@]}"; do
  [[ -f "$target/debug/$bin" && -f "$target/debug/$bin.d" ]] \
    || skip "$target/debug/$bin or its dep-info was not built"
  method="$out/method-$bin.json"
  said="$("$epr" artifact describe --project eprfs --bin "$bin" --rustflags "${RUSTFLAGS-}" \
    --sources "$sources" --crate "${crate[$bin]}" --repo "$repo" --out "$method" \
    "${recorded[@]}" 2>&1)" || skip "describe $bin refused: $(reason_of "$said")"
  said="$("$epr" artifact key --method "$method" --repo "$repo" --print input 2>"$out/key-$bin.err")" \
    || skip "key $bin refused: $(reason_of "$said
$(cat "$out/key-$bin.err")")"
  input[$bin]="$(tail -n 1 <<<"$said")"
done

declare -A output
for bin in "${bins[@]}"; do
  said="$(timeout 600 "$epr" artifact put --method "$out/method-$bin.json" --input "${input[$bin]}" \
    --bytes "$target/debug/$bin" --project eprfs --commit "$tested_head" \
    --release "$release" --release-bin "$bin" --cross --repo "$repo" --json 2>"$out/put-$bin.err")" \
    || skip "put $bin refused: $(reason_of "$said
$(cat "$out/put-$bin.err")")"
  printf '%s\n' "$said" >"$out/put-$bin.json"
  output[$bin]="$(json_field "$said" 'd.get("output")')"
  ingested="$(json_field "$ingest_json" "d.get(\"bins\", {}).get(\"$bin\")")"
  if [[ -n "$ingested" && "${output[$bin]}" != "$ingested" ]]; then
    skip "put $bin recorded ${output[$bin]:-no output}, not the ingested $ingested"
  fi
  [[ "$(json_field "$said" '"yes" if d.get("crosses") else ""')" == yes ]] \
    || skip "put $bin was recorded but does not cross"
done
echo "[gate] epr producer put: release $release at ${tested_head:0:12}, epr ${output[epr]}, bin-launch ${output[bin-launch]} (crosses)"
