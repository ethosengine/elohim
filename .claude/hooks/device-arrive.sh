#!/bin/sh
# Pre-native launchers cannot call device arrive: queue the shared bootstrap first.
root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd) || exit 0
if "${EPR_BIN:-epr}" bin status --tool epr --json >/dev/null 2>&1; then
  "${EPR_BIN:-epr}" device arrive || true
else
  node "$root/genesis/agentic/native-epr-bootstrap.mjs" "$root" || true
fi
