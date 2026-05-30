# Che Browser Feedback — L1 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a headless browser launch reliably in the Eclipse Che container and ship an auth-aware `look` command that renders any surface to a screenshot + structured capture the agent can read.

**Architecture:** The only break is that `world.ts:81` needs Playwright's bundled chromium, which isn't installed (`~/.cache/ms-playwright` empty; `HOME` is ephemeral). Fix = install chromium to a persistent `/projects` path via an env var + idempotent setup script — **no `world.ts` change**. Then a thin `scripts/look.ts` reuses the existing `PlaywrightDevice` (identical console/network/error capture) to render → `shot.png` + `capture.json`.

**Tech Stack:** Node 22 ESM (NodeNext, `.js` import extensions), TypeScript via `tsx`, `node:test` runner, Playwright 1.58, the existing a2o framework (`PlaywrightDevice`, `fixtureCredentials`).

**Spec:** `genesis/docs/superpowers/specs/2026-05-30-che-browser-feedback-foundation-design.md`

---

## File Structure

| File | Responsibility |
|---|---|
| `devfile.yaml` (modify) | Persist `PLAYWRIGHT_BROWSERS_PATH` under the `/projects` PVC for future workspaces |
| `genesis/a2o/package.json` (modify) | `a2o:setup` (idempotent browser install) + `look` scripts |
| `genesis/a2o/scripts/look.ts` (create) | The primitive: `parseArgs` + `runLook` + CLI `main`. One cohesive command. |
| `genesis/a2o/scripts/__tests__/look.test.ts` (create) | Unit test for `parseArgs`; hermetic integration test for `runLook` against a `file://` URL |
| `genesis/a2o/CLAUDE.md` (modify) | Document `look` under **Tools** |

`reports/look/` needs **no** gitignore change — `genesis/a2o/.gitignore:3` already ignores `reports/`.

---

## Task 1: Persist the browser path + idempotent setup, and bootstrap this workspace

**Files:**
- Modify: `devfile.yaml` (env block under `components[].container.env`)
- Modify: `genesis/a2o/package.json` (`scripts`)

- [ ] **Step 1: Add the persistent browser-path env to `devfile.yaml`**

Find the env block and add the entry after `RUST_BACKTRACE`:

```yaml
        - name: RUST_BACKTRACE
          value: '1'
        - name: PLAYWRIGHT_BROWSERS_PATH
          value: /projects/.cache/ms-playwright
```

(Anchor: the existing `- name: RUST_BACKTRACE` / `value: '1'` pair in `components[0].container.env`.)

- [ ] **Step 2: Add `a2o:setup` and `look` scripts to `genesis/a2o/package.json`**

In the `"scripts"` object, add these two entries (place near `test:unit`):

```json
    "a2o:setup": "PLAYWRIGHT_BROWSERS_PATH=\"${PLAYWRIGHT_BROWSERS_PATH:-/projects/.cache/ms-playwright}\" playwright install chromium",
    "look": "tsx scripts/look.ts",
```

- [ ] **Step 3: Bootstrap the browser in THIS workspace**

The devfile env only reaches *future* workspaces; install now for the current one.

Run:
```bash
cd /projects/elohim/genesis/a2o && pnpm a2o:setup
```
Expected: Playwright downloads Chromium to `/projects/.cache/ms-playwright/chromium-*` (~1 min first time). Re-running is a no-op ("is already installed").

- [ ] **Step 4: Verify a real headless launch via the bundled browser**

Run (from `genesis/a2o`, so `playwright` resolves, with the persistent path):
```bash
cd /projects/elohim/genesis/a2o && \
PLAYWRIGHT_BROWSERS_PATH=/projects/.cache/ms-playwright node -e "import('playwright').then(async (pw)=>{const b=await pw.chromium.launch({headless:true});const p=await b.newPage();await p.setContent('<h1 data-testid=t>ok</h1>');console.log('LAUNCH_OK',await p.locator('[data-testid=t]').textContent());await b.close();}).catch(e=>{console.error('LAUNCH_FAIL',e.message);process.exit(1);})"
```
Expected: `LAUNCH_OK ok` and exit 0. (This is the fact that was failing — bundled chromium now resolves, no 54 s hang.)

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add devfile.yaml genesis/a2o/package.json
git commit -m "feat(a2o): persist Playwright browser under /projects + a2o:setup

