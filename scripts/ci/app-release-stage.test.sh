#!/usr/bin/env bash
# Regression coverage for app-release-stage.sh — the App pipeline's one native
# delivery step (native-delivery N6 fleet leg).
#
#   1. publish PUBLISHES and every peer adopts → exit 0 and
#      `APP-RELEASE-STAGE released=<cid> adopted=matthew,jessica`;
#   2. publish reports CURRENT → exit 0, and adoption of the CURRENT release is
#      still measured (a rebuild re-proves a lagging peer);
#   3. a peer has not adopted inside the bound → exit 3 (delivered, not yet
#      proven — UNSTABLE, never 0 and never the hard 1), naming it pending;
#   4. a peer refuses app_bundle_cannot_boot → exit 1 (the Jenkinsfile's hard
#      FAILURE), `refused=app_bundle_cannot_boot`;
#   5. publish refuses → exit 2, the summary names publish's own refusal line;
#   6. usage;
#   7-10. APP_RELEASE_CHANNEL_CREATE=1 (the [app:channel-create] steward act):
#      404 → the ceremony creates the channel, binds the unbound slugs, then
#      publish runs; 200 + every slug bound → no create, no bind; unset + 404 →
#      no create, publish's own refusal; a failed create → exit 2
#      `refused=channel-create-failed`, nothing published;
#   11. a bind the ceremony refuses → exit 2 `refused=channel-bind-failed`;
#   12-13. the branch gate: a non-dev branch is skipped green with no publish;
#      dev proceeds;
#   14. a bind that never reads back as elected → exit 2 channel-bind-not-elected;
#   15. the script's own failure → exit 2 internal-error (exit 1 = cannot-boot only);
#   16. the existence probe answered 503 catching-up, then 404 → re-probed, and the
#      create proceeds;
#   17. the existence probe answered a plain 503 → exit 2 channel-create-failed,
#      no create;
#   18. a half-created channel (row 200, /head 404) → the ceremony's create runs
#      (it resumes the notarize) and APP-RELEASE-CHANNEL-RESUMED, then publish.
#   19-21. the ceremony's catching-up budget ran out (its last line reads `still
#      catching-up after <n>s`) on the create, the bind, or the publish → exit 5
#      `not-ready=upstream-catching-up`, a deploy intent (elect one, phase release)
#      the orchestrator re-dispatches on, nothing (further) published — App #1732.
#
# publish-app-release.sh and verify-app-adoption.sh are stubbed at their
# documented seams (APP_RELEASE_PUBLISH_SCRIPT, APP_RELEASE_VERIFY_SCRIPT), and
# the ceremony at APP_RELEASE_TSX: what is under test is this script's
# composition, not theirs (each has its own tests).

set -euo pipefail

REPO_ROOT="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
SCRIPT="${REPO_ROOT}/scripts/ci/app-release-stage.sh"
TEST_ROOT="$(mktemp -d)"
SERVER_PID=""
cleanup() {
  [ -n "${SERVER_PID}" ] && kill "${SERVER_PID}" 2>/dev/null || true
  rm -rf "${TEST_ROOT}"
}
trap cleanup EXIT

fail() { echo "not ok - $*" >&2; exit 1; }

# The publish stub: STUB_PUBLISH selects the answer.
cat > "${TEST_ROOT}/publish.sh" <<'SH'
echo "publish $*" >> "${STUB_LOG}"
echo "  · elohim-host-landing (browser): sha256-aaa (1K)"
case "${STUB_PUBLISH}" in
    published) echo "APP-RELEASE-PUBLISHED channel=runtime:app-bundle:alpha:dev release=bafyRelease1" ;;
    current)   echo "APP-RELEASE-CURRENT channel=runtime:app-bundle:alpha:dev release=uhCkkCurrent1" ;;
    catching-up)
        echo 'release-ceremony: publish: PATCH http://d/db/content/runtime:app-bundle:alpha:dev returned 503: {"status":"catching-up","retryAfter":2,"cause":"upstream"} — still catching-up after 300.4s (budget 300s, 150 retries)' >&2
        echo "APP-RELEASE-REFUSED reason=publish channel=runtime:app-bundle:alpha:dev — see the ceremony's output above" >&2; exit 2 ;;
    refused)   echo "APP-RELEASE-REFUSED reason=channel-absent channel=runtime:app-bundle:alpha:dev — a steward runs 'release-ceremony.ts channel create runtime:app-bundle:alpha:dev' once" >&2; exit 2 ;;
