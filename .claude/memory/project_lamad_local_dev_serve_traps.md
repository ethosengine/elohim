---
name: project_lamad_local_dev_serve_traps
title: Lamad local dev-serve traps
description: Rendering app/lamad locally needs --serve-path /lamad/ and DOORWAY_TARGET=localhost:8888 (mesh doorway); anonymous alpha content reads are reach-gated.
metadata: 
  node_type: memory
  title: Lamad local dev serve traps
  type: project
  originSessionId: 4a1e973b-6af5-4b35-87da-4d48c835b86d
  modified: 2026-09-02T04:12:22.157Z
---

Rendering the lamad step player locally (verified 2026-09-02):

- `pnpm start:alpha` in `app/lamad` serves the bundle at `/` while `index.html` carries `<base href="/lamad/">`, so module scripts come back as `text/html` and the page stays blank. Run `pnpm exec ng serve --proxy-config proxy.conf.alpha.mjs --serve-path /lamad/` (port 4300) and render `http://localhost:4300/lamad/path/<id>/step/<n>`.
- `environment.client.doorwayUrl` is `http://localhost:8888`; when the local mesh is up that port is the mesh doorway, which holds the seeded paths AND the manifesto blob. Point the vite proxy at it too with `DOORWAY_TARGET=http://localhost:8888`, otherwise `/blob/...` goes to alpha and 404s and the body renders as a bare `sha256-…` string.
- Anonymous reads of most lamad content on doorway-alpha return `Authentication required` (`requiredReach: community`); the deployed app gets the body through its own session, the local proxy does not.
- A stale `app/elohim-elements/elohim-imagodei/dist` (missing `probeDoorway`) breaks the lamad dev build; rebuild the element package with `pnpm build` there first.
- `pnpm look` writes full-page screenshots, so sticky rails and the TOC gutter cannot be judged from them; a small Playwright scroll+viewport probe (see [[project_agent_eyes_look_live_peer_loop]]) is the way to verify sticky behaviour.

**Why:** the step-player redesign lost an hour to blank renders and missing bodies before these four facts were pinned.
**How to apply:** any eyes-first pass on `app/lamad` — start the server with the serve path, proxy to 8888, then look.
