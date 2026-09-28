#!/bin/bash
# Grade ONE genesis peer's own rows to their authored reach, through that peer's
# own conductor (genesis/a2o/scripts/steward-grade.ts).
#
# Runs before the peer's content seed. Rows an older seeder stamped `private` by
# default answer the anonymous seeder 403 and so were never corrected; this step
# proves (via the node's conductor, never a storage header) that the node's own
# agent authored each such row and widens it to exactly its authored commons/public
# reach. Idempotent: rows already at their authored reach are `current` no-ops.
#
# Externalized from genesis/Jenkinsfile (runContentSeedStage) to keep the
# pipeline's single CPS dispatch method heredoc-free — see CLAUDE.md
# "Jenkinsfile Size Limit".
#
# Invoked via sh(returnStatus: true, ...): exit 0 = no failures (refusals are
# printed and counted, not failures); non-zero = a failure the caller marks
# UNSTABLE before continuing to the peer's seed.
#
# Args:
#   $1 = genesis peer humanId (for log lines)
#   $2 = genesis peer storage URL (host:port, no scheme)
#   $3 = the peer's storage Service name (e.g. elohim-matthew-alpha)
#   $4 = the peer's namespace (e.g. elohim-alpha)
#
# The conductor lives on its own Service, `<service>-conductor` (rung 2,
# 2026-08-31; genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml),
# admin WS :4444 / app WS :4445 — the same derivation getConductorAppUrls() uses.
# Reaching them needs the jenkins→elohim-alpha NetworkPolicy admission documented
# above getConductorAppUrls() in genesis/Jenkinsfile.
# Overrides: STEWARD_ADMIN_WS / STEWARD_APP_WS, STEWARD_GRADE_CLOSURE
# (default foundations-christian-technology), STEWARD_GRADE_DRY_RUN=1.
set -uo pipefail

GP_HUMAN_ID="$1"
GP_STORAGE_URL="$2"
GP_SERVICE="$3"
GP_NAMESPACE="$4"

CONDUCTOR_HOST="${GP_SERVICE}-conductor.${GP_NAMESPACE}.svc.cluster.local"
ADMIN_WS="${STEWARD_ADMIN_WS:-ws://${CONDUCTOR_HOST}:4444}"
APP_WS="${STEWARD_APP_WS:-ws://${CONDUCTOR_HOST}:4445}"
CLOSURE="${STEWARD_GRADE_CLOSURE:-foundations-christian-technology}"
DRY=()
if [ "${STEWARD_GRADE_DRY_RUN:-0}" = "1" ]; then DRY=(--dry-run); fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"

echo "═══════════════════════════════════════════════════════════"
echo "🎚️  STEWARD GRADE: ${GP_HUMAN_ID} (storage ${GP_STORAGE_URL}, conductor ${ADMIN_WS} / ${APP_WS})"
echo "    closure ${CLOSURE} — widen own-authored rows to their authored reach"
echo "═══════════════════════════════════════════════════════════"

cd "${REPO_ROOT}/genesis/a2o" || { echo "❌ genesis/a2o missing"; exit 1; }
npx tsx scripts/steward-grade.ts \
    --closure "${CLOSURE}" \
    --storage "http://${GP_STORAGE_URL}" \
    --admin-ws "${ADMIN_WS}" \
    --app-ws "${APP_WS}" \
    "${DRY[@]}"
RC=$?
echo "STEWARD_GRADE_EXIT=${RC} (${GP_HUMAN_ID})"
exit "${RC}"
