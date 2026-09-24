#!/usr/bin/env bash
# Hermetic regression test for the content-keyed edge roll (2026-09-22 CI
# wall-clock repair). No cluster, no registry, no network: kubectl / oras /
# curl are stub executables in a temp dir.
#
#   scripts/ci/storage-input-digest.sh     build-input identity of the storage image
#   scripts/ci/storage-workload-image.sh   hold-or-roll for the storage image
#   scripts/ci/pod-inputs-fingerprint.sh   "did anything a pod consumes move?"
#   scripts/ci/happ-roll-key.sh            DNA-pipeline content key for the hApp
#   scripts/ci/conductor-happ-stamp.sh     hold-or-roll for the conductor hApp stamp
#   scripts/ci/fleet-coordswap-dispatch.sh COORDSWAP_RESULT_FILE verdict record
#   elohim/holochain/Jenkinsfile           wiring (no apply-log "unchanged" grep)
#
# Run: bash scripts/ci/content-keyed-roll.test.sh
set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
CI="${REPO_ROOT}/scripts/ci"
T="$(mktemp -d)"
trap 'rm -rf "${T}"' EXIT
fail() { echo "FAIL: $*" >&2; exit 1; }
pass() { echo "ok - $*"; }
D1="sha256:$(printf 'a%.0s' $(seq 64))"
D2="sha256:$(printf 'b%.0s' $(seq 64))"

# ─── 1. storage-input-digest.sh ─────────────────────────────────────────────
R="${T}/repo"
mkdir -p "${R}/elohim/elohim-storage/src" "${R}/crates/a" "${R}/elohim/rakia" "${R}/elohim/holochain-conductor" "${R}/scripts/ci" "${R}/doorway"   # empty dirs = unpopulated submodules, as a CI checkout leaves them
cd "${R}"
git init -q . && git config user.email t@t && git config user.name t
cat > elohim/elohim-storage/Dockerfile <<'EOF'
FROM ${CONDUCTOR_SOURCE_IMAGE} AS conductor-source
COPY elohim/elohim-storage/Cargo.toml ./
COPY crates ./crates
COPY elohim/rakia/schemas /rakia/schemas
COPY elohim/elohim-storage/src ./src
COPY --from=builder /app/target/release/x /usr/local/bin/x
EOF
echo '[package]' > elohim/elohim-storage/Cargo.toml
echo 'fn main(){}' > elohim/elohim-storage/src/main.rs
echo 'a' > crates/a/lib.rs
echo 'build' > scripts/ci/build-storage-image.sh
echo 'doorway' > doorway/main.rs
git add -A
git update-index --add --cacheinfo "160000,$(printf '1%.0s' $(seq 40)),elohim/holochain-conductor"
git update-index --add --cacheinfo "160000,$(printf '2%.0s' $(seq 40)),elohim/rakia"
git commit -qm base
DIG="${CI}/storage-input-digest.sh"

base="$(bash "${DIG}")"
[[ "${base}" =~ ^sha256:[a-f0-9]{64}$ ]] || fail "digest shape: '${base}'"
pass "storage digest has sha256 shape"

echo 'doorway2' > doorway/main.rs && git commit -qam "doorway only"
[ "$(bash "${DIG}")" = "${base}" ] || fail "a doorway-only commit moved the storage digest"
pass "doorway-only commit leaves the storage digest unchanged"

echo 'fn main(){ 1; }' > elohim/elohim-storage/src/main.rs && git commit -qam "storage src"
s1="$(bash "${DIG}")"
[ "${s1}" != "${base}" ] || fail "a storage src change did not move the digest"
pass "storage src change moves the digest"

git update-index --cacheinfo "160000,$(printf '3%.0s' $(seq 40)),elohim/rakia" && git commit -qm "rakia bump"
s2="$(bash "${DIG}")"
[ "${s2}" != "${s1}" ] || fail "a submodule bump under a COPY source did not move the digest"
pass "submodule-contained COPY source resolves to its gitlink"

