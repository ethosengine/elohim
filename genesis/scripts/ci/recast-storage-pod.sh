#!/bin/bash
# Recast Storage Pod — wipe ONE human's elohim-storage state and prove the pod
# comes back on the byte-profile store (re-genesis 1, the address crossing).
#
# Replaces reset-storage-pod.sh, which was wrong three ways: it exec'd container
# `elohim-storage` while the consolidated pod's container is `elohim-node`; it
# deleted only content.db (leaving blobs/ in the retired `blobs/<4>/sha256-…`
# layout, the iroh store, graph.db and sync.sled); and it hid every failure
# behind `|| true`, so a reset that did nothing printed "reset initiated".
#
# CLUSTER OPS ARE OPERATOR-OWNED. This script runs `kubectl` — it is invoked by
# the genesis Jenkinsfile's "Reset Storage" stage behind RESET_STORAGE=true, or
# by the operator by hand. An agent in the dev environment never runs it.
#
# Fail-closed: `set -euo pipefail`, no `|| true`, every remote step is checked,
# every removal is verified absent, and the post-restart proof refuses (exit 3)
# a pod still carrying the retired layout or not initialising fresh.
#
# Usage:
#   recast-storage-pod.sh <service> <namespace> <humanId> [recast|verify]
#     recast  (default)  verify the container exists, remove the declared state,
#                        verify it is gone, delete the pod (the StatefulSet
#                        recreates it; migrations and the byte store initialise
#                        fresh), print a receipt line. Does NOT wait for Ready —
#                        the Jenkinsfile owns the (peer-tolerant) readiness wait.
#     verify             run once the pod is Ready: the retired layout is absent
#                        and the store initialised fresh; print the receipt.
#
# Env (all optional):
#   RECAST_POD          pod name              (default: <service>-0)
#   RECAST_CONTAINER    container to exec in  (default: elohim-node)
#   RECAST_STORAGE_DIR  storage data dir inside the container. Default: the
#                       container's own $STORAGE_DIR, else /data. (The retired
#                       script's `/data/content.db` is the evidence for /data;
#                       elohim-storage reads its root from STORAGE_DIR.)
#   RECAST_HTTP_PORT    storage HTTP port inside the container (default 8090)
#   RECAST_FRESH_BLOB_MAX  verify refuses a store reporting more blobs than this
#                       after restart (default 0: nothing seeds before the seeder)
#   RECAST_KUBECTL      kubectl binary (default: kubectl)
#   RECAST_RECEIPT_DIR  also append the receipt line to <dir>/storage-recast.jsonl
#   RECAST_DRY_RUN=1    print the commands, run nothing (exit 0)
#
# Declared storage state REMOVED (relative to the storage dir; read from
# elohim-storage: db/mod.rs `content.db`, main.rs `graph.db` + `sync.sled`,
# blob_store.rs `blobs/`, p2p_iroh/config.rs `blobs_iroh/`, and the byte store's
# `chunks/` `index/` `quarantine/` `tmp/` layout). SQLite runs WAL, so each
# database's -wal and -shm siblings go with it:
STATE_PATHS=(
    content.db content.db-wal content.db-shm
    graph.db graph.db-wal graph.db-shm
    sync.sled
    blobs
    blobs_iroh
    chunks
    index
    quarantine
    tmp
)
# Deliberately KEPT (identity and operator material, not content state):
#   identity.key  iroh.key  runtime-config.toml  release-adoption/  credentials/
#   ark/ (death-witness spool)  compute-payloads/ (TTL-leased attachments)
# Re-keying a peer is a different act (it retires the peer's standing).
set -euo pipefail

SERVICE="${1:?usage: recast-storage-pod.sh <service> <namespace> <humanId> [recast|verify]}"
NAMESPACE="${2:?namespace required}"
HUMAN_ID="${3:?humanId required}"
MODE="${4:-recast}"

POD="${RECAST_POD:-${SERVICE}-0}"
CONTAINER="${RECAST_CONTAINER:-elohim-node}"
HTTP_PORT="${RECAST_HTTP_PORT:-8090}"
FRESH_BLOB_MAX="${RECAST_FRESH_BLOB_MAX:-0}"
KUBECTL="${RECAST_KUBECTL:-kubectl}"
DRY_RUN="${RECAST_DRY_RUN:-0}"

case "${MODE}" in recast|verify) ;; *) echo "recast-storage-pod: unknown mode '${MODE}' (recast|verify)" >&2; exit 2 ;; esac

kx() { # kx <sh -c script> — run a script in the target container; failure is fatal
    "${KUBECTL}" exec -n "${NAMESPACE}" "${POD}" -c "${CONTAINER}" -- sh -c "$1"
}

if [ "${DRY_RUN}" = "1" ]; then
    echo "[dry-run] ${MODE} ${HUMAN_ID}: ${KUBECTL} exec -n ${NAMESPACE} ${POD} -c ${CONTAINER}"
    echo "[dry-run] state paths: ${STATE_PATHS[*]}"
    exit 0
fi

# Resolve the storage dir inside the container (STORAGE_DIR wins, else /data).
resolve_dir() {
    local d
    if [ -n "${RECAST_STORAGE_DIR:-}" ]; then
        d="${RECAST_STORAGE_DIR}"
    else
        d="$(kx 'printf %s "${STORAGE_DIR:-/data}"')"
    fi
    case "${d}" in
        /?*) ;;
        *) echo "recast-storage-pod: refusing storage dir '${d}' (must be an absolute path)" >&2; exit 2 ;;
    esac
    [ "${d}" != "/" ] || { echo "recast-storage-pod: refusing storage dir '/'" >&2; exit 2; }
    printf '%s' "${d}"
}

