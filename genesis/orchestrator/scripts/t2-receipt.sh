#!/usr/bin/env bash
# T2 receipt — the pre-push pawl for the evidence ladder's household rung
# (spec ratchet-to-delivery-dataplane-sdk-lanes, lane D rung D2; evidence-ladder-push-left §3
# "ascend-only": a dataplane change is pushed only after the household mesh has seen it).
#
# Serving changes require every deliverability station on the current source.
# Other dataplane changes retain the advisory recency receipt unless --strict.
#
# usage: t2-receipt.sh --changed <file: one repo-relative path per line> [--reports <dir>] [--strict]
set -u
CHANGED_FILE=""; REPORTS_DIR=""; STRICT="${T2_RECEIPT:-}"
while [ $# -gt 0 ]; do
  case "$1" in
    --changed) CHANGED_FILE="$2"; shift 2 ;;
    --reports) REPORTS_DIR="$2"; shift 2 ;;
    --strict) STRICT=strict; shift ;;
    *) echo "t2-receipt: unknown arg $1" >&2; exit 2 ;;
  esac
done
[ -n "$CHANGED_FILE" ] && [ -r "$CHANGED_FILE" ] || { echo "t2-receipt: --changed <file> required" >&2; exit 2; }
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
REPORTS_DIR="${REPORTS_DIR:-$REPO_ROOT/genesis/a2o/reports}"

# The paths whose behaviour only the household mesh can witness (T2 in the ladder).
DATAPLANE_RE='^(elohim/elohim-storage/src/(p2p|sync|reconcile|p2p_iroh)/|doorway/doorway-service/src/)'
SERVING_RE='^(app/scripts/lint-ssr-entry\.mjs|elohim/sdk/scripts/|scripts/ci/(stage-spa-blob|verify-served-shell|verify-projected-head|same-doorway-curl)|doorway/doorway-service/src/|elohim/elohim-storage/src/(services/content_service\.rs|services/head_adoption\.rs|db/content|routes/apps|http|ssr|app_deliverability|sync/|p2p/(projection_reconcile|blob|view|content)|p2p_iroh/|reconcile/)|genesis/a2o/(features/dataplane/(epr-app-deliverability|served-shell-boots)|steps/dataplane/epr-app-deliverability|scripts/(browser-shell|verify-served-shell|lib/sut)|src/framework/served-shell-boot))'
touched=$(grep -E "$DATAPLANE_RE|$SERVING_RE" "$CHANGED_FILE" || true)
[ -n "$touched" ] || exit 0
serving_touched=$(grep -E "$SERVING_RE" "$CHANGED_FILE" || true)
if [ -n "$serving_touched" ]; then
  if node "$REPO_ROOT/genesis/orchestrator/scripts/serving-receipt.mjs" "$REPO_ROOT" "$REPORTS_DIR"; then
    exit 0
  fi
  echo '[pre-push] NO-SERVING-RECEIPT (refused): every browser, SSR and recovery station must pass on the current source.' >&2
  echo '[pre-push] Produce the receipt: just test mesh features/dataplane/epr-app-deliverability.feature' >&2
  exit 1
fi

newest_change=0
while IFS= read -r f; do
  [ -f "$REPO_ROOT/$f" ] || continue
  m=$(stat -c %Y "$REPO_ROOT/$f" 2>/dev/null || echo 0)
  [ "$m" -gt "$newest_change" ] && newest_change=$m
done <<< "$touched"

newest_report=""; newest_report_m=0
for r in "$REPORTS_DIR"/sprint-report-household-*.json; do
  [ -f "$r" ] || continue
  m=$(stat -c %Y "$r" 2>/dev/null || echo 0)
  if [ "$m" -gt "$newest_report_m" ]; then newest_report_m=$m; newest_report="$r"; fi
done

if [ -n "$newest_report" ] && [ "$newest_report_m" -ge "$newest_change" ]; then
  echo "[pre-push] T2 receipt: $(basename "$newest_report") is newer than the dataplane changes it covers."
  exit 0
fi

scope_hint='@dataplane'
echo "$touched" | grep -q '^doorway/' && scope_hint='@dataplane or @doorway'
cat <<BANNER
[pre-push] ── NO-T2-RECEIPT ($( [ "$STRICT" = strict ] && echo "REFUSED${serving_touched:+ — serving paths changed: the boot-through-doorway lane must run}" || echo "warn-only; T2_RECEIPT=strict to refuse")) ──
[pre-push]   dataplane paths changed with no household sprint-report newer than them:
$(echo "$touched" | sed 's/^/[pre-push]     /')
[pre-push]   the household mesh is the authority for these paths (evidence ladder T2).
[pre-push]   produce the receipt:  just mesh start && just mesh prologue && just test mesh '$scope_hint'
[pre-push]   (writes genesis/a2o/reports/sprint-report-household-<run>.json; this leg reads its mtime)
[pre-push] ─────────────────────────────────────────────────────────────────────────
BANNER
[ "$STRICT" = "strict" ] && exit 1
exit 0