esac
SH

# The verify stub: STUB_VERIFY selects each peer's answer.
cat > "${TEST_ROOT}/verify.sh" <<'SH'
echo "verify $*" >> "${STUB_LOG}"
cid="$2"
shift 2
case "${STUB_VERIFY}" in
    adopted) for p in "$@"; do echo "APP-ADOPTED ${p} release=${cid}"; done; exit 0 ;;
    pending)
        echo "APP-ADOPTED matthew release=${cid}"
        echo "APP-NOT-ADOPTED jessica state=waiting reason=soak"
        exit 3 ;;
    unbound)
        echo "APP-ADOPTED matthew release=${cid}"
        echo "APP-NOT-ADOPTED jessica state=refused reason=app_slug_not_bound_to_channel"
        exit 3 ;;
    cannot-boot)
        echo "APP-CANNOT-BOOT matthew detail=main.js 404"
        exit 1 ;;
esac
SH

# The doorway: GET /db/content/<channel> answers 200 once <root>/channel-exists
# exists; GET /db/content/<channel>/head names a headActionHash once
# <root>/channel-notarized exists (else 404, as storage answers a row with no
# notarized head); GET /db/content/<slug> serves <root>/rows/<slug>.json; else
# 404. A channel read answers 503 catching-up while <root>/catching-up holds a
# count above 0 (each answer decrements it), and a plain 503 while
# <root>/plain-503 exists.
mkdir -p "${TEST_ROOT}/rows"
cat > "${TEST_ROOT}/doorway.mjs" <<'JS'
import http from 'node:http';
import fs from 'node:fs';
const [port, root] = process.argv.slice(2);
http.createServer((req, res) => {
  const id = decodeURIComponent(req.url).replace(/^\/db\/content\//, '');
  const row = `${root}/rows/${id}.json`;
  const json = (status, value) => {
    res.writeHead(status, { 'content-type': 'application/json' });
    res.end(JSON.stringify(value));
  };
  const countFile = `${root}/catching-up`;
  const pending = fs.existsSync(countFile) ? Number(fs.readFileSync(countFile, 'utf8')) : 0;
  if (id.startsWith('runtime:') && pending > 0) {
    fs.writeFileSync(countFile, String(pending - 1));
    return json(503, { status: 'catching-up', retryAfter: 1, cause: 'upstream' });
  }
  if (id.startsWith('runtime:') && fs.existsSync(`${root}/plain-503`)) {
    return json(503, { error: 'Conductor bridge unavailable' });
  }
  if (id.startsWith('runtime:') && id.endsWith('/head')) {
    return fs.existsSync(`${root}/channel-notarized`)
      ? json(200, { headActionHash: 'uhCkkChannelHead' })
      : json(404, { error: 'no notarized head declared for this content' });
  }
  if (id.startsWith('runtime:') && fs.existsSync(`${root}/channel-exists`)) {
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end('{}');
  } else if (!id.startsWith('runtime:') && fs.existsSync(row)) {
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(fs.readFileSync(row));
  } else {
    res.writeHead(404, { 'content-length': 0 });
    res.end();
  }
}).listen(Number(port), '127.0.0.1');
JS
PORT=$(( 20000 + RANDOM % 20000 ))
node "${TEST_ROOT}/doorway.mjs" "${PORT}" "${TEST_ROOT}" &
SERVER_PID=$!
DOORWAY="http://127.0.0.1:${PORT}"
for _ in $(seq 1 50); do
  curl -s -o /dev/null "${DOORWAY}/" && break
  sleep 0.1
done

# The ceremony: logs its argv; `channel create` makes the channel exist, `channel
# bind <slug> <channel>` writes the slug's row naming the channel (read-your-write).
write_tsx_stub() {
  cat > "${TEST_ROOT}/tsx.sh" <<'SH'
#!/bin/bash
echo "tsx $*" >> "${STUB_LOG}"
case "$*" in
  *" channel create "*)
    : > "${STUB_ROOT}/channel-exists"; : > "${STUB_ROOT}/channel-notarized"
    echo '{"verb":"channel create"}' ;;
  *" channel bind "*)
    set -- $*; shift 3
    printf '{"id":"%s","metadata":{"releaseChannel":"%s"}}' "$1" "$2" > "${STUB_ROOT}/rows/$1.json"
    echo '{"verb":"channel bind","tier":"staging"}' ;;
