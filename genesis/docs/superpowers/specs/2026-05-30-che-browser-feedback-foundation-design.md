---
status: Design
---

# Che Browser Feedback — L1 Foundation: Wiring Fix + `look` Primitive

> Spec 1 of 2. This is the **foundation**: make a headless browser actually launch in the
> Eclipse Che container, and give the agentic developer a lightweight "render → see" command.
> Spec 2 (`2026-05-30-che-browser-completion-oracle-design.md`) builds the `/shift` + `/deliver`
> completion gate on top of this. Build L1 first; it ships standalone and is the hard dependency.

## Why now — the discovery that reshaped the problem

The ask started as "add Playwright to the Che container so the agentic loop can see browser
test results locally instead of round-tripping through Jenkins." Investigation showed the
problem is far narrower and better-shaped than "add Playwright from scratch":

| Layer | State (verified 2026-05-30) |
|---|---|
| Headless Chrome in Che | ✅ Works. `/opt/chrome-linux64/chrome --headless=new --dump-dom` and `--screenshot` both exit 0. The dbus / ANGLE / EGL / GPU errors in stderr are **cosmetic** (no display/GPU/dbus needed for headless DOM + raster). `ldd` reports no missing libs. |
| The agent seeing the result | ✅ Works. A raw-Chrome screenshot → PNG → the `Read` tool (multimodal) renders it; layout, color, and type all came back legible. |
| Playwright framework | ✅ Mature. `PlaywrightDevice` (Browser+Context+Page wrapper with console/error/network capture), 120+ `data-testid` selectors in `src/framework/pages/selectors.ts`, browser cucumber profile (`E2E_DEVICE_MODE=playwright`), `BasePage.testId()`. |
| `/deliver` local pathway | ✅ **Already designed.** `/deliver` documents "Local fast iteration — `pnpm hc:start` then `cd genesis/a2o && pnpm test:browser`… Screenshots land in `genesis/a2o/reports/screenshots/<feature-slug>/`" with a full artifact channel (screenshot + `cucumber-report-browser.json` + `errors-{device}.json`). CI fresh-render is the **fallback**. |
| **Playwright → browser launch in Che** | ❌ **The one broken thing.** |

**The single break:** `genesis/a2o/src/framework/world.ts:81-84` calls
`pw.chromium.launch({ headless })` with no `executablePath`/`channel`, so it needs Playwright's
**bundled** chromium. That browser is not installed — `~/.cache/ms-playwright` is empty.
Pointing `executablePath` at the image's system Chrome-for-Testing 131 instead **hangs ~54s
then drops the CDP connection** (`Target page, context or browser has been closed`) because
Playwright 1.58.2 expects a protocol-matched Chromium, not 131.

**Conclusion:** the Jenkins round-trip the operator has been living with is not the design — it
is the *symptom* of the local browser never launching in Che. Fix that one wiring gap and the
already-designed local loop comes alive for both consumers (cucumber `@browser` scenarios and
the new `look` primitive).

## Goal

1. A headless browser launches reliably in the Che container, durably across workspace
   recreation, with **no change to `world.ts`** — so the existing `pnpm test:browser` cucumber
   pathway runs locally in Che.
2. A lightweight, auth-aware **`look`** command: render any surface → screenshot + structured
   console/network/DOM capture → predictable paths the agent reads. The "glance at the app
   mid-iteration" primitive, lighter than writing a cucumber scenario.

## Non-goals (L1)

- The `/shift` + `/deliver` completion gate, the Objective `visual` block, the kickoff baseline
  ritual — all of that is **L2** (separate spec).
- Pixel-diffing, scriptable mini-flow languages, multi-tab/Tauri browser work.
- Baking the browser into the che-devworkspaces image — see "Eventual migration" below; that is
  operator-owned and deferred.

## Design — Part A: the wiring fix

**Chosen approach (A): install Playwright's version-matched chromium to a persistent path.**

