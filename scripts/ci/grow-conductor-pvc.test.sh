#!/usr/bin/env bash
# Regression coverage for grow-conductor-pvc.sh's filesystem reading.
#
# HOST-SIDE TEST ONLY — never runs in the deploy container. A kubectl stub on
# PATH plays the cluster: the claim, the pod that mounts it, and `df` inside the
# pod come from the scenario's files. The script under test must stay
# bash + coreutils + kubectl (scripts/ci .epr-meta rule).
#
#   a. request meets the declared size, df reads 0 free     → CONDUCTOR-VOLUME-FULL, exit 0
#   b. request meets, df reads 60 % free of 20 GiB           → `ok` + a filesystem line, no FULL
#   c. request meets, df reads 7 GiB capacity, 1.2 GiB free  → "under the declared" line, no FULL
#   d. request meets, no pod mounts the claim                → "filesystem not read", exit 0
#   e. claim absent                                          → "absent — nothing to grow", exit 0
#   f. the stub's df answers nothing                         → "answered nothing", exit 0
set -euo pipefail

# No git: the deploy container may run this beside the script it covers.
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SCRIPT="${SCRIPT:-${REPO_ROOT}/scripts/ci/grow-conductor-pvc.sh}"
TEST_ROOT="$(mktemp -d)"
trap 'rm -rf "${TEST_ROOT}"' EXIT

fail() { echo "FAIL: $*" >&2; exit 1; }

CLAIM=holochain-data-elohim-adam-alpha-0
NS=elohim-alpha
POD=elohim-adam-alpha-conductor-0

mkdir -p "${TEST_ROOT}/bin"
cat > "${TEST_ROOT}/bin/kubectl" <<'STUB'
#!/usr/bin/env bash
# Scenario files under $SCENARIO: pvc (tab fields or absent), pods, pod, df.
case "$1 $2" in
  "get pvc")
    [ -f "$SCENARIO/pvc" ] || exit 1
    cat "$SCENARIO/pvc" ;;
  "get pods")
    cat "$SCENARIO/pods" 2>/dev/null ;;
  "get pod")
    cat "$SCENARIO/pod" 2>/dev/null ;;
  "exec "*)
    echo "$*" >> "$SCENARIO/exec.log"
    cat "$SCENARIO/df" 2>/dev/null ;;
  *) echo "stub: unexpected kubectl $*" >&2; exit 2 ;;
esac
STUB
chmod +x "${TEST_ROOT}/bin/kubectl"
export PATH="${TEST_ROOT}/bin:${PATH}"

scenario() { # <name> → sets SCENARIO with the common pod shape
  export SCENARIO="${TEST_ROOT}/$1"
  mkdir -p "$SCENARIO"
  printf 'shem-zfs\t20Gi\t20Gi' > "$SCENARIO/pvc"
  printf '%s\tholochain-data=%s conductor-config= \n%s\tdata=storage-data-elohim-adam-alpha-0 \n' \
    "$POD" "$CLAIM" "elohim-adam-alpha-0" > "$SCENARIO/pods"
  printf 'elohim-conductor\tholochain-data=/var/local/lib/holochain conductor-config=/etc/holochain/conductor-config.yaml \nws-proxy\t\n' > "$SCENARIO/pod"
}

run() { # → OUT, CODE
  set +e
  OUT="$(bash "$SCRIPT" "$CLAIM" "$NS" 20Gi 2>&1)"
  CODE=$?
  set -e
}

# a. full: 6.3 GiB capacity, 0 free (adam on shem, 2026-10-08)
scenario full
printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\ntank/k8s/pvc-087fffec 6623232 6623232 0 100%% /var/local/lib/holochain\n' > "$SCENARIO/df"
run
[ "$CODE" -eq 0 ] || fail "a: exit $CODE"
grep -q "ok — the live request already meets" <<<"$OUT" || fail "a: no ok line: $OUT"
grep -q "CONDUCTOR-VOLUME-FULL" <<<"$OUT" || fail "a: no CONDUCTOR-VOLUME-FULL: $OUT"
grep -q "refquota=20Gi" <<<"$OUT" || fail "a: the operator move is not named: $OUT"
grep -q -- "-c elohim-conductor -- df -P -k /var/local/lib/holochain" "$SCENARIO/exec.log" || fail "a: df not read through the mounting container: $(cat "$SCENARIO/exec.log")"
echo "a ok"

# b. healthy: 20 GiB, 60 % free
scenario healthy
printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\ntank/k8s/pvc-087fffec 20971520 8388608 12582912 40%% /var/local/lib/holochain\n' > "$SCENARIO/df"
run
[ "$CODE" -eq 0 ] || fail "b: exit $CODE"
grep -q "CONDUCTOR-VOLUME-FULL" <<<"$OUT" && fail "b: FULL on a healthy volume: $OUT"
grep -q "filesystem at /var/local/lib/holochain reads capacity 20.00 GiB, used 8.00 GiB, free 12.00 GiB" <<<"$OUT" || fail "b: no filesystem line: $OUT"
echo "b ok"

# c. short: capacity 7 GiB of a declared 20Gi, 1.2 GiB free (gertrude's shape)
scenario short
printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\ntank/k8s/pvc-5bd 7340032 6081740 1258292 83%% /var/local/lib/holochain\n' > "$SCENARIO/df"
run
[ "$CODE" -eq 0 ] || fail "c: exit $CODE"
grep -q "CONDUCTOR-VOLUME-FULL" <<<"$OUT" && fail "c: FULL at 17 % free: $OUT"
grep -q "under the declared 20Gi by more than a tenth" <<<"$OUT" || fail "c: no short line: $OUT"
echo "c ok"

# d. no pod mounts the claim
scenario nopod
printf 'elohim-adam-alpha-0\tdata=storage-data-elohim-adam-alpha-0 \n' > "$SCENARIO/pods"
run
[ "$CODE" -eq 0 ] || fail "d: exit $CODE"
grep -q "filesystem not read — no pod mounts the claim" <<<"$OUT" || fail "d: $OUT"
[ -f "$SCENARIO/exec.log" ] && fail "d: exec called with no pod"
echo "d ok"

# e. claim absent
scenario absent
rm "$SCENARIO/pvc"
run
[ "$CODE" -eq 0 ] || fail "e: exit $CODE"
grep -q "absent — nothing to grow" <<<"$OUT" || fail "e: $OUT"
echo "e ok"

# f. df answers nothing
scenario silent
: > "$SCENARIO/df"
run
[ "$CODE" -eq 0 ] || fail "f: exit $CODE"
grep -q "answered nothing" <<<"$OUT" || fail "f: $OUT"
echo "f ok"

echo "grow-conductor-pvc.test.sh: all scenarios pass"