esac
SH
  chmod +x "${TEST_ROOT}/tsx.sh"
}
write_tsx_stub

BUNDLES="elohim-host-landing:browser:/d/a/browser:/ elohim-host-landing:server:/d/a/server:/ lamad-spa:browser:/d/l/browser:/lamad/ lamad-spa:server:/d/l/server:/lamad/"

run() {
    STUB_LOG="${TEST_ROOT}/stub.log" STUB_ROOT="${TEST_ROOT}" \
    APP_RELEASE_TSX="${TEST_ROOT}/tsx.sh" \
    APP_RELEASE_CHANNEL_VISIBLE_SECS="${VISIBLE_SECS:-5}" APP_RELEASE_READBACK_POLL_SECS="${POLL_SECS:-0}" \
    APP_RELEASE_BUNDLES="${BUNDLES}" \
    APP_RELEASE_PUBLISH_SCRIPT="${TEST_ROOT}/publish.sh" \
    APP_RELEASE_VERIFY_SCRIPT="${TEST_ROOT}/verify.sh" \
    APP_RELEASE_STAGE_OUT="${TEST_ROOT}/stage.out" \
        bash "${SCRIPT}" "${DOORWAY}" "${TEST_ROOT}/manifest.json" matthew,jessica
}
line_of() { grep -n '' "${TEST_ROOT}/stub.log" | grep -m1 -- "$1" | cut -d: -f1; }

# 1. published, both adopt.
rm -f "${TEST_ROOT}/stub.log"
set +e; out="$(STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "adopted: expected exit 0, got $rc: $out"
[ "$(tail -n1 <<<"$out")" = "APP-RELEASE-STAGE released=bafyRelease1 adopted=matthew,jessica" ] \
  || fail "adopted: expected the released/adopted summary last: $out"
grep -qx "verify ${DOORWAY} bafyRelease1 matthew jessica" "${TEST_ROOT}/stub.log" \
  || fail "adopted: verify is asked for the published cid on each peer: $(cat "${TEST_ROOT}/stub.log")"
grep -qx "publish ${DOORWAY} ${TEST_ROOT}/manifest.json" "${TEST_ROOT}/stub.log" \
  || fail "adopted: publish goes through the ONE doorway named"
[ "$(grep -c '' "${TEST_ROOT}/stage.out")" -eq 4 ] && grep -q '^APP-ADOPTED jessica ' "${TEST_ROOT}/stage.out" \
  || fail "adopted: APP_RELEASE_STAGE_OUT carries exactly the APP-* lines: $(cat "${TEST_ROOT}/stage.out")"
! grep -q '^tsx ' "${TEST_ROOT}/stub.log" || fail "adopted: no steward act without APP_RELEASE_CHANNEL_CREATE"
echo "ok 1 - a published release every peer adopts is delivered (exit 0)"

# 2. current → nothing published, adoption of the CURRENT release still measured.
rm -f "${TEST_ROOT}/stub.log"
set +e; out="$(STUB_PUBLISH=current STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "current: expected exit 0, got $rc: $out"
grep -qx "verify ${DOORWAY} uhCkkCurrent1 matthew jessica" "${TEST_ROOT}/stub.log" \
  || fail "current: the current release's adoption is measured: $(cat "${TEST_ROOT}/stub.log")"
[ "$(tail -n1 <<<"$out")" = "APP-RELEASE-STAGE released=uhCkkCurrent1 adopted=matthew,jessica" ] \
  || fail "current: expected the adopted summary for the current release: $out"
echo "ok 2 - an already-current release publishes nothing, and its adoption is re-proven (exit 0)"

# 2b. current, a peer still lagging → exit 3, not a silent green.
set +e; out="$(STUB_PUBLISH=current STUB_VERIFY=pending run)"; rc=$?; set -e
[ "$rc" -eq 3 ] || fail "current+pending: expected exit 3, got $rc: $out"
echo "ok 2b - a rebuild of a current release still names the lagging peer (exit 3)"

