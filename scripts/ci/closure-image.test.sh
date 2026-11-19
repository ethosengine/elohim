#!/usr/bin/env bash
# Hermetic fixture: git object reads and registry I/O are executable stubs.
# Never invokes a real git, registry, container runtime or compiler.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
T="$(mktemp -d "${TMPDIR:-/tmp}/closure-test.XXXXXX")"
export TMPDIR="$T"
trap 'rm -rf "$T"' EXIT
mkdir -p "$T/bin" "$T/repo/scripts/ci" "$T/repo/elohim/holochain" \
    "$T/repo/elohim/elohim-storage" "$T/repo/elohim/elohim-agent/elohim-agent-sdk/src" \
    "$T/repo/doorway/doorway-service" "$T/repo/source" "$T/repo/declared"
export FIXTURE="$T" CALLS="$T/calls"
export PATH="$T/bin:$PATH"
cp "$ROOT/scripts/ci/closure-input-digest.sh" "$ROOT/scripts/ci/storage-input-digest.sh" \
    "$ROOT/scripts/ci/harbor-oracle.sh" "$ROOT/scripts/ci/build-storage-image.sh" \
    "$ROOT/scripts/ci/build-doorway-app-image.sh" "$ROOT/scripts/ci/build-agent-sdk-image.sh" \
    "$T/repo/scripts/ci/"
# The doubles are bash: the suite calls them ~1,500 times, and a node double paid a
# runtime start-up (~50 ms) on every call. They answer each call exactly as the node
# doubles they replace did (transcript-checked, byte for byte, for every call the suite makes).
cat > "$T/bin/git" <<'SH'
#!/usr/bin/env bash
export LC_ALL=C
printf 'stub git %s\n' "$*" >> "$CALLS"
if [ "${1-}" = merge-base ]; then
  [ "${2-}" = --is-ancestor ] && [ "${3-}" = cccccccccccccccccccccccccccccccccccccccc ] && [ "${4-}" = origin/dev ] || exit 98
  [ "${ORACLE_CASE-}" = off-dev ] && exit 1
  exit 0
fi
if [ "${1-}" = status ]; then [ "${DIRTY-}" = 1 ] && echo ' M source/value'; exit 0; fi
p=''
for a in "$@"; do case "$a" in HEAD:*) p="${a#HEAD:}"; break ;; esac; done
# The bytes the object id is a sha1 of: a file's content as hex; a directory's sorted entries,
# each name followed by its own bytes, joined by newlines.
bytes() {
  if [ -d "$1" ]; then
    local first=1 entry
    shopt -s dotglob nullglob
    for entry in "$1"/*; do
      [ "$first" = 1 ] || printf '\n'
      first=0
      printf '%s' "${entry##*/}"
      bytes "$entry"
    done
  else
    od -An -v -tx1 "$1" | tr -d ' \n'
  fi
}
if [ "${1-}" = rev-parse ]; then
  if [ "$p" = elohim/holochain-conductor ] || [ "$p" = vendor/fork ]; then echo "${PIN:-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"; exit 0; fi
  [ -n "$p" ] && [ -e "$p" ] || exit 1
  sum="$(bytes "$p" | sha1sum)"
  echo "${sum%% *}"
  exit 0
fi
if [ "${1-}" = ls-tree ]; then
  path="${!#}"
  if [ "$path" = elohim/holochain-conductor ] || [ "$path" = vendor/fork ]; then
    printf '160000 commit aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\t%s\n' "$path"
  else
    printf '100644 blob bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\t%s\n' "$path"
  fi
  exit 0
