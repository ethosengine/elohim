import { strict as assert } from 'node:assert';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { resolvePeerUrl } from '../../src/framework/dataplane/surfaces.js';
import { loadHouseholdMeshFixture } from '../../src/framework/fixtures/household-mesh.js';

import { omni, chip, mark, backSelector, panel, frameSelector } from './chrome-testids.js';

import type {
  ChromeContext,
  ContentIsland,
} from '../../../../app/elohim-app/src/app/generated/chrome-context.js';
import type { E2EWorld } from '../../src/framework/world.js';
import type { BrowserVisit } from '../../steps/dataplane/epr-app-deliverability.helpers.js';
import type { BrowserContext, Page } from 'playwright';

export type Content = ContentIsland;
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../../../..');
export async function island(page: Page): Promise<ChromeContext> {
  return page
    .locator('script#elohim-omni-context')
    .evaluate(el => JSON.parse(el.textContent ?? '{}') as ChromeContext);
}
export async function content(page: Page): Promise<Content> {
  const c = (await island(page)).content;
  assert.ok(c, 'Missing content island');
  return c;
}
export function entryUrl(slug: string, peerName: string): string {
  return `${resolvePeerUrl(peerName)}/apps/${encodeURIComponent(slug)}/index.html`;
}
/** Browser-only paired-name entry; Node probes and ordinary delivery keep entryUrl. */
export function chromeEntryUrl(slug: string, peerName: string): string {
  const url = new URL(entryUrl(slug, peerName));
  const doorway =
    peerName === 'alpha-A' ||
    peerName === 'elohim.host' ||
    peerName.startsWith('E2E_DOORWAY_') ||
    Object.values(loadHouseholdMeshFixture().doorways ?? {}).some(
      peer => peer.url && new URL(peer.url).origin === url.origin
    );
  if (doorway && ['localhost', '127.0.0.1'].includes(url.hostname))
    url.hostname = process.env['MESH_APPS_PUBLIC_NAME'] ?? 'elohim.localhost';
  return url.href;
}
export function requireShellVisit(visit: BrowserVisit, peerName: string): ShellVisit {
  assert.ok('mainHeaders' in visit, `${peerName}: no shell visit recorded`);
  return visit as ShellVisit;
}
export interface AppsWorker {
  script: string;
}

/** Serve this source's worker on every origin, including nested content frames. */
export async function routeAppsWorker(context: BrowserContext, worker: AppsWorker): Promise<void> {
  await context.route('**/apps-sw.js', async route =>
    route.fulfill({ body: worker.script, contentType: 'application/javascript' })
  );
}

export async function buildAppsWorker(): Promise<AppsWorker> {
  try {
    // eslint-disable-next-line import/no-extraneous-dependencies -- Reuse the installed build tool.
    const { build } = await import('esbuild');
    const result = await build({
      absWorkingDir: REPO_ROOT,
      entryPoints: ['app/elohim-app/src/apps-sw.ts'],
      bundle: true,
      format: 'iife',
      target: 'es2020',
      write: false,
    });
    return { script: result.outputFiles[0].text };
  } catch (error) {
    throw new Error(
      `Could not build apps worker from ${REPO_ROOT}/app/elohim-app/src/apps-sw.ts: ${String(error)}`
    );
  }
}

/**
 * What a browser saw when it opened a content app's entry address as a PAGE.
 *
 * Extends the ordinary visit so the reused boot steps ("reported no uncaught
 * error", "every asset … arrived") read it unchanged: the capture below listens
 * at the page level, which includes every request and error the FRAME makes.
 * The top document is the doorway's shell, so the root/bootstrap fields of the
 * base shape describe nothing there; the app's own are the `frame*` fields.
 */
