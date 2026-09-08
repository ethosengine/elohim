#!/usr/bin/env bash
# T2 receipt — the pre-push pawl for the evidence ladder's household rung
# (spec ratchet-to-delivery-dataplane-sdk-lanes, lane D rung D2; evidence-ladder-push-left §3
# "ascend-only": a dataplane change is pushed only after the household mesh has seen it).
#
# WARN-ONLY by default: prints a NO-T2-RECEIPT banner when the push touches a dataplane path and
# no household sprint-report is newer than the newest changed file. T2_RECEIPT=strict (or --strict)
# turns the banner into a refusal. It never blocks a push the container cannot prove — the mesh may
# be down; the banner names the command that produces the receipt.
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
# The SERVING paths: a change here is refused without a household receipt that EXERCISED
# @concern:doorway-failover (the boot-through-doorway lane), regardless of T2_RECEIPT — spec
# 2026-09-08 epr-app-deliverability-through-doorway D4a. A newer report that skipped the concern
# is not a receipt for it.
SERVING_RE='^(doorway/doorway-service/src/(render|projection)/|doorway/doorway-service/src/routes/apps\.rs|elohim/elohim-storage/src/services/content_service\.rs)'
SERVING_CONCERN='doorway-failover'
touched=$(grep -E "$DATAPLANE_RE" "$CHANGED_FILE" || true)
[ -n "$touched" ] || exit 0
serving_touched=$(grep -E "$SERVING_RE" "$CHANGED_FILE" || true)
if [ -n "$serving_touched" ]; then STRICT=strict; fi

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
  if [ -n "$serving_touched" ]; then
    # The receipt must have EXERCISED the serving concern, not merely postdate the change.
    if python3 - "$newest_report" "$SERVING_CONCERN" <<'PY'
import json, sys
d = json.load(open(sys.argv[1])); c = sys.argv[2]
ex = set((d.get("declared") or {}).get("exercised") or [])
byc = (d.get("summary") or {}).get("byConcern") or {}
ok = c in ex and (byc.get(c, {}).get("failed", 0) == 0) and (byc.get(c, {}).get("passed", 0) > 0)
sys.exit(0 if ok else 1)
PY
    then
      echo "[pre-push] T2 receipt: $(basename "$newest_report") exercised @concern:$SERVING_CONCERN green and is newer than the serving-path changes."
      exit 0
    fi
    echo "[pre-push] ── NO-SERVING-RECEIPT (refused) ─────────────────────────────────────" >&2
    echo "[pre-push]   serving paths changed but the newest household report did not exercise @concern:$SERVING_CONCERN green:" >&2
    echo "$serving_touched" | sed 's/^/[pre-push]     /' >&2
    echo "[pre-push]   produce one: just test mesh features/dataplane/epr-app-deliverability.feature   (household mesh up: just mesh start && just mesh wait)" >&2
    rm -f "$CHANGED_FILE" 2>/dev/null; exit 1
  fi
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
