#!/usr/bin/env bash
# The eprfs gate's producer leg: offer the release the gate just built and ingested to this
# person's other devices, so a sibling at the same head runs `epr bin follow --tool epr` with no
# local build (backlog steward-device-arrival row 1; native-throughput DoD 1).
#
#   eprfs-produce.sh <tested-head> <tree-clean> <ingest-json>
#
# Called by the root justfile's `_eprfs-ingest-around` in the same shell environment its
# `cargo build` ran in, right after the ingest. For each of epr and bin-launch: `epr artifact
# describe` (the bin-build view as sources, the environment recorded as it is, the declared target
# from pool-policy.json declared_targets when one names this host and workspace), `epr artifact key`,
# then `epr artifact put --release <ingested release> --release-bin <bin> --cross` of the gate's
# own bytes in $CARGO_TARGET_DIR/debug. No second build: the method records the environment the
# gate's build ran under, and `put` re-checks that this process's environment is that record.
# Under a declared target the build is replayed once with --message-format=json (a no-op over its
# fresh units) and put is handed those messages as --build-messages, the evidence of the C level.
#
# A put never fails the gate. Every refusal prints one line
#   [gate] epr producer put skipped (<reason>; gate remains green)
# and exits 0. EPRFS_PRODUCER=off turns the leg off. EPR_PRODUCER_BIN overrides the driving epr
# (default: the epr this gate built).
#
# Before any describe, the C the build compiled is checked against the target's declared level
# (pool-policy.json declared_targets, backlog steward-device-arrival row 29.2: a v3-default gcc put
# AVX2 into ring's C and a v2 sibling died with SIGILL running the drawn epr). A finding prints
#   [gate] epr producer put withheld (<first finding>; gate remains green)
# and exits 0: these bytes are kept from siblings, and the device's own gate outcome is unchanged.
# EPRFS_C_BASELINE_CHECK overrides the checker (default: c-baseline.mjs beside this script); an
# override prints one `[gate] epr producer C baseline check overridden: ...` line. The reason is
# the checker's stdout; its stderr goes to the gate log.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
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

withhold() {
  echo "[gate] epr producer put withheld ($1; gate remains green)"
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

# The triple the build compiled for, from the compiler that built it (`rustc -vV`'s host line, the
# triple describe records): one source, so the record looked up below is the build's own and an
# x86_64 musl or gnux32 build never reads the gnu record (E12 L2a rework, ruling 3).
host_triple="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')"
[[ -n "$host_triple" ]] || withhold "rustc -vV names no host triple, so the build's target is unknown"

# The C level, in this same environment the build ran in. A missing checker withholds too: an
# offer whose C level nobody read is the one that trapped on shem.
check="${EPRFS_C_BASELINE_CHECK:-$here/c-baseline.mjs}"
# An override decides what gates an offer, so the gate log names it, before the check runs.
[[ -n "${EPRFS_C_BASELINE_CHECK:-}" ]] \
  && echo "[gate] epr producer C baseline check overridden: EPRFS_C_BASELINE_CHECK=$check (not $here/c-baseline.mjs)"
[[ -f "$check" ]] || withhold "the C baseline check $check is not there"
# The finding is the checker's stdout alone; its stderr (a node warning, a trace) goes to the gate
# log as it is, never into the withheld reason.
said="$(node "$check" --triple "$host_triple")"
status=$?
if ((status != 0)); then
  finding="$(reason_of "$said")"
  withhold "${finding:-the C baseline check exited $status}"
fi

# The declared target, read from the record at this checkout (pool-policy.json declared_targets,
# row 29.2) for the build's triple and this workspace, never written here: describe takes it as
# --target-isa / --target-cflags, and `put` checks the recorded CFLAGS_<triple> and the compile
# commands of the build that produced the bytes (the --build-messages below) against it before it
# writes the env:isa-c marker (E12 L2a). No
# record for this host or workspace declares nothing, and put then writes no marker. A record that
# names this host and workspace but cannot be read whole withholds: an offer whose target nobody
# could read is the one that trapped on shem.
policy="genesis/agentic/pool-policy.json"
said="$(python3 -c 'import json, os, sys
path, triple, workspace = sys.argv[1:4]
if not os.path.exists(path):
    sys.exit(0)
try:
    with open(path, encoding="utf-8") as f:
        record = json.load(f).get("declared_targets") or {}
except Exception:
    print(f"{sys.argv[4]} cannot be read, so its declared target is unknown")
    sys.exit(2)
target = record.get(triple) if isinstance(record, dict) and triple else None
if not isinstance(target, dict) or workspace not in (target.get("workspaces") or []):
    sys.exit(0)
for field in ("isa", "cflags"):
    if not isinstance(target.get(field), str) or not target[field].strip():
        print(f"declared target {triple} in {sys.argv[4]} has no {field}")
        sys.exit(2)
print(target["isa"])
print(target["cflags"])' "$repo/$policy" "$host_triple" elohim/eprfs "$policy")" \
  || withhold "${said:-$policy could not be read}"
declared=()
if [[ -n "$said" ]]; then
  mapfile -t isa_cflags <<<"$said"
  declared=(--target-isa "${isa_cflags[0]}" --target-cflags "${isa_cflags[1]}")
fi

# The environment as the build saw it. `describe` records PATH, RUSTFLAGS (given) and the native
# toolchain variables itself; every other build variable set here is recorded as is, so the
# method states what the bytes were built under and `put`'s environment check holds by
# construction. The benign set mirrors epr-cli's BENIGN_BUILD_ENV. Of the C family only the
# declared target's triple-suffixed flags are recorded, as --env beside the declared target, so
# put compares the one with the other; the checker refuses the rest.
recorded=()
while IFS= read -r name; do
  case "$name" in
    CARGO_HOME | RUSTUP_HOME | CARGO_TARGET_DIR | CARGO_BUILD_JOBS | RUST_TEST_THREADS | RUST_BACKTRACE | RUST_LOG) ;;
    RUSTFLAGS) ;;
    CARGO_* | RUST*) recorded+=(--env "$name=${!name}") ;;
    CFLAGS_x86_64_unknown_linux_gnu | CXXFLAGS_x86_64_unknown_linux_gnu) recorded+=(--env "$name=${!name}") ;;
  esac
