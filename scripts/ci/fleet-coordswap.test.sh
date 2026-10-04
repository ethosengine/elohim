#!/usr/bin/env bash
set -euo pipefail

ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
DRIVER="$(cd "$(dirname "$0")" && pwd)/fleet-coordswap.sh"
HAPP="$ROOT/test.happ"
CALLS="$ROOT/calls"
printf bundle > "$HAPP"

fail() { echo "FAIL: $*" >&2; exit 1; }

curl() {
  local args="$*" url apply peer body
  url="${!#}"
  apply=false
  case "$url" in *'apply=true'*) apply=true ;; esac
  peer="${url#http://}"; peer="${peer%%/*}"
  printf '%s %s\n' "$peer" "$apply" >> "$CALLS"
  body="$(cat "$ROOT/$peer-$apply.json")"
  printf '%s\n200' "$body"
}
export -f curl
export CALLS ROOT

mixed='{"appId":"elohim","apply":false,"roles":[{"role":"imagodei","drifted":true,"applied":false,"error":null},{"role":"lamad","drifted":true,"applied":false,"error":"dnaHashMismatch: wrong lineage"}],"driftedCount":2,"appliedCount":0}'
clean='{"appId":"elohim","apply":false,"roles":[],"driftedCount":0,"appliedCount":0}'
applied='{"appId":"elohim","apply":true,"roles":[{"role":"lamad","drifted":true,"applied":true,"error":null}],"driftedCount":1,"appliedCount":1}'

printf '%s' "$mixed" > "$ROOT/one-false.json"
set +e
bash "$DRIVER" --happ "$HAPP" --peers one=http://one --json > "$ROOT/status.json"
rc=$?
set -e
[ "$rc" -eq 2 ] || fail "status with a role error returned $rc, expected 2"
jq -e '.[0].verdict == "error" and (.[0].note | contains("dnaHashMismatch"))' "$ROOT/status.json" >/dev/null \
  || fail "status did not expose the role error"

# The DNA builder has python3 but no jq. Exercise that exact parser fallback
# under a minimal PATH so jq cannot be discovered from the host.
mkdir -p "$ROOT/python-path"
for command in bash basename cat head mktemp python3 rm sed tail tr; do
  ln -s "$(command -v "$command")" "$ROOT/python-path/$command"
done
set +e
PATH="$ROOT/python-path" bash "$DRIVER" --happ "$HAPP" --peers one=http://one --json > "$ROOT/status-python.json"
rc=$?
set -e
[ "$rc" -eq 2 ] || fail "python fallback status with a role error returned $rc, expected 2"
jq -e '.[0].verdict == "error" and (.[0].note | contains("dnaHashMismatch"))' "$ROOT/status-python.json" >/dev/null \
  || fail "python fallback did not expose the role error"

: > "$CALLS"
printf '%s' "$mixed" > "$ROOT/one-false.json"
printf '%s' "$clean" > "$ROOT/two-false.json"
set +e
bash "$DRIVER" --happ "$HAPP" --peers one=http://one,two=http://two --apply --json > "$ROOT/refused.json"
rc=$?
set -e
[ "$rc" -eq 1 ] || fail "apply precheck with a role error returned $rc, expected 1"
[ "$(cat "$CALLS")" = 'one false' ] || fail "failed precheck made an apply call or reached the next peer"
jq -e '.[0].verdict == "failed-pre-check" and (.[0].note | contains("dnaHashMismatch"))' "$ROOT/refused.json" >/dev/null \
  || fail "apply refusal did not report the role error"

zero_error='{"appId":"elohim","apply":true,"roles":[{"role":"lamad","drifted":false,"applied":false,"error":"get_dna_definition failed"}],"driftedCount":0,"appliedCount":0}'

# An HTTP-200 apply response can carry a per-role error independently of its
# aggregate drift count. It must stop before verification or the next peer.
: > "$CALLS"
printf '%s' '{"appId":"elohim","apply":false,"roles":[{"role":"lamad","drifted":true,"applied":false,"error":null}],"driftedCount":1,"appliedCount":0}' > "$ROOT/one-false.json"
printf '%s' "$zero_error" > "$ROOT/one-true.json"
set +e
bash "$DRIVER" --happ "$HAPP" --peers one=http://one,two=http://two --apply --json > "$ROOT/apply-error.json"
rc=$?
set -e
[ "$rc" -eq 1 ] || fail "per-role apply error returned $rc, expected 1"
[ "$(cat "$CALLS")" = $'one false\none true' ] || fail "apply error reached verification or the next peer"
jq -e '.[0].verdict == "failed-apply"' "$ROOT/apply-error.json" >/dev/null \
  || fail "apply response role error was not surfaced"

