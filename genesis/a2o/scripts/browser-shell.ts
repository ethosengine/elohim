/** Visitor boot capture shared by the household story and CI. */
import { chromium } from 'playwright';

const rootSelector = 'app-root, lamad-root';

export async function captureBrowserShell(url: string, timeoutMs = 30_000, screenshot?: string) {
  const visit = {
    url,
    pageErrors: [] as string[],
    failedRequests: [] as { url: string; failure: string }[],
    httpErrors: [] as { url: string; status: number }[],
    rootText: '',
    rootPresent: false,
    bootstrapReady: false,
  };
  const browser = await chromium.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });
  try {
    const page = await browser.newPage();
    const expectedOrigin = new URL(url).origin;
    page.on('request', request => {
      const previous = request.redirectedFrom();
      if (
        previous &&
        new URL(previous.url()).origin === expectedOrigin &&
        new URL(request.url()).origin !== expectedOrigin
      ) {
        visit.pageErrors.push(`cross-doorway redirect: ${previous.url()} -> ${request.url()}`);
      }
    });
    page.on('pageerror', error => visit.pageErrors.push(error.message));
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
      if (new URL(page.url()).origin !== expectedOrigin)
        throw new Error(`navigation left doorway: ${url} -> ${page.url()}`);
      if (!response?.ok())
        throw new Error(`navigation returned ${response?.status() ?? 'no response'}`);
      await page.waitForFunction(
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
      const root = await page.locator(rootSelector).first().textContent();
      visit.rootText = root?.trim() ?? '';
      visit.rootPresent = true;
      visit.bootstrapReady =
        (await page.locator(rootSelector).first().getAttribute('data-app-ready')) === 'true';
    } catch (error) {
      visit.pageErrors.push(`navigation/boot: ${String(error)}`);
    }
    if (screenshot) await page.screenshot({ path: screenshot, fullPage: true });
  } finally {
    await browser.close();
  }
  return visit;
}

export function browserShellFailures(visit: Awaited<ReturnType<typeof captureBrowserShell>>) {
  const local = (url: string) => new URL(url).origin === new URL(visit.url).origin;
  return [
    ...visit.pageErrors,
    ...visit.failedRequests
      .filter(item => local(item.url))
      .map(item => `${item.url}: ${item.failure}`),
    ...visit.httpErrors
      .filter(item => local(item.url))
      .map(item => `${item.url}: HTTP ${item.status}`),
    ...(visit.bootstrapReady ? [] : ['client bootstrap did not complete']),
    ...(!visit.rootPresent || !visit.rootText ? ['application root did not render'] : []),
  ];
}
