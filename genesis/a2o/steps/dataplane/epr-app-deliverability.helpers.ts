/**
 * Fixtures for the EPR-app deliverability story
 * (features/dataplane/epr-app-deliverability.feature).
 *
 * Three jobs, and nothing else lives here:
 *   1. build a tiny but REAL browser bundle on disk (index.html + one entry
 *      script + one stylesheet + version.json), coherent or deliberately not;
 *   2. publish it through the SAME script CI publishes with
 *      (scripts/ci/stage-spa-blob.sh), so the story exercises the deploy path
 *      rather than a re-implementation of it;
 *   3. open a page in a real browser and report what the browser saw.
 *
 * Spec: genesis/docs/superpowers/specs/2026-09-08-epr-app-deliverability-through-doorway.md
 */

import { execFile } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

/** <repo>/genesis/a2o/steps/dataplane/<this file> → up four is the repo root. */
export const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');

/** The deploy path this story must exercise — never a re-implementation of it. */
export const STAGE_SPA_BLOB = join(REPO_ROOT, 'scripts', 'ci', 'stage-spa-blob.sh');

/**
 * Each doorway re-reads its peer's declared head on a fixed tick —
 * BUNDLE_HEADS_TICK_SECS in doorway-service (spec D1). Two ticks plus slack for
 * the read itself is the bound every convergence assertion in this story uses.
 */
export const BUNDLE_HEADS_TICK_SECS = 30;
export const CONVERGENCE_BOUND_MS = BUNDLE_HEADS_TICK_SECS * 2 * 1000 + 15_000;

/**
 * Where fixture bundles are built. Under the a2o reports tree (gitignored, and
 * owned by this repo) rather than a world-writable system temp directory.
 */
function fixtureRoot(): string {
  const root = process.env['A2O_FIXTURE_DIR'] ?? join(process.cwd(), 'reports', 'fixtures');
  mkdirSync(root, { recursive: true });
  return root;
}

export interface FixtureBundle {
  /** Directory holding the bundle's files — what stage-spa-blob.sh zips. */
  dir: string;
  /** `main-<token>.js`, the browser entry point the index page names. */
  entryScript: string;
  /** `styles-<token>.css`. */
  styleSheet: string;
  /** The build stamp written into version.json (`commit`). */
  stamp: string;
  /** False when the entry script was deliberately left out of the directory. */
  coherent: boolean;
}

/**
 * Build a bundle on disk that a browser can genuinely boot.
 *
 * The page leaves an empty `<app-root>` for the script to fill, which is the
 * same shape elohim-app's own index.html has — so one "the app root has
 * content" assertion reads the fixture app and the real app identically.
 *
 * `coherent: false` writes the index page that NAMES the entry script and then
 * does not write the script. That is the 2026-09-04 outage in miniature: every
 * other file resolves, and the app never starts.
 */
export function buildFixtureBundle(opts: { coherent: boolean }): FixtureBundle {
  // Content-addressed names, so two bundles built in the same millisecond are still
  // distinguishable — a collision here would make "the PREVIOUS era" unobservable.
  const token = `${Date.now().toString(36)}${randomBytes(4).toString('hex')}`.toUpperCase();
  const dir = mkdtempSync(join(fixtureRoot(), 'epr-app-'));
  const entryScript = `main-${token}.js`;
  const styleSheet = `styles-${token}.css`;
  const stamp = `fixture-${token}`;

  writeFileSync(
    join(dir, 'index.html'),
    [
      '<!doctype html>',
      '<html lang="en">',
      '  <head>',
      '    <meta charset="utf-8" />',
      `    <title>EPR app deliverability fixture ${token}</title>`,
      '    <base href="./" />',
      `    <link rel="stylesheet" href="${styleSheet}" />`,
      '  </head>',
      '  <body>',
      '    <app-root></app-root>',
      `    <script src="${entryScript}" type="module"></script>`,
      '  </body>',
      '</html>',
      '',
    ].join('\n')
  );

  writeFileSync(join(dir, styleSheet), 'app-root { display: block; font-family: sans-serif; }\n');

  if (opts.coherent) {
    writeFileSync(
      join(dir, entryScript),
      [
        '// The whole app: fill the root element the page left empty.',
        "const root = document.querySelector('app-root');",
        `root.textContent = 'EPR app deliverability fixture ${token} is running.';`,
        `root.setAttribute('data-fixture-stamp', ${JSON.stringify(stamp)});`,
        '',
      ].join('\n')
    );
  }

  writeFileSync(
    join(dir, 'version.json'),
    `${JSON.stringify(
      {
        commit: stamp,
        version: '0.0.0-fixture',
        buildTime: new Date().toISOString(),
        environment: 'mesh-fixture',
        service: 'epr-app-deliverability-fixture',
      },
      null,
      2
    )}\n`
  );

  return { dir, entryScript, styleSheet, stamp, coherent: opts.coherent };
}

