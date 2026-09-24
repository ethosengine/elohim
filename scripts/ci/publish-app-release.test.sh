#!/usr/bin/env bash
# Regression coverage for publish-app-release.sh against a real local HTTP
# server (native-delivery sprint Lane N5).
#
#   1. a channel that does not exist is refused (exit 2) naming the steward's
#      one-time `channel create`, and nothing is packaged or published;
#   2. a channel whose current release already carries exactly these artifact
#      digests is CURRENT (exit 0): no packager run, no publish — idempotent by
#      content;
#   3. otherwise the manifest is packaged through the doorway (--put-via
#      doorway, one --app-artifact per (app, kind), lineage parent = the
#      channel's current head) and published at staging over the doorway
#      transport, and the release cid is reported.
#
# The zip recipe and the tsx runner are stubbed at their documented seams
# (BUNDLE_ZIP_LIB, APP_RELEASE_TSX): what is under test is the script's own
# decisions, not the SDK packager or the Node CLIs (each has its own tests).

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/publish-app-release.sh"
TEST_ROOT="$(mktemp -d)"
SERVER_PID=""
CHANNEL="runtime:app-bundle:alpha:dev"

cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

# The doorway: GET /db/content/<id>[/head] serves <routes>/<id>[.head].json, else 404.
mkdir -p "${TEST_ROOT}/routes"
cat > "${TEST_ROOT}/server.py" <<'PY'
import http.server, os, sys, urllib.parse
ROOT = sys.argv[2]
class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        path = urllib.parse.unquote(self.path.split("?")[0])
        name = None
        if path.startswith("/db/content/"):
            rest = path[len("/db/content/"):]
            name = rest[:-len("/head")] + ".head" if rest.endswith("/head") else rest
        file = os.path.join(ROOT, (name or "").replace(":", "_") + ".json")
        if name and os.path.exists(file):
            data = open(file, "rb").read()
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        else:
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[1])), Handler).serve_forever()
PY
PORT=$(( 20000 + RANDOM % 20000 ))
python3 "${TEST_ROOT}/server.py" "${PORT}" "${TEST_ROOT}/routes" &
SERVER_PID=$!
DOORWAY="http://127.0.0.1:${PORT}"
for _ in $(seq 1 50); do
  curl -s -o /dev/null "${DOORWAY}/" && break
  sleep 0.1
done

# The zip recipe, stubbed: the "zip" of a dist is its path's bytes, so each
# (app, kind) has its own stable digest.
cat > "${TEST_ROOT}/bundle-zip-stub.sh" <<'SH'
bundle_zip() {
    mkdir -p "$3"
    BUNDLE_ZIP_ARCHIVE="$3/$2.zip"
    printf 'zip of %s' "$1" > "${BUNDLE_ZIP_ARCHIVE}"
    BUNDLE_ZIP_HASH="sha256-$(sha256sum "${BUNDLE_ZIP_ARCHIVE}" | awk '{print $1}')"
    BUNDLE_ZIP_SIZE="1K"
}
SH

# The tsx runner, stubbed: records every invocation; the packager writes an
# empty manifest to --out, the ceremony answers with a release cid.
cat > "${TEST_ROOT}/tsx" <<SH
#!/bin/bash
echo "\$*" >> "${TEST_ROOT}/tsx.log"
case "\$1" in
  *epr-release-package.ts)
    while [ \$# -gt 0 ]; do [ "\$1" = --out ] && { echo '{}' > "\$2"; }; shift; done ;;
  *release-ceremony.ts)
    echo '{"verb":"publish","transport":"doorway","releaseCid":"uhCkkNewRelease","tier":"staging"}' ;;
esac
SH
chmod +x "${TEST_ROOT}/tsx"

BUNDLES="elohim-host-landing:browser:/dist/landing/browser:/ elohim-host-landing:server:/dist/landing/server:/ lamad-spa:browser:/dist/lamad/browser:/lamad/ lamad-spa:server:/dist/lamad/server:/lamad/"
run() {
  APP_RELEASE_BUNDLES="${BUNDLES}" BUNDLE_ZIP_LIB="${TEST_ROOT}/bundle-zip-stub.sh" \
    APP_RELEASE_TSX="${TEST_ROOT}/tsx" APP_RELEASE_WORK_DIR="${TEST_ROOT}/work" \
    bash "${SCRIPT}" "${DOORWAY}" "${TEST_ROOT}/manifest.json"
}
fail() { echo "FAIL: $*" >&2; exit 1; }

