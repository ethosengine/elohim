#!/usr/bin/env bash
# address-profile-lint.sh -- durable lint for the address profile (regenesis-1 T0b).
# Refuses, in tracked files outside the embedded allowlist:
#   R1 sha256-<64hex>   R2 blake3-<64hex>
#   R3 a bafkrei GATE (^bafkrei, bafkrei[a-z2-7] class) in code on a byte surface (BYTE_SURFACE).
#      `.startsWith('bafkrei')` assertions are not detected: they assert a citation, not admit bytes.
#      A bafkrei LITERAL is a citation-class value (profile 3) and is never refused;
#      .epr-meta governance files are citation surfaces and are skipped.
#   R4 whole-token blob_hash|shard_hash(es)|blobHash|serverBlobHash|blake3Hash identifiers on the
#      blob/shard plane (elohim/ doorway/ app/ steward/ genesis/seeder genesis/a2o scripts/ci).
#      `blake3Hasher` (a hasher object) is not an identifier of this class. Historic SQL migrations
#      and the git-plane brit tree are out of scope (see ALLOW).
# Legitimate and never matched: bafkr4i bytes, bafyrei records, bafkrei/sha256:<16hex> citations,
# OCI `sha256:` image digests.
# Modes: default RATCHET against scripts/ci/address-profile-lint.baseline (`<rule> <path> <count>`,
# sorted): exit 1 when a (rule,path) count exceeds its baseline entry or has none; exit 0 at/under
# it, with a 'paydown available' note when a count fell. --write-baseline regenerates it and refuses
# to raise anything unless ADDRESS_LINT_RAISE=1. --strict is zero-tolerance (exit 1 on any hit).
# Per-line escape: the text `address-lint: allow` on the line.
# Env: ADDRESS_LINT_ROOT (default: repo toplevel), ADDRESS_LINT_ALLOWLIST (file of extra globs, one per line).
# Markdown is allowlisted (prose); OCI `sha256:` digests never match R1.
# File list: `git ls-files` (read-only) when the root is a git repo, else find.
set -u
mode=ratchet
for a in "$@"; do case "$a" in --strict) mode=strict;; --write-baseline) mode=write;; *) echo "usage: $0 [--strict|--write-baseline]" >&2; exit 2;; esac; done
root="${ADDRESS_LINT_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$root" || exit 2

ALLOW=(
  'scripts/ci/address-profile-lint.sh' 'scripts/ci/address-profile-lint.test.sh'  # the lint and its fixtures name the patterns
  'scripts/ci/build-storage-image.sh'
  'scripts/ci/harbor-oracle.sh'
  'elohim/eprfs/epr-cli/src/flow/cites.rs'
  'genesis/data/*' 'genesis/docs/*' 'genesis/research/*' 'genesis/local-dev/*'
  '.eprfs/*' '.claude/*' '.agents/*' '.epr-meta/*/objects/*' 'refs/notes/*'
  # historic SQL migrations are immutable history; the crossing migration itself renames the columns
  'elohim/elohim-storage/migrations/*'
  # brit is the git plane: git blob/tree object hashes are not the byte-address plane
  'elohim/brit/*'
  # .epr-meta bodies are governance prose and citation-class policy objects
  '*.epr-meta' '*/.epr-meta/*'
  '*.md' '*.mdx' '*.lock' '*pnpm-lock.yaml' '*package-lock.json' '*.min.js' '*.map'
)
if [ -n "${ADDRESS_LINT_ALLOWLIST:-}" ] && [ -f "$ADDRESS_LINT_ALLOWLIST" ]; then
  while IFS= read -r g; do [ -n "$g" ] && ALLOW+=("$g"); done < "$ADDRESS_LINT_ALLOWLIST"
fi

if git -C "$root" rev-parse --git-dir >/dev/null 2>&1; then
  files=$(git -C "$root" ls-files --recurse-submodules 2>/dev/null || git -C "$root" ls-files)
else
  files=$(find . -type f ! -path './.git/*' ! -path '*/node_modules/*' | sed 's|^\./||')
fi

# R1/R2 only: tests that assert a retired spelling is REFUSED (the literal is the fixture).
REFUSAL_FIXTURES=(
  'elohim/elohim-storage/src/services/release_adoption/verify.rs'
  'elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs'
  'elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs'
)
fixture() { local f=$1 g; for g in "${REFUSAL_FIXTURES[@]}"; do [ "$f" = "$g" ] && return 0; done; return 1; }
allowed() { local f=$1 g; for g in "${ALLOW[@]}"; do case "$f" in $g) return 0;; esac; done; return 1; }