- Add one env var to the workspace so the install survives container recreation. `HOME=/home/user`
  is **ephemeral** (not on the persistent `/projects` PVC), so `~/.cache/ms-playwright` would be
  wiped on every workspace restart. Persist it under `/projects`:
  - `devfile.yaml` → `components[tools].container.env`: add
    `PLAYWRIGHT_BROWSERS_PATH=/projects/.cache/ms-playwright`.
- Add an idempotent setup script, `pnpm a2o:setup` (in `genesis/a2o/package.json`):
  `playwright install chromium` only when the browser is absent at `PLAYWRIGHT_BROWSERS_PATH`.
  Safe to run repeatedly; ~1 min + ~170 MB on first run, no-op thereafter. (170 MB is negligible
  against the `/projects` PVC; system deps are already present — raw Chrome runs, `ldd` clean.)
- **Immediate bootstrap for the current workspace** (done during implementation, not just
  documented): export `PLAYWRIGHT_BROWSERS_PATH` and run `playwright install chromium` so the
  browser exists now, before the devfile change has propagated to a fresh workspace.
- **No change to `world.ts`.** Once the bundled chromium exists at the resolved path,
  `pw.chromium.launch({ headless })` resolves it. This fixes cucumber and `look` with one change.

**Rejected / deferred alternatives (record the reasoning):**

- **(B) Reuse system Chrome 131 via `executablePath`/`channel` + pin Playwright down to ~1.49.**
  Rejected: fights the 1.58.2 pin; the 1.58+131 mismatch is *exactly* the 54 s hang observed;
  downgrading a shared dep risks the whole a2o suite and its newer-API usage.