# Subsequent cases need distinct first and post-apply dry-run responses.
curl() {
  local args="$*" url apply peer n body
  url="${!#}"; apply=false; case "$url" in *'apply=true'*) apply=true ;; esac
  peer="${url#http://}"; peer="${peer%%/*}"
  printf '%s %s\n' "$peer" "$apply" >> "$CALLS"
  n="$(awk -v p="$peer" '$1==p {n++} END {print n+0}' "$CALLS")"
  if [ "$peer" = one ] && [ "$apply" = false ] && [ "$n" -gt 1 ]; then
    body="$(cat "$ROOT/one-false-after.json")"
  else
    body="$(cat "$ROOT/$peer-$apply.json")"
  fi
  printf '%s\n200' "$body"
}
export -f curl

# Likewise, zero remaining drift cannot hide a role error during verification.
: > "$CALLS"
printf '%s' '{"appId":"elohim","apply":false,"roles":[{"role":"lamad","drifted":true,"applied":false,"error":null}],"driftedCount":1,"appliedCount":0}' > "$ROOT/one-false.json"
printf '%s' "$applied" > "$ROOT/one-true.json"
printf '%s' "$zero_error" > "$ROOT/one-false-after.json"
set +e
bash "$DRIVER" --happ "$HAPP" --peers one=http://one,two=http://two --apply --json > "$ROOT/verify-error.json"
rc=$?
set -e
[ "$rc" -eq 1 ] || fail "per-role verification error returned $rc, expected 1"
[ "$(cat "$CALLS")" = $'one false\none true\none false' ] || fail "verification error reached the next peer"
jq -e '.[0].verdict == "failed-verify"' "$ROOT/verify-error.json" >/dev/null \
  || fail "verification role error was not surfaced"

: > "$CALLS"
printf '%s' '{"appId":"elohim","apply":false,"roles":[{"role":"lamad","drifted":true,"applied":false,"error":null}],"driftedCount":1,"appliedCount":0}' > "$ROOT/one-false.json"
printf '%s' "$applied" > "$ROOT/one-true.json"
# The third call to peer one and the only call to peer two are both clean.
printf '%s' "$clean" > "$ROOT/two-false.json"
printf '%s' "$clean" > "$ROOT/one-false-after.json"
bash "$DRIVER" --happ "$HAPP" --peers one=http://one,two=http://two --apply --json > "$ROOT/applied.json"
[ "$(cat "$CALLS")" = $'one false\none true\none false\ntwo false' ] || fail "good rolling apply call order changed"
jq -e '.[0].verdict == "updated" and .[1].verdict == "up-to-date"' "$ROOT/applied.json" >/dev/null \
  || fail "good apply/status semantics changed"

# --all-apps: the peer answers for every app on its conductor. A hosted app on
# another DNA lineage is reported and does not hold the rollout; a pending role
# on a hosted app is applied; a blocking error on a hosted app stops the roll.
all_pending='{"apply":false,"apps":[{"appId":"elohim","roles":[]},{"appId":"hosted-a","roles":[{"role":"lamad","drifted":true,"applied":false,"error":null},{"role":"imagodei","drifted":true,"applied":false,"error":"dnaHashMismatch: other lineage"}]}],"driftedCount":2,"appliedCount":0,"pendingCount":1,"blockingErrors":[]}'
all_applied='{"apply":true,"apps":[],"driftedCount":2,"appliedCount":1,"pendingCount":0,"blockingErrors":[]}'
all_clean='{"apply":false,"apps":[],"driftedCount":1,"appliedCount":0,"pendingCount":0,"blockingErrors":[]}'
: > "$CALLS"
printf '%s' "$all_pending" > "$ROOT/one-false.json"
printf '%s' "$all_applied" > "$ROOT/one-true.json"
printf '%s' "$all_clean" > "$ROOT/one-false-after.json"
printf '%s' "$all_clean" > "$ROOT/two-false.json"
bash "$DRIVER" --happ "$HAPP" --peers one=http://one,two=http://two --apply --all-apps --json > "$ROOT/all-apps.json"
[ "$(cat "$CALLS")" = $'one false\none true\none false\ntwo false' ] || fail "all-apps rolling apply call order changed"
jq -e '.[0].verdict == "updated" and .[1].verdict == "up-to-date"' "$ROOT/all-apps.json" >/dev/null \
  || fail "all-apps: a hosted cell on another lineage held the rollout open"

: > "$CALLS"
printf '%s' '{"apply":false,"apps":[],"driftedCount":1,"appliedCount":0,"pendingCount":1,"blockingErrors":["hosted-a/lamad: get_dna_definition failed"]}' > "$ROOT/one-false.json"
set +e
PATH="$ROOT/python-path" bash "$DRIVER" --happ "$HAPP" --peers one=http://one,two=http://two --apply --all-apps --json > "$ROOT/all-apps-blocked.json"
rc=$?
set -e
[ "$rc" -eq 1 ] || fail "all-apps blocking error returned $rc, expected 1"
[ "$(cat "$CALLS")" = 'one false' ] || fail "all-apps blocking error reached an apply or the next peer"
jq -e '.[0].verdict == "failed-pre-check" and (.[0].note | contains("hosted-a/lamad"))' "$ROOT/all-apps-blocked.json" >/dev/null \
  || fail "all-apps blocking error did not name the app and role"

echo 'fleet-coordswap: role-error refusal, first-peer stop, clean apply, and all-apps passed'