world.ts:81 needs bundled chromium; HOME is ephemeral so install to the
persistent /projects PVC via PLAYWRIGHT_BROWSERS_PATH. a2o:setup is idempotent.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `look` argument parser (pure, TDD)

**Files:**
- Create: `genesis/a2o/scripts/look.ts`
- Test: `genesis/a2o/scripts/__tests__/look.test.ts`

- [ ] **Step 1: Write the failing test**

Create `genesis/a2o/scripts/__tests__/look.test.ts`:

```typescript
import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { parseArgs } from '../look.js';

describe('parseArgs', () => {
  it('parses a bare url', () => {
    const o = parseArgs(['https://example.test/path']);
    assert.equal(o.url, 'https://example.test/path');
    assert.equal(o.as, undefined);
  });

  it('parses all flags', () => {
    const o = parseArgs([
      'https://example.test',
      '--as', 'Matthew',
      '--doorway', 'https://doorway.test',
      '--wait-testid', 'app-root',
      '--out', 'my-slug',
      '--viewport', '800x600',
    ]);
    assert.equal(o.as, 'Matthew');
    assert.equal(o.doorway, 'https://doorway.test');
    assert.equal(o.waitTestid, 'app-root');
    assert.equal(o.out, 'my-slug');
    assert.deepEqual(o.viewport, { width: 800, height: 600 });
  });

  it('throws when url is missing', () => {
    assert.throws(() => parseArgs([]), /Usage: look/);
    assert.throws(() => parseArgs(['--as', 'Matthew']), /Usage: look/);
  });

  it('throws on a bad --viewport', () => {
    assert.throws(() => parseArgs(['https://x.test', '--viewport', 'huge']), /--viewport expects WxH/);
  });

  it('throws on an unknown flag', () => {
    assert.throws(() => parseArgs(['https://x.test', '--nope', 'v']), /Unknown flag: --nope/);
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/look.test.ts`
Expected: FAIL — cannot resolve `../look.js` (module does not exist yet).

- [ ] **Step 3: Create `look.ts` with types + `parseArgs` only**

Create `genesis/a2o/scripts/look.ts`:

```typescript
/**
 * `look` — lightweight auth-aware "render & see" primitive.
 *
 * Renders a URL (optionally as a logged-in fixture human), screenshots it,
 * and writes a structured console/network/DOM capture the agent reads.
 * Reuses PlaywrightDevice so observability matches the cucumber suite exactly.
 */

export interface LookOptions {
  url: string;
  as?: string;
  doorway?: string;
  waitTestid?: string;
  out?: string;
  viewport?: { width: number; height: number };
}

const USAGE =
  'Usage: look <url> [--as <FixtureHuman>] [--doorway <id|url>] ' +
  '[--wait-testid <id>] [--out <slug>] [--viewport <WxH>]';

export function parseArgs(argv: string[]): LookOptions {
  const args = [...argv];
  const url = args.shift();
  if (!url || url.startsWith('--')) throw new Error(USAGE);

  const opts: LookOptions = { url };
  for (let i = 0; i < args.length; i++) {
    const flag = args[i];
    const val = args[i + 1];
    switch (flag) {
      case '--as': opts.as = val; i++; break;
      case '--doorway': opts.doorway = val; i++; break;
      case '--wait-testid': opts.waitTestid = val; i++; break;
      case '--out': opts.out = val; i++; break;
      case '--viewport': {
        const m = /^(\d+)x(\d+)$/.exec(val ?? '');
        if (!m) throw new Error(`--viewport expects WxH (e.g. 1280x800), got: ${val}`);
        opts.viewport = { width: Number(m[1]), height: Number(m[2]) };
        i++;
        break;
      }
      default: throw new Error(`Unknown flag: ${flag}`);
    }
  }
  return opts;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/look.test.ts`
Expected: PASS — all `parseArgs` cases green.

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/look.ts genesis/a2o/scripts/__tests__/look.test.ts
git commit -m "feat(a2o): look primitive — argument parser

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `look` render core — `runLook` (unauth), reusing PlaywrightDevice

**Files:**
- Modify: `genesis/a2o/scripts/look.ts`
- Test: `genesis/a2o/scripts/__tests__/look.test.ts`

- [ ] **Step 1: Write the failing integration test (hermetic, `file://`)**

Append to `genesis/a2o/scripts/__tests__/look.test.ts`:

