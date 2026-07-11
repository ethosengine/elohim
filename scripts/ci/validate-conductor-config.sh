#!/usr/bin/env bash
# Render-time conductor-config validation — the seam-smoke gate that would
# have caught the dead `ice_servers` key (2026-07-11: Holochain passes
# webrtc_config VERBATIM into tx5's serde-camelCase WebRtcConfig, so a
# snake_case key is silently dropped and the fleet ran with ZERO ICE servers
# since inception — see backlog
# genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md).
#
# Usage: validate-conductor-config.sh <rendered-manifest.yaml> [...more]
# Exits non-zero (FAILING the render step) if any rendered manifest's
# conductor-config.yaml:
#   - still uses a known-dead key spelling (ice_servers)
#   - has a webrtc_config with no iceServers entry
#   - has an iceServers list that is empty or has an entry without urls
set -euo pipefail

fail=0
for manifest in "$@"; do
  python3 - "$manifest" <<'PY' || fail=1
import sys, yaml

path = sys.argv[1]
docs = list(yaml.safe_load_all(open(path)))
checked = 0
for doc in docs:
    if not isinstance(doc, dict) or doc.get("kind") != "ConfigMap":
        continue
    cc = (doc.get("data") or {}).get("conductor-config.yaml")
    if not cc:
        continue
    checked += 1
    conf = yaml.safe_load(cc)
    net = (conf or {}).get("network") or {}
    webrtc = net.get("webrtc_config")
    if webrtc is None:
        print(f"FAIL {path}: conductor-config has no network.webrtc_config "
              f"(conductors would run with zero ICE servers)")
        sys.exit(1)
    if "ice_servers" in webrtc:
        print(f"FAIL {path}: webrtc_config uses the DEAD key 'ice_servers' — "
              f"tx5's WebRtcConfig is serde camelCase; the key is 'iceServers' "
              f"(this exact misspelling ran the fleet with zero ICE servers "
              f"until 2026-07-11)")
        sys.exit(1)
    ice = webrtc.get("iceServers")
    if not ice or not isinstance(ice, list):
        print(f"FAIL {path}: webrtc_config.iceServers missing or empty")
        sys.exit(1)
    for i, entry in enumerate(ice):
        if not isinstance(entry, dict) or not entry.get("urls"):
            print(f"FAIL {path}: iceServers[{i}] has no urls")
            sys.exit(1)
    has_turn = any(
        u.startswith(("turn:", "turns:"))
        for entry in ice for u in entry.get("urls", [])
    )
    if not has_turn:
        # Advisory only — STUN-only is legal but cannot traverse the
        # shem<->on-prem NAT pair; a relay fallback should stay present.
        print(f"WARN {path}: iceServers has no TURN relay fallback "
              f"(srflx-only fails silently across the genesis-pair NATs)")
if checked == 0:
    print(f"FAIL {path}: no conductor-config.yaml ConfigMap found "
          f"(validator misuse or template drift)")
    sys.exit(1)
print(f"OK {path}: conductor webrtc_config.iceServers valid ({checked} config[s])")
PY
done
exit $fail