# 1. channel absent → refused, nothing packaged.
rm -f "${TEST_ROOT}/tsx.log"
set +e; out="$(run 2>&1)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "absent channel: expected exit 2, got $rc: $out"
grep -q "channel create ${CHANNEL}" <<<"$out" || fail "absent channel: the refusal names the steward act: $out"
[ ! -e "${TEST_ROOT}/tsx.log" ] || fail "absent channel: nothing may be packaged or published"
echo "ok 1 - an absent channel is refused before anything is packaged"

# The digests this build produces, as the stub zips them.
shas=""
for spec in ${BUNDLES}; do
  IFS=':' read -r _ kind dist _ <<<"${spec}"
  shas="${shas}\"$(printf 'zip of %s' "${dist}" | sha256sum | awk '{print $1}')\","
done
shas="[${shas%,}]"

# 2. the current release carries exactly these digests → CURRENT, no publish.
node -e '
  const shas = JSON.parse(process.argv[1]);
  process.stdout.write(JSON.stringify({ id: process.argv[2], reach: "commons",
    metadata: { kind: "release-manifest", manifest: { artifactClass: "app-bundle",
      artifacts: shas.map(sha256 => ({ sha256 })) } } }));
' "${shas}" "${CHANNEL}" > "${TEST_ROOT}/routes/${CHANNEL//:/_}.json"
echo '{"headActionHash":"uhCkkCurrentRelease"}' > "${TEST_ROOT}/routes/${CHANNEL//:/_}.head.json"
rm -f "${TEST_ROOT}/tsx.log"
out="$(run 2>&1)" || fail "current release: expected exit 0: $out"
grep -q "APP-RELEASE-CURRENT channel=${CHANNEL} release=uhCkkCurrentRelease" <<<"$out" \
  || fail "current release: expected the CURRENT line: $out"
[ ! -e "${TEST_ROOT}/tsx.log" ] || fail "current release: no packager or publish run"
echo "ok 2 - a release carrying exactly these digests is current, and nothing is re-published"

# 3. a different current release → package through the doorway, publish at staging.
node -e '
  process.stdout.write(JSON.stringify({ id: process.argv[1], reach: "commons",
    metadata: { kind: "release-manifest", manifest: { artifactClass: "app-bundle",
      artifacts: [{ sha256: "0".repeat(64) }] } } }));
' "${CHANNEL}" > "${TEST_ROOT}/routes/${CHANNEL//:/_}.json"
rm -f "${TEST_ROOT}/tsx.log"
out="$(run 2>&1)" || fail "new release: expected exit 0: $out"
grep -q "APP-RELEASE-PUBLISHED channel=${CHANNEL} release=uhCkkNewRelease" <<<"$out" \
  || fail "new release: expected the PUBLISHED line: $out"
pkg="$(grep epr-release-package.ts "${TEST_ROOT}/tsx.log")"
for want in "--artifact-class app-bundle" "--put-via doorway" "--peer ${DOORWAY}" \
    "--lineage-parent uhCkkCurrentRelease" "--channel-id ${CHANNEL}" \
    "--app-artifact lamad-spa:server:" "--app-mount lamad-spa=/lamad/" "--soak-secs 60" \
    "--attestation-threshold 1"; do
  grep -q -- "${want}" <<<"${pkg}" || fail "new release: packager argv lacks '${want}': ${pkg}"
done
[ "$(grep -o -- '--app-artifact' <<<"${pkg}" | wc -l)" -eq 4 ] || fail "new release: one --app-artifact per (app, kind)"
grep -q "release-ceremony.ts publish ${TEST_ROOT}/manifest.json --transport doorway --doorway ${DOORWAY}" \
  "${TEST_ROOT}/tsx.log" || fail "new release: published over the doorway transport"
echo "ok 3 - a new release is packaged through ONE doorway and published at staging"

# 4. a channel with no release yet (its head is the channel root) → first release.
echo "{\"id\":\"${CHANNEL}\",\"reach\":\"commons\",\"metadata\":{\"kind\":\"release-channel\"}}" \
  > "${TEST_ROOT}/routes/${CHANNEL//:/_}.json"
rm -f "${TEST_ROOT}/tsx.log"
out="$(run 2>&1)" || fail "first release: expected exit 0: $out"
grep -q -- "--first-release" "${TEST_ROOT}/tsx.log" || fail "first release: packaged with --first-release"
echo "ok 4 - the first release on a channel names no lineage parent"