```typescript
import { mkdtemp, writeFile, readFile, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { runLook } from '../look.js';

describe('runLook (file:// hermetic render)', () => {
  it('renders a local file to shot.png + capture.json', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'look-test-'));
    const html = join(dir, 'page.html');
    await writeFile(
      html,
      '<!doctype html><title>Look Smoke</title><h1 data-testid="probe">rendered</h1>',
    );

    const result = await runLook({ url: pathToFileURL(html).href, out: 'unit-smoke' });

    assert.equal(result.ok, true);
    assert.equal(result.title, 'Look Smoke');
    assert.equal(result.as, null);
    assert.deepEqual(result.pageErrors, []);
    // Files exist and are non-empty.
    assert.ok((await stat(result.shotPath)).size > 0, 'shot.png written');
    const capture = JSON.parse(await readFile(result.capturePath, 'utf8'));
    assert.equal(capture.ok, true);
    assert.equal(capture.title, 'Look Smoke');
  });
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/look.test.ts`
Expected: FAIL — `runLook` is not exported from `../look.js`.

- [ ] **Step 3: Implement `runLook` in `look.ts`**

Add these imports at the top of `genesis/a2o/scripts/look.ts` (below the file doc comment):

```typescript
import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { fixtureCredentials } from '../src/framework/fixtures/humans.js';
```

Add the result type next to `LookOptions`:

```typescript
export interface LookResult {
  url: string;
  finalUrl: string;
  title: string;
  ok: boolean;
  as: string | null;
  viewport: string;
  waitedFor: string | null;
  durationMs: number;
  console: { type: string; text: string }[];
  pageErrors: string[];
  failedRequests: { url: string; failure?: string }[];
  shotPath: string;
  capturePath: string;
}

const DEFAULT_BROWSERS_PATH = '/projects/.cache/ms-playwright';
const REPORTS_DIR = 'reports/look';
```

Add `runLook` (after `parseArgs`):

```typescript
export async function runLook(opts: LookOptions): Promise<LookResult> {
  const started = Date.now();
  const viewport = opts.viewport ?? { width: 1280, height: 800 };
  const outDir = resolve(REPORTS_DIR, opts.out ?? 'latest');
  await mkdir(outDir, { recursive: true });
  const shotPath = join(outDir, 'shot.png');
  const capturePath = join(outDir, 'capture.json');
  const doorwayUrl = opts.doorway ?? process.env['E2E_DOORWAY_ALPHA'] ?? '';

  // Launch protocol-matched bundled chromium (Task 1's fix makes this resolve).
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let browser: any;
  try {
    const pw = await import('playwright');
    browser = await pw.chromium.launch({ headless: true });
  } catch (e) {
    const msg = (e as Error).message;
    if (/Executable doesn't exist|playwright install|ms-playwright/i.test(msg)) {
      const p = process.env['PLAYWRIGHT_BROWSERS_PATH'] ?? DEFAULT_BROWSERS_PATH;
      throw new Error(
        `No Playwright browser found. Install it:\n` +
          `  PLAYWRIGHT_BROWSERS_PATH=${p} npx playwright install chromium\n` +
          `  (or: pnpm a2o:setup)\n\nOriginal: ${msg}`,
      );
    }
    throw e;
  }

  const origin = safeOrigin(opts.url);
  const device = new PlaywrightDevice('look', origin, doorwayUrl, browser);
  let ok = true;
  let finalUrl = opts.url;
  let title = '';
  try {
    await device.init();
    // Override the device's default 1280x720 viewport.
    await (
      device.page as unknown as {
        setViewportSize(s: { width: number; height: number }): Promise<void>;
      }
    ).setViewportSize(viewport);

    if (opts.as) {
      const creds = fixtureCredentials(opts.as);
      await device.login({ identifier: creds.identifier, password: creds.password });
    }

    try {
      await device.page.goto(opts.url, { waitUntil: 'networkidle', timeout: 30_000 });
    } catch {
      ok = false; // nav/idle timeout — still capture what rendered
    }

    if (opts.waitTestid) {
      try {
        await device.page
          .locator(`[data-testid="${opts.waitTestid}"]`)
          .waitFor({ state: 'visible', timeout: 15_000 });
      } catch {
        ok = false;
      }
    }

    finalUrl = device.page.url();
    title = await device.page.title();
    await device.page.screenshot({ path: shotPath, fullPage: true });
  } finally {
    await browser.close();
  }

  if (device.pageErrors.length > 0) ok = false;

  const result: LookResult = {
    url: opts.url,
    finalUrl,
    title,
    ok,
    as: opts.as ?? null,
    viewport: `${viewport.width}x${viewport.height}`,
    waitedFor: opts.waitTestid ? `data-testid=${opts.waitTestid}` : null,
    durationMs: Date.now() - started,
    console: device.consoleLogs.map((c) => ({ type: c.level, text: c.text })),
    pageErrors: device.pageErrors.map((p) => p.message),
    failedRequests: device.failedRequests.map((r) => ({ url: r.url, failure: r.failure })),
    shotPath,
    capturePath,
  };
  await writeFile(capturePath, JSON.stringify(result, null, 2));
  return result;
}

/** Origin for the device's appUrl; tolerant of file:// and bad input. */
function safeOrigin(url: string): string {
  try {
    return new URL(url).origin;
  } catch {
    return '';
  }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cd /projects/elohim/genesis/a2o && pnpm exec tsx --test scripts/__tests__/look.test.ts`