fi
IFS=,; echo "unexpected stub git call: $*" >&2; exit 1
SH
cat > "$T/bin/curl" <<'SH'
#!/usr/bin/env bash
export LC_ALL=C
fail() { echo "$*" >&2; exit 1; }
args=("$@") mode="${ORACLE_CASE:-hit}"
arg_after() { local i; for ((i = 0; i < ${#args[@]}; i++)); do [ "${args[$i]}" = "$1" ] && { printf '%s' "${args[$((i + 1))]-}"; return; }; done; printf '%s' "${args[0]-}"; }
for secret in "${HARBOR_USER-}" "${HARBOR_PASS-}" fixture-token; do
  [ -n "$secret" ] || continue
  for a in "${args[@]}"; do
    if [[ "$a" == *"$secret"* ]]; then printf leaked > "$FIXTURE/secret-in-argv"; fail 'secret in curl argv'; fi
  done
done
config=''
for ((i = 0; i < ${#args[@]}; i++)); do
  [ "${args[$i]}" = --config ] || continue
  i=$((i + 1)); path="${args[$i]}"
  for target in "$path" "$(dirname "$path")"; do
    perm="$(stat -c %a "$target")" || fail "cannot stat $target"
    (( 8#$perm & 8#077 )) && fail 'non-private auth file'
  done
  config+="$(cat "$path"; printf x)"; config="${config%x}"
done
authenticated=0; [[ "$config" == *'Authorization: Bearer fixture-token'* ]] && authenticated=1
url=undefined
for a in "${args[@]}"; do case "$a" in https://*) url="$a"; break ;; esac; done
head=0; for a in "${args[@]}"; do [ "$a" = -I ] && head=1; done
printf 'stub curl %s %s\n' "$([ "$head" = 1 ] && echo HEAD || echo GET)" "$url" >> "$CALLS"
[ "$mode" = unreachable ] && exit 7
out="$(arg_after -o)"
if [[ "$url" == */service/token\?* ]]; then
  [[ "$config" == *"${HARBOR_USER-undefined}:${HARBOR_PASS-undefined}"* ]] || fail 'missing credentials'
  printf '%s' '{"token":"fixture-token"}' > "$out"; exit 0
fi
sha_of() { local sum; sum="$(printf '%s' "$1" | sha256sum)"; printf 'sha256:%s' "${sum%% *}"; }
closure="${KEY-}" revision=cccccccccccccccccccccccccccccccccccccccc
[ "$mode" = unequal ] && closure="sha256:0000000000000000000000000000000000000000000000000000000000000000"
[ "$mode" = bad-revision ] && revision=dev
labels=()
[ "$mode" = missing-label ] || { [ -z "${KEY+set}" ] && [ "$mode" != unequal ]; } || labels+=("\"elohim.host/closure\":\"$closure\"")
[ "$mode" = missing-revision ] || labels+=("\"org.opencontainers.image.revision\":\"$revision\"")
cfg="{\"config\":{\"Labels\":{$(IFS=,; printf '%s' "${labels[*]}")}}}"
[ "$mode" = config-invalid-json ] && cfg='{broken'
if [ "$mode" = missing-config ]; then cfg_ref='{}'; else cfg_ref="{\"digest\":\"$(sha_of "$cfg")\"}"; fi
manifest="{\"schemaVersion\":2,\"mediaType\":\"application/vnd.oci.image.manifest.v1+json\",\"config\":$cfg_ref}"
[ "$mode" = invalid-json ] && manifest='{broken'
amd64=''
[ "$mode" = index-no-amd64 ] || amd64=",{\"digest\":\"$(sha_of "$manifest")\",\"platform\":{\"os\":\"linux\",\"architecture\":\"amd64\"}},{\"digest\":\"sha256:2222222222222222222222222222222222222222222222222222222222222222\",\"platform\":{\"os\":\"linux\",\"architecture\":\"amd64\"}}"
index="{\"schemaVersion\":2,\"mediaType\":\"application/vnd.oci.image.index.v1+json\",\"manifests\":[{\"digest\":\"sha256:1111111111111111111111111111111111111111111111111111111111111111\",\"platform\":{\"os\":\"linux\",\"architecture\":\"arm64\"}}$amd64]}"
body="$manifest"; [[ "$mode" == index* ]] && body="$index"
digest="$(sha_of "$body")"
if [ "$head" = 1 ]; then
  if [ "$mode" = no-digest ]; then : > "$(arg_after -D)"; else printf 'Docker-Content-Digest: %s\r\n' "$digest" > "$(arg_after -D)"; fi
  if [ "$mode" = 404 ]; then printf 404; elif [ "$mode" = auth ] && [ "$authenticated" = 0 ]; then printf 401; else printf 200; fi
  exit 0
fi
[ "$mode" = auth ] && [ "$authenticated" = 0 ] && fail 'GET lacks auth'
blob=0 child=0
[[ "$url" == *"/blobs/$(sha_of "$cfg")" ]] && blob=1
[[ "$url" == *"/manifests/$(sha_of "$manifest")" ]] && [[ "$mode" == index* ]] && child=1
if [ "$blob" = 1 ]; then payload="$cfg"; elif [ "$child" = 1 ]; then payload="$manifest"
elif [[ "$url" == *"/manifests/$digest" ]]; then payload="$body"
else fail 'read was not pinned to an expected digest'; fi
failure=fetch; [ "$child" = 1 ] && failure=index-child; [ "$blob" = 1 ] && failure=config
[ "$mode" = "$failure-unreachable" ] && exit 7
if [ "$mode" = digest-mismatch ] || [ "$mode" = "$failure-digest-mismatch" ]; then printf '%s ' "$payload" > "$out"; else printf '%s' "$payload" > "$out"; fi
if [ "$mode" = "$failure-404" ]; then printf 404; else printf 200; fi
exit 0
SH
cat > "$T/bin/buildctl" <<'STUB'
#!/usr/bin/env bash
printf 'buildctl %s\n' "$*" >> "$CALLS"
STUB
cat > "$T/bin/nerdctl" <<'STUB'
#!/usr/bin/env bash
printf 'nerdctl %s\n' "$*" >> "$CALLS"
STUB
cat > "$T/bin/pnpm" <<'STUB'
#!/usr/bin/env bash
printf 'pnpm %s\n' "$*" >> "$CALLS"
STUB
cat > "$T/bin/rustc" <<'STUB'
#!/usr/bin/env bash
echo 'rustc fixture'
STUB
chmod +x "$T/bin/"*
mkdir -p "$T/repo/doorway/doorway-app/dist/doorway-app/browser" "$T/repo/workspace/package"
cat > "$T/repo/elohim/holochain/build-manifest.json" <<'JSON'
{"steps":{"cargo-build-storage":{"inputs":{"sources":["declared/**"]}},"cargo-build-doorway":{"inputs":{"sources":["declared/**"]}},"build-doorway-app":{"inputs":{"sources":["doorway/doorway-app/**","workspace/package/**","pnpm-lock.yaml"],"buildProcess":["scripts/ci/build-doorway-app-image.sh","scripts/ci/closure-input-digest.sh","scripts/ci/harbor-oracle.sh","elohim/holochain/Jenkinsfile"]}},"build-agent-sdk":{"inputs":{"sources":["elohim/elohim-agent/elohim-agent-sdk/**"],"buildProcess":["scripts/ci/build-agent-sdk-image.sh","scripts/ci/closure-input-digest.sh","scripts/ci/harbor-oracle.sh","elohim/holochain/Jenkinsfile"]}}}}
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
def runDoorwayAppStageBody() {
    sh "bash scripts/ci/build-doorway-app-image.sh"
}
def runAgentSdkStageBody() {
    sh "bash scripts/ci/build-agent-sdk-image.sh"
}
GROOVY
printf 'bash scripts/ci/nested.sh\n' > "$T/repo/scripts/ci/gate.sh"
printf 'true\n' > "$T/repo/scripts/ci/nested.sh"
printf 'COPY source/ /app/\nCOPY --from=builder /app/bin /bin\n' > "$T/repo/elohim/elohim-storage/Dockerfile"
printf 'COPY ["source/", "/app/"]\n' > "$T/repo/doorway/doorway-service/Dockerfile"
printf 'FROM nginx:alpine\nCOPY nginx.conf /etc/nginx/conf.d/default.conf\nCOPY dist/doorway-app/browser /usr/share/nginx/html\n' > "$T/repo/doorway/doorway-app/Dockerfile"
printf 'server {}\n' > "$T/repo/doorway/doorway-app/nginx.conf"
printf '{"name":"doorway-app"}\n' > "$T/repo/doorway/doorway-app/package.json"
printf '{"name":"workspace-package"}\n' > "$T/repo/workspace/package/package.json"
printf 'lockfileVersion: 9\n' > "$T/repo/pnpm-lock.yaml"
printf 'FROM node:24-alpine\nCOPY package.json ./\nCOPY tsconfig.json ./\nCOPY src ./src\n' > "$T/repo/elohim/elohim-agent/elohim-agent-sdk/Dockerfile"
printf '{"name":"@elohim/agent-sdk"}\n' > "$T/repo/elohim/elohim-agent/elohim-agent-sdk/package.json"
printf '{"compilerOptions":{}}\n' > "$T/repo/elohim/elohim-agent/elohim-agent-sdk/tsconfig.json"
printf 'export const sdk = true;\n' > "$T/repo/elohim/elohim-agent/elohim-agent-sdk/src/index.ts"
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
[ "$(key build-doorway-app)" = "$(key build-doorway-app)" ] || fail 'doorway app identical inputs changed key'
app_baseline="$(key build-doorway-app)"
printf 'storage-only\n' > elohim/elohim-storage/unrelated
[ "$app_baseline" = "$(key build-doorway-app)" ] || fail 'storage-only source moved doorway app key'
printf 'doorway-service-only\n' > doorway/doorway-service/unrelated
[ "$app_baseline" = "$(key build-doorway-app)" ] || fail 'doorway-service-only source moved doorway app key'
for input in doorway/doorway-app/package.json workspace/package/package.json pnpm-lock.yaml; do
    cp "$input" "$T/saved"
    printf '\n# mutation\n' >> "$input"
    [ "$app_baseline" != "$(key build-doorway-app)" ] || fail "$input not keyed for build-doorway-app"
    cp "$T/saved" "$input"
done
if DIRTY=1 key build-doorway-app > "$T/dirty-app-out" 2>/dev/null; then fail 'dirty doorway app closure accepted'; fi
[ ! -s "$T/dirty-app-out" ] || fail 'dirty doorway app closure printed a key'
echo 'PASS: build-doorway-app ignores storage/doorway-service and keys app, workspace dependency, lockfile and dirty refusal'
agent_baseline="$(key build-agent-sdk)"
[ "$agent_baseline" = "$(key build-agent-sdk)" ] || fail 'agent SDK identical inputs changed key'
agent_material="$(bash scripts/ci/closure-input-digest.sh build-agent-sdk --material | sha256sum | cut -d ' ' -f1)"
[ "$agent_baseline" = "sha256:$agent_material" ] || fail 'agent SDK material digest differs from key'
printf 'doorway-app-only\n' > doorway/doorway-app/unrelated
[ "$agent_baseline" = "$(key build-agent-sdk)" ] || fail 'doorway-app-only source moved agent SDK key'
for input in elohim/elohim-agent/elohim-agent-sdk/Dockerfile elohim/elohim-agent/elohim-agent-sdk/package.json \
    elohim/elohim-agent/elohim-agent-sdk/tsconfig.json elohim/elohim-agent/elohim-agent-sdk/src/index.ts \
    scripts/ci/build-agent-sdk-image.sh; do
    cp "$input" "$T/saved"
    printf '\n# mutation\n' >> "$input"
    [ "$agent_baseline" != "$(key build-agent-sdk)" ] || fail "$input not keyed for build-agent-sdk"
    cp "$T/saved" "$input"
done
if DIRTY=1 key build-agent-sdk > "$T/dirty-agent-out" 2>/dev/null; then fail 'dirty agent SDK closure accepted'; fi
[ ! -s "$T/dirty-agent-out" ] || fail 'dirty agent SDK closure printed a key'
echo 'PASS: build-agent-sdk ignores unrelated app sources and keys Dockerfile, package, tsconfig, src and dirty refusal'
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
export DOORWAY_APP_TAG="closure-${KEY:7:16}" DOORWAY_APP_CLOSURE="$KEY" DOORWAY_APP_CLOSURE_MATERIAL="$KEY"
: > "$CALLS"
bash scripts/ci/build-doorway-app-image.sh > /dev/null
for expected in 'pnpm install --frozen-lockfile --filter doorway-app...' 'pnpm --filter doorway-app build' \
    "--label elohim.host/closure=$KEY" "--label elohim.host/closure-material=$KEY" \
    "--label org.opencontainers.image.revision=$GIT_COMMIT_HASH" \
    "-t doorway-app:closure-${KEY:7:16}" '-f Dockerfile .'; do
    grep -Fq -- "$expected" "$CALLS" || fail "build-doorway-app missing $expected"
done
echo 'PASS: build-doorway-app workspace build, closure labels and content tag'
export AGENT_SDK_TAG="closure-${KEY:7:16}" AGENT_SDK_CLOSURE="$KEY" AGENT_SDK_CLOSURE_MATERIAL="$KEY"
: > "$CALLS"
bash scripts/ci/build-agent-sdk-image.sh > /dev/null
for expected in "--label elohim.host/closure=$KEY" "--label elohim.host/closure-material=$KEY" \
    "--label org.opencontainers.image.revision=$GIT_COMMIT_HASH" \
    "-t elohim-agent-sdk:closure-${KEY:7:16}" '-f Dockerfile .'; do
    grep -Fq -- "$expected" "$CALLS" || fail "build-agent-sdk missing $expected"
done
echo 'PASS: build-agent-sdk closure labels and content tag'
[ ! -e "$T/secret-in-argv" ] || fail 'secret appeared in curl argv'
echo 'PASS: closure-image suite (no network, real git or cargo)'