# 3. a peer pending past the bound → exit 3, exactly.
set +e; out="$(STUB_PUBLISH=published STUB_VERIFY=pending run)"; rc=$?; set -e
[ "$rc" -eq 3 ] || fail "pending: expected exit 3 (not 0, not 1), got $rc: $out"
[ "$(tail -n1 <<<"$out")" = "APP-RELEASE-STAGE released=bafyRelease1 adopted=matthew pending=jessica" ] \
  || fail "pending: expected the pending summary: $out"
echo "ok 3 - a peer not adopted inside the bound is delivered-not-proven (exit 3)"

# 3b. a peer refused as not bound → exit 3, and the summary names the cause.
set +e; out="$(STUB_PUBLISH=published STUB_VERIFY=unbound run)"; rc=$?; set -e
[ "$rc" -eq 3 ] || fail "unbound: expected exit 3, got $rc: $out"
[ "$(tail -n1 <<<"$out")" = "APP-RELEASE-STAGE released=bafyRelease1 adopted=matthew pending=jessica bind-not-visible=jessica" ] \
  || fail "unbound: expected the bind-not-visible summary: $out"
echo "ok 3b - a peer refusing as not bound is named bind-not-visible in the summary (exit 3)"

# 4. cannot boot → exit 1.
set +e; out="$(STUB_PUBLISH=published STUB_VERIFY=cannot-boot run)"; rc=$?; set -e
[ "$rc" -eq 1 ] || fail "cannot-boot: expected exit 1, got $rc: $out"
[ "$(tail -n1 <<<"$out")" = "APP-RELEASE-STAGE refused=app_bundle_cannot_boot" ] \
  || fail "cannot-boot: expected the refusal summary: $out"
grep -q '^APP-CANNOT-BOOT matthew ' <<<"$out" || fail "cannot-boot: the peer's detail line is passed through"
echo "ok 4 - a release a peer cannot boot is refused (exit 1)"

# 5. publish refuses → exit 2, naming publish's own last line.
rm -f "${TEST_ROOT}/stub.log"
set +e; out="$(STUB_PUBLISH=refused STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "publish refused: expected exit 2, got $rc: $out"
grep -q "^APP-RELEASE-STAGE refused=APP-RELEASE-REFUSED reason=channel-absent .*channel create runtime:app-bundle:alpha:dev" <<<"$(tail -n1 <<<"$out")" \
  || fail "publish refused: the summary names publish's refusal line: $out"
! grep -q '^verify ' "${TEST_ROOT}/stub.log" || fail "publish refused: no adoption measure"
! grep -q '^tsx ' "${TEST_ROOT}/stub.log" || fail "publish refused: nothing is created without the steward act"
echo "ok 5 - a refused publish is refused (exit 2) with publish's reason (and no create without the steward act)"

# 6. usage.
set +e; bash "${SCRIPT}" "${DOORWAY}" "${TEST_ROOT}/m.json" >/dev/null 2>&1; rc=$?; set -e
[ "$rc" -eq 64 ] || fail "usage: expected exit 64, got $rc"
echo "ok 6 - a missing peer list is a usage error (exit 64)"

# 7. steward act on an absent channel with unbound slugs → create, bind each slug once, then publish.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-exists" "${TEST_ROOT}/channel-notarized" "${TEST_ROOT}"/rows/*.json
echo '{"id":"elohim-host-landing","metadata":{}}' > "${TEST_ROOT}/rows/elohim-host-landing.json"
echo '{"id":"lamad-spa","metadata":{"releaseChannel":"runtime:app-bundle:other:x"}}' > "${TEST_ROOT}/rows/lamad-spa.json"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "create+bind: expected exit 0, got $rc: $out"
grep -q "release-ceremony.ts channel create runtime:app-bundle:alpha:dev --reach commons --transport doorway --doorway ${DOORWAY}\$" \
  "${TEST_ROOT}/stub.log" || fail "create+bind: the ceremony's channel create ran over the doorway: $(cat "${TEST_ROOT}/stub.log")"
for slug in elohim-host-landing lamad-spa; do
  [ "$(grep -c "channel bind ${slug} runtime:app-bundle:alpha:dev --transport doorway --doorway ${DOORWAY}\$" "${TEST_ROOT}/stub.log")" -eq 1 ] \
    || fail "create+bind: ${slug} is bound exactly once over the doorway: $(cat "${TEST_ROOT}/stub.log")"