export interface ShellVisit extends BrowserVisit {
  /** Available only when the scenario retains the browser session. */
  page?: Page;
  /** Status of the main (top-level document) response. */
  mainStatus: number;
  mainFromServiceWorker: boolean;
  /** Lower-cased header names of the main response. */
  mainHeaders: Record<string, string>;
  /** Console messages of type `error`, from the top document and the frame. */
  consoleErrors: string[];
  /** Island and rendered Holders card captured from the mounted chip. */
  content?: Content;
  holdersText?: string;
  holdersHref?: string | null;
  /** The frame element's `src` attribute as written, and its resolved form. */
  frameSrc: string | null;
  frameSrcResolved: string | null;
  /** The framed app's own document: its url, root and bootstrap marker. */
  frameUrl: string | null;
  frameRootPresent: boolean;
  frameRootText: string;
  frameBootstrapReady: boolean;
}

/**
 * Open one URL as a top-level page and report the shell the doorway wrapped
 * around it, then wait for the app in the shell's frame to start in the frame's
 * own document. Same capture surface as `captureBrowserShell`, widened to the
 * frame (`page.on` events already include subframe traffic).
 */
export interface ShellVisitSession {
  blockServiceWorkers?: boolean;
  requireOriginPair?: boolean;
  prepare(page: import('playwright').Page): Promise<void>;
  retain(browser: import('playwright').Browser, page: import('playwright').Page): void;
}
export const shellVisitSessions = new WeakMap<E2EWorld, ShellVisitSession>();

export async function visitShellInBrowser(
  url: string,
  timeoutMs = 30_000,
  appsWorker?: AppsWorker,
  session?: ShellVisitSession,
  finalOrigin = new URL(url).origin
): Promise<ShellVisit> {
  const { chromium } = await import('playwright');
  const rootSelector = 'app-root, lamad-root';
  const visit: ShellVisit = {
    url,
    pageErrors: [],
    failedRequests: [],
    httpErrors: [],
    rootText: '',
    rootPresent: false,
    bootstrapReady: false,
    mainStatus: 0,
    mainFromServiceWorker: false,
    mainHeaders: {},
    consoleErrors: [],

    frameSrc: null,
    frameSrcResolved: null,
    frameUrl: null,
    frameRootPresent: false,
    frameRootText: '',
    frameBootstrapReady: false,
  };
  let started = Date.now();
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });
  try {
    const page = await browser.newPage({
      serviceWorkers: session?.blockServiceWorkers ? 'block' : 'allow',
    });
    session?.retain(browser, page);
    if (session) visit.page = page;
    await session?.prepare(page);
    const expectedOrigin = new URL(url).origin;
    if (appsWorker) {
      await page
        .context()
        .route(`${expectedOrigin}/apps-sw.js`, async route =>
          route.fulfill({ body: appsWorker.script, contentType: 'application/javascript' })
        );
      await page.goto(`${expectedOrigin}/`, { waitUntil: 'domcontentloaded', timeout: timeoutMs });
      try {
        await page.evaluate(async () => {
          await navigator.serviceWorker.register('/apps-sw.js', { scope: '/apps/' });
        });
        await page.waitForFunction(
          async () => {
            const registration = await navigator.serviceWorker.getRegistration('/apps/');
            return registration?.active?.state === 'activated';
          },
          undefined,
          { timeout: 30_000 }
        );
      } catch (error) {
        assert.fail(`Apps worker did not activate on ${expectedOrigin}: ${String(error)}`);
      }
    }
    // Capture the entry visit, excluding the worker's installation page.
    if (appsWorker) started = Date.now();
    page.on('pageerror', error => visit.pageErrors.push(error.message));
    page.on('console', message => {
      if (message.type() === 'error') visit.consoleErrors.push(message.text());
    });
    page.on('requestfailed', request =>
      visit.failedRequests.push({
        url: request.url(),
        failure: request.failure()?.errorText ?? 'unknown',
      })
    );
    page.on('response', response => {
      if (response.status() >= 400)
        visit.httpErrors.push({ url: response.url(), status: response.status() });
    });
    try {
      const response = await page.goto(url, { waitUntil: 'load', timeout: timeoutMs });
      if (new URL(page.url()).origin !== finalOrigin)
        throw new Error(`navigation left doorway: ${url} -> ${page.url()}`);
      visit.mainStatus = response?.status() ?? 0;
      visit.mainFromServiceWorker = response?.fromServiceWorker() ?? false;
      visit.mainHeaders = response?.headers() ?? {};
      if (!response?.ok())
        throw new Error(`navigation returned ${response?.status() ?? 'no response'}`);

      // The pill container has zero height; its absolutely positioned chip is visible.
      await page.locator(chip).waitFor({ state: 'visible', timeout: timeoutMs });
      visit.content = await content(page);
      if (session?.requireOriginPair)
        assert.equal(
          visit.content.isolation.boundary,
          'origin-pair',
          `${new URL(page.url()).host}: doorway name is not apps-paired; check DOORWAY_APPS_NAMES and MESH_APPS_PUBLIC_NAME`
        );
      if (!session) {
        visit.holdersText = await card(page, 'holders');
        visit.holdersHref = await page
          .locator('[data-page-info-card="holders"] [data-page-info-epr-link]')
          .getAttribute('href');
        await page.keyboard.press('Escape');
        // Escape closes page info; the open omnibar hides the chip, so collapse it with its ×.
        if ((await page.locator(omni).getAttribute('data-omni-state')) === 'expanded')
          await page.locator(`${omni} .omni-collapse`).click();
      }
      const frameElement = page.locator(frameSelector);
      await frameElement.waitFor({ state: 'visible', timeout: timeoutMs });
      if ((await frameElement.count()) > 0) {
        visit.frameSrc = await frameElement.getAttribute('src');
        if (visit.frameSrc) visit.frameSrcResolved = new URL(visit.frameSrc, page.url()).toString();
        const handle = await frameElement.elementHandle();
        const frame = await handle?.contentFrame();
        if (frame) {
          visit.frameUrl = frame.url();
          await frame.waitForFunction(
            (selector: string) => {
              const root = document.querySelector(selector);
              return (
                root?.getAttribute('data-app-ready') === 'true' &&
                !!root.textContent?.trim() &&
                root.getBoundingClientRect().height > 0
              );
            },
            rootSelector,
            { timeout: timeoutMs }
          );
          // Include deferred startup failures after the root first paints.
          await page.waitForTimeout(1000);
          const root = frame.locator(rootSelector).first();
          visit.frameRootText = (await root.textContent())?.trim() ?? '';
          visit.frameRootPresent = true;
          visit.frameBootstrapReady = (await root.getAttribute('data-app-ready')) === 'true';
        }
      }
    } catch (error) {
      visit.pageErrors.push(`navigation/boot: ${String(error)}`);
    }
  } finally {
    await (session ? Promise.resolve() : browser.close());
  }
  if (Date.now() - started > 45_000)
    visit.pageErrors.push('Browser visit exceeded its 45-second deadline');
  return visit;
}

