---
name: agent-eyes-look-live-peer-loop
title: "Agent eyes: look + live-peer dev loop"
description: pnpm look (a2o) renders any URL to shot.png I can Read; pnpm start:alpha = local UI on live alpha data, no local stack; read-mostly rail against alpha.
metadata: 
  node_type: memory
  type: project
  originSessionId: 5415d4ae-087b-4aa6-97ab-613211946a18
---

Agent eyes in Eclipse Che are LANDED and verified (2026-06-10). Two primitives:

1. `cd genesis/a2o && pnpm look <url> [--as <FixtureHuman>] [--wait-testid <id>] [--out <slug>]`
   → writes `reports/look/<slug>/{shot.png,capture.json}`; Read shot.png (multimodal) to see it.
   Chromium lives in the XDG cache `/nix/xdg/cache/ms-playwright` (persistent /nix PVC), playwright
   locked 1.59.1 monorepo-wide. Works against deployed alpha directly (~7s/render).
2. Local UI × live alpha data (no local stack, no mocks): `cd app/elohim-app && pnpm start:alpha`
   (proxy.conf.alpha.mjs → https://doorway-alpha.elohim.host, DOORWAY_TARGET overrides), then
   `look http://localhost:4200/<surface>`. HMR + look = the UI polish loop.

Rails: read-mostly against alpha (writes are deliberate; never seed/mutate alpha from the loop);
L2 visual done-gates stay on deterministic fixture data, live loop is polish/diagnosis only.
Spec series: superpowers/specs 2026-05-30 L1 (look) + L2 (visual gate, landed in
agentic-developer SKILL.md) + 2026-06-10 L3 (live-peer loop). `--as` auth through the
local proxy is NOT yet verified (L3 open item). [[ci-playwright-image]]