export function removeFixtureBundle(bundle: FixtureBundle): void {
  rmSync(bundle.dir, { recursive: true, force: true });
}

export interface StageOutcome {
  /** stage-spa-blob.sh's exit code. 2 = the peer judged the bundle BROKEN. */
  code: number;
  output: string;
  /** `sha256-…`, parsed off the script's own "blob hash:" line. */
  blobHash: string;
}

/**
 * Publish a bundle exactly the way CI publishes one.
 *
 * `declare: false` hands the peer the BYTES only (DO_PATCH unset) — the leg CI
 * runs against every serving host. `declare: true` also PATCHes the head onto
 * the record (DO_PATCH=1) — the leg CI runs exactly once, through whichever
 * doorway can reach a live conductor.
 */
export async function stageBundle(opts: {
  bundle: FixtureBundle;
  slug: string;
  doorwayUrl: string;
  declare: boolean;
  kind?: 'browser' | 'server';
}): Promise<StageOutcome> {
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    DO_PATCH: opts.declare ? '1' : '0',
    // One attempt: a household mesh this run owns has no cluster churn to ride
    // out, and a retry ladder would only blur which leg actually failed.
    STAGE_BLOB_ATTEMPTS: '1',
  };
  let code = 0;
  let output = '';
  try {
    const { stdout, stderr } = await execFileAsync(
      'bash',
      [STAGE_SPA_BLOB, opts.bundle.dir, opts.slug, opts.doorwayUrl, opts.kind ?? 'browser'],
      { env, cwd: REPO_ROOT, maxBuffer: 8 * 1024 * 1024 }
    );
    output = `${stdout}\n${stderr}`;
  } catch (error) {
    const failure = error as { code?: number; stdout?: string; stderr?: string; message?: string };
    code = typeof failure.code === 'number' ? failure.code : 1;
    output = `${failure.stdout ?? ''}\n${failure.stderr ?? ''}\n${failure.message ?? ''}`;
  }
  const match = /blob hash:\s*(sha256-[0-9a-f]+)/i.exec(output);
  return { code, output, blobHash: match?.[1] ?? '' };
}

export interface BrowserVisit {
  url: string;
  /** Uncaught JS errors the page threw. */
  pageErrors: string[];
  /** Network-level failures (aborted, DNS, connection refused). */
  failedRequests: { url: string; failure: string }[];
  /** HTTP responses the page received with a status >= 400. */
  httpErrors: { url: string; status: number }[];
  /** Text content of the app's root element after load, trimmed. */
  rootText: string;
  /** True when an `<app-root>` element was present at all. */
  rootPresent: boolean;
}

/**
 * Open one URL in a real headless browser and report what the browser saw.
 *
 * Deliberately NOT PlaywrightDevice: that device exists to drive a logged-in
 * human through the app, and every assertion here is about a page a stranger is
 * handed before any identity exists. Same capture surface (console, pageerror,
 * requestfailed) plus the HTTP error RESPONSES `look` had to add, because a
 * 404'd entry script never appears as a network FAILURE.
 */
export async function visitInBrowser(url: string, timeoutMs = 30_000): Promise<BrowserVisit> {
  const { chromium } = await import('playwright');
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });
  const visit: BrowserVisit = {
    url,
    pageErrors: [],
    failedRequests: [],
    httpErrors: [],
    rootText: '',
    rootPresent: false,
  };
  try {
    const page = await browser.newPage();
    page.on('pageerror', error => visit.pageErrors.push(error.message));
    page.on('requestfailed', request =>
      visit.failedRequests.push({
        url: request.url(),
        failure: request.failure()?.errorText ?? 'unknown',
      })
    );
    page.on('response', response => {
      if (response.status() >= 400) {
        visit.httpErrors.push({ url: response.url(), status: response.status() });
      }
    });
    await page.goto(url, { waitUntil: 'networkidle', timeout: timeoutMs }).catch(() => undefined);
    const root = await page.evaluate(() => {
      const el = document.querySelector('app-root');
      return { present: el !== null, text: (el?.textContent ?? '').trim() };
    });
    visit.rootPresent = root.present;
    visit.rootText = root.text;
  } finally {
    await browser.close().catch(() => undefined);
  }
  return visit;
}

/** Poll `check` until it returns true or the bound expires; returns the elapsed ms or null. */
export async function pollUntil(
  check: () => Promise<boolean>,
  boundMs: number,
  intervalMs = 3_000
): Promise<number | null> {
  const started = Date.now();
  for (;;) {
    if (await check()) return Date.now() - started;
    if (Date.now() - started >= boundMs) return null;
    await new Promise(done => setTimeout(done, intervalMs));
  }
}

/** Only the requests the browser sent to THIS doorway — nobody else's contract. */
export function sameOrigin(entryUrl: string, candidate: string): boolean {
  try {
    return new URL(candidate).origin === new URL(entryUrl).origin;
  } catch {
    return false;
  }
}
