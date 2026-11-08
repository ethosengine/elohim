#!/usr/bin/env bash
# address-profile-lint.sh refuses a planted sha256- blob address and passes the
# allowlisted OCI `sha256:` line. Runs on a throwaway non-git tree.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
mkdir -p "$T/scripts/ci" "$T/genesis/seeder/src"
H64="$(printf 'a%.0s' $(seq 64))"
echo "image=harbor/x@sha256:$H64" > "$T/scripts/ci/build-storage-image.sh"
echo "const ok = 'bafkr4i';" > "$T/genesis/seeder/src/clean.ts"
ADDRESS_LINT_ROOT="$T" bash "$HERE/address-profile-lint.sh" --strict >/dev/null || { echo "FAIL clean tree refused"; exit 1; }
echo "ok   clean tree + OCI digest passes"
echo "const a = 'sha256-$H64';" > "$T/genesis/seeder/src/bad.ts"
if ADDRESS_LINT_ROOT="$T" bash "$HERE/address-profile-lint.sh" --strict >/dev/null; then echo "FAIL planted sha256- passed"; exit 1; fi
echo "ok   planted sha256- address refused"
# --- sharpened rules ---
run() { ADDRESS_LINT_ROOT="$T" bash "$HERE/address-profile-lint.sh" --strict >/dev/null; }
rm -f "$T/genesis/seeder/src/bad.ts"
CITE=bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e
mkdir -p "$T/.epr-meta" "$T/elohim/elohim-storage/src" "$T/elohim/brit/x"
echo "{\"policy\":\"$CITE\",\"gate\":\"^bafkrei[a-z2-7]{52}\"}" > "$T/.epr-meta/policy.json"
run || { echo "FAIL citation bafkrei in .epr-meta refused"; exit 1; }
echo "ok   citation bafkrei in .epr-meta passes"
echo "const c = '$CITE';" > "$T/genesis/seeder/src/lit.ts"
run || { echo "FAIL bafkrei literal refused"; exit 1; }
echo "ok   bafkrei literal (citation) passes"
echo "const re = /^bafkrei[a-z2-7]{52}\$/;" > "$T/elohim/elohim-storage/src/gate.ts"
if run; then echo "FAIL byte-surface bafkrei gate passed"; exit 1; fi
echo "ok   byte-surface bafkrei gate refused"
echo "const re = /^bafkrei[a-z2-7]{52}\$/; // address-lint: allow" > "$T/elohim/elohim-storage/src/gate.ts"
run || { echo "FAIL allow escape ignored"; exit 1; }
echo "ok   per-line allow escape honoured"
rm -f "$T/elohim/elohim-storage/src/gate.ts"
echo "const blake3Hasher = hasherFrom(x);" > "$T/genesis/seeder/src/hasher.ts"
run || { echo "FAIL blake3Hasher refused"; exit 1; }
echo "ok   blake3Hasher object passes"
echo "let blob_hash = 1;" > "$T/elohim/elohim-storage/src/id.rs"
if run; then echo "FAIL blob_hash identifier passed"; exit 1; fi
echo "ok   blob_hash identifier refused"
rm -f "$T/elohim/elohim-storage/src/id.rs"
echo "let blob_hash = 1;" > "$T/elohim/brit/x/git.rs"
run || { echo "FAIL git-plane blob_hash refused"; exit 1; }
echo "ok   brit git-plane blob_hash passes"
# --- ratchet mode ---
R="$(mktemp -d)"; trap 'rm -rf "$T" "$R"' EXIT
mkdir -p "$R/scripts/ci" "$R/elohim/elohim-storage/src"
rt() { ADDRESS_LINT_ROOT="$R" bash "$HERE/address-profile-lint.sh" "$@"; }
printf 'let blob_hash = 1;\nlet shard_hash = 2;\n' > "$R/elohim/elohim-storage/src/a.rs"
rt --write-baseline >/dev/null || { echo "FAIL first --write-baseline"; exit 1; }
grep -q '^R4-hash-identifier elohim/elohim-storage/src/a.rs 2$' "$R/scripts/ci/address-profile-lint.baseline" || { echo "FAIL baseline content"; exit 1; }
rt >/dev/null || { echo "FAIL count at baseline refused"; exit 1; }
echo "ok   count at baseline passes"
printf 'let blob_hash = 1;\nlet shard_hash = 2;\nlet blobHash = 3;\n' > "$R/elohim/elohim-storage/src/a.rs"
if rt >/dev/null; then echo "FAIL new hit in baselined file passed"; exit 1; fi
echo "ok   new hit in a baselined file refused"
if rt --write-baseline >/dev/null; then echo "FAIL write-baseline raised without override"; exit 1; fi
echo "ok   --write-baseline refuses to raise without ADDRESS_LINT_RAISE"
ADDRESS_LINT_RAISE=1 rt --write-baseline >/dev/null || { echo "FAIL override refused"; exit 1; }
rt >/dev/null || { echo "FAIL raised baseline refused"; exit 1; }
echo "ok   ADDRESS_LINT_RAISE=1 raises"
printf 'let blob_hash = 1;\n' > "$R/elohim/elohim-storage/src/a.rs"
out=$(rt) || { echo "FAIL count below baseline refused"; exit 1; }
case "$out" in *"paydown available"*) ;; *) echo "FAIL no paydown note"; exit 1;; esac
echo "ok   count below baseline passes with paydown note"
printf 'let blob_hash = 1;\n' > "$R/elohim/elohim-storage/src/b.rs"
if rt >/dev/null; then echo "FAIL new file with hit passed"; exit 1; fi
echo "ok   new file with a hit refused"
rt --strict >/dev/null && { echo "FAIL strict passed with hits"; exit 1; }
echo "ok   --strict stays zero-tolerance"
echo "ALL PASS"