# R3 byte surfaces: where a blob/shard/release-artifact address is admitted or routed.
# Citation-class code (eprfs, epr, brit, agentic, recall, .epr-meta) is deliberately outside.
BYTE_SURFACE=(
  'elohim/elohim-storage/*' 'elohim/elohim-cache-core/*' 'doorway/doorway-service/*' 'crates/elohim-address/*'
  'elohim/sdk/epr-ts/*' 'elohim/sdk/storage-client-ts/*' 'elohim/sdk/schemas/v1/*' 'genesis/seeder/*'
  'genesis/a2o/steps/*' 'scripts/ci/*' 'elohim/rakia/*' 'app/*/src/*' 'steward/node/src/*'
)
onbyte() { local f=$1 g; for g in "${BYTE_SURFACE[@]}"; do case "$f" in $g) return 0;; esac; done; return 1; }
all=(); addr=(); bytes=(); scoped=()
while IFS= read -r f; do
  [ -f "$f" ] || continue
  allowed "$f" && continue
  all+=("$f"); fixture "$f" || addr+=("$f")
  case "$f" in *.rs|*.ts|*.tsx|*.js|*.mjs|*.json|*.sh) onbyte "$f" && bytes+=("$f");; esac
  case "$f" in elohim/*|doorway/*|app/*|steward/*|genesis/seeder/*|genesis/a2o/*|scripts/ci/*) scoped+=("$f");; esac
done <<< "$files"

out=$(mktemp); trap 'rm -f "$out"' EXIT
scan() { # label pattern files...
  local label=$1 pat=$2; shift 2
  [ $# -gt 0 ] || return 0
  printf '%s\0' "$@" | xargs -0 grep -InHP -e "$pat" -- 2>/dev/null \
    | grep -v 'address-lint: allow' | cut -c1-240 | sed "s|^|[$label] |" >> "$out"
}
scan R1-sha256-address 'sha256-[0-9a-fA-F]{64}' "${addr[@]}"
scan R2-blake3-address 'blake3-[0-9a-fA-F]{64}' "${addr[@]}"
if [ ${#bytes[@]} -gt 0 ]; then
  printf '%s\0' "${bytes[@]}" | xargs -0 grep -InHP -e '(\^bafkrei|bafkrei\[a-z2-7)' -- 2>/dev/null \
    | grep -viE 'cite|fingerprint|drift|bafyrei|address-lint: allow' | cut -c1-240 | sed 's|^|[R3-bafkrei-gate] |' >> "$out"
fi
scan R4-hash-identifier '(?<![A-Za-z0-9])(blob_hash|shard_hash|shard_hashes|blobHash|serverBlobHash|blake3Hash)(?![A-Za-z0-9])' "${scoped[@]}"

export LC_ALL=C
counts=$(sed -E 's/^\[([^]]*)\] ([^:]*):.*/\1 \2/' "$out" | sort | uniq -c | awk '{print $2" "$3" "$1}' | sort)
n=$(wc -l < "$out")
BASE="${ADDRESS_LINT_BASELINE:-$root/scripts/ci/address-profile-lint.baseline}"

if [ "$mode" = strict ]; then
  if [ "$n" -gt 0 ]; then
    cat "$out"
    nf=$(sed -E 's/^\[[^]]*\] ([^:]*):.*/\1/' "$out" | sort -u | wc -l)
    echo "address-profile-lint: REFUSED -- $n violation(s) in $nf file(s)"
    exit 1
  fi
  echo "address-profile-lint: ok (0 violations)"
  exit 0
fi

# Ratchet: compare per-(rule,path) counts with the committed baseline.
base_counts() { [ -f "$BASE" ] && grep -vE '^\s*(#|$)' "$BASE" | sort || true; }
if [ "$mode" = write ]; then
  raised=$(join -j1 -t$'\t' <(base_counts | awk '{print $1"|"$2"\t"$3}') <(printf '%s\n' "$counts" | grep . | awk '{print $1"|"$2"\t"$3}') \
    | awk -F'\t' '$3>$2{print $1" "$2" -> "$3}')
  newk=$(join -v2 -j1 -t$'\t' <(base_counts | awk '{print $1"|"$2"\t"$3}') <(printf '%s\n' "$counts" | grep . | awk '{print $1"|"$2"\t"$3}') | tr '\t' ' ')
  # a baseline that exists only raises when a count grows or a new (rule,path) appears
  if [ -f "$BASE" ] && { [ -n "$raised" ] || [ -n "$newk" ]; } && [ "${ADDRESS_LINT_RAISE:-0}" != 1 ]; then
    echo "address-profile-lint: --write-baseline REFUSED -- would raise the baseline (ADDRESS_LINT_RAISE=1 to override):"
    [ -n "$raised" ] && printf '  raised: %s\n' "$raised"
    [ -n "$newk" ] && printf '  new:    %s\n' "$newk"
    exit 1
  fi
  printf '%s\n' "$counts" | grep . > "$BASE" || : > "$BASE"
  echo "address-profile-lint: baseline written ($(wc -l < "$BASE") entries, $n hit(s))"
  exit 0
fi

bad=0; paydown=0
declare -A base
while read -r r f c; do [ -n "$r" ] && base["$r|$f"]=$c; done < <(base_counts)
declare -A cur
while read -r r f c; do [ -n "$r" ] && cur["$r|$f"]=$c; done <<< "$counts"
for k in "${!cur[@]}"; do
  b=${base[$k]:-0}
  if [ "${cur[$k]}" -gt "$b" ]; then
    bad=1; r=${k%%|*}; f=${k#*|}
    echo "[$r] $f: ${cur[$k]} hit(s), baseline $b"
    grep -F "[$r] $f:" "$out" | head -20
  fi
done
for k in "${!base[@]}"; do
  [ "${cur[$k]:-0}" -lt "${base[$k]}" ] && paydown=$((paydown+1))
done
if [ "$bad" = 1 ]; then
  echo "address-profile-lint: REFUSED -- hits exceed the baseline (new debt); fix them or add a reasoned 'address-lint: allow'"
  exit 1
fi
if [ "$paydown" -gt 0 ]; then
  echo "address-profile-lint: paydown available -- $paydown (rule,path) count(s) below baseline; run --write-baseline to ratchet down"
fi
echo "address-profile-lint: ok (ratchet; $n hit(s) within baseline)"
