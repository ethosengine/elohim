#!/usr/bin/env bash
# Hermetic fixture: git object reads and registry I/O are executable stubs.
# Never invokes a real git, registry, container runtime or compiler.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
T="$(mktemp -d "${TMPDIR:-/tmp}/closure-test.XXXXXX")"
export TMPDIR="$T"
trap 'rm -rf "$T"' EXIT
mkdir -p "$T/bin" "$T/repo/scripts/ci" "$T/repo/elohim/holochain" \
    "$T/repo/elohim/elohim-storage" "$T/repo/doorway/doorway-service" "$T/repo/source" "$T/repo/declared"
export FIXTURE="$T" CALLS="$T/calls"
export PATH="$T/bin:$PATH"
cp "$ROOT/scripts/ci/closure-input-digest.sh" "$ROOT/scripts/ci/storage-input-digest.sh" \
    "$ROOT/scripts/ci/harbor-oracle.sh" "$ROOT/scripts/ci/build-storage-image.sh" "$T/repo/scripts/ci/"
cat > "$T/bin/git" <<'JS'
#!/usr/bin/env node
const fs = require('node:fs'), crypto = require('node:crypto');
const a = process.argv.slice(2);
fs.appendFileSync(process.env.CALLS, `stub git ${a.join(' ')}\n`);
if (a[0] === 'merge-base') {
  if (a[1] !== '--is-ancestor' || a[2] !== 'c'.repeat(40) || a[3] !== 'origin/dev') process.exit(98);
  process.exit(process.env.ORACLE_CASE === 'off-dev' ? 1 : 0);
}
if (a[0] === 'status') { if(process.env.DIRTY === '1') console.log(' M source/value'); process.exit(0); }
const p = (a.find(x => x.startsWith('HEAD:')) || '').slice(5);
if (a[0] === 'rev-parse') {
  if (p === 'elohim/holochain-conductor' || p === 'vendor/fork') { console.log(process.env.PIN || 'a'.repeat(40)); process.exit(0); }
  if (!p || !fs.existsSync(p)) process.exit(1);
  function bytes(path) {
    if (fs.statSync(path).isDirectory()) return fs.readdirSync(path).sort().map(n => n+bytes(`${path}/${n}`)).join('\n');
    return fs.readFileSync(path).toString('hex');
  }
  console.log(crypto.createHash('sha1').update(bytes(p)).digest('hex')); process.exit(0);
}
if (a[0] === 'ls-tree') {
  const path = a.at(-1);
  if (path === 'elohim/holochain-conductor' || path === 'vendor/fork') console.log(`160000 commit ${'a'.repeat(40)}\t${path}`);
  else console.log(`100644 blob ${'b'.repeat(40)}\t${path}`);
  process.exit(0);
}
throw Error(`unexpected stub git call: ${a}`);
JS
cat > "$T/bin/curl" <<'JS'
#!/usr/bin/env node
const fs = require('node:fs'), crypto = require('node:crypto');
const a = process.argv.slice(2), mode = process.env.ORACLE_CASE || 'hit';
for (const secret of [process.env.HARBOR_USER, process.env.HARBOR_PASS, 'fixture-token'].filter(Boolean)) {
  if (a.some(arg => arg.includes(secret))) {
    fs.writeFileSync(`${process.env.FIXTURE}/secret-in-argv`, 'leaked');
    throw Error('secret in curl argv');
  }
}
let curlConfig = '';
for (let i = 0; i < a.length; i++) if (a[i] === '--config') {
  const path = a[++i];
  if ((fs.statSync(path).mode & 0o077) || (fs.statSync(require('node:path').dirname(path)).mode & 0o077)) throw Error('non-private auth file');
  curlConfig += fs.readFileSync(path, 'utf8');
}
const authenticated = curlConfig.includes('Authorization: Bearer fixture-token');
const url = a.find(x => x.startsWith('https://'));
fs.appendFileSync(process.env.CALLS, `stub curl ${a.includes('-I') ? 'HEAD' : 'GET'} ${url}\n`);
if (mode === 'unreachable') process.exit(7);
const out = a[a.indexOf('-o')+1];
if (url.includes('/service/token?')) { if (!curlConfig.includes(process.env.HARBOR_USER + ':' + process.env.HARBOR_PASS)) throw Error('missing credentials'); fs.writeFileSync(out, '{"token":"fixture-token"}'); process.exit(0); }
const labels = {'elohim.host/closure':process.env.KEY, 'org.opencontainers.image.revision':'c'.repeat(40)};
if (mode === 'missing-label') delete labels['elohim.host/closure'];
if (mode === 'missing-revision') delete labels['org.opencontainers.image.revision'];
if (mode === 'unequal') labels['elohim.host/closure'] = `sha256:${'0'.repeat(64)}`;
if (mode === 'bad-revision') labels['org.opencontainers.image.revision'] = 'dev';
const hash = body => `sha256:${crypto.createHash('sha256').update(body).digest('hex')}`;
const config = mode === 'config-invalid-json' ? '{broken' : JSON.stringify({config:{Labels:labels}});
const manifest = mode === 'invalid-json' ? '{broken' : JSON.stringify({schemaVersion:2,
  mediaType:'application/vnd.oci.image.manifest.v1+json',
  config: mode === 'missing-config' ? {} : {digest:hash(config)}});