done < <(compgen -e | LC_ALL=C sort)

out="$target/producer"
mkdir -p "$out" || skip "$out cannot be created"
for bin in "${bins[@]}"; do
  [[ -f "$target/debug/$bin" && -f "$target/debug/$bin.d" ]] \
    || skip "$target/debug/$bin or its dep-info was not built"
done

# The build's own evidence of its C level (E12 L2a rework, ruling 1): under a declared target, the
# gate's build is replayed with the ingest build's package arguments and --message-format=json, a
# no-op over its fresh units that still names each bin's executable and every build script's
# OUT_DIR, beside which cargo keeps cc's `running:` log (the record's CC_ENABLE_DEBUG_OUTPUT). put
# binds the messages to the bytes by the executable and judges every compile command in them, and
# stamps env:isa-c only on that evidence. A replay that fails withholds: these bytes have no
# evidence then.
build_args=(-p elohim-epr-cli --bin epr -p elohim-device-runtime --bin bin-launch --features elohim-epr-cli/blob-store)
evidence=()
if ((${#declared[@]})); then
  messages="$out/build-messages.json"
  (cd "$repo/elohim/eprfs" && cargo build "${build_args[@]}" --message-format=json) \
    >"$messages" 2>"$out/build-messages.err" \
    || withhold "the build's --message-format=json replay failed: $(reason_of "$(cat "$out/build-messages.err")")"
  evidence=(--build-messages "$messages")
fi

declare -A input
for bin in "${bins[@]}"; do
  method="$out/method-$bin.json"
  said="$("$epr" artifact describe --project eprfs --bin "$bin" --features blob-store --rustflags "${RUSTFLAGS-}" \
    --sources "$sources" --crate "${crate[$bin]}" --repo "$repo" --out "$method" \
    "${declared[@]}" "${recorded[@]}" 2>&1)" || skip "describe $bin refused: $(reason_of "$said")"
  said="$("$epr" artifact key --method "$method" --repo "$repo" --print input 2>"$out/key-$bin.err")" \
    || skip "key $bin refused: $(reason_of "$said
$(cat "$out/key-$bin.err")")"
  input[$bin]="$(tail -n 1 <<<"$said")"
done

declare -A output
for bin in "${bins[@]}"; do
  said="$(timeout 600 "$epr" artifact put --method "$out/method-$bin.json" --input "${input[$bin]}" \
    --bytes "$target/debug/$bin" --project eprfs --commit "$tested_head" \
    --release "$release" --release-bin "$bin" "${evidence[@]}" --cross --repo "$repo" --json \
    2>"$out/put-$bin.err")" \
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