done
[ "$(line_of 'channel create')" -lt "$(line_of 'channel bind')" ] \
  && [ "$(line_of 'channel bind lamad-spa')" -lt "$(line_of '^[0-9]*:publish ')" ] \
  || fail "create+bind: create, then bind, then publish: $(cat "${TEST_ROOT}/stub.log")"
grep -qx "APP-RELEASE-CHANNEL-CREATED channel=runtime:app-bundle:alpha:dev" <<<"$out" \
  || fail "create+bind: expected the CREATED line: $out"
grep -qx "APP-RELEASE-CHANNEL-BOUND slugs=elohim-host-landing,lamad-spa" <<<"$out" \
  || fail "create+bind: expected the BOUND line: $out"
echo "ok 7 - the steward act creates an absent channel, binds every unbound slug, then publishes"

# 8. steward act, channel exists, every slug bound → no create, no bind.
rm -f "${TEST_ROOT}/stub.log"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "exists+bound: expected exit 0, got $rc: $out"
! grep -q '^tsx ' "${TEST_ROOT}/stub.log" || fail "exists+bound: nothing is re-created or re-bound: $(cat "${TEST_ROOT}/stub.log")"
grep -qx "APP-RELEASE-CHANNEL-EXISTS channel=runtime:app-bundle:alpha:dev" <<<"$out" \
  || fail "exists+bound: expected the EXISTS line: $out"
grep -qx "APP-RELEASE-CHANNEL-ALREADY-BOUND slugs=elohim-host-landing,lamad-spa" <<<"$out" \
  || fail "exists+bound: expected the ALREADY-BOUND line: $out"
echo "ok 8 - the steward act on an existing, bound channel creates and binds nothing"

# 9. no steward act + 404 → no create; publish's own absent-channel refusal stands.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-exists" "${TEST_ROOT}/channel-notarized"
set +e; out="$(STUB_PUBLISH=refused STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "no-create+404: expected exit 2, got $rc: $out"
! grep -q '^tsx ' "${TEST_ROOT}/stub.log" || fail "no-create+404: nothing is created without the steward act"
grep -q "^APP-RELEASE-STAGE refused=APP-RELEASE-REFUSED reason=channel-absent" <<<"$(tail -n1 <<<"$out")" \
  || fail "no-create+404: publish's own refusal names the absent channel: $out"
echo "ok 9 - without the steward act an absent channel is publish's own refusal"

# 10. the ceremony's create fails → exit 2, channel-create-failed, nothing bound or published.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-exists" "${TEST_ROOT}/channel-notarized"
printf '#!/bin/bash\necho "tsx $*" >> "${STUB_LOG}"\necho "release-ceremony: POST /db/content/bulk returned 503" >&2\nexit 1\n' > "${TEST_ROOT}/tsx.sh"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "create failed: expected exit 2, got $rc: $out"
grep -q "^APP-RELEASE-STAGE refused=channel-create-failed channel=runtime:app-bundle:alpha:dev exit=1 release-ceremony: POST /db/content/bulk returned 503" \
  <<<"$(tail -n1 <<<"$out")" || fail "create failed: expected the channel-create-failed summary: $out"
! grep -q -e '^publish ' -e 'channel bind' "${TEST_ROOT}/stub.log" || fail "create failed: nothing is bound or published"
echo "ok 10 - a failed channel create refuses (exit 2) before anything is bound or published"

# 11. the ceremony's bind fails → exit 2, channel-bind-failed, nothing published.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}"/rows/*.json; : > "${TEST_ROOT}/channel-exists"; : > "${TEST_ROOT}/channel-notarized"
printf '#!/bin/bash\necho "tsx $*" >> "${STUB_LOG}"\necho "channel bind: no content elohim-host-landing behind the doorway" >&2\nexit 1\n' > "${TEST_ROOT}/tsx.sh"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "bind failed: expected exit 2, got $rc: $out"
grep -q "^APP-RELEASE-STAGE refused=channel-bind-failed channel=runtime:app-bundle:alpha:dev slug=elohim-host-landing exit=1 " \
  <<<"$(tail -n1 <<<"$out")" || fail "bind failed: expected the channel-bind-failed summary: $out"
! grep -q '^publish ' "${TEST_ROOT}/stub.log" || fail "bind failed: nothing is published"
echo "ok 11 - a failed bind refuses (exit 2) before anything is published"
write_tsx_stub

