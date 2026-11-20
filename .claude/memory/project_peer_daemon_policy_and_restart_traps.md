---
name: project-peer-daemon-policy-and-restart-traps
title: "Peer daemon policy + restart traps"
description: epr peer daemon reads crossing policy from its checkout; a stale main checkout silently denies artifact-produce crossing. Restart traps too.
metadata:
  node_type: memory
  title: Peer daemon policy + restart traps
  type: project
  originSessionId: ab4528a1-72ba-4517-856c-4277b174922c
  modified: 2026-11-13T17:39:39.939Z
---

`epr peer serve` loads `<root>/genesis/agentic/pool-policy.json` at start. When the main checkout
is behind dev, its `berth.channel.crosses` lacks `artifact-produce@1`, and since the allow-list is
applied both when projecting a device's channel and when folding a sibling's, **no build record
crosses in either direction, silently** (2026-11-12, ethosengine + shem; likely also the
2026-11-09 F2r `no-record`). Cure without syncing the checkout: keep `--root /projects/elohim`
and pass `--policy <a dev copy>` (stable copy at `genesis/local-dev/one-image/daemon/pool-policy.json`).
Changing `--root` instead moves the device's served flow store.

**Why:** cost a day of the K9 receipt; symptom looked like a fold cap.

**How to apply:**
- Empty `holders` for a sibling's known record → compare `epr peer status` `rules:` path/digest
  on both devices before suspecting the fold.
- The daemon refuses a non-release binary (`daemon-release-unverified`); only the device
  `current` release or an explicit `EPR_BIN=<exact path>` candidate runs. Start the new one and
  confirm it is up BEFORE killing the old (a script that killed first left the channel down).
- Never `pgrep -f`/`kill` a pattern inline in a Bash command: the pattern is in the shell's own
  command line, so it kills the calling shell too (exit 144 left no daemon). Put it in a script
  file, or use `ps -eo pid,args | grep "[e]pr peer serve"`.

- Relay env is `EPR_PEER_RELAYS` (comma list, `n0`, `off`); `EPR_PEER_RELAY_URL` was a one-release
  compat read and has since been removed (callers moved 2026-11-13, a0bec2549); set only EPR_PEER_RELAYS.
- Restart in a staggered window with the sibling (one side always up), reading both sides' `rules:`
  digest equal before calling it done. Cross-check: `epr artifact fetch --output <CID> --dest
  <pool>/**/mesh-bin/<name>` (no `--expect-output` with `--output`; a session-pinned old epr lacks
  `artifact`, so call the bin-store `current/epr`).

Related: [[project-regenesis1-landing-traps-receipt-and-epr-closure]], [[feedback-cross-device-berth-notes-need-follow]].
