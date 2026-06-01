---
name: project_che_browser_feedback_loop
description: L1 browser feedback loop landed in Che — pnpm look render primitive + Playwright wiring; gotchas + L2 completion-oracle pointer
metadata: 
  node_type: memory
  type: project
  originSessionId: b54284e1-2315-4796-96a2-37fa74ba34d4
---

L1 of the Che browser feedback loop landed 2026-05-30 (specs:
`genesis/docs/superpowers/specs/2026-05-30-che-browser-feedback-{foundation,completion-oracle}-design.md`;
plan: `genesis/docs/plans/2026-05-30-che-browser-feedback-foundation-plan.md`). The agent can now
render any surface headless in Che and SEE it: `pnpm look <url> [--as <FixtureHuman>] [--wait-testid id]`
→ `genesis/a2o/reports/look/<latest|slug>/{shot.png,capture.json}` → Read the PNG (multimodal) + capture.json.
Reuses `PlaywrightDevice` capture. `pnpm a2o:setup` provisions the browser once.

**Non-obvious gotchas (the whys, not in code):**
- **Headless works in Che; the system Chrome-for-Testing 131 does NOT.** Pointing Playwright at
  `/opt/chrome-linux64/chrome` (131) hangs ~54s then drops CDP (version mismatch). Use Playwright's
  **version-matched bundled Chromium** (1.59.1 → Chromium 147, launches ~265ms). Never `channel:'chrome'`
  against the image's 131.
- **Browsers persist via `XDG_CACHE_HOME=/nix/xdg/cache` (devfile), NOT `~/.cache`** (HOME=/home/user is
  ephemeral). So **no `PLAYWRIGHT_BROWSERS_PATH`** is set — adding one would create a competing cache.
- **One Playwright version is locked on purpose** (`pnpm.overrides` → playwright/playwright-core 1.59.1).
  Divergent versions (a2o was ^1.50.0→1.58.2, seeder 1.59.1) each pin a different Chromium revision, so the
  shared cache had doubled to **1.8GB** (chromium-1208+1217 + unused firefox+webkit). Pruned to **631MB**
  (one chromium + headless-shell + ffmpeg; both consumers only `chromium.launch`). `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1`
  stops install-time re-spray. Chromium 147 is the ubuntu24.04 fallback build (ubi10 "not officially supported" — works).
- **Shared-browser "Target closed" cascade:** running a large @browser set (~110 scenarios) on world.ts's single
  shared browser → it dies partway → later scenarios fail `Target page... has been closed`. Pre-existing a2o
  characteristic, not a wiring fault. L2's loop renders should NOT lean on one long-lived shared browser.

**Pre-existing finding (flagged, not fixed):** pnpm v10.30.3 deprecates the `pnpm` field in root
`package.json` (libsodium override + Angular patch + the new playwright override). Still honored via the
lockfile today; warrants a holistic migration to `pnpm-workspace.yaml` as its own cleanup.

**Delivery finding surfaced by `look`:** `doorway-alpha/dashboard` returns `{"error":"File not found in app: dashboard"}`
(doorway routes `dashboard` as an EPR-app slug and misses) — the exact CI-green-≠-visible gap. See
[[project_epr_projection_serving_chain]].

**L2 — completion oracle: LANDED 2026-05-31** (spec `2026-05-30-che-browser-completion-oracle-design.md`,
plan `...-completion-oracle-plan.md`). Reframed mid-flight: the agentic-developer skill ALREADY had a full
visual-validation dimension (SKILL.md §"Visual validation as an integration candidate dimension" — `visualValidation`
buckets via `@elohim-visually-validated` tag in `aggregate.ts`, steward screenshot verdict, per-iteration journaling),
but CI-artifact-driven (the genesis browser stage that "often did not run"). So L2 EXTENDED it, didn't duplicate —
the old spec's parallel Objective `visual` block + verdict were DROPPED. Three deltas, all in skill/command/template
prose (no schema/aggregate change): (1) **local generation** — `/shift` renders `@browser` cucumber +
`build-sprint-report --profile browser` locally in Che (the `--profile browser` flag is MANDATORY — `aggregate.ts`
`isPlaywrightProfile` gates `visualValidation` emission, else buckets silently absent), closing the Jenkins round-trip;
(2) **hard done-gate** — opt-in via a journal `Visual gate: on/off` flag (NOT a schema field), requiring
`validatedRegressed == 0` over in-scope tagged scenarios across two local renders; gate-off shifts are byte-identical;
(3) **kickoff baseline** render. Relates to [[feedback_deliver_drive_mode_no_menu]] and
[[project_deliver_authority_discipline_paired_verdicts]].
