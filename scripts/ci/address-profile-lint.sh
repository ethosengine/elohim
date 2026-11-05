#!/usr/bin/env bash
# address-profile-lint.sh -- durable lint for the address profile (regenesis-1 T0b).
# Refuses, in tracked files outside the embedded allowlist:
#   R1 sha256-<64hex>   R2 blake3-<64hex>
#   R3 a ^bafkrei gate / bafkrei[a-z2-7]{52} regex on a byte surface (.rs .ts .tsx .js .mjs .json .sh),
#      excluding cite/fingerprint/drift/bafyrei (record) lines
#   R4 blob_hash|shard_hash|blobHash|serverBlobHash|blake3Hash identifiers under
#      elohim/ doorway/ app/ steward/ genesis/seeder genesis/a2o scripts/ci
# Exit 1 with a listing and a count; exit 0 when clean.
# Per-line escape: the text `address-lint: allow` on the line.
# Env: ADDRESS_LINT_ROOT (default: repo toplevel), ADDRESS_LINT_ALLOWLIST (file of extra globs, one per line).
# Markdown is allowlisted (prose); OCI `sha256:` digests never match R1.
# File list: `git ls-files` (read-only) when the root is a git repo, else find.
set -u
root="${ADDRESS_LINT_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
cd "$root" || exit 2

ALLOW=(
  'scripts/ci/address-profile-lint.sh'
  'scripts/ci/build-storage-image.sh'
  'scripts/ci/harbor-oracle.sh'
  'elohim/eprfs/epr-cli/src/flow/cites.rs'
  'genesis/data/*' 'genesis/docs/*' 'genesis/research/*' 'genesis/local-dev/*'
  '.eprfs/*' '.claude/*' '.agents/*' '.epr-meta/*/objects/*' 'refs/notes/*'
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

allowed() { local f=$1 g; for g in "${ALLOW[@]}"; do case "$f" in $g) return 0;; esac; done; return 1; }

all=(); bytes=(); scoped=()
while IFS= read -r f; do
  [ -f "$f" ] || continue
  allowed "$f" && continue
  all+=("$f")
  case "$f" in *.rs|*.ts|*.tsx|*.js|*.mjs|*.json|*.sh) bytes+=("$f");; esac
  case "$f" in elohim/*|doorway/*|app/*|steward/*|genesis/seeder/*|genesis/a2o/*|scripts/ci/*) scoped+=("$f");; esac
done <<< "$files"

out=$(mktemp); trap 'rm -f "$out"' EXIT
scan() { # label pattern files...
  local label=$1 pat=$2; shift 2
  [ $# -gt 0 ] || return 0
  printf '%s\0' "$@" | xargs -0 grep -InHP -e "$pat" -- 2>/dev/null \
    | grep -v 'address-lint: allow' | cut -c1-240 | sed "s|^|[$label] |" >> "$out"
}
scan R1-sha256-address 'sha256-[0-9a-fA-F]{64}' "${all[@]}"
scan R2-blake3-address 'blake3-[0-9a-fA-F]{64}' "${all[@]}"
if [ ${#bytes[@]} -gt 0 ]; then
  printf '%s\0' "${bytes[@]}" | xargs -0 grep -InHP -e '(\^bafkrei|bafkrei\[a-z2-7\]|bafkrei[a-z2-7]{52})' -- 2>/dev/null \
    | grep -viE 'cite|fingerprint|drift|bafyrei|address-lint: allow' | cut -c1-240 | sed 's|^|[R3-bafkrei-gate] |' >> "$out"
fi
scan R4-hash-identifier '(blob_hash|shard_hash|blobHash|serverBlobHash|blake3Hash)' "${scoped[@]}"

n=$(wc -l < "$out")
if [ "$n" -gt 0 ]; then
  cat "$out"
  nf=$(sed -E 's/^\[[^]]*\] ([^:]*):.*/\1/' "$out" | sort -u | wc -l)
  echo "address-profile-lint: REFUSED -- $n violation(s) in $nf file(s)"
  exit 1
fi
echo "address-profile-lint: ok (0 violations)"
