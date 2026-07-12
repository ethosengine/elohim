#!/usr/bin/env bash
# Render-time conductor-config validation — the seam-smoke gate that would
# have caught the dead `ice_servers` key (2026-07-11: Holochain passes
# webrtc_config VERBATIM into tx5's serde-camelCase WebRtcConfig, so a
# snake_case key is silently dropped and the fleet ran with ZERO ICE servers
# since inception — see backlog
# genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md).
#
# DEPENDENCY-FREE by hard requirement: this runs in the deploy container,
# which has bash + coreutils and NOT PyYAML (edge #1183 lesson — a python
# `import yaml` here failed every human deploy before a single kubectl
# apply). Textual checks are exactly what this gate needs: it is a
# drift/typo guard on a template we own, not a schema validator.
#
# Usage: validate-conductor-config.sh <rendered-manifest.yaml> [...more]
# Exits non-zero (FAILING the render step) if any manifest's embedded
# conductor-config:
#   - still uses a known-dead key spelling (ice_servers)
#   - has no webrtc_config with an iceServers list
#   - has an iceServers list with no `urls` entry
set -euo pipefail

fail=0
for manifest in "$@"; do
  if ! grep -q "conductor-config.yaml" "$manifest"; then
    echo "FAIL ${manifest}: no conductor-config.yaml ConfigMap found (validator misuse or template drift)"
    fail=1
    continue
  fi
  if grep -qE '^[[:space:]]*ice_servers:' "$manifest"; then
    echo "FAIL ${manifest}: webrtc_config uses the DEAD key 'ice_servers' — tx5's WebRtcConfig is serde camelCase; the key is 'iceServers' (this exact misspelling ran the fleet with zero ICE servers until 2026-07-11)"
    fail=1
    continue
  fi
  if ! grep -qE '^[[:space:]]*webrtc_config:' "$manifest"; then
    echo "FAIL ${manifest}: conductor-config has no network.webrtc_config (conductors would run with zero ICE servers)"
    fail=1
    continue
  fi
  if ! grep -qE '^[[:space:]]*iceServers:' "$manifest"; then
    echo "FAIL ${manifest}: webrtc_config has no iceServers list"
    fail=1
    continue
  fi
  # At least one urls entry must follow the iceServers key (list-item or
  # inline-array form).
  if ! awk '/^[[:space:]]*iceServers:/{found=1} found && /urls/{ok=1} END{exit ok?0:1}' "$manifest"; then
    echo "FAIL ${manifest}: iceServers has no urls entry"
    fail=1
    continue
  fi
  if ! grep -qE 'turns?:' "$manifest"; then
    # Advisory only — STUN-only is legal but cannot traverse the
    # shem<->on-prem NAT pair; a relay fallback should stay present.
    echo "WARN ${manifest}: iceServers has no TURN relay fallback (srflx-only fails silently across the genesis-pair NATs)"
  fi
  echo "OK ${manifest}: conductor webrtc_config.iceServers valid"
done
exit $fail
