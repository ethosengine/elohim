/**
 * `look` — lightweight auth-aware "render & see" primitive.
 *
 * Renders a URL (optionally as a logged-in fixture human), screenshots it,
 * and writes a structured console/network/DOM capture the agent reads.
 * Reuses PlaywrightDevice so observability matches the cucumber suite exactly.
 */

import { mkdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium, type Browser } from 'playwright';

import { PlaywrightDevice } from '../src/framework/devices/playwright-device.js';
import { fixtureCredentials } from '../src/framework/fixtures/humans.js';

export interface LookOptions {
  url: string;
  as?: string;
  doorway?: string;
  waitTestid?: string;
  out?: string;
  viewport?: { width: number; height: number };
}

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
  /** HTTP responses with status >= 400 — the URLs behind bare console 404s. */
  httpErrors: { url: string; status: number }[];
  shotPath: string;
  capturePath: string;
}

const REPORTS_DIR = 'reports/look';

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
      case '--as':
        opts.as = val;
        i++;
        break;
      case '--doorway':
        opts.doorway = val;
        i++;
        break;
      case '--wait-testid':
        opts.waitTestid = val;
        i++;
        break;
      case '--out':
        opts.out = val;
        i++;
        break;
      case '--viewport': {
        const m = /^(\d+)x(\d+)$/.exec(val ?? '');
        if (!m) throw new Error(`--viewport expects WxH (e.g. 1280x800), got: ${val}`);
        opts.viewport = { width: Number(m[1]), height: Number(m[2]) };
        i++;
        break;
      }
      default:
        throw new Error(`Unknown flag: ${flag}`);
    }
  }
  return opts;
}

export async function runLook(opts: LookOptions): Promise<LookResult> {
  const started = Date.now();
  const viewport = opts.viewport ?? { width: 1280, height: 800 };
  const outDir = resolve(REPORTS_DIR, opts.out ?? 'latest');
  await mkdir(outDir, { recursive: true });
  const shotPath = join(outDir, 'shot.png');
  const capturePath = join(outDir, 'capture.json');
  const doorwayUrl = opts.doorway ?? process.env['E2E_DOORWAY_ALPHA'] ?? '';

  // Launch the version-matched browser (Task 1 provisioned it via the XDG cache).
  let browser: Browser;
  try {
    browser = await chromium.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-dev-shm-usage'],
    });
  } catch (e) {
    const msg = (e as Error).message;
    if (/Executable doesn't exist|playwright install|ms-playwright/i.test(msg)) {
      throw new Error(
        `No Playwright browser found. Provision it with:\n` +
          `  cd genesis/a2o && pnpm a2o:setup\n\nOriginal: ${msg}`
      );
    }
    throw e;
  }

  // PlaywrightDevice declares a minimal structural browser stub; the real
  // Browser satisfies it but isn't nominally assignable.
  const device = new PlaywrightDevice(
    'look',
    safeOrigin(opts.url),
    doorwayUrl,
    browser as unknown as ConstructorParameters<typeof PlaywrightDevice>[3]
  );

  let result: LookResult;
  const httpErrors: { url: string; status: number }[] = [];
  try {
    await device.init();
    // PlaywrightDevice's failedRequests only carries network-level failures
    // (requestfailed); HTTP error RESPONSES (4xx/5xx) never surface there, so
    // capture.json showed "404" console noise with no URL. Record them here —
    // additive field, framework device untouched (other consumers unaffected).
    device.page.on('response', (...args: unknown[]) => {
      const resp = args[0] as { url(): string; status(): number };
      const status = resp.status();
      if (status >= 400) httpErrors.push({ url: resp.url(), status });
    });
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

    let ok = true;
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

    const finalUrl = device.page.url();
    const title = await device.page.title();
    await device.page.screenshot({ path: shotPath, fullPage: true });
    if (device.pageErrors.length > 0) ok = false;

    result = {
      url: opts.url,
      finalUrl,
      title,
      ok,
      as: opts.as ?? null,
      viewport: `${viewport.width}x${viewport.height}`,
      waitedFor: opts.waitTestid ? `data-testid=${opts.waitTestid}` : null,
      durationMs: Date.now() - started,
      console: device.consoleLogs.map(c => ({ type: c.level, text: c.text })),
      pageErrors: device.pageErrors.map(p => p.message),
      failedRequests: device.failedRequests.map(r => ({ url: r.url, failure: r.failure })),
      httpErrors,
      shotPath,
      capturePath,
    };
  } finally {
    await browser.close();
  }

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
  main().catch(e => {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(2);
  });
}
