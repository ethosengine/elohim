---
name: ci-playwright image is ci-builder + playwright layer
description: ethosengine/ci-playwright inherits ci-builder (node:20 + corepack pnpm); bundles playwright@1.59.1; lockfile/image version drift triggers one-time chromium redownload
type: project
originSessionId: 81491e3a-1dae-4ea1-a00a-def3f332fbfd
---
`harbor.ethosengine.com/ethosengine/ci-playwright:latest` is a thin layer over `ci-builder`, not a Microsoft Playwright base image. Inherited from ci-builder: Node 20.20.2, corepack/pnpm 10.33.4, `CHROME_BIN=/usr/bin/chromium-wrapper` env var, root user (no USER directive). Adds: playwright@1.59.1 + `/ms-playwright/chromium-1217` (Chrome 147) + ffmpeg-1011.

**Why:** Empirical verification 2026-05-06 against the new image. We added it as a sidecar in `genesis/Jenkinsfile` for the browser-mode E2E stage; my prior assumption that root-launch needed `--no-sandbox` was wrong (Playwright auto-disables sandbox in containers).

**How to apply:**
- Workspace pattern works: `pnpm install` in builder seeds `node_modules`; playwright container runs `npx cucumber-js` against the same workspace volume.
- Version drift is real: genesis/a2o resolves playwright@1.58.2; image bundles 1.59.1. First run after a lockfile bump that doesn't match the image will trigger a chromium redownload to `/ms-playwright` — succeeds as root, one-time slowdown not a failure. If perf matters, rebuild the image with the lockfile's pinned version.
- `CHROME_BIN=/usr/bin/chromium-wrapper` is inherited and ignored by Playwright (uses its own browser locator). Any non-Playwright tool reading it points to system chromium (Debian package), not the bundled Chrome for Testing — surprising if you debug from there.
- Don't assume "Playwright image" means a clean Microsoft base — it's our ci-builder lineage, so anything CI-builder-shaped (corepack, NPM_TOKEN-aware, harbor pull-creds) Just Works.
