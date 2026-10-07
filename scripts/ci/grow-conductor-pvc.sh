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

claim="$1"
namespace="$2"
size="$3"

say() { echo "conductor-pvc: $claim: $*"; }

to_bytes() { # <k8s quantity> -> integer bytes (Ki/Mi/Gi/Ti and K/M/G/T)
  local q="$1" n unit
  n="${q%%[^0-9.]*}"
  unit="${q#"$n"}"
  case "$unit" in
    Ki) awk -v n="$n" 'BEGIN{printf "%d", n*1024}' ;;
    Mi) awk -v n="$n" 'BEGIN{printf "%d", n*1024*1024}' ;;
    Gi) awk -v n="$n" 'BEGIN{printf "%d", n*1024*1024*1024}' ;;
    Ti) awk -v n="$n" 'BEGIN{printf "%d", n*1024*1024*1024*1024}' ;;
    K|k) awk -v n="$n" 'BEGIN{printf "%d", n*1000}' ;;
    M) awk -v n="$n" 'BEGIN{printf "%d", n*1000000}' ;;
    G) awk -v n="$n" 'BEGIN{printf "%d", n*1000000000}' ;;
    T) awk -v n="$n" 'BEGIN{printf "%d", n*1000000000000}' ;;
    "") printf '%d' "${n%.*}" ;;
    *) echo 0 ;;
  esac
}

if ! pvc_json="$(kubectl get pvc "$claim" -n "$namespace" -o json 2>/dev/null)"; then
  say "absent — nothing to grow (the claim appears at the conductor's first boot); declared $size"
  exit 0
fi

class="$(printf '%s' "$pvc_json" | jq -r '.spec.storageClassName // empty')"
requested="$(printf '%s' "$pvc_json" | jq -r '.spec.resources.requests.storage // empty')"
capacity="$(printf '%s' "$pvc_json" | jq -r '.status.capacity.storage // empty')"
say "live class=${class:-?} requested=${requested:-?} capacity=${capacity:-?} declared=$size"

if [ -n "$requested" ] && [ "$(to_bytes "$requested")" -ge "$(to_bytes "$size")" ]; then
  say "ok — the live request already meets the declared size"
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