git update-index --cacheinfo "160000,$(printf '4%.0s' $(seq 40)),elohim/holochain-conductor" && git commit -qm "conductor pin"
s3="$(bash "${DIG}")"
[ "${s3}" != "${s2}" ] || fail "a conductor pin bump did not move the digest"
pass "conductor gitlink is keyed"

echo 'dirty' >> crates/a/lib.rs
set +e; out="$(bash "${DIG}" 2>/dev/null)"; rc=$?; set -e
[ "${rc}" -ne 0 ] && [ -z "${out}" ] || fail "dirty tracked input must refuse (rc=${rc} out='${out}')"
git checkout -q -- crates/a/lib.rs
pass "dirty tracked input refuses (cannot judge)"

echo 'COPY missing/dir ./x' >> elohim/elohim-storage/Dockerfile && git commit -qam "bad copy"
set +e; out="$(bash "${DIG}" 2>/dev/null)"; rc=$?; set -e
[ "${rc}" -ne 0 ] && [ -z "${out}" ] || fail "unresolvable source must refuse"
pass "unresolvable COPY source refuses"
cd "${T}"

# ─── stubs ──────────────────────────────────────────────────────────────────
BIN="${T}/bin"; mkdir -p "${BIN}"
# kubectl stub: answers from files under $KSTATE, logs every call.
cat > "${BIN}/kubectl" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "${KSTATE}/calls"
[ -f "${KSTATE}/fail" ] && exit 1
case "$*" in
  *'elohim-node'*)                         cat "${KSTATE}/live_image" 2>/dev/null ;;
  *'annotations.elohim\.host/storage-inputs'*) cat "${KSTATE}/live_inputs" 2>/dev/null ;;
  *'template.metadata.annotations.elohim\.host/happ-digest'*) cat "${KSTATE}/live_stamp" 2>/dev/null ;;
  *'annotations.elohim\.host/happ-roll-key'*) cat "${KSTATE}/live_key" 2>/dev/null ;;
  get\ StatefulSet/*)                      cat "${KSTATE}/tmpl_$(echo "$2" | cut -d/ -f2)" 2>/dev/null ;;
  get\ ConfigMap/*)                        cat "${KSTATE}/data_$(echo "$2" | cut -d/ -f2)" 2>/dev/null ;;
  annotate*)                               : ;;
esac
exit 0
EOF
# curl stub: manifest HEAD code from $KSTATE/harbor_code; token endpoint.
cat > "${BIN}/curl" <<'EOF'
#!/usr/bin/env bash
echo "curl $*" >> "${KSTATE}/calls"
case "$*" in
  *service/token*) echo '{"token":"tok"}'; exit 0 ;;
  *Bearer*) cat "${KSTATE}/harbor_code_bearer" 2>/dev/null || echo 200; exit 0 ;;
  *) cat "${KSTATE}/harbor_code" 2>/dev/null || echo 200; exit 0 ;;
esac
EOF
chmod +x "${BIN}/kubectl" "${BIN}/curl"
export KUBECTL="${BIN}/kubectl" PATH="${BIN}:${PATH}"
reset() { rm -rf "${T}/k"; mkdir -p "${T}/k"; export KSTATE="${T}/k"; }
GITDIR="${T}/g"; mkdir -p "${GITDIR}"; (cd "${GITDIR}" && git init -q . && git config user.email t@t && git config user.name t && git commit -q --allow-empty -m "plain commit")

# ─── 2. storage-workload-image.sh ───────────────────────────────────────────
SWI="${CI}/storage-workload-image.sh"
LIVE="harbor.ethosengine.com/ethosengine/elohim-storage:1.0.0-dev-old"
NEW="harbor.ethosengine.com/ethosengine/elohim-storage:1.0.0-dev-new"
swi() { (cd "${GITDIR}" && bash "${SWI}" sts ns "${NEW}" "$1" 2>"${T}/err"); }

reset
[ "$(swi "${D1}")" = "${NEW}" ] || fail "first rollout must take the built image"
pass "storage: first rollout takes the built image"

reset; echo "${LIVE}" > "${KSTATE}/live_image"; echo "${D1}" > "${KSTATE}/live_inputs"
[ "$(swi "${D1}")" = "${LIVE}" ] || fail "matching digest must HOLD the live image"
grep -q '⏭️' "${T}/err" || fail "hold must be logged with the skip marker"
pass "storage: inputs unchanged → holds live image, logged"

[ "$(swi "${D2}")" = "${NEW}" ] || fail "digest moved must roll"
pass "storage: inputs changed → rolls"

[ "$(swi '')" = "${NEW}" ] || fail "empty digest (cannot judge) must roll"
pass "storage: empty digest → rolls"

rm "${KSTATE}/live_inputs"
[ "$(swi "${D1}")" = "${NEW}" ] || fail "no recorded digest must roll"
pass "storage: no recorded digest → rolls"

echo "${D1}" > "${KSTATE}/live_inputs"; echo 404 > "${KSTATE}/harbor_code"
[ "$(swi "${D1}")" = "${NEW}" ] || fail "held tag gone from Harbor must roll"
pass "storage: held tag 404 in Harbor → rolls"

echo 401 > "${KSTATE}/harbor_code"; echo 200 > "${KSTATE}/harbor_code_bearer"
[ "$(swi "${D1}")" = "${LIVE}" ] || fail "401 then token 200 must hold"
echo 401 > "${KSTATE}/harbor_code"; echo 404 > "${KSTATE}/harbor_code_bearer"
[ "$(swi "${D1}")" = "${NEW}" ] || fail "401 then token 404 must roll"
echo 503 > "${KSTATE}/harbor_code"
[ "$(swi "${D1}")" = "${LIVE}" ] || fail "inconclusive registry must hold (running pod needs no pull)"
grep -q WARNING "${T}/err" || fail "inconclusive registry must warn"
pass "storage: registry token dance + inconclusive-holds-with-warning"

rm -f "${KSTATE}/harbor_code"
[ "$(STORAGE_ROLL=1 swi "${D1}")" = "${NEW}" ] || fail "STORAGE_ROLL=1 must roll"
(cd "${GITDIR}" && git commit -q --allow-empty -m "please [storage-roll]")
[ "$(swi "${D1}")" = "${NEW}" ] || fail "[storage-roll] must roll"
(cd "${GITDIR}" && git commit -q --allow-empty -m "plain again")
pass "storage: operator overrides roll"

bash "${SWI}" --record sts ns "${D1}"
grep -q "annotate statefulset/sts -n ns elohim.host/storage-inputs=${D1} --overwrite" "${KSTATE}/calls" || fail "record must annotate"
bash "${SWI}" --record sts ns ''
grep -q "annotate statefulset/sts -n ns elohim.host/storage-inputs-" "${KSTATE}/calls" || fail "empty record must remove annotation"
pass "storage: --record writes/removes the object annotation"

# ─── 3. pod-inputs-fingerprint.sh ───────────────────────────────────────────
FP="${CI}/pod-inputs-fingerprint.sh"
M="${T}/m.yaml"
cat > "${M}" <<'EOF'
apiVersion: v1
kind: ConfigMap
metadata:
  name: p-config
  labels:
    app.kubernetes.io/version: "abc"
data:
  x: y
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: p
spec:
  template:
    metadata:
      labels:
        name: nested-name-ignored
---
kind: Service
metadata:
  name: p-svc
EOF
reset
echo '{"spec":{"containers":[{"image":"a"}]}}' > "${KSTATE}/tmpl_p"
echo '{"x":"y"}' > "${KSTATE}/data_p-config"
t1="$(bash "${FP}" "${M}" ns template)"; a1="$(bash "${FP}" "${M}" ns all)"
[[ "${t1}" =~ ^sha256: ]] && [[ "${a1}" =~ ^sha256: ]] || fail "fingerprint shape"
[ "$(bash "${FP}" "${M}" ns template)" = "${t1}" ] || fail "fingerprint must be stable"
grep -q 'get Service' "${KSTATE}/calls" && fail "Services must not be read"
echo '{"x":"z"}' > "${KSTATE}/data_p-config"
[ "$(bash "${FP}" "${M}" ns template)" = "${t1}" ] || fail "template mode must ignore ConfigMap data"
[ "$(bash "${FP}" "${M}" ns all)" != "${a1}" ] || fail "all mode must see ConfigMap data"
echo '{"spec":{"containers":[{"image":"b"}]}}' > "${KSTATE}/tmpl_p"
[ "$(bash "${FP}" "${M}" ns template)" != "${t1}" ] || fail "template change must move the fingerprint"
rm "${KSTATE}/tmpl_p"
[ "$(bash "${FP}" "${M}" ns template)" != "${t1}" ] || fail "an absent workload must differ from a present one"
pass "fingerprint: stable, template vs all, absent objects"
touch "${KSTATE}/fail"
set +e; out="$(bash "${FP}" "${M}" ns all 2>/dev/null)"; rc=$?; set -e
[ "${rc}" -ne 0 ] && [ -z "${out}" ] || fail "kubectl failure must refuse"
rm "${KSTATE}/fail"
printf 'kind: Service\nmetadata:\n  name: s\n' > "${T}/svc.yaml"
set +e; out="$(bash "${FP}" "${T}/svc.yaml" ns all 2>/dev/null)"; rc=$?; set -e
[ "${rc}" -ne 0 ] && [ -z "${out}" ] || fail "manifest without workloads must refuse (never a vacuous 'unchanged')"
pass "fingerprint: kubectl failure / empty parse refuse"

# ─── 4. happ-roll-key.sh ────────────────────────────────────────────────────
HRK="${CI}/happ-roll-key.sh"
H="${T}/h"; mkdir -p "${H}"
printf 'lamad=uhC0kAAA\nmishpat=uhC0kBBB\n' > "${H}/hashes"
printf '# comment\n\nmishpat=uhC0kBBB\n  lamad=uhC0kAAA \n' > "${H}/hashes-reordered"
echo 'roles: []' > "${H}/happ.yaml"
printf 'bundle-1' > "${H}/b1.happ"; printf 'bundle-2' > "${H}/b2.happ"
ok_result() { printf 'SUCCESS sha256:%s\n' "$(sha256sum "$1" | awk '{print $1}')" > "${H}/result"; }
ok_result "${H}/b1.happ"; k1="$(bash "${HRK}" "${H}/hashes" "${H}/happ.yaml" "${H}/b1.happ" "${H}/result" 2>/dev/null)"
ok_result "${H}/b2.happ"; k2="$(bash "${HRK}" "${H}/hashes-reordered" "${H}/happ.yaml" "${H}/b2.happ" "${H}/result" 2>/dev/null)"
[[ "${k1}" == dna-set:sha256:* ]] || fail "SUCCESS must key on the DNA set: ${k1}"
[ "${k1}" = "${k2}" ] || fail "same DNA set, different coordinator bytes, both hot-swapped: keys must match"
pass "roll key: identical DNA set + hot-swap SUCCESS → same key regardless of bundle bytes"

stale="$(bash "${HRK}" "${H}/hashes" "${H}/happ.yaml" "${H}/b1.happ" "${H}/result" 2>/dev/null)"   # result is for b2
[[ "${stale}" == bundle:sha256:* ]] || fail "a result for another bundle must not grant dna-set: ${stale}"
b1="$(bash "${HRK}" "${H}/hashes" "${H}/happ.yaml" "${H}/b1.happ" 2>/dev/null)"
b2="$(bash "${HRK}" "${H}/hashes" "${H}/happ.yaml" "${H}/b2.happ" 2>/dev/null)"
[[ "${b1}" == bundle:* ]] && [ "${b1}" != "${b2}" ] || fail "without hot-swap any byte change must move the key"
printf 'DEFERRED sha256:%s\n' "$(sha256sum "${H}/b1.happ" | awk '{print $1}')" > "${H}/result"
[[ "$(bash "${HRK}" "${H}/hashes" "${H}/happ.yaml" "${H}/b1.happ" "${H}/result" 2>/dev/null)" == bundle:* ]] || fail "DEFERRED must key on bytes"
pass "roll key: no/stale/DEFERRED hot-swap → bundle bytes keyed"

ok_result "${H}/b1.happ"
echo 'roles: [changed]' > "${H}/happ2.yaml"
[ "$(bash "${HRK}" "${H}/hashes" "${H}/happ2.yaml" "${H}/b1.happ" "${H}/result" 2>/dev/null)" != "${k1}" ] || fail "happ.yaml modifiers must be keyed"
printf 'lamad=uhC0kCCC\nmishpat=uhC0kBBB\n' > "${H}/hashes-moved"
[ "$(bash "${HRK}" "${H}/hashes-moved" "${H}/happ.yaml" "${H}/b1.happ" "${H}/result" 2>/dev/null)" != "${k1}" ] || fail "a moved DNA hash must move the key"
printf 'lamad=garbage\n' > "${H}/bad"
set +e; out="$(bash "${HRK}" "${H}/bad" "${H}/happ.yaml" "${H}/b1.happ" 2>/dev/null)"; rc=$?; set -e
[ "${rc}" -ne 0 ] && [ -z "${out}" ] || fail "malformed hashes must refuse"
pass "roll key: happ.yaml + DNA hash moves keyed; malformed refuses"

# ─── 5. conductor-happ-stamp.sh ─────────────────────────────────────────────
CHS="${CI}/conductor-happ-stamp.sh"
cat > "${BIN}/oras" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  *--descriptor*) cat "${KSTATE}/oras_desc" 2>/dev/null ;;
  "manifest fetch "*) cat "${KSTATE}/oras_manifest" 2>/dev/null ;;
esac
EOF
chmod +x "${BIN}/oras"
export ORAS="${BIN}/oras"
KEY_A="dna-set:sha256:$(printf 'c%.0s' $(seq 64))"
KEY_B="dna-set:sha256:$(printf 'd%.0s' $(seq 64))"
manifest_with_key() { printf '{"annotations":{"org.opencontainers.image.created":"x","elohim.host/roll-key":"%s"}}' "$1" > "${KSTATE}/oras_manifest"; }
chs() { (cd "${GITDIR}" && bash "${CHS}" dev-latest sts-conductor ns 2>"${T}/err"); }

reset
set +e; out="$(chs)"; rc=$?; set -e
[ "${rc}" -ne 0 ] && [ -z "${out}" ] || fail "unresolvable digest must exit 1 (Groovy keeps its fail-safe marker)"
pass "hApp stamp: unresolved digest → exit 1"

echo "{\"digest\":\"${D2}\"}" > "${KSTATE}/oras_desc"; manifest_with_key "${KEY_A}"
[ "$(chs)" = "${D2} ${KEY_A}" ] || fail "first rollout must stamp the new digest"
echo "${D1}" > "${KSTATE}/live_stamp"
[ "$(chs)" = "${D2} ${KEY_A}" ] || fail "no recorded key must stamp the new digest"
echo "${KEY_A}" > "${KSTATE}/live_key"
[ "$(chs)" = "${D1} ${KEY_A}" ] || fail "same content key must HOLD the live stamp"
grep -q '⏭️' "${T}/err" || fail "hold must be logged with the skip marker"
pass "hApp stamp: identical content key holds the live stamp across a new OCI digest"

echo "${KEY_B}" > "${KSTATE}/live_key"
[ "$(chs)" = "${D2} ${KEY_A}" ] || fail "content moved must stamp the new digest"
echo "${KEY_A}" > "${KSTATE}/live_key"
echo '{"annotations":{}}' > "${KSTATE}/oras_manifest"
[ "$(chs)" = "${D2} -" ] || fail "artifact without a key must fall back to the manifest digest"
manifest_with_key "${KEY_A}"
[ "$(CONDUCTOR_ROLL=true chs)" = "${D2} ${KEY_A}" ] || fail "CONDUCTOR_ROLL must stamp the new digest"
(cd "${GITDIR}" && git commit -q --allow-empty -m "x [conductor-roll]")
[ "$(chs)" = "${D2} ${KEY_A}" ] || fail "[conductor-roll] must stamp the new digest"
(cd "${GITDIR}" && git commit -q --allow-empty -m "plain")
pass "hApp stamp: moved key / keyless artifact / operator overrides roll"

bash "${CHS}" --record sts-conductor ns "${KEY_A}"
grep -q "annotate statefulset/sts-conductor -n ns elohim.host/happ-roll-key=${KEY_A} --overwrite" "${KSTATE}/calls" || fail "record must annotate"
bash "${CHS}" --record sts-conductor ns -
grep -q "elohim.host/happ-roll-key-" "${KSTATE}/calls" || fail "'-' record must remove"
pass "hApp stamp: --record writes/removes the object annotation"

# ─── 6. fleet-coordswap-dispatch.sh verdict record ──────────────────────────
DD="${T}/dispatch"; mkdir -p "${DD}"
cp "${CI}/fleet-coordswap-dispatch.sh" "${DD}/"
printf 'bundle' > "${DD}/b.happ"; echo '{"humans":[]}' > "${DD}/d.json"
BSHA="sha256:$(sha256sum "${DD}/b.happ" | awk '{print $1}')"
for rc_case in "0 SUCCESS" "4 DEFERRED" "1 INCOMPLETE"; do
  set -- ${rc_case}
  printf '#!/usr/bin/env bash\nexit %s\n' "$1" > "${DD}/fleet-coordswap.sh"
  echo stale > "${DD}/result"
  DEPLOYMENTS_JSON="${DD}/d.json" COORDSWAP_PEERS=a=http://a COORDSWAP_RESULT_FILE="${DD}/result" bash "${DD}/fleet-coordswap-dispatch.sh" "${DD}/b.happ" alpha >/dev/null
  [ "$(cat "${DD}/result")" = "$2 ${BSHA}" ] || fail "driver rc $1 must record '$2 ${BSHA}', got '$(cat "${DD}/result")'"
done
echo stale > "${DD}/result"
COORDSWAP_ENABLE=false COORDSWAP_RESULT_FILE="${DD}/result" bash "${DD}/fleet-coordswap-dispatch.sh" "${DD}/b.happ" alpha >/dev/null
[ ! -s "${DD}/result" ] || fail "a skipped hot-swap must leave an EMPTY result (stale verdicts never survive)"
pass "coordswap dispatch records its verdict + bundle sha; skips truncate"

# ─── 7. Jenkinsfile wiring ──────────────────────────────────────────────────
JF="${REPO_ROOT}/elohim/holochain/Jenkinsfile"
grep -qF 'apps/${stsName}[[:space:]]+unchanged' "${JF}" && fail "conductor CHANGED still keyed on the apply log"
grep -qF 'apps/${humanConfig.resourcePrefix}[[:space:]]+unchanged' "${JF}" && fail "storage restart still keyed on the apply log"
grep -q "podInputsFingerprint(outputFile, humanConfig.namespace, 'template')" "${JF}" || fail "conductor phase must fingerprint its pod template"
grep -q "podInputsFingerprint(outputFile, humanConfig.namespace, 'all')" "${JF}" || fail "storage path must fingerprint pod inputs"
grep -q "scripts/ci/storage-workload-image.sh' --record" "${JF}" || fail "storage path must record its input digest"
grep -q "scripts/ci/conductor-happ-stamp.sh' --record" "${JF}" || fail "conductor path must record its hApp roll key"
grep -q "scripts/ci/happ-roll-key.sh" "${REPO_ROOT}/elohim/holochain/dna/Jenkinsfile" || fail "DNA pipeline must stamp the roll key"
for f in _edgenode-consolidated.template.yaml adam-firstman.yaml _edgenode-conductor.template.yaml adam-firstman-conductor.yaml; do
  awk '/^  template:/{t=1} /^[^ ]/{t=0} t && /DEPLOY_VERSION_PLACEHOLDER|restartedAt:/ && !/^[[:space:]]*#/' \
    "${REPO_ROOT}/genesis/orchestrator/manifests/humans/${f}" | grep -q . && fail "${f}: commit-varying value inside the pod template"
done
pass "Jenkinsfile + manifests wiring"

echo "ALL PASS"
