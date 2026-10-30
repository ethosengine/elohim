#!/usr/bin/env bash
# closure-input-digest.sh <build-storage|build-doorway> [--material]
# Run at repo root. stdout: sha256:<hex>, or its canonical material list.
# Exit 0: complete clean closure; exit 1: cannot judge (no key); exit 2: usage.
# HEAD oids describe tracked inputs; dirty inputs refuse. Untracked generated
# outputs and mutable base tags retain the existing digest's accepted gap.
# The material's SHA256 is also the key: salt and step are material entries.
set -euo pipefail
export GIT_OPTIONAL_LOCKS=0
STEP="${1:-}"
case "$STEP" in
    build-storage) DOCKERFILE=elohim/elohim-storage/Dockerfile ;;
    build-doorway) DOCKERFILE=doorway/doorway-service/Dockerfile ;;
    *) echo 'usage: closure-input-digest.sh <build-storage|build-doorway> [--material]' >&2; exit 2 ;;
esac
[ "$#" -le 2 ] && { [ "$#" -eq 1 ] || [ "$2" = --material ]; } || exit 2
die() { echo "closure-input-digest: $*" >&2; exit 1; }

# Node is a builder dependency; deploy consumes the key saved in build.env.
# Follow the gate/build helper calls and their shell recipes transitively.
# The whole Jenkinsfile blob is also a material, so rewiring changes the key.
# Unsupported source syntax refuses rather than silently dropping an input.
closure_sources() {
    node - "$STEP" "$DOCKERFILE" <<'JS'
const fs = require('node:fs');
const step = process.argv[2], dockerfile = process.argv[3];
const manifest = 'elohim/holochain/build-manifest.json';
const jenkins = 'elohim/holochain/Jenkinsfile';
const sources = new Set([manifest, jenkins, dockerfile,
  'scripts/ci/build-storage-image.sh', 'scripts/ci/closure-input-digest.sh',
  'scripts/ci/harbor-oracle.sh', 'scripts/ci/storage-input-digest.sh', 'elohim/holochain-conductor']);
for (const path of ['.dockerignore', `${dockerfile}.dockerignore`]) if (fs.existsSync(path)) sources.add(path);
function add(source) {
  source = source.replace(/^\.\//, '').replace(/\/\*\*$/, '').replace(/\/$/, '');
  if (!source || /[\s*?\[\]$]/.test(source) || source.startsWith('/'))
    throw Error(`unsupported input: ${source}`);
  sources.add(source);
}
const inputs = JSON.parse(fs.readFileSync(manifest)).steps[`cargo-${step}`].inputs;
for (const source of [...inputs.sources, ...(inputs.buildProcess || [])]) add(source);
const docker = fs.readFileSync(dockerfile, 'utf8').replace(/\\\r?\n/g, ' ');
let copies = 0;
for (const line of docker.split('\n')) {
  const copy = line.match(/^\s*COPY\s+(.+)$/i);
  if (!copy || /--from(?:=|\s)/.test(copy[1])) continue;
  const args = copy[1].replace(/--[\w-]+=\S+\s*/g, '').trim();
  const parts = args.startsWith('[') ? JSON.parse(args) : args.split(/\s+/);
  if (parts.length < 2) throw Error('unparsed COPY');
  parts.slice(0, -1).forEach(add); copies++;
}
if (!copies) throw Error('no COPY sources');
const jenkinsText = fs.readFileSync(jenkins, 'utf8');
const defs = new Map([...jenkinsText.matchAll(/^def (\w+)\([^\n]*\) \{\n([\s\S]*?)^\}/gm)].map(m => [m[1], m[2]]));
const helpers = ['runQualityGates', step === 'build-storage' ? 'runStorageStageBody' : 'runDoorwayStageBody'];
const seenHelpers = new Set(), recipeTexts = [];
while (helpers.length) {
  const name = helpers.pop();
  if (seenHelpers.has(name)) continue;
  seenHelpers.add(name);
  const body = defs.get(name);
  if (!body) throw Error(`missing recipe helper ${name}`);
  recipeTexts.push(body);
  for (const call of body.matchAll(/\b(\w+)\(/g)) if (defs.has(call[1])) helpers.push(call[1]);
}
function references(text) {
  // Also finds $CI/foo.sh and $SCRIPT_DIR/foo.sh references in shell tests.
  for (const m of text.replace(/^\s*(?:#|\/\/).*$/gm, '').matchAll(/\b([A-Za-z0-9_.-]+\.sh)\b/g)) {
    const path = `scripts/ci/${m[1]}`;
    if (fs.existsSync(path)) { add(path); pending.push(path); }
  }
}
const pending = [...sources].filter(p => p.endsWith('.sh')), seen = new Set();
recipeTexts.forEach(references);
while (pending.length) {
  const path = pending.pop();
  if (seen.has(path)) continue;
  seen.add(path);
  references(fs.readFileSync(path, 'utf8'));
}
console.log([...sources].sort().join('\n'));
JS
}

# Object id of <path> in HEAD; for a path inside a submodule, the gitlink of
# the nearest enclosing submodule. Empty on failure.
resolve_oid() {
    local path="$1" probe oid
    oid="$(git rev-parse -q --verify "HEAD:${path}" 2>/dev/null || true)"
    if [ -n "${oid}" ]; then
        printf '%s' "${oid}"
        return 0
    fi
    probe="${path}"
    while [ "${probe}" != "${probe%/*}" ]; do
        probe="${probe%/*}"
        # A gitlink shows as mode 160000 / type commit in the parent tree.
        if git ls-tree HEAD -- "${probe}" 2>/dev/null | awk '{print $2}' | grep -qx commit; then
            GITLINK_OF_LAST="${probe}"
            git rev-parse -q --verify "HEAD:${probe}" 2>/dev/null || true
            return 0
        fi
    done
    return 0
}

TMP_OID="$(mktemp)"
trap 'rm -f "${TMP_OID}"' EXIT

SOURCES="$(closure_sources)" || die "cannot enumerate closure"

LINES="salt v2 step ${STEP}"$'\n'
STATUS_PATHS=()
for src in ${SOURCES}; do
    GITLINK_OF_LAST=""
    # Not a command substitution: resolve_oid must set GITLINK_OF_LAST here.
    resolve_oid "${src}" > "${TMP_OID}"
    oid="$(cat "${TMP_OID}")"
    [ -n "${oid}" ] || die "cannot resolve '${src}' in HEAD — refusing to guess"
    LINES+="${src} ${oid}"$'\n'
    if [ -z "${GITLINK_OF_LAST}" ] && git ls-tree HEAD -- "${src}" 2>/dev/null | awk '{print $2}' | grep -qx commit; then
        GITLINK_OF_LAST="${src}"
    fi
    if [ -n "${GITLINK_OF_LAST}" ]; then
        # A pathspec INSIDE a submodule makes `git status` fatal, and an
        # uncloned `update = none` submodule is normal in CI. Judge a gitlink
        # by its checkout instead: absent/uncloned is fine (the build cannot
        # read it either); cloned at a DIFFERENT commit than HEAD pins is dirt.
        if [ -e "${GITLINK_OF_LAST}/.git" ]; then
            checked="$(git -C "${GITLINK_OF_LAST}" rev-parse HEAD 2>/dev/null || true)"
            pinned="$(git rev-parse -q --verify "HEAD:${GITLINK_OF_LAST}" 2>/dev/null || true)"
            [ -n "${checked}" ] && [ "${checked}" = "${pinned}" ] \
                || die "submodule ${GITLINK_OF_LAST} checked out at ${checked:-?}, HEAD pins ${pinned:-?} — cannot judge"
        fi
    else
        STATUS_PATHS+=("${src}")
    fi
done

# The Docker build reads the WORKSPACE, not HEAD. A tracked input edited in
# place (a pipeline step rewriting a source before the build) would make the
# HEAD ids lie, so any such modification voids the digest. Untracked files are
# not considered: build outputs regenerated deterministically from tracked
# sources are the normal case under these paths.
DIRTY="$(git status --porcelain --untracked-files=no -- "${STATUS_PATHS[@]}" 2>&1 || echo '?? status-failed')"
[ -z "${DIRTY}" ] || die "tracked closure inputs modified in the workspace — cannot judge: $(printf '%s' "${DIRTY}" | head -3 | tr '\n' ' ')"

if [ "${2:-}" = '--material' ]; then printf '%s' "${LINES}"; exit 0; fi
printf 'sha256:%s\n' "$(printf '%s' "${LINES}" | sha256sum | awk '{print $1}')"
