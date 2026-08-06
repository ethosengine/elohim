#!/usr/bin/env bash
# dead-config-lint.sh — fail when a declared knob has no reader.
#
# WHY: ListDocumentsSince and AnnounceChange were fully handled on the receive
# side and never constructed; the sync plane paid O(peers x corpus) for months
# because a wired-looking mechanism was inert. Same class as enable_eviction:true
# with zero readers. A knob with no reader is not a feature, it is a claim.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fail=0

# symbol : directory to search for a READER (construction / consumption site)
check() {
  local sym="$1" dir="$2" note="$3"
  local hits
  hits=$(grep -rn --include=*.rs "$sym" "$ROOT/$dir" 2>/dev/null \
         | grep -v "^.*sync_protocol.rs" | grep -vc "^$" || true)
  if [ "${hits:-0}" -eq 0 ]; then
    echo "DEAD-CONFIG: $sym has no reader in $dir — $note"
    fail=1
  fi
}

check "SyncRequest::ListDocumentsSince" "elohim/elohim-storage/src/p2p/sync_round.rs" \
      "the round opener must construct it (habits sync-scale-honesty)"
check "inc_sync_in_sync" "elohim/elohim-storage/src/p2p" \
      "the InSync client arm must count the shortcut"
check "PROJECTION_INVENTORY_TABLE_COLLECTIVES" "elohim/elohim-storage/src/p2p/projection_reconcile.rs" \
      "the collectives reconcile arm must ASK for the table the responder serves \
(a served table nobody requests is a claim, not a feature)"
check "call_zome_imagodei" "elohim/elohim-storage/src/services" \
      "the imagodei cell route must have a caller — an unrouted cell is the \
ZomeNotFound class this method exists to close"
check "inc_content_head_adopted" "elohim/elohim-storage/src/services/head_adoption.rs" \
      "the adopt-before-author pre-flight must COUNT its adoptions — a flat-zero \
elohim_content_head_adopted_total is how 'we now adopt instead of re-authoring' \
stays an unfalsifiable claim"
check "HeadElection::PreserveExistingDeclaration" "elohim/elohim-storage/src/services/reanchor_backfill.rs" \
      "the re-author sweep must elect PRESERVE — a heal-class author that declares \
re-crowns its own root every restart, which IS the divergence defect"
check "ViewKind::ContentHeadRecord" "elohim/elohim-storage/src/p2p/head_record_client.rs" \
      "a view kind the responder serves but nobody requests is a claim, not a feature \
(same rule as PROJECTION_INVENTORY_TABLE_COLLECTIVES above)"

# NOT WIRED YET: the inverse check — "does this read key have a setter?" — lives at
# .claude/scripts/config-provenance.py and finds real cases (ELOHIM_IROH_RELAY_URL is
# read by the iroh transport and set by nothing, so the leg ships relay-less while its
# boot sentinel prints green). It is NOT called from here because this script's runtime
# is unverified for python3 and the deploy-container invariant (edge #1183) is bash +
# coreutils only. A check that silently no-ops in CI is an instrument with no reader —
# the exact class this script exists to catch. Wire it once the runtime is confirmed,
# or port the query to bash.

exit $fail
