#!/usr/bin/env bash
# scripts/ci/tests/conductor-recovery-receipt.test.sh
# run: bash scripts/ci/tests/conductor-recovery-receipt.test.sh
#
# Hermetic: builds an evidence tree in the exact layout
# capture-rollout-evidence.sh writes (<root>/<ns>--statefulset--<name>/pods.yaml
# and <pod>--<container>--{current,previous}.log) and checks one cycle-time row
# per conductor pod. No kubectl, no network.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
receipt="$here/../conductor-recovery-receipt.sh"
root="$(mktemp -d)"
only="$(mktemp -d)"
trap 'rm -rf "$root" "$only"' EXIT
pass=0
fail() { echo "FAIL $*" >&2; exit 1; }
ok() { echo "PASS $*"; pass=$((pass + 1)); }

conductor_pods_yaml() { # <dir> <pod> <creationTimestamp>
  mkdir -p "$1"
  cat > "$1/pods.yaml" <<YAML
\$ kubectl get pods -n elohim-alpha -l app=x -o yaml
apiVersion: v1
items:
- apiVersion: v1
  kind: Pod
  metadata:
    creationTimestamp: "$3"
    generateName: ${2%-*}-
    labels:
      name: not-the-pod-name
    name: $2
    namespace: elohim-alpha
    ownerReferences:
    - apiVersion: apps/v1
      kind: StatefulSet
      name: ${2%-*}
  spec:
    containers:
    - name: conductor
  status:
    containerStatuses:
    - name: conductor
      state:
        running:
          startedAt: "2026-09-25T11:59:00Z"
    startTime: "$3"
kind: List

[exit=0]
YAML
}

ns=elohim-alpha
# matthew: JSON storage logs across previous + current container logs.
conductor_pods_yaml "$root/$ns--statefulset--elohim-matthew-alpha-conductor" \
  elohim-matthew-alpha-conductor-0 2026-09-25T10:00:00Z
sdir="$root/$ns--statefulset--elohim-matthew-alpha"
mkdir -p "$sdir"
cat > "$sdir/elohim-matthew-alpha-0--elohim-node--previous.log" <<'LOG'
$ kubectl logs elohim-matthew-alpha-0 -n elohim-alpha -c elohim-node --previous --tail=200
{"timestamp":"2026-09-25T09:40:00.000000Z","level":"INFO","fields":{"message":"conductor app is RUNNING again — a zome call on this role SUCCEEDED","role":"lamad","not_running_secs":3},"target":"elohim_storage::conductor_bridge_health"}

[exit=0]
LOG
cat > "$sdir/elohim-matthew-alpha-0--elohim-node--current.log" <<'LOG'
$ kubectl logs elohim-matthew-alpha-0 -n elohim-alpha -c elohim-node --tail=200
{"timestamp":"2026-09-25T10:01:00.000000Z","level":"WARN","fields":{"message":"conductor app is NOT RUNNING (CellDisabled)","role":"lamad"},"target":"elohim_storage::conductor_bridge_health"}
{"timestamp":"2026-09-25T10:07:12.250000Z","level":"INFO","fields":{"message":"conductor app is RUNNING again — a zome call on this role SUCCEEDED, which is the only proof of recovery this node accepts; the enable backoff is reset","role":"lamad","not_running_secs":372,"enable_attempts":4},"target":"elohim_storage::conductor_bridge_health"}
{"timestamp":"2026-09-25T10:12:30.500000Z","level":"INFO","fields":{"message":"conductor app is RUNNING again — a zome call on this role SUCCEEDED, which is the only proof of recovery this node accepts; the enable backoff is reset","role":"infrastructure","not_running_secs":98,"enable_attempts":6},"target":"elohim_storage::conductor_bridge_health"}
{"timestamp":"2026-09-25T10:40:00.000000Z","level":"INFO","fields":{"message":"conductor app is RUNNING again — a zome call on this role SUCCEEDED","role":"lamad","not_running_secs":20},"target":"elohim_storage::conductor_bridge_health"}

[exit=0]
LOG

# jessica: storage log carries no recovery line after the restart.
conductor_pods_yaml "$root/$ns--statefulset--elohim-jessica-alpha-conductor" \
  elohim-jessica-alpha-conductor-0 2026-09-25T10:20:00Z
