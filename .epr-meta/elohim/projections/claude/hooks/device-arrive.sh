#!/bin/sh
# Pre-native launchers cannot call device arrive: queue the shared bootstrap first.
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd) || exit 0
if [ "${EPR_BIN+x}" = x ]; then
  # Exit 3 is the check's own refusal: EPR_BIN names a release this device's store ingested
  # before its head. A candidate build is exit 0 and stays set; any other failure (a crash, no
  # node) proves nothing about EPR_BIN, so it is left as it is.
  (unset GATE_LIVE_RUN; node "$root/genesis/orchestrator/gate-preflight.mjs" --epr-bin >/dev/null 2>&1)
  if [ $? -eq 3 ]; then
    old_epr_bin=$EPR_BIN
    unset EPR_BIN
    if [ -n "${CLAUDE_ENV_FILE:-}" ] && [ -w "$CLAUDE_ENV_FILE" ]; then
      printf '%s\n' 'unset EPR_BIN' >> "$CLAUDE_ENV_FILE" || true
    fi
    printf '%s\n' "[device] EPR_BIN=$old_epr_bin is older than this device's head epr; unset for this session. The terminal that started this session still exports it: run \"unset EPR_BIN\" there, or open a new terminal."
  fi
fi
if "${EPR_BIN:-epr}" bin status --tool epr --json >/dev/null 2>&1; then
  "${EPR_BIN:-epr}" device arrive && "${EPR_BIN:-epr}" session open
  true
else
  node "$root/genesis/agentic/native-epr-bootstrap.mjs" "$root" || true
fi
