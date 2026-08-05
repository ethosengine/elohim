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
#
# IROH-TARGET branch (Wave 2 relay-sovereignty design, §4.2 "Config validation
# must move with the flip"): once a target builds transport-iroh, this ICE
# check validates a dead key and a NEW check must validate relay_url instead.
# Gated OFF by default (prod/staging stay on tx5 this wave) — set
# IROH_TARGET=1 to enable; the tx5 ICE checks above are UNCHANGED either way.
# Negative self-test: `IROH_TARGET=1 validate-conductor-config.sh <manifest
# with relay_url: "https://use1-1.relay.iroh.network./">` must FAIL
# ("relay_url host matches the *.iroh.network n0 default — D1 violation").
iroh_target="${IROH_TARGET:-0}"
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

  # IROH-TARGET branch (default OFF — see header). Mechanical form of D1: a
  # transport-iroh conductor must carry an explicit, self-hosted relay_url,
  # never n0's public default, and it must be the relay of the human's own
  # primaryDoorway (cross-checked against the rendered signal_url, since both
  # are derived from the same primaryDoorway in deployHumanManifest).
  if [ "$iroh_target" = "1" ]; then
    relay_line=$(grep -E '^[[:space:]]*relay_url:' "$manifest" || true)
    if [ -z "$relay_line" ]; then
      echo "FAIL ${manifest}: IROH_TARGET=1 but no network.relay_url found (D1: every transport-iroh conductor-config surface must carry an explicit relay_url — never the n0 default)"
      fail=1
      continue
    fi
    relay_url=$(printf '%s' "$relay_line" | sed -E 's/^[[:space:]]*relay_url:[[:space:]]*"?([^"[:space:]]*)"?[[:space:]]*$/\1/')
    case "$relay_url" in
      https://*) : ;;
      *)
        echo "FAIL ${manifest}: relay_url '${relay_url}' is not https (D1 requires an explicit https self-hosted relay)"
        fail=1
        continue
        ;;
    esac
    relay_host=$(printf '%s' "$relay_url" | sed -E 's#^https://([^/:]+).*#\1#')
    # Strip a trailing root-label dot (FQDN form, e.g. n0's
    # use1-1.relay.iroh.network.) before the n0-domain check — it is the
    # same host either way.
    relay_host_nodot="${relay_host%.}"
    case "$relay_host_nodot" in
      *.iroh.network)
        echo "FAIL ${manifest}: relay_url host '${relay_host}' matches the *.iroh.network n0 default — D1 violation (self-hosted relay required, never n0's public fleet)"
        fail=1
        continue
        ;;
    esac
    case "$relay_host" in
      relay.alpha.elohim.host|relay.elohim.host) : ;;
      *)
        echo "FAIL ${manifest}: relay_url host '${relay_host}' is neither relay.alpha.elohim.host nor relay.elohim.host (the two known per-doorway relays)"
        fail=1
        continue
        ;;
    esac
    signal_line=$(grep -E '^[[:space:]]*signal_url:' "$manifest" || true)
    signal_url=$(printf '%s' "$signal_line" | sed -E 's/^[[:space:]]*signal_url:[[:space:]]*"?([^"[:space:]]*)"?[[:space:]]*$/\1/')
    case "$signal_url" in
      wss://signal.alpha.elohim.host) expected_relay_host="relay.alpha.elohim.host" ;;
      wss://signal.elohim.host) expected_relay_host="relay.elohim.host" ;;
      *)
        echo "FAIL ${manifest}: unrecognized signal_url '${signal_url}' — cannot derive the expected primaryDoorway relay host to cross-check relay_url against"
        fail=1
        continue
        ;;
    esac
    if [ "$relay_host" != "$expected_relay_host" ]; then
      echo "FAIL ${manifest}: relay_url host '${relay_host}' does not match the primaryDoorway relay '${expected_relay_host}' derived from signal_url '${signal_url}'"
      fail=1
      continue
    fi
    echo "OK ${manifest}: IROH-TARGET relay_url '${relay_url}' matches primaryDoorway (${expected_relay_host})"
  fi
done
exit $fail