sdir="$root/$ns--statefulset--elohim-jessica-alpha"
mkdir -p "$sdir"
cat > "$sdir/elohim-jessica-alpha-0--elohim-node--current.log" <<'LOG'
{"timestamp":"2026-09-25T10:21:00.000000Z","level":"WARN","fields":{"message":"conductor app is NOT RUNNING (CellDisabled)","role":"lamad"},"target":"elohim_storage::conductor_bridge_health"}
LOG

# james: conductor evidence but no storage evidence at all.
conductor_pods_yaml "$root/$ns--statefulset--elohim-james-alpha-conductor" \
  elohim-james-alpha-conductor-0 2026-09-25T10:40:00Z

# adam: plain-text (non-JSON) line, e.g. an operator's Loki export dropped into
# the same layout under the collector's <pod>--<source>.log naming.
conductor_pods_yaml "$root/$ns--statefulset--elohim-adam-alpha-conductor" \
  elohim-adam-alpha-conductor-0 2026-09-25T11:00:00Z
sdir="$root/$ns--statefulset--elohim-adam-alpha"
mkdir -p "$sdir"
cat > "$sdir/elohim-adam-alpha-0--loki.log" <<'LOG'
2026-09-25T11:03:05.000000Z  INFO elohim_storage::conductor_bridge_health: conductor app is RUNNING again — a zome call on this role SUCCEEDED role="lamad" not_running_secs=180
LOG

# A non-conductor workload directory must be ignored.
mkdir -p "$root/$ns--deployment--elohim-doorway-alpha"
printf 'kind: List\n' > "$root/$ns--deployment--elohim-doorway-alpha/pods.yaml"

set +e
out="$(bash "$receipt" --was '56 min–6 h (25dd2d0be)' "$root")"
rc=$?
set -e
echo "$out"

[ "$(printf '%s\n' "$out" | grep -c '^| ')" -eq 4 ] || fail "expected exactly 4 table rows (one per conductor pod)"
ok "one row per conductor pod"

row() { printf '%s\n' "$out" | grep -F "$1" || true; }

r="$(row 'elohim-matthew-alpha')"
case "$r" in
  *'| 56 min–6 h (25dd2d0be) |'*'12m30s'*'2026-09-25T10:00:00Z'*'2026-09-25T10:12:30'*'infrastructure'*'2 roles'*) ok "matthew: slowest role after restart" ;;
  *) fail "matthew row wrong: $r" ;;
esac
case "$r" in *'09:40'*|*'10:40'*) fail "matthew row used a pre-restart or repeat line: $r" ;; esac
ok "matthew: pre-restart and repeat recoveries not counted"

r="$(row 'elohim-jessica-alpha')"
case "$r" in *UNMEASURED*'no "conductor app is RUNNING again" line'*) ok "jessica: honest absence" ;; *) fail "jessica row wrong: $r" ;; esac

r="$(row 'elohim-james-alpha')"
case "$r" in *UNMEASURED*'no storage log'*) ok "james: missing storage evidence named" ;; *) fail "james row wrong: $r" ;; esac

r="$(row 'elohim-adam-alpha')"
case "$r" in *'3m05s'*'lamad'*'1 role'*) ok "adam: text-format line parsed" ;; *) fail "adam row wrong: $r" ;; esac

if printf '%s\n' "$out" | grep -q 'doorway'; then fail "non-conductor workload produced a row"; fi
ok "non-conductor workloads ignored"

[ "$rc" -eq 3 ] || fail "expected exit 3 when any pod is unmeasured, got $rc"
ok "exit 3 when any row is UNMEASURED"

cp -r "$root/$ns--statefulset--elohim-matthew-alpha-conductor" "$root/$ns--statefulset--elohim-matthew-alpha" "$only/"
bash "$receipt" "$only" > "$only.out" || fail "all-measured tree should exit 0"
grep -Fq '56 min–6 h' "$only.out" || fail "default --was should name the pre-fix window"
rm -f "$only.out"
ok "exit 0 when every row is measured (default --was)"

set +e
bash "$receipt" >/dev/null 2>&1; rc=$?
bash "$receipt" "$root/does-not-exist" >/dev/null 2>&1; rc2=$?
set -e
{ [ "$rc" -eq 64 ] && [ "$rc2" -eq 64 ]; } || fail "usage errors should exit 64 (got $rc, $rc2)"
ok "usage errors exit 64"

if grep -v '^[[:space:]]*#' "$receipt" | grep -Eq '(^|[^a-z-])kubectl([^a-z-]|$)'; then
  fail "receipt must never call kubectl"
fi
ok "receipt never calls kubectl"

echo "conductor-recovery-receipt: $pass checks passed"
