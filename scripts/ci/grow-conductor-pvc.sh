#!/usr/bin/env bash
# Grow one human's conductor chain volume to the size deployments.json declares.
#
# The conductor StatefulSet mounts `holochain-data-<prefix>-0` by claim name. The
# claim was minted by a volumeClaimTemplate that no longer exists, so until a
# human declares `conductorStorage`, nothing in the repo says how large it is
# (adam's read 7.4 GB and filled on 2026-10-06: four million peer-meta writes
# refused with SQLite 778; eve's stood at 90%). This step is the declared size's
# reconciler. It is WARN-ONLY: a volume that cannot be grown in place is named,
# with the move the operator has to make, and the roll goes on.
#
# Usage: grow-conductor-pvc.sh <claim> <namespace> <size>   (size like 20Gi)
#
# Reads the live claim because the repo cannot know its storage class: the class
# is immutable, so a manifest with the wrong one would make `kubectl apply`
# fail, and the repo records neither which class each live claim has nor
# whether that class allows expansion.
set -euo pipefail
# A defect in this script must never fail a conductor deploy: it is a reconciler
# of a declared size, and the roll it precedes is the thing that matters.
trap 'echo "conductor-pvc: ${1:-?}: CONDUCTOR-PVC-GROW-SKIPPED — this script failed at line $LINENO; the roll goes on"; exit 0' ERR

claim="$1"
namespace="$2"
size="$3"

say() { echo "conductor-pvc: $claim: $*"; }