Expected: PASS — `runLook` renders the file, writes both artifacts, `ok===true`, title `Look Smoke`. (Requires Task 1's browser install.)

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/look.ts genesis/a2o/scripts/__tests__/look.test.ts
git commit -m "feat(a2o): look render core — reuse PlaywrightDevice capture

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: `look` CLI wrapper + browser-missing guard verification

**Files:**
- Modify: `genesis/a2o/scripts/look.ts`

- [ ] **Step 1: Add the CLI `main` and run-if-main guard to `look.ts`**

Add to the imports at the top of `look.ts`:

```typescript
import { fileURLToPath } from 'node:url';
```

Append at the end of `look.ts`:

```typescript
async function main(): Promise<void> {
  let opts: LookOptions;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (e) {
    console.error((e as Error).message);
    process.exit(2);
  }
  const result = await runLook(opts);
  // Print the two paths the agent reads next.
  console.log(result.shotPath);
  console.log(result.capturePath);
  process.exit(result.ok ? 0 : 1);
}

// Run only when invoked directly (not when imported by tests).
if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((e) => {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(2);
  });
}
```

- [ ] **Step 2: Verify the CLI renders and prints paths**

Run:
```bash
cd /projects/elohim/genesis/a2o && \
printf '<!doctype html><title>CLI Smoke</title><h1>cli ok</h1>' > /tmp/look-cli.html && \
pnpm look "file:///tmp/look-cli.html" --out cli-smoke; echo "exit=$?"
```
Expected: prints `…/reports/look/cli-smoke/shot.png` and `…/capture.json`, then `exit=0`. Then `Read` the printed `shot.png` and confirm it shows "cli ok".

- [ ] **Step 3: Verify the browser-missing guard fails fast**

Run (point at an empty browsers path to simulate a fresh workspace):
```bash
cd /projects/elohim/genesis/a2o && \
PLAYWRIGHT_BROWSERS_PATH=/tmp/empty-browsers pnpm look "file:///tmp/look-cli.html" --out guard-test; echo "exit=$?"
```
Expected: prints the remediation block (`PLAYWRIGHT_BROWSERS_PATH=/tmp/empty-browsers npx playwright install chromium` / `pnpm a2o:setup`) and a non-zero `exit`. No hang.

- [ ] **Step 4: Confirm lint + typecheck are clean for the new file**

Run: `cd /projects/elohim/genesis/a2o && pnpm typecheck && pnpm exec eslint scripts/look.ts`
Expected: no errors. (The single `eslint-disable` for the browser `any` is intentional and matches the device-file idiom.)

- [ ] **Step 5: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/look.ts
git commit -m "feat(a2o): look CLI wrapper + browser-missing fail-fast guard

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: `--as` auth-aware render (verify against alpha)

The `--as` code path is already implemented (Task 3's `runLook` calls `device.login` when `opts.as` is set). This task **verifies** it end-to-end against a real doorway, since fixture-human login cannot be exercised hermetically.

**Files:** none changed (verification only). If a defect is found, fix it in `look.ts` and re-commit.

- [ ] **Step 1: Render an authenticated surface as a fixture human**

Run (uses the alpha doorway; `Matthew` is the admin fixture):
```bash
cd /projects/elohim/genesis/a2o && \
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
pnpm look "https://doorway-alpha.elohim.host/dashboard" \
  --as Matthew --doorway https://doorway-alpha.elohim.host \
  --wait-testid dashboard-tab-overview --out auth-smoke; echo "exit=$?"
```
Expected: `exit=0`; `reports/look/auth-smoke/capture.json` shows `"as":"Matthew"`, `"ok":true`, `"waitedFor":"data-testid=dashboard-tab-overview"`.

- [ ] **Step 2: Read the screenshot and confirm the authenticated state**

`Read` `genesis/a2o/reports/look/auth-smoke/shot.png`.
Expected: the dashboard renders as a logged-in user (not the login/threshold page). If it shows login, the auth injection failed — debug `device.login`/`injectAuth` before proceeding.

- [ ] **Step 3: Commit (only if a fix was needed)**

```bash
cd /projects/elohim
git add genesis/a2o/scripts/look.ts
git commit -m "fix(a2o): look --as auth-aware render

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```
If no fix was needed, skip — the path was already proven in Step 1.

---

## Task 6: Document `look` + verify the cucumber consumer

**Files:**
- Modify: `genesis/a2o/CLAUDE.md`

- [ ] **Step 1: Add a `look` entry to the Tools section of `genesis/a2o/CLAUDE.md`**

Under `## Tools`, append a bullet:

```markdown
- **Render & see (`look`)**: `pnpm look <url> [--as <FixtureHuman>] [--doorway <id|url>] [--wait-testid <id>] [--out <slug>] [--viewport WxH]` — renders a surface headless in Che, writes `reports/look/<latest|slug>/{shot.png,capture.json}`, prints both paths. The fast "glance at the app" loop; reuses `PlaywrightDevice` capture. First run needs `pnpm a2o:setup` (installs Chromium to `/projects/.cache/ms-playwright`).
```

- [ ] **Step 2: Verify the cucumber `@browser` consumer now runs locally in Che**

Run a single existing browser scenario against alpha (proves the shared wiring fix serves cucumber, not just `look`):
```bash
cd /projects/elohim/genesis/a2o && \
E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host \
pnpm test:browser -- --tags '@browser-only' 2>&1 | tail -30
```
Expected: the browser launches (no 54 s hang / "Target closed"); scenarios execute and report pass/fail. Any scenario *logic* failures are out of scope here — the success criterion is **the browser launches and the suite runs** where it previously could not.

- [ ] **Step 3: Commit**

```bash
cd /projects/elohim
git add genesis/a2o/CLAUDE.md
git commit -m "docs(a2o): document look render primitive in Tools

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage:**
- Wiring fix (Approach A): persistent `PLAYWRIGHT_BROWSERS_PATH` (Task 1 Step 1), idempotent `a2o:setup` (Task 1 Step 2), immediate bootstrap (Task 1 Step 3), no `world.ts` change (never touched) ✓
- `look` interface — url/`--as`/`--doorway`/`--wait-testid`/`--out`/`--viewport` (Task 2 parser; Task 3 behavior) ✓
- `capture.json` schema fields (Task 3 `LookResult`) ✓
- Reuse `PlaywrightDevice` capture (Task 3) ✓
- Stable `latest` overwrite default + `--out` slug (Task 3 `outDir`) ✓
- Fail-fast browser-missing guard with exact command (Task 3 catch; Task 4 Step 3 verify) ✓
- Auth-aware `--as` (Task 3 login branch; Task 5 verify) ✓
- `reports/look/` gitignored — already covered by `reports/` (noted; no task needed) ✓
- CLAUDE.md Tools doc (Task 6) ✓
- Cucumber consumer verified (Task 6 Step 2) ✓
- Verification: alpha render, auth render, cucumber run, guard (Tasks 4–6) ✓

**Placeholder scan:** No TBD/TODO; every code step shows complete code; commands have expected output. ✓

**Type consistency:** `LookOptions`/`LookResult` defined in Task 2/3 and used consistently; `parseArgs`/`runLook`/`main` names stable across tasks; `device.login({identifier,password})` matches verified `LoginRequest`; `fixtureCredentials` returns `{identifier,password,displayName}` (verified). ✓

**Out of scope (L2, separate plan):** Objective `visual` block, kickoff baseline ritual, `/shift`+`/deliver` done-gate, shared verdict procedure.