# The container must exist on the pod: the retired script's wrong container
# name is the failure this check makes impossible to repeat silently.
containers="$("${KUBECTL}" get pod "${POD}" -n "${NAMESPACE}" -o jsonpath='{.spec.containers[*].name}')"
case " ${containers} " in
    *" ${CONTAINER} "*) ;;
    *) echo "recast-storage-pod: pod ${NAMESPACE}/${POD} has containers [${containers}] — no '${CONTAINER}' (set RECAST_CONTAINER)" >&2
       exit 2 ;;
esac

DIR="$(resolve_dir)"

receipt() { # receipt <phase> <detail-json-fragment>
    local line
    line="{\"event\":\"storage-recast\",\"phase\":\"$1\",\"human\":\"${HUMAN_ID}\",\"pod\":\"${NAMESPACE}/${POD}\",\"container\":\"${CONTAINER}\",\"storageDir\":\"${DIR}\",$2,\"at\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}"
    echo "RECEIPT ${line}"
    if [ -n "${RECAST_RECEIPT_DIR:-}" ]; then
        mkdir -p "${RECAST_RECEIPT_DIR}"
        printf '%s\n' "${line}" >> "${RECAST_RECEIPT_DIR}/storage-recast.jsonl"
    fi
}

if [ "${MODE}" = "recast" ]; then
    echo "Recasting ${HUMAN_ID}: ${NAMESPACE}/${POD} [${CONTAINER}] storage dir ${DIR}"

    # Remove every declared path, then assert each is gone. `rm -rf` of an absent
    # path is success by design; the verification loop is what makes this closed.
    paths="${STATE_PATHS[*]}"
    kx "set -eu; cd '${DIR}'; for p in ${paths}; do rm -rf -- \"./\$p\"; done; \
        for p in ${paths}; do if [ -e \"./\$p\" ] || [ -L \"./\$p\" ]; then echo \"still present: \$p\" >&2; exit 1; fi; done"
    echo "   removed: ${paths}"

    # Restart so migrations and the byte store initialise from nothing. The
    # StatefulSet recreates the pod; readiness is the caller's wait.
    "${KUBECTL}" delete pod "${POD}" -n "${NAMESPACE}" --wait=false
    receipt recast "\"removed\":\"${paths}\",\"restart\":\"requested\""
    echo "   ✅ ${HUMAN_ID} recast initiated"
    exit 0
fi

# ---- verify (pod Ready) ------------------------------------------------------
echo "Verifying ${HUMAN_ID}: ${NAMESPACE}/${POD} initialised fresh"

# 1. The retired layout is absent: no sha256-named blob file anywhere under
#    blobs/ (the old `blobs/<first4>/sha256-<hex>`), and no sha256-/blake3- named
#    entry in the iroh or chunk stores.
legacy="$(kx "set -eu; cd '${DIR}'; for d in blobs blobs_iroh chunks index; do [ -d \"./\$d\" ] || continue; find \"./\$d\" \( -name 'sha256-*' -o -name 'blake3-*' \) -print | head -5; done")"
if [ -n "${legacy}" ]; then
    echo "recast-storage-pod: RETIRED LAYOUT STILL PRESENT on ${NAMESPACE}/${POD}:" >&2
    printf '%s\n' "${legacy}" >&2
    receipt verify "\"result\":\"refused\",\"reason\":\"retired-layout-present\""
    exit 3
fi

# 2. A fresh content database exists (migrations ran on the new schema).
kx "set -eu; cd '${DIR}'; [ -s ./content.db ]" || {
    echo "recast-storage-pod: ${NAMESPACE}/${POD} has no content.db after restart — storage did not initialise" >&2
    receipt verify "\"result\":\"refused\",\"reason\":\"no-content-db\""
    exit 3
}

# 3. The health endpoint answers ok and reports a store with no seeded blobs.
health="$(kx "set -eu; if command -v curl >/dev/null 2>&1; then curl -fsS -m 10 http://127.0.0.1:${HTTP_PORT}/health; \
    elif command -v wget >/dev/null 2>&1; then wget -q -T 10 -O - http://127.0.0.1:${HTTP_PORT}/health; \
    else echo 'neither curl nor wget in container' >&2; exit 127; fi")" || {
    echo "recast-storage-pod: ${NAMESPACE}/${POD} health endpoint unreachable on :${HTTP_PORT}" >&2
    receipt verify "\"result\":\"refused\",\"reason\":\"health-unreachable\""
    exit 3
}
case "${health}" in
    *'"status":"ok"'*|*'"status": "ok"'*) ;;
    *) echo "recast-storage-pod: health is not ok: ${health}" >&2
       receipt verify "\"result\":\"refused\",\"reason\":\"health-not-ok\""
       exit 3 ;;
esac
blobs="$(printf '%s' "${health}" | sed -n 's/.*"blobs"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p')"
if [ -n "${blobs}" ] && [ "${blobs}" -gt "${FRESH_BLOB_MAX}" ]; then
    echo "recast-storage-pod: store reports ${blobs} blobs after restart (> ${FRESH_BLOB_MAX}) — not a fresh store" >&2
    receipt verify "\"result\":\"refused\",\"reason\":\"not-fresh\",\"blobs\":${blobs}"
    exit 3
fi

receipt verify "\"result\":\"fresh\",\"blobs\":${blobs:-null},\"health\":\"ok\""
echo "   ✅ ${HUMAN_ID} storage initialised fresh on the byte-profile store"