# 12. branch gate: a non-dev branch has no channel → skipped green, nothing runs.
rm -f "${TEST_ROOT}/stub.log"
set +e; out="$(APP_RELEASE_BRANCH=main APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "branch main: expected exit 0, got $rc: $out"
[ "$(tail -n1 <<<"$out")" = "APP-RELEASE-STAGE skipped=branch-main-has-no-channel" ] \
  || fail "branch main: expected the skipped summary: $out"
[ ! -s "${TEST_ROOT}/stub.log" ] || fail "branch main: nothing is created, bound, published or measured: $(cat "${TEST_ROOT}/stub.log")"
grep -qx "APP-RELEASE-STAGE skipped=branch-main-has-no-channel" "${TEST_ROOT}/stage.out" \
  || fail "branch main: the skip reaches APP_RELEASE_STAGE_OUT"
echo "ok 12 - a non-dev branch is skipped green with no publish"

# 13. branch gate: dev proceeds.
rm -f "${TEST_ROOT}/stub.log"
set +e; out="$(APP_RELEASE_BRANCH=dev STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] && grep -q '^publish ' "${TEST_ROOT}/stub.log" || fail "branch dev: expected a publish, exit 0; got $rc: $out"
echo "ok 13 - the dev branch publishes on its channel"

# 14. a bind that exits 0 but never READS BACK naming the channel (a staging bind
#     out-voted by an earned head) → exit 2 channel-bind-not-elected, nothing published.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}"/rows/*.json; : > "${TEST_ROOT}/channel-exists"; : > "${TEST_ROOT}/channel-notarized"
echo '{"id":"elohim-host-landing","metadata":{}}' > "${TEST_ROOT}/rows/elohim-host-landing.json"
printf '#!/bin/bash\necho "tsx $*" >> "${STUB_LOG}"\necho "{\\"verb\\":\\"channel bind\\",\\"tier\\":\\"staging\\"}"\n' > "${TEST_ROOT}/tsx.sh"
set +e; out="$(VISIBLE_SECS=2 POLL_SECS=1 APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "not elected: expected exit 2, got $rc: $out"
grep -q "^APP-RELEASE-STAGE refused=channel-bind-not-elected channel=runtime:app-bundle:alpha:dev slug=elohim-host-landing " \
  <<<"$(tail -n1 <<<"$out")" || fail "not elected: expected the channel-bind-not-elected summary: $out"
! grep -q '^publish ' "${TEST_ROOT}/stub.log" || fail "not elected: nothing is published"
echo "ok 14 - a bind that does not read back as elected refuses (exit 2) before the first release"
write_tsx_stub

# 15. this script's own failure is exit 2 internal-error — exit 1 is reserved for
#     a peer's cannot-boot verdict.
set +e; out="$(APP_RELEASE_STAGE_OUT=/nonexistent-dir/stage.out bash "${SCRIPT}" "${DOORWAY}" "${TEST_ROOT}/m.json" matthew 2>/dev/null)"; rc=$?; set -e
[ "$rc" -eq 2 ] || fail "internal error: expected exit 2, got $rc: $out"
grep -q "^APP-RELEASE-STAGE refused=internal-error line=[0-9]" <<<"$out" || fail "internal error: expected the internal-error line: $out"
echo "ok 15 - an internal failure is exit 2 (internal-error), never the cannot-boot exit 1"

# 16. the existence probe answered catching-up twice (a doorway just restarted),
#     then 404 → re-probed within the bound, and the create proceeds.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-exists" "${TEST_ROOT}/channel-notarized" "${TEST_ROOT}"/rows/*.json
for slug in elohim-host-landing lamad-spa; do
  printf '{"id":"%s","metadata":{"releaseChannel":"runtime:app-bundle:alpha:dev"}}' "${slug}" > "${TEST_ROOT}/rows/${slug}.json"
done
echo 2 > "${TEST_ROOT}/catching-up"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run 2>"${TEST_ROOT}/err")"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "catching-up then 404: expected exit 0, got $rc: $out $(cat "${TEST_ROOT}/err")"
[ "$(grep -c '503 catching-up — re-probing' "${TEST_ROOT}/err")" -eq 2 ] \
  || fail "catching-up then 404: expected two re-probe lines: $(cat "${TEST_ROOT}/err")"
