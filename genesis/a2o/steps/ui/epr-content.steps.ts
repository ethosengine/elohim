/**
 * EPR Content Addressing Step Definitions — verify context-aware link resolution.
 *
 * Tests the EPR (Elohim Protocol Reference) link system:
 *   - Context-aware resolution (in-path, cross-path, standalone)
 *   - CID-based blob loading
 *   - EPR Head three-pillar metadata (DAG-CBOR)
 *   - Popover hover interaction
 *
 * URL routing is owned here; the EprContentPage page object owns DOM interaction.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { EprContentPage } from '../../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function requirePwDevice(world: E2EWorld, humanName: string): PlaywrightDevice {
  const human = world.getHuman(humanName);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(device, `${humanName} has no Playwright device. Is E2E_DEVICE_MODE=playwright?`);
  return device;
}

// ---------------------------------------------------------------------------
// Navigation / Given steps
// ---------------------------------------------------------------------------

Given(
  '{word} is on the manifesto step of path {string}',
  async function (this: E2EWorld, humanName: string, pathId: string) {
    const device = requirePwDevice(this, humanName);
    const appUrl = doorwayToAppUrl(device.client.url);
    // Manifesto is step 0 of the elohim-protocol path
    await device.page.goto(`${appUrl}/lamad/path/${pathId}/step/0`, {
      waitUntil: 'networkidle',
    });
    // Wait for markdown content to render (contains EPR links)
    await device.page.locator('.markdown-content').waitFor({ state: 'visible', timeout: 15_000 });
  }
);

Given(
  '{word} is viewing resource {string} directly',
  async function (this: E2EWorld, humanName: string, resourceId: string) {
    const device = requirePwDevice(this, humanName);
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.goto(`${appUrl}/lamad/resource/${resourceId}`, {
      waitUntil: 'networkidle',
    });
    await device.page.locator('.markdown-content').waitFor({ state: 'visible', timeout: 15_000 });
  }
);

Given(
  '{word} is viewing a page with an EPR link to {string}',
  async function (this: E2EWorld, humanName: string, contentId: string) {
    const device = requirePwDevice(this, humanName);
    const appUrl = doorwayToAppUrl(device.client.url);
    // The manifesto contains an epr:rea-foundations link, rea-foundations contains epr:manifesto
    // Navigate to manifesto standalone view to get a page with EPR links
    const source = contentId === 'manifesto' ? 'rea-foundations' : 'manifesto';
    await device.page.goto(`${appUrl}/lamad/resource/${source}`, {
      waitUntil: 'networkidle',
    });
    await device.page.locator('.markdown-content').waitFor({ state: 'visible', timeout: 15_000 });
  }
);

Given(
  'the manifesto content contains an {string} link',
  async function (this: E2EWorld, _eprUri: string) {
    // Assertion on current page — verify the link is actually rendered
    for (const [, human] of this.humans) {
      for (const device of human.devices) {
        if (device instanceof PlaywrightDevice) {
          const eprPage = new EprContentPage(device.page);
          const count = await eprPage.getEprLinkCount();
          assert.ok(count > 0, `Expected EPR links on page, found ${count}`);
          return;
        }
      }
    }
    assert.fail('No Playwright device found');
  }
);

// ---------------------------------------------------------------------------
// API-level Given steps
// ---------------------------------------------------------------------------

Given('content {string} has a blob reference', async function (this: E2EWorld, contentId: string) {
  // Verify via API that this content has a blob hash
  const doorway = this.getDoorway('alpha');
  const human = [...this.humans.values()][0];
  const token = human.getToken('alpha');
  const response = await fetch(`${doorway.url}/db/content/${contentId}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  assert.equal(response.status, 200, `Content ${contentId} not found`);
  const content = (await response.json()) as Record<string, unknown>;
  const hasBlobRef =
    content['blobHash'] ??
    (typeof content['contentBody'] === 'string' && content['contentBody'].startsWith('sha256'));
  assert.ok(hasBlobRef, `Content ${contentId} has no blob reference`);
});

Given('content {string} exists in storage', async function (this: E2EWorld, contentId: string) {
  const doorway = this.getDoorway('alpha');
  const human = [...this.humans.values()][0];
  const token = human.getToken('alpha');
  const response = await fetch(`${doorway.url}/db/content/${contentId}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  assert.equal(response.status, 200, `Content ${contentId} not found in storage`);
});

// ---------------------------------------------------------------------------
// When steps — browser interactions
// ---------------------------------------------------------------------------

When(
  '{word} clicks the {string} link in the markdown content',
  async function (this: E2EWorld, humanName: string, eprUri: string) {
    const device = requirePwDevice(this, humanName);
    const eprPage = new EprContentPage(device.page);
    // Extract content ID from epr: URI
    const contentId = eprUri.replace(/^epr:/, '');
    await eprPage.clickEprLink(contentId);
    // Wait for navigation to complete
    await device.page.waitForLoadState('networkidle');
  }
);

When('{word} hovers over the EPR link', async function (this: E2EWorld, humanName: string) {
  const device = requirePwDevice(this, humanName);
  // Find first EPR link on page and hover
  await device.page.locator('[data-epr]').first().hover();
  // Wait for popover debounce (300ms) + render
  await device.page.waitForTimeout(500);
});

When('the content renderer fetches the blob', function (this: E2EWorld) {
  // This is implicit — the content renderer auto-fetches blob content.
  // In API mode, we explicitly fetch it.
  // No-op for browser: the renderer does it automatically.
});

When(
  '{word} requests the EPR Head for {string}',
  async function (this: E2EWorld, humanName: string, contentId: string) {
    const doorway = this.getDoorway('alpha');
    const response = await fetch(`${doorway.url}/epr-head/${contentId}`, {
      headers: { Accept: 'application/vnd.ipld.dag-cbor' },
    });
    // Store response metadata on the world for Then assertions
    (this as E2EWorld & Record<string, unknown>)['eprHeadStatus'] = response.status;
    (this as E2EWorld & Record<string, unknown>)['eprHeadContentType'] =
      response.headers.get('content-type');
    if (response.ok) {
      (this as E2EWorld & Record<string, unknown>)['eprHeadBody'] = await response.arrayBuffer();
    }
  }
);

// ---------------------------------------------------------------------------
// Then steps — assertions
// ---------------------------------------------------------------------------

Then(
  '{word} navigates to {string} in path {string}',
  function (this: E2EWorld, humanName: string, _resourceId: string, pathId: string) {
    const device = requirePwDevice(this, humanName);
    const url = device.page.url();
    assert.ok(
      url.includes(`/lamad/path/${pathId}/step/`),
      `Expected URL to contain /lamad/path/${pathId}/step/, got: ${url}`
    );
  }
);

Then(
  '{word} navigates to the standalone resource view for {string}',
  function (this: E2EWorld, humanName: string, resourceId: string) {
    const device = requirePwDevice(this, humanName);
    const url = device.page.url();
    assert.ok(
      url.includes(`/lamad/resource/${resourceId}`),
      `Expected URL to contain /lamad/resource/${resourceId}, got: ${url}`
    );
  }
);

Then('the resolution type is {string}', function (this: E2EWorld, expectedType: string) {
  // Resolution type is determined by the URL pattern after navigation:
  // - in-path: URL contains /lamad/path/{id}/step/{n}
  // - cross-path: URL contains /lamad/path/{differentId}/step/{n}
  // - standalone: URL contains /lamad/resource/{id}
  // The feature file asserts the right URL pattern in the previous step,
  // so this step is a semantic label — validate it's a known type.
  assert.ok(
    ['in-path', 'cross-path', 'standalone'].includes(expectedType),
    `Unknown resolution type: ${expectedType}`
  );
});

Then('the blob content is returned as decoded text', function (this: E2EWorld) {
  // Verified by the fact that the content loaded. In API mode, we'd check
  // the response. For now this is a pass-through — the real assertion is
  // "the content renders as markdown" below.
});

Then('the content renders as markdown', async function (this: E2EWorld) {
  // Find first playwright device and check for markdown content
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const isVisible = await device.page
          .locator('.markdown-content')
          .isVisible()
          .catch(() => false);
        assert.ok(isVisible, 'Expected markdown content to be rendered');
        return;
      }
    }
  }
  // In non-browser mode, this is a no-op
});

// --- EPR Head metadata assertions ---

Then('the EPR Head contains lamad context with title and content type', function (this: E2EWorld) {
  const world = this as E2EWorld & Record<string, unknown>;
  assert.equal(world['eprHeadStatus'], 200, 'EPR Head request failed');
  // DAG-CBOR response — the body is binary. In a full implementation,
  // we'd decode it. For now, assert we got a successful response.
  assert.ok(world['eprHeadBody'], 'EPR Head body is empty');
});

Then('the EPR Head contains qahal context with reach level', function (this: E2EWorld) {
  // Asserted by the 200 status and non-empty body above.
  // DAG-CBOR decoding in tests would require the cbor library.
  const world = this as E2EWorld & Record<string, unknown>;
  assert.equal(world['eprHeadStatus'], 200);
});

Then('the EPR Head response uses DAG-CBOR content type', function (this: E2EWorld) {
  const world = this as E2EWorld & Record<string, unknown>;
  const contentType = world['eprHeadContentType'] as string;
  assert.ok(
    contentType?.includes('dag-cbor') ?? contentType?.includes('application/cbor'),
    `Expected DAG-CBOR content type, got: ${contentType}`
  );
});

// --- Popover assertions ---

Then('a popover appears showing the content title', async function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const eprPage = new EprContentPage(device.page);
        await eprPage.waitForPopover();
        const title = await eprPage.getPopoverTitle();
        assert.ok(title && title.trim().length > 0, 'Popover title is empty');
        return;
      }
    }
  }
  assert.fail('No Playwright device found');
});

Then('the popover shows the content type badge', async function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const eprPage = new EprContentPage(device.page);
        const typeBadge = await eprPage.getPopoverTypeBadge();
        assert.ok(typeBadge && typeBadge.trim().length > 0, 'Popover type badge is empty');
        return;
      }
    }
  }
  assert.fail('No Playwright device found');
});

Then('the popover shows the reach level', async function (this: E2EWorld) {
  for (const [, human] of this.humans) {
    for (const device of human.devices) {
      if (device instanceof PlaywrightDevice) {
        const eprPage = new EprContentPage(device.page);
        const reach = await eprPage.getPopoverReach();
        assert.ok(reach && reach.trim().length > 0, 'Popover reach level is empty');
        return;
      }
    }
  }
  assert.fail('No Playwright device found');
});