- **(C) Bake `playwright install --with-deps chromium` into the che-devworkspaces udi-plus image.**
  Deferred, not rejected — this is the *eventual* durable home (zero per-workspace cost). It is a
  cross-repo, operator-driven image rebuild (per the "operator owns image builds" rule). Documented
  as a follow-up; L1 lands repo-level value today and the env-var + setup approach forward-compats
  cleanly (when C lands, the setup script's check simply finds the browser already present).

## Design — Part B: the `look` primitive

A thin `genesis/a2o/scripts/look.ts`, exposed as `pnpm look`, that **reuses `PlaywrightDevice`**
so its console/pageerror/requestfailed capture and `getErrors()` are identical to what the
cucumber suite records — and the Part A fix lights it up for free.

### Interface

```
pnpm look <url> [--as <FixtureHuman>] [--doorway <id|url>] [--wait-testid <id>] \
                [--out <slug>] [--viewport <WxH>]
```

| Arg | Meaning |
|---|---|
| `<url>` (required) | Surface to render. URL-agnostic: local `ng serve`, full `hc:start` stack, or deployed alpha. |
| `--as <FixtureHuman>` | Optional. Log in as a fixture human (via `fixtureCredentials(name)` + the device's existing login path — the same one the `@auth` cucumber steps use) **before** navigating. This is the *auth-aware* depth. Omit → unauthenticated render. |
| `--doorway <id\|url>` | Optional. Doorway base URL for `--as` login (default: `E2E_DOORWAY_ALPHA`). |
| `--wait-testid <id>` | Optional. Wait for one `data-testid` to be visible before screenshotting. The single concession to "let it render" — **not** a scripting language (interactive flows stay in cucumber). |
| `--out <slug>` | Optional. Output subdir name. Default `latest` (stable overwrite, so the agent reads the same path each loop). A slug preserves a specific render. |
| `--viewport <WxH>` | Optional. Default `1280x800`. |

### Behavior

1. Launch its own browser with the same call `world.ts` uses (`chromium.launch({ headless: true })`,
   now resolvable via the Part A fix) — a standalone script, not a cucumber world. If the browser
   is missing, **fail fast** printing the exact remediation:
   `PLAYWRIGHT_BROWSERS_PATH=/projects/.cache/ms-playwright npx playwright install chromium`
   (or `pnpm a2o:setup`). Exit non-zero.
2. Construct `PlaywrightDevice`, call `init()` (wires console/pageerror/requestfailed capture).
3. If `--as`, perform fixture-human login first.
4. `goto(url)`, `waitForLoadState`, optional `--wait-testid` wait.
5. Full-page `screenshot` → `shot.png`.
6. Write `capture.json` (schema below). Print **both** absolute paths to stdout.
7. Exit code: 0 when `ok` (page loaded, no pageErrors, wait satisfied); non-zero otherwise — so a
   caller (human or the L2 loop) can branch on it.

### Output

Written to `genesis/a2o/reports/look/<latest|slug>/`:

- `shot.png` — full-page screenshot (the agent `Read`s this; multimodal).
- `capture.json`:

```jsonc
{
  "url": "<requested>",
  "finalUrl": "<after redirects>",
  "title": "<document.title>",
  "ok": true,
  "as": "Matthew | null",
  "viewport": "1280x800",
  "waitedFor": "data-testid=... | null",
  "durationMs": 1234,
  "console": [ { "type": "error|warning|log", "text": "..." } ],
  "pageErrors": [ "<uncaught JS error>" ],
  "failedRequests": [ { "url": "...", "failure": "..." } ]
}
```

`reports/look/` is gitignored.

> **L2 contract note:** this `shot.png` + `capture.json` pair is exactly what L2's kickoff
> baseline and done-candidate verdict consume. The format is fixed here so L2 is not a retrofit.

## Data flow

```
operator/agent ── pnpm look <url> [--as] ──▶ look.ts
                                              │  PlaywrightDevice.init() (capture wiring)
                                              │  [optional fixture-human login]
                                              │  goto → wait → screenshot
                                              ▼
                         reports/look/latest/{shot.png, capture.json}
                                              │
        Read(shot.png) [multimodal] + Read(capture.json) ──▶ agent sees + reasons
```

## Files touched

| File | Change |
|---|---|
| `devfile.yaml` | + `PLAYWRIGHT_BROWSERS_PATH=/projects/.cache/ms-playwright` env (durability for future workspaces) |
| `genesis/a2o/package.json` | + `a2o:setup` (idempotent browser install) and `look` scripts |
| `genesis/a2o/scripts/look.ts` | **new** — the primitive |
| `genesis/a2o/.gitignore` (or repo root) | ensure `reports/look/` is ignored |
| `genesis/a2o/CLAUDE.md` | + a `look` entry under **Tools** so the loop knows it exists |

No change to `world.ts`, the device classes, `selectors.ts`, or any `.feature` file.

## Verification (real, not asserted — both proven before "done")

1. `pnpm a2o:setup` installs chromium to the persistent path; re-run is a no-op.
2. `pnpm look https://doorway-alpha.elohim.host` produces `shot.png` + `capture.json{"ok":true}`;
   the implementer `Read`s `shot.png` and confirms it rendered.
3. `pnpm look <local-or-alpha-login-surface> --as Matthew --wait-testid <known-id>` renders the
   authenticated surface (proves the auth-aware path).
4. One existing `@browser` cucumber scenario renders locally in Che (`E2E_DEVICE_MODE=playwright`)
   instead of hanging — proving the shared wiring fix serves the cucumber consumer too.
5. Browser-missing remediation: temporarily point `PLAYWRIGHT_BROWSERS_PATH` at an empty dir and
   confirm `look` fails fast with the exact install command and a non-zero exit.

## Eventual migration (follow-up, operator-owned — not L1)

Fold `playwright install --with-deps chromium` into the che-devworkspaces udi-plus Dockerfile so
the browser ships in the image and the per-workspace `a2o:setup` install becomes a no-op
everywhere. Repo-side stays unchanged; `PLAYWRIGHT_BROWSERS_PATH` and `a2o:setup` forward-compat.
