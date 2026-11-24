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
  # Bounded below the harness's 30 s kill: arrival measured 14 s on a quiet device and 30 s under
  # a push gate's load (2026-11-24), where the harness killed it without a word. A bound of our own
  # fails open and says so.
  timeout 20 sh -c '"$0" device arrive && "$0" session open' "${EPR_BIN:-epr}"
  [ $? -eq 124 ] && printf '%s\n' '[device] arrive exceeded 20s and was stopped; run "epr device arrive && epr session open" when the device is quieter.'
  true
else
  node "$root/genesis/agentic/native-epr-bootstrap.mjs" "$root" || true
fi
