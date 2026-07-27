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
      "the round opener must construct it (spine sync-scale-honesty)"
check "inc_sync_in_sync" "elohim/elohim-storage/src/p2p" \
      "the InSync client arm must count the shortcut"

exit $fail