grep -q ' channel create runtime:app-bundle:alpha:dev ' "${TEST_ROOT}/stub.log" \
  || fail "catching-up then 404: the create ran: $(cat "${TEST_ROOT}/stub.log")"
grep -qx "APP-RELEASE-CHANNEL-CREATED channel=runtime:app-bundle:alpha:dev" <<<"$out" \
  || fail "catching-up then 404: expected the CREATED line: $out"
rm -f "${TEST_ROOT}/catching-up"
echo "ok 16 - a catching-up existence probe is re-probed, and a 404 after it creates the channel"

# 17. a plain 503 (no catching-up body) on the existence probe is final.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-exists" "${TEST_ROOT}/channel-notarized"
: > "${TEST_ROOT}/plain-503"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run 2>"${TEST_ROOT}/err")"; rc=$?; set -e
rm -f "${TEST_ROOT}/plain-503"
[ "$rc" -eq 2 ] || fail "plain 503: expected exit 2, got $rc: $out"
grep -q "^APP-RELEASE-STAGE refused=channel-create-failed channel=runtime:app-bundle:alpha:dev status=503 " \
  <<<"$(tail -n1 <<<"$out")" || fail "plain 503: expected the channel-create-failed summary: $out"
! grep -q 're-probing' "${TEST_ROOT}/err" || fail "plain 503: no re-probe: $(cat "${TEST_ROOT}/err")"
[ ! -s "${TEST_ROOT}/stub.log" ] || fail "plain 503: nothing is created or published: $(cat "${TEST_ROOT}/stub.log")"
echo "ok 17 - a plain 503 on the existence probe refuses (exit 2) without a create"

# 18. a half-created channel (App #1731: the row landed, the notarize did not) →
#     the ceremony's create runs to resume it, RESUMED, then publish.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-notarized"
: > "${TEST_ROOT}/channel-exists"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 0 ] || fail "half-created: expected exit 0, got $rc: $out"
grep -q "release-ceremony.ts channel create runtime:app-bundle:alpha:dev --reach commons --transport doorway --doorway ${DOORWAY}\$" \
  "${TEST_ROOT}/stub.log" || fail "half-created: the ceremony's create ran to resume it: $(cat "${TEST_ROOT}/stub.log")"
grep -qx "APP-RELEASE-CHANNEL-RESUMED channel=runtime:app-bundle:alpha:dev" <<<"$out" \
  || fail "half-created: expected the RESUMED line: $out"
! grep -q '^APP-RELEASE-CHANNEL-EXISTS ' <<<"$out" || fail "half-created: a half-created channel is not EXISTS: $out"
[ "$(line_of 'channel create')" -lt "$(line_of '^[0-9]*:publish ')" ] \
  || fail "half-created: resume, then publish: $(cat "${TEST_ROOT}/stub.log")"
echo "ok 18 - a half-created channel is resumed (notarized), not skipped as existing"

# 19-21. the ceremony's catching-up budget spent → exit 5 NOT READY, an intent, no publish.
CATCHING='release-ceremony: channel create: PATCH http://d/db/content/runtime:app-bundle:alpha:dev returned 503: {"status":"catching-up","retryAfter":7,"cause":"upstream","circuit":"closed","errorStreak":0} — still catching-up after 120.3s (budget 120s, 60 retries)'
INTENT="${TEST_ROOT}/deploy-intent.json"
COMMIT_SHA=7d27b04c788302827c4f65fad34802e5952875aa
defer_run() {
    DEPLOY_INTENT_OUT="${INTENT}" DEPLOY_INTENT_COMMIT="${COMMIT_SHA}" \
    DEPLOY_INTENT_DOORWAYS="${DOORWAY}/ https://doorway.elohim.host" run
}
assert_deferred() {  # <case> <rc> <out> <elapsed> <retryAfter>
    [ "$2" -eq 5 ] || fail "$1: expected exit 5, got $2: $3"
    [ "$(tail -n1 <<<"$3")" = "APP-RELEASE-STAGE not-ready=upstream-catching-up doorway=${DOORWAY} elapsed=${4}" ] \
      || fail "$1: expected the not-ready summary last: $3"
    grep -qx "APP-RELEASE-STAGE not-ready=upstream-catching-up doorway=${DOORWAY} elapsed=${4}" "${TEST_ROOT}/stage.out" \
      || fail "$1: the not-ready line reaches APP_RELEASE_STAGE_OUT"
    ! grep -q '^APP-RELEASE-STAGE refused=' <<<"$3" || fail "$1: a deferral is not a refusal: $3"
    node -e '
      const i = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
      const want = { kind: "deploy-intent", elect: "one", phase: "release", commit: process.argv[2], env: "dev" };
      for (const [k, v] of Object.entries(want)) if (i[k] !== v) throw new Error(`${k}=${JSON.stringify(i[k])}`);
      if (JSON.stringify(i.doorways) !== JSON.stringify([process.argv[3], "https://doorway.elohim.host"])) throw new Error(`doorways=${i.doorways}`);
      const n = i.notReady;
      if (n.length !== 1 || n[0].doorway !== process.argv[3] || n[0].face !== "upstream-catching-up" || n[0].retryAfter !== Number(process.argv[4]) || i.retryAfter !== n[0].retryAfter) throw new Error(`notReady=${JSON.stringify(n)}`);
    ' "${INTENT}" "${COMMIT_SHA}" "${DOORWAY}" "$5" || fail "$1: the deploy intent's shape: $(cat "${INTENT}")"
}

