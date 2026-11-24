#!/usr/bin/env bash
# T2 receipt — the pre-push pawl for the evidence ladder's household rung
# (spec ratchet-to-delivery-dataplane-sdk-lanes, lane D rung D2; evidence-ladder-push-left §3
# "ascend-only": a dataplane change is pushed only after the household mesh has seen it).
#
# Serving changes require every station of the household story that witnesses them, on the
# current source (a signed validation attestation, else a sprint-report file), when the push
# names refs/heads/dev or refs/heads/main (steward ruling 2026-11-24: refuse only at dev and main).
# A push to any other ref (a self/ hand-over, a feature branch) prints one line naming the
# receipt still owed before dev and passes. Without --targets, or with an empty or unreadable
# list, the receipt is mandatory: a caller that cannot say where the push goes is refused as if
# it went to dev.
# Other dataplane changes retain the advisory recency receipt unless --strict.
#
# usage: t2-receipt.sh --changed <file: one repo-relative path per line> [--reports <dir>]
#                      [--targets <file: one pushed remote ref per line>] [--strict]
set -u
CHANGED_FILE=""; REPORTS_DIR=""; TARGETS_FILE=""; STRICT="${T2_RECEIPT:-}"
while [ $# -gt 0 ]; do
  case "$1" in
    --changed) CHANGED_FILE="$2"; shift 2 ;;
    --targets) TARGETS_FILE="$2"; shift 2 ;;
    --reports) REPORTS_DIR="$2"; shift 2 ;;
    --strict) STRICT=strict; shift ;;
    *) echo "t2-receipt: unknown arg $1" >&2; exit 2 ;;
  esac
done
[ -n "$CHANGED_FILE" ] && [ -r "$CHANGED_FILE" ] || { echo "t2-receipt: --changed <file> required" >&2; exit 2; }
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
REPORTS_DIR="${REPORTS_DIR:-$REPO_ROOT/genesis/a2o/reports}"

# Integration push: one of the pushed refs is dev or main, or the targets are unknown.
INTEGRATION=1
if [ -n "$TARGETS_FILE" ] && [ -r "$TARGETS_FILE" ] && grep -q '[^[:space:]]' "$TARGETS_FILE"; then
  INTEGRATION=0
  while IFS= read -r target_ref; do
    case "$target_ref" in refs/heads/dev | refs/heads/main) INTEGRATION=1 ;; esac
  done < "$TARGETS_FILE"
fi

# The paths whose behaviour only the household mesh can witness (T2 in the ladder).
# SERVING_RE paths are mandatory whatever T2_RECEIPT says. serving-receipt.mjs owns which household
# story's receipt admits each: features/dataplane/app-delivery-refuses-fast.feature (the not-ready
# window) for fleet-write-readiness.sh, scripts/ci/lib/ and release_adoption/; either it or
# epr-app-deliverability.feature for stage-spa-blob.sh; deliverability for the rest. A signed
# household ValidationAttestation (refs/notes/brit/validate/a2o-household/…) is preferred over a
# report file; both must match the current source.
DATAPLANE_RE='^(elohim/elohim-storage/src/(p2p|sync|reconcile|p2p_iroh)/|doorway/doorway-service/src/)'
SERVING_RE='^(elohim/elohim-render/|app/scripts/lint-ssr-entry\.mjs|elohim/sdk/scripts/|scripts/ci/(stage-spa-blob|verify-served-shell|verify-projected-head|same-doorway-curl|fleet-write-readiness\.sh|lib/)|doorway/doorway-service/src/|elohim/elohim-storage/src/(services/content_service\.rs|services/head_adoption\.rs|services/release_adoption/|db/content|routes/apps|http|ssr|app_deliverability|sync/|p2p/(projection_reconcile|blob|view|content)|p2p_iroh/|reconcile/)|genesis/a2o/(features/dataplane/(epr-app-deliverability|served-shell-boots)|steps/dataplane/epr-app-deliverability|scripts/(browser-shell|verify-served-shell|lib/sut)|src/framework/served-shell-boot))'
# Governance metadata is not source-under-test: a `.epr-meta` directory or manifest under any
# component (habit atoms recording evidence) is never a dataplane or serving change. Mirrors
# isGovernancePath in genesis/a2o/scripts/lib/sut.ts; serving-receipt.test.mjs pins the two.
GOVERNANCE_RE='(^|/)\.epr-meta(/|$)'
candidates=$(grep -vE "$GOVERNANCE_RE" "$CHANGED_FILE" || true)
touched=$(printf '%s\n' "$candidates" | grep -E "$DATAPLANE_RE|$SERVING_RE" || true)
[ -n "$touched" ] || exit 0
serving_touched=$(printf '%s\n' "$candidates" | grep -E "$SERVING_RE" || true)
if [ -n "$serving_touched" ]; then
  serving_list=$(mktemp)
  printf '%s\n' "$serving_touched" > "$serving_list"
  if [ "$INTEGRATION" -eq 0 ]; then
    # Not dev or main: the receipt is owed, not refused. Its own lines collapse into one.
    # stdout (the receipt it found) passes through; stderr is read for the stories still owed.
    { receipt_err=$(node "$REPO_ROOT/genesis/orchestrator/scripts/serving-receipt.mjs" "$REPO_ROOT" "$REPORTS_DIR" "$serving_list" 2>&1 1>&3 3>&-); receipt_rc=$?; } 3>&1
    rm -f "$serving_list"
    [ "$receipt_rc" -eq 0 ] && exit 0
    owed=$(printf '%s\n' "$receipt_err" | sed -n 's/^\[pre-push\] missing household receipt — produce it: //p' | paste -sd ';' - | sed 's/;/; /g')
    [ -n "$owed" ] || owed="the serving receipt could not be read ($(printf '%s\n' "$receipt_err" | grep -m1 . | sed 's/^\[pre-push\] //'))"
    echo "[pre-push] serving receipt owed before dev (this push does not reach dev or main, so it is not refused): $owed"
    exit 0
  fi
  node "$REPO_ROOT/genesis/orchestrator/scripts/serving-receipt.mjs" "$REPO_ROOT" "$REPORTS_DIR" "$serving_list"
  receipt_rc=$?
  rm -f "$serving_list"
  [ "$receipt_rc" -eq 0 ] && exit 0
  echo '[pre-push] NO-SERVING-RECEIPT (refused): every station of the story that witnesses these serving paths must pass on the current source.' >&2
  echo "$serving_touched" | sed 's/^/[pre-push]     /' >&2
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
