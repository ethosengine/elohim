#!/usr/bin/env bash
# Render-time Holochain 0.7 conductor-config validation.
#
# DEPENDENCY-FREE by hard requirement: this runs in the deploy container,
# which has bash + coreutils and NOT PyYAML (edge #1183 lesson — a python
# `import yaml` here failed every human deploy before a single kubectl apply).
# Textual checks are sufficient for this template seam; Lane F's local-mesh
# boot remains the runtime parser proof.
#
# Usage: validate-conductor-config.sh <rendered-manifest.yaml> [...more]
# Exits non-zero if any embedded conductor-config:
#   - contains either removed tx5 network key (they hard-fail a 0.7 conductor)
#   - has no explicit relay_url
#   - uses the *.iroh.network public default
set -euo pipefail

fail=0
for manifest in "$@"; do
  if ! grep -q "conductor-config.yaml" "$manifest"; then
    echo "FAIL ${manifest}: no conductor-config.yaml ConfigMap found (validator misuse or template drift)"
    fail=1
    continue
  fi
  if grep -qE '^[[:space:]]*(signal[_]url|webrtc[_]config):' "$manifest"; then
    echo "FAIL ${manifest}: removed tx5 network keys hard-fail a 0.7 conductor"
    fail=1
    continue
  fi

  relay_line=$(grep -E '^[[:space:]]*relay_url:' "$manifest" || true)
  if [ -z "$relay_line" ]; then
    echo "FAIL ${manifest}: no network.relay_url found (every 0.7 conductor-config surface must carry its doorway's explicit relay_url — never the n0 default)"
    fail=1
    continue
  fi
  relay_url=$(printf '%s' "$relay_line" | sed -E 's/^[[:space:]]*relay_url:[[:space:]]*"?([^"[:space:]]*)"?[[:space:]]*$/\1/')
  relay_host=$(printf '%s' "$relay_url" | sed -E 's#^https://([^/:]+).*#\1#')
  # Strip a trailing root-label dot (FQDN form) before the n0-domain check.
  relay_host_nodot="${relay_host%.}"
  case "$relay_host_nodot" in
    *.iroh.network)
      echo "FAIL ${manifest}: relay_url host '${relay_host}' matches the *.iroh.network n0 default — D1 violation (self-hosted relay required, never n0's public fleet)"
      fail=1
      continue
      ;;
  esac

  echo "OK ${manifest}: Holochain 0.7 config has relay_url '${relay_url}' and no tx5 keys"
done
exit $fail