const index = JSON.stringify({schemaVersion:2, mediaType:'application/vnd.oci.image.index.v1+json', manifests:[
  {digest:`sha256:${'1'.repeat(64)}`,platform:{os:'linux',architecture:'arm64'}},
  ...(mode === 'index-no-amd64' ? [] : [
    {digest:hash(manifest),platform:{os:'linux',architecture:'amd64'}},
    {digest:`sha256:${'2'.repeat(64)}`,platform:{os:'linux',architecture:'amd64'}},
  ]),
]});
const body = mode.startsWith('index') ? index : manifest;
const digest = hash(body);
if (a.includes('-I')) {
  fs.writeFileSync(a[a.indexOf('-D')+1], mode === 'no-digest' ? '' : `Docker-Content-Digest: ${digest}\r\n`);
  process.stdout.write(mode === '404' ? '404' : mode === 'auth' && !authenticated ? '401' : '200');
} else {
  if (mode === 'auth' && !authenticated) throw Error('GET lacks auth');
  const blob = url.endsWith(`/blobs/${hash(config)}`);
  const child = url.endsWith(`/manifests/${hash(manifest)}`) && mode.startsWith('index');
  let payload;
  if (blob) payload = config;
  else if (child) payload = manifest;
  else if (url.endsWith(`/manifests/${digest}`)) payload = body;
  else throw Error('read was not pinned to an expected digest');
  const failure = blob ? 'config' : child ? 'index-child' : 'fetch';
  if (mode === `${failure}-unreachable`) process.exit(7);
  fs.writeFileSync(out, mode === 'digest-mismatch' || mode === `${failure}-digest-mismatch` ? payload+' ' : payload);
  process.stdout.write(mode === `${failure}-404` ? '404' : '200');
}

JS
cat > "$T/bin/buildctl" <<'STUB'
#!/usr/bin/env bash
printf 'buildctl %s\n' "$*" >> "$CALLS"
STUB
cat > "$T/bin/nerdctl" <<'STUB'
#!/usr/bin/env bash
printf 'nerdctl %s\n' "$*" >> "$CALLS"
STUB
cat > "$T/bin/rustc" <<'STUB'
#!/usr/bin/env bash
echo 'rustc fixture'
STUB
chmod +x "$T/bin/"*
cat > "$T/repo/elohim/holochain/build-manifest.json" <<'JSON'
{"steps":{"cargo-build-storage":{"inputs":{"sources":["declared/**"]}},"cargo-build-doorway":{"inputs":{"sources":["declared/**"]}}}}
JSON
cat > "$T/repo/elohim/holochain/Jenkinsfile" <<'GROOVY'
def runQualityGates() {
    sh "bash scripts/ci/gate.sh"
}
def runStorageStageBody() {
    buildStorageImage()
}
def buildStorageImage() {
    sh "bash scripts/ci/build-storage-image.sh"
}
def runDoorwayStageBody() {
    sh "bash scripts/ci/build-storage-image.sh build-doorway"
}
GROOVY
printf 'bash scripts/ci/nested.sh\n' > "$T/repo/scripts/ci/gate.sh"
printf 'true\n' > "$T/repo/scripts/ci/nested.sh"
printf 'COPY source/ /app/\nCOPY --from=builder /app/bin /bin\n' > "$T/repo/elohim/elohim-storage/Dockerfile"
printf 'COPY ["source/", "/app/"]\n' > "$T/repo/doorway/doorway-service/Dockerfile"
printf 'source\n' > "$T/repo/source/value"
printf 'declared\n' > "$T/repo/declared/value"
cd "$T/repo"
fail() { echo "FAIL: $*" >&2; exit 1; }
key() { bash scripts/ci/closure-input-digest.sh "$1"; }
for step in build-storage build-doorway; do
    baseline="$(key "$step")"
    [ "$baseline" = "$(key "$step")" ] || fail 'identical inputs changed key'
    material="$(bash scripts/ci/closure-input-digest.sh "$step" --material | sha256sum | cut -d ' ' -f1)"
    [ "$baseline" = "sha256:$material" ] || fail 'material digest differs from key'
    docker=elohim/elohim-storage/Dockerfile
    [ "$step" = build-storage ] || docker=doorway/doorway-service/Dockerfile
    for input in "$docker" scripts/ci/build-storage-image.sh scripts/ci/closure-input-digest.sh \
        scripts/ci/nested.sh scripts/ci/harbor-oracle.sh scripts/ci/storage-input-digest.sh elohim/holochain/Jenkinsfile declared/value source/value; do
        cp "$input" "$T/saved"
        printf '\n# mutation\n' >> "$input"
        [ "$baseline" != "$(key "$step")" ] || fail "$input not keyed for $step"
        cp "$T/saved" "$input"
    done
    [ "$baseline" != "$(PIN=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb key "$step")" ] || fail 'gitlink not keyed'
    if DIRTY=1 key "$step" > "$T/dirty-out" 2>/dev/null; then fail 'dirty closure accepted'; fi
    [ ! -s "$T/dirty-out" ] || fail 'dirty closure printed a key'
    echo "PASS: $step stable closure, material, COPY/declared inputs, recipes, Jenkinsfile, derivation, conductor pin, dirty refusal"
