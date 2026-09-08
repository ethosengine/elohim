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
import { createHash, randomBytes } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
export function buildFixtureBundle(opts: { coherent: boolean; baseHref?: string }): FixtureBundle {
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
      `    <base href="${opts.baseHref ?? './'}" />`,
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
        `root.setAttribute('data-fixture-stamp', ${JSON.stringify(stamp)});
root.setAttribute('data-app-ready', 'true');`,
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
    // Use the publisher's bounded retry policy. Canonical concurrent writers
    // can conflict after the deliberately induced peer restart; verdict2 stays terminal.
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
  bootstrapReady: boolean;
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
  const { captureBrowserShell } = await import('../../scripts/browser-shell.js');
  const started = Date.now();
  const capture = await captureBrowserShell(url, timeoutMs);
  if (Date.now() - started > 45_000)
    capture.pageErrors.push('Browser visit exceeded its 45-second deadline');
  return capture;
}

/** Poll `check` until it returns true or the bound expires; returns the elapsed ms or null. */
export async function pollUntil(
  check: () => Promise<boolean>,
  boundMs: number,
  intervalMs = 3_000
): Promise<number | null> {
  if (boundMs <= 0) return null;
  const started = Date.now();
  for (;;) {
    const ready = await check().catch(() => false);
    if (Date.now() - started >= boundMs) return null;
    if (ready) return Date.now() - started;
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

/** Server fixture implements the same export contract AngularRenderer loads. */
export function buildServerFixture(browser: FixtureBundle): FixtureBundle {
  const dir = mkdtempSync(join(fixtureRoot(), 'epr-server-'));
  writeFileSync(join(dir, 'version.json'), readFileSync(join(browser.dir, 'version.json')));
  const html = readFileSync(join(browser.dir, 'index.html'), 'utf8').replace(
    '<app-root></app-root>',
    `<app-root data-ssr-stamp="${browser.stamp}">SSR ${browser.stamp}</app-root>`
  );
  writeFileSync(
    join(dir, 'main.server.mjs'),
    `export default function bootstrap() {}\nexport async function renderApplication() { return ${JSON.stringify(html)}; }\n`
  );
  return { ...browser, dir };
}

export async function meshControl(action: string, ...args: string[]): Promise<void> {
  await execFileAsync(
    'bash',
    [join(REPO_ROOT, 'app/elohim-app/scripts/hc-mesh.sh'), action, ...args],
    {
      cwd: REPO_ROOT,
      env: process.env,
      timeout: 180_000,
      maxBuffer: 4 * 1024 * 1024,
    }
  );
}

/** Deliberate fault injection after the normal package/publish path refused. */
export async function stageInvalidFixture(
  bundle: FixtureBundle,
  doorwayUrl: string
): Promise<string> {
  if (bundle.coherent)
    throw new Error('fault injector accepts only a deliberately incoherent fixture');
  const archiveDir = mkdtempSync(join(fixtureRoot(), 'invalid-package-'));
  try {
    const archive = join(archiveDir, 'invalid.zip');
    await execFileAsync('zip', ['-X', '-qr', archive, '.'], { cwd: bundle.dir });
    const bytes = readFileSync(archive);
    const hash = `sha256-${createHash('sha256').update(bytes).digest('hex')}`;
    const response = await fetch(`${doorwayUrl}/admin/seed/blob`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/zip',
        'X-Blob-Hash': hash,
        'X-API-Key': process.env['STORAGE_API_KEY_ADMIN'] ?? '',
      },
      body: new Uint8Array(bytes),
      signal: AbortSignal.timeout(30_000),
    });
    const body = await response.text();
    if (
      !response.ok ||
      !(JSON.parse(body) as { forwarded_to_storage?: boolean }).forwarded_to_storage
    ) {
      throw new Error(`fault fixture did not reach storage: ${response.status} ${body}`);
    }
    return hash;
  } finally {
    rmSync(archiveDir, { recursive: true, force: true });
  }
}

/** Recorded PID plus start tick: PID reuse cannot disguise a renderer restart. */
export function doorwayIncarnations(): string[] {
  // eslint-disable-next-line sonarjs/publicly-writable-directories -- Read-only owned-mesh PID receipts; no temporary file creation.
  const mesh = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';
  return ['a', 'b'].map(name => readFileSync(join(mesh, 'pids', `doorway-${name}`), 'utf8').trim());
}

/** Read only the owned doorway's restart log; offsets distinguish this incarnation. */
export function doorwayRestartLog(name: string): string {
  // eslint-disable-next-line sonarjs/publicly-writable-directories -- read-only owned mesh receipt
  const mesh = process.env['MESH_DIR'] ?? '/tmp/elohim-local-mesh';
  try {
    return readFileSync(join(mesh, 'logs', `doorway-restart-${name}.log`), 'utf8');
  } catch {
    return '';
  }
}

/** Fixture authoring retries only the conductor's explicit optimistic-write conflict. */
export async function postFixtureCommitment(url: string, init: RequestInit) {
  const deadline = Date.now() + 60_000;
  for (let attempt = 0; ; attempt++) {
    const response = await fetch(url, {
      ...init,
      signal: AbortSignal.timeout(Math.max(1, deadline - Date.now())),
    });
    const text = await response.text();
    if (
      response.status !== 503 ||
      !text.includes('source chain head has moved since the bundle began') ||
      Date.now() >= deadline
    ) {
      return { ok: response.ok, status: response.status, text };
    }
    await new Promise(resolve =>
      setTimeout(resolve, Math.min(5000, 1000 * (attempt + 1), Math.max(1, deadline - Date.now())))
    );
  }
}