export async function openInfo(p: Page): Promise<void> {
  if ((await p.locator(omni).getAttribute('data-omni-state')) !== 'expanded')
    await p.locator(chip).click();
  if (!(await p.locator(panel).isVisible())) await p.locator(mark).click();
  const back = p.locator(backSelector);
  if (await back.isVisible()) await back.click();
}
export async function card(p: Page, row: string): Promise<string> {
  await openInfo(p);
  await p.locator(`[data-page-info-row="${row}"]`).click();
  return (await p.locator(`[data-page-info-card="${row}"]`).innerText()).trim();
}
export async function checkLink(p: Page, row: string): Promise<void> {
  await card(p, row);
  const c = await content(p);
  const link = p.locator(`[data-page-info-card="${row}"] [data-page-info-epr-link]`);
  const href = await link.getAttribute('href');
  assert.ok(href);
  const url = new URL(href, p.url());
  assert.equal(url.origin, new URL(p.url()).origin);
  assert.equal(url.pathname, `/epr/${c.recordId}`);
  assert.equal(url.searchParams.get('via'), c.delivery.path.find(h => h.role === 'doorway')?.id);
  assert.equal(url.searchParams.get('peer'), c.delivery.path.find(h => h.role === 'holder')?.peer);
  assert.equal(url.hash, `#${row}`);
  assert.equal(
    await link.evaluate(
      el => el.parentElement === el.closest('[data-page-info-card]')?.lastElementChild
    ),
    true
  );
}