to_bytes() { # <k8s quantity> -> integer bytes (Ki/Mi/Gi/Ti and K/M/G/T)
  # Bash 64-bit arithmetic, not awk: the CI agent's awk prints any `%d` above
  # 2^31 as 2147483647, so 20Gi read as 2 GiB there and every comparison past
  # that line was wrong (edge #1576, where the regression test caught it first).
  local q="$1" n unit mult int frac scale
  n="${q%%[^0-9.]*}"
  unit="${q#"$n"}"
  case "$unit" in
    Ki) mult=1024 ;;
    Mi) mult=$((1024 * 1024)) ;;
    Gi) mult=$((1024 * 1024 * 1024)) ;;
    Ti) mult=$((1024 * 1024 * 1024 * 1024)) ;;
    K|k) mult=1000 ;;
    M) mult=1000000 ;;
    G) mult=1000000000 ;;
    T) mult=1000000000000 ;;
    "") mult=1 ;;
    *) echo 0; return 0 ;;
  esac
  int="${n%%.*}"
  frac=""
  case "$n" in *.*) frac="${n#*.}" ;; esac
  scale=$((10 ** ${#frac}))
  echo $(( (10#${int:-0} * scale + 10#${frac:-0}) * mult / scale ))
}

# jsonpath, not jq: the edge build container has no jq (the 2026-08-31 fleet
# dispatch learned the same; edge #1565 learned it again at this line and HELD
# every conductor after eve's).
if ! fields="$(kubectl get pvc "$claim" -n "$namespace" \
  -o jsonpath='{.spec.storageClassName}{"\t"}{.spec.resources.requests.storage}{"\t"}{.status.capacity.storage}' 2>/dev/null)"; then
  say "absent — nothing to grow (the claim appears at the conductor's first boot); declared $size"
  exit 0
fi
IFS=$'\t' read -r class requested capacity <<<"$fields"
say "live class=${class:-?} requested=${requested:-?} capacity=${capacity:-?} declared=$size"

# The claim can say 20Gi while the filesystem under it is full. On a ZFS dataset
# with `quota` (shem-zfs: 2026-10-01, and again 2026-10-08 after the retention
# cut did not hold) snapshots count against the quota, so what the conductor can
# write shrinks ~1.4 GiB/day while the claim object stays at 20Gi and this step
# said `ok` to it on edge #1568–#1572. Only the mount shows it: read `df` through
# the pod that mounts the claim (bash + coreutils + kubectl only — this runs in
# the deploy container; `df -P -k` is POSIX so busybox and coreutils agree).
# Warn-only like the rest of this script; a roll is never failed by a reading.
read_filesystem() {
  local pods pod volname container mount line total used avail
  pods="$(kubectl get pods -n "$namespace" -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{range .spec.volumes[*]}{.name}{"="}{.persistentVolumeClaim.claimName}{" "}{end}{"\n"}{end}' 2>/dev/null || true)"
  pod="$(printf '%s\n' "$pods" | awk -F'\t' -v c="$claim" 'index(" " $2 " ", "=" c " ") { print $1; exit }')"
  if [ -z "$pod" ]; then
    say "filesystem not read — no pod mounts the claim yet (it appears with the conductor's pod)"
    return 0
  fi
  volname="$(printf '%s\n' "$pods" | awk -F'\t' -v p="$pod" '$1 == p { n = split($2, a, " "); for (i = 1; i <= n; i++) { split(a[i], kv, "="); if (kv[2] != "") print kv[1] "=" kv[2] } }' | awk -F'=' -v c="$claim" '$2 == c { print $1; exit }')"
  # which container mounts that volume, and where
  read -r container mount <<<"$(kubectl get pod "$pod" -n "$namespace" -o jsonpath='{range .spec.containers[*]}{.name}{"\t"}{range .volumeMounts[*]}{.name}{"="}{.mountPath}{" "}{end}{"\n"}{end}' 2>/dev/null \
    | awk -F'\t' -v v="$volname" '{ n = split($2, a, " "); for (i = 1; i <= n; i++) { split(a[i], kv, "="); if (kv[1] == v) { print $1, kv[2]; exit } } }')"
  if [ -z "${mount:-}" ]; then
    say "filesystem not read — pod $pod mounts the claim as volume '${volname:-?}' but no container mounts that volume"
    return 0
  fi
  # The exec's error is the reading when the container is not running: eve's
  # conductor was CrashLoopBackOff on its full volume (edge #1578) and this line
  # said only "answered nothing", hiding the one fact the operator needed.
  local err why
  err="$(mktemp)"
  line="$(kubectl exec -n "$namespace" "$pod" -c "$container" -- df -P -k "$mount" 2>"$err" | awk 'NR == 2' || true)"
  why="$(head -n 1 "$err" 2>/dev/null || true)"
  rm -f "$err"
  if [ -z "$line" ]; then
    say "filesystem not read — df $mount in $pod/$container answered nothing${why:+ ($why)}; a container that is not running cannot be asked (a conductor crash-looping on a full volume is the 2026-10-08 shape) — read the claim's kubelet capacity series instead"
    return 0
  fi
  read -r _ total used avail _ <<<"$line"
  total=$((total * 1024)); used=$((used * 1024)); avail=$((avail * 1024))
  local gib=1073741824
  local shown
  shown="$(awk -v t="$total" -v u="$used" -v a="$avail" -v g="$gib" 'BEGIN { printf "capacity %.2f GiB, used %.2f GiB, free %.2f GiB", t/g, u/g, a/g }')"
  # Full means the conductor cannot take a chain write: under 5 % or under 512 MiB free.
  if [ "$((avail * 20))" -lt "$total" ] || [ "$avail" -lt 536870912 ]; then
    say "CONDUCTOR-VOLUME-FULL — the claim says ${requested:-?} but the filesystem at $mount in $pod reads $shown: on a ZFS dataset with quota, snapshots count against it (shem 2026-10-01, 2026-10-08); operator: zfs set refquota=$size quota=none on the dataset behind this claim (or take the k8s datasets out of sanoid autosnap), then recycle this conductor's pod so lair and SQLite see the space"
  elif [ "$((total * 10))" -lt "$((declared_bytes * 9))" ]; then
    say "filesystem at $mount reads $shown — under the declared $size by more than a tenth: the quota is being consumed outside the filesystem (snapshots); not yet full"
  else
    say "filesystem at $mount reads $shown"
  fi
  return 0
}

declared_bytes="$(to_bytes "$size")"
if [ -n "$requested" ] && [ "$(to_bytes "$requested")" -ge "$declared_bytes" ]; then
  say "ok — the live request already meets the declared size"
  read_filesystem
  exit 0
fi

expandable="false"
if [ -n "$class" ]; then
  expandable="$(kubectl get storageclass "$class" -o jsonpath='{.allowVolumeExpansion}' 2>/dev/null || echo false)"
fi
if [ "$expandable" != "true" ]; then
  say "CONDUCTOR-PVC-GROW-REFUSED — class '${class:-?}' does not allow expansion (allowVolumeExpansion=${expandable:-false}); operator: move the data to a volume of at least $size under the same claim name, as doorway-B's key volume was moved on 2026-09-30"
  exit 0
fi

patch="{\"spec\":{\"resources\":{\"requests\":{\"storage\":\"$size\"}}}}"
if kubectl patch pvc "$claim" -n "$namespace" --type merge -p "$patch"; then
  say "request raised to $size; the filesystem grows when the volume's pod next restarts if the class needs that (status.conditions shows FileSystemResizePending until then)"
  kubectl get pvc "$claim" -n "$namespace" -o jsonpath='{range .status.conditions[*]}{.type}={.status} {end}{"\n"}' 2>/dev/null || true
else
  say "CONDUCTOR-PVC-GROW-REFUSED — the cluster refused the raised request (see above); the live volume is unchanged"
fi
exit 0
