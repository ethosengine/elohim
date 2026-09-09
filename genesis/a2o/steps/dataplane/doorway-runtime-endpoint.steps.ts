import { strict as assert } from 'node:assert';

import { When, Then } from '@cucumber/cucumber';

import { chromium } from 'playwright';

import { resolvePeerUrl } from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

interface ContentVisit {
  origin: string;
  metadataUrl: string;
  requests: string[];
  contentStatus?: number;
  contentId?: string;
  title?: string;
  visibleTitle: string;
  body: string;
  errors: string[];
}

const visits = new WeakMap<E2EWorld, ContentVisit>();

When(
  'an anonymous household reader opens {string} at {string} through doorway {string}',
  { timeout: 60_000 },
  async function (this: E2EWorld, contentId: string, path: string, doorway: string) {
    const origin = new URL(resolvePeerUrl(doorway)).origin;
    const metadataPath = `/db/content/${encodeURIComponent(contentId)}`;
    const visit: ContentVisit = {
      origin,
      metadataUrl: `${origin}${metadataPath}`,
      requests: [],
      visibleTitle: '',
      body: '',
      errors: [],
    };
    visits.set(this, visit);
    const browser = await chromium.launch({
      headless: true,
      args: ['--no-sandbox', '--disable-dev-shm-usage'],
    });
    try {
      const page = await browser.newPage();
      page.on('request', request => visit.requests.push(request.url()));
      page.on('pageerror', error => visit.errors.push(error.message));
      // Observe an actual application request; never fetch content on its behalf.
      const metadata = page
        .waitForResponse(response => response.url() === visit.metadataUrl, { timeout: 30_000 })
        .then(async response => {
          visit.contentStatus = response.status();
          const row = (await response.json()) as { id?: string; title?: string };
          visit.contentId = row.id;
          visit.title = row.title;
        })
        .catch(error => visit.errors.push(`content response: ${String(error)}`));
      const response = await page.goto(new URL(path, origin).href, {
        waitUntil: 'load',
        timeout: 30_000,
      });
      assert.equal(response?.status(), 200, 'visitor page must be served');
      assert.equal(new URL(page.url()).origin, origin, 'visit must remain on its doorway');
      await metadata;
      if (visit.contentId === contentId && visit.contentStatus === 200) {
        await page
          .locator('[data-testid="epr-home-focal"]')
          .waitFor({ state: 'visible', timeout: 15_000 });
        await page.waitForFunction(
          () =>
            (document.querySelector('[data-testid="epr-home-focal"]')?.textContent?.trim().length ??
              0) > 100,
          undefined,
          { timeout: 15_000 }
        );
        visit.visibleTitle = (
          await page.locator('[data-testid="epr-home-title"]').innerText()
        ).trim();
        visit.body = (await page.locator('[data-testid="epr-home-focal"]').innerText()).trim();
      }
      await page.waitForTimeout(1000);
      assert.equal(
        visit.contentId,
        contentId,
        'browser must receive the requested content identity'
      );
    } finally {
      await browser.close();
    }
  }
);

Then(
  'the app has fetched that content successfully through the visited doorway',
  function (this: E2EWorld) {
    const visit = visits.get(this);
    assert.ok(visit, 'visit the content first');
    assert.equal(visit.contentStatus, 200, JSON.stringify(visit));
    assert.ok(visit.requests.includes(visit.metadataUrl), 'app must issue the content request');
  }
);

Then("the reader sees that content's title and rendered body", function (this: E2EWorld) {
  const visit = visits.get(this);
  assert.ok(visit?.title, 'content metadata must name a title');
  assert.ok(visit.visibleTitle.includes(visit.title), 'rendered title must match received content');
  assert.ok(visit.body.length > 100, 'a shell or empty content body is not readable content');
  assert.deepEqual(visit.errors, [], 'content visit must have no uncaught errors');
});

Then('the app has sent no substrate requests to a different origin', function (this: E2EWorld) {
  const visit = visits.get(this);
  assert.ok(visit, 'visit the content first');
  const escaped = visit.requests.filter(raw => {
    const url = new URL(raw);
    return (
      /^\/(?:api|db|blob|epr-head|health)(?:\/|$)/.test(url.pathname) && url.origin !== visit.origin
    );
  });
  assert.deepEqual(escaped, [], 'content routing must not escape to the compiled production host');
});