# 19. the create (resuming a half-created channel) spends the budget.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}/channel-notarized" "${INTENT}"
: > "${TEST_ROOT}/channel-exists"
printf '%s\n' "${CATCHING}" > "${TEST_ROOT}/catching.txt"
printf '#!/bin/bash\necho "tsx $*" >> "${STUB_LOG}"\necho "release-ceremony: PATCH /db/content/x 503 catching-up — retry in 7s (0.0s/120s)" >&2\ncat "${STUB_ROOT}/catching.txt" >&2\nexit 1\n' > "${TEST_ROOT}/tsx.sh"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted defer_run)"; rc=$?; set -e
assert_deferred "create catching-up" "$rc" "$out" 120 7
grep -q ' channel create ' "${TEST_ROOT}/stub.log" || fail "create catching-up: the create ran"
! grep -q -e '^publish ' -e 'channel bind' -e '^verify ' "${TEST_ROOT}/stub.log" \
  || fail "create catching-up: nothing is bound, published or measured: $(cat "${TEST_ROOT}/stub.log")"
echo "ok 19 - a create whose catching-up budget ran out is NOT READY (exit 5, intent), not refused"

# 19b. no DEPLOY_INTENT_OUT → still exit 5, no intent written.
rm -f "${INTENT}"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted run)"; rc=$?; set -e
[ "$rc" -eq 5 ] && [ ! -e "${INTENT}" ] || fail "no intent out: expected exit 5 and no intent, got $rc: $out"
echo "ok 19b - without DEPLOY_INTENT_OUT the deferral writes no intent (exit 5)"

# 20. the bind spends the budget.
rm -f "${TEST_ROOT}/stub.log" "${TEST_ROOT}"/rows/*.json "${INTENT}"
: > "${TEST_ROOT}/channel-notarized"
printf '#!/bin/bash\necho "tsx $*" >> "${STUB_LOG}"\nsed "s/channel create/channel bind/" "${STUB_ROOT}/catching.txt" >&2\nexit 1\n' > "${TEST_ROOT}/tsx.sh"
set +e; out="$(APP_RELEASE_CHANNEL_CREATE=1 STUB_PUBLISH=published STUB_VERIFY=adopted defer_run)"; rc=$?; set -e
assert_deferred "bind catching-up" "$rc" "$out" 120 7
grep -q ' channel bind ' "${TEST_ROOT}/stub.log" || fail "bind catching-up: the bind ran"
! grep -q '^publish ' "${TEST_ROOT}/stub.log" || fail "bind catching-up: nothing is published"
echo "ok 20 - a bind whose catching-up budget ran out is NOT READY (exit 5, intent)"
write_tsx_stub

# 21. the publish spends the budget (the ceremony's line precedes publish's own REFUSED).
rm -f "${TEST_ROOT}/stub.log" "${INTENT}"
set +e; out="$(STUB_PUBLISH=catching-up STUB_VERIFY=adopted defer_run)"; rc=$?; set -e
assert_deferred "publish catching-up" "$rc" "$out" 300 2
! grep -q '^verify ' "${TEST_ROOT}/stub.log" || fail "publish catching-up: no adoption measure"
echo "ok 21 - a publish whose catching-up budget ran out is NOT READY (exit 5, intent)"