done
[ "$(key build-storage)" = "$(bash scripts/ci/storage-input-digest.sh)" ] || fail 'wrapper differs'
# Uncloned path inside a gitlink resolves to the superproject pin.
printf 'COPY vendor/fork/schema/ /schema\n' >> elohim/elohim-storage/Dockerfile
key build-storage > /dev/null
printf 'COPY absent-file /missing\n' >> elohim/elohim-storage/Dockerfile
if key build-storage > "$T/missing-out" 2>/dev/null; then fail 'unresolved COPY accepted'; fi
[ ! -s "$T/missing-out" ] || fail 'unresolved COPY printed a key'
echo 'PASS: thin storage wrapper, uncloned gitlink fallback, unresolved COPY refuses'
export HARBOR_USER=fixture-robot HARBOR_PASS=fixture-password
export KEY="sha256:$(printf 'key' | sha256sum | cut -d ' ' -f1)"
for case in hit auth index index-no-amd64 index-child-404 index-child-unreachable index-child-digest-mismatch 404 missing-label missing-revision unequal unreachable off-dev no-digest fetch-404 fetch-unreachable invalid-json bad-revision digest-mismatch missing-config config-404 config-unreachable config-invalid-json config-digest-mismatch; do
    result="$(ORACLE_CASE="$case" bash scripts/ci/harbor-oracle.sh ethosengine/elohim-storage "$KEY")"
    if [ "$case" = hit ] || [ "$case" = auth ] || [ "$case" = index ]; then
        [[ "$result" =~ ^hit\ sha256:[a-f0-9]{64}$ ]] || fail "$case: $result"
    else
        [[ "$result" = miss\ * ]] || fail "$case: $result"
    fi
    case "$case" in
        index-no-amd64) [ "$result" = 'miss invalid-manifest-or-no-amd64' ] || fail "$case: $result" ;;
        missing-label|missing-revision) [ "$result" = 'miss missing-or-unequal-label' ] || fail "$case: $result" ;;
    esac
    echo "PASS: oracle $case -> $result"
done
if bash scripts/ci/harbor-oracle.sh bad invalid > /dev/null; then fail 'invalid key accepted'; else [ "$?" = 2 ] || fail 'usage status'; fi
# Assert the actual builder invocation, with no real BuildKit/container/compiler.
export STORAGE_TAG="closure-${KEY:7:16}" DOORWAY_TAG="closure-${KEY:7:16}"
export STORAGE_CLOSURE="$KEY" DOORWAY_CLOSURE="$KEY" STORAGE_CLOSURE_MATERIAL="$KEY" DOORWAY_CLOSURE_MATERIAL="$KEY"
export GIT_COMMIT_HASH=cccccccccccccccccccccccccccccccccccccccc
for step in build-storage build-doorway; do
    : > "$CALLS"
    bash scripts/ci/build-storage-image.sh "$step" > /dev/null
    image=elohim-storage; dockerfile=elohim/elohim-storage/Dockerfile
    if [ "$step" = build-doorway ]; then image=elohim-doorway; dockerfile=doorway/doorway-service/Dockerfile; fi
    for expected in "--build-arg CACHE_BUST=$KEY" "--label elohim.host/closure=$KEY" \
        "--label elohim.host/closure-material=$KEY" "--label org.opencontainers.image.revision=$GIT_COMMIT_HASH" \
        'nerdctl -n k8s.io build' "-t $image:closure-${KEY:7:16}" "-f $dockerfile ."; do
        grep -Fq -- "$expected" "$CALLS" || fail "$step missing $expected"
    done
    if grep -Eq 'nerdctl.*(push|tag|load)|buildctl.* build |annotation[.]|type=oci' "$CALLS"; then fail 'builder published, aliased or used an OCI archive'; fi
    echo "PASS: $step closure cache bust, nerdctl build and config labels; no archive, push or alias"
done
[ ! -e "$T/secret-in-argv" ] || fail 'secret appeared in curl argv'
echo 'PASS: closure-image suite (no network, real git or cargo)'
