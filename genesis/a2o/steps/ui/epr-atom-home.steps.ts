/**
 * Step definitions for the EPR atom home (`/epr/{id}`) — the shell-owned
 * universal resource address. See
 * genesis/a2o/features/content/epr-atom-home.feature (@concern:epr-atom-home).
 *
 * Framework: Playwright + @cucumber/cucumber. Every step here assumes
 * E2E_DEVICE_MODE=playwright (the feature is tagged @browser-only).
 *
 * The three commons scenarios (conversation / message / "Where people stand")
 * stay @wip until the commons plan lands; their step definitions below exist
 * so `--dry-run` resolves every step in the feature file, not because the
 * behaviour is implemented yet.
 */

import { strict as assert } from 'node:assert';

import { Given, Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { EPR_HOME, EprHomePage } from '../../src/framework/pages/index.js';
import { doorwayToAppUrl } from '../../src/framework/utils/url.js';
import { E2EWorld } from '../../src/framework/world.js';

function requirePwDevice(world: E2EWorld, humanName: string): PlaywrightDevice {
  const human = world.getHuman(humanName);
  const device = human.devices.find(d => d.type === 'playwright') as PlaywrightDevice | undefined;
  assert.ok(device, `${humanName} has no Playwright device. Is E2E_DEVICE_MODE=playwright?`);
  return device;
}

function home(world: E2EWorld, humanName: string): { page: EprHomePage; appUrl: string } {
  const device = requirePwDevice(world, humanName);
  return { page: new EprHomePage(device.page), appUrl: doorwayToAppUrl(device.client.url) };
}

// --- arrival ---

When(
  '{word} opens the atom home for {string}',
  async function (this: E2EWorld, humanName: string, id: string) {
    const { page, appUrl } = home(this, humanName);
    await page.goto(appUrl, id);
  }
);

When(
  '{word} opens the atom home for {string} as a cold link',
  async function (this: E2EWorld, humanName: string, id: string) {
    const device = requirePwDevice(this, humanName);
    await device.page.evaluate(() => sessionStorage.removeItem('elohim.session-nav-stack.v1'));
    const { page, appUrl } = home(this, humanName);
    await page.goto(appUrl, id);
  }
);

Given(
  '{word} is viewing the atom home for {string}',
  async function (this: E2EWorld, humanName: string, id: string) {
    const { page, appUrl } = home(this, humanName);
    await page.goto(appUrl, id);
  }
);

When(
  '{word} follows a link to the atom home for {string}',
  async function (this: E2EWorld, humanName: string, id: string) {
    // Walks through the protocol (records the handoff) rather than a cold goto.
    const device = requirePwDevice(this, humanName);
    const appUrl = doorwayToAppUrl(device.client.url);
    await device.page.evaluate(
      ([href, label]) => {
        const key = 'elohim.session-nav-stack.v1';
        const stack = JSON.parse(sessionStorage.getItem(key) ?? '[]') as unknown[];
        stack.push({ url: location.pathname, cid: '', label: document.title, ts: Date.now() });
        stack.push({ url: href, cid: '', label, ts: Date.now() + 1 });
        sessionStorage.setItem(key, JSON.stringify(stack));
      },
      [`/epr/${id}`, id]
    );
    await new EprHomePage(device.page).goto(appUrl, id);
  }
);

When(
  '{word} follows the related link to {string}',
  async function (this: E2EWorld, humanName: string, id: string) {
    const { page } = home(this, humanName);
    await page.clickRelated(id);
  }
);

// --- identity ---

Then('the atom home shows the title {string}', async function (this: E2EWorld, title: string) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.title(), title);
});

Then('the atom home shows the reach chip {string}', async function (this: E2EWorld, reach: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.chipText('reach')).includes(reach));
});

Then('the atom home shows the notarized chip', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.chipText('notarized')).includes('Notarized'));
});

Then('the atom home shows no {string} control', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok(!(await page.bodyText()).includes(label), `found "${label}" on the atom home`);
});

Then('the atom home shows no trust percentage', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  const holds = await page.legText('holds');
  assert.ok(!holds.includes('%'), `holding leg carries a percentage: ${holds}`);
});

// --- focal shape ---

Then('the focal slot renders the content at full width', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.focalShape(), 'immersive');
  assert.ok(await page.focalFullWidth(), 'focal is not full width');
});

Then('the focal slot renders the content in the reading shape', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.focalShape(), 'reading');
});

Then('the legs sit in a rail beside the content', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok(await page.legsBesideContent(), 'legs are not beside the reading column');
});

// --- legs ---

Then(
  'the atom home shows the legs {string}, {string}, {string}, {string}',
  async function (this: E2EWorld, a: string, b: string, c: string, d: string) {
    const { page } = home(this, 'Matthew');
    const expected: Record<string, 'holds' | 'lives' | 'governed' | 'from'> = {
      'Who holds it': 'holds',
      'Where this lives': 'lives',
      "How it's governed": 'governed',
      'Where it came from': 'from',
    };
    for (const label of [a, b, c, d]) {
      const leg = expected[label];
      assert.ok(leg, `unknown leg label ${label}`);
      assert.ok(await page.legVisible(leg), `leg ${label} not visible`);
      assert.ok((await page.legText(leg)).includes(label));
    }
  }
);

Then('the leg {string} is present', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.bodyText()).includes(label));
});

Then(
  'the leg "Who holds it" reads the holding sentence the doorway reports for {string}',
  async function (this: E2EWorld, id: string) {
    const { page } = home(this, 'Matthew');
    const doorway = this.getDoorway('alpha');
    const res = await fetch(`${doorway.url}/api/v1/resilience/${encodeURIComponent(id)}/household`);
    assert.equal(res.status, 200);
    const body = (await res.json()) as { feltStatus?: { headline: string } };
    assert.ok(body.feltStatus, 'no feltStatus on the household snapshot');
    assert.ok((await page.legText('holds')).includes(body.feltStatus.headline));
  }
);

Then(
  'the leg "Who holds it" shows the household floor as {string}',
  async function (this: E2EWorld, floor: string) {
    const { page } = home(this, 'Matthew');
    assert.ok((await page.legText('holds')).includes(`${floor} households`));
  }
);

Then(
  'the shard map and replica counts stay behind a {string} link',
  async function (this: E2EWorld, label: string) {
    const { page } = home(this, 'Matthew');
    const body = await page.bodyText();
    assert.ok(
      !/shard map|shards located|replica/i.test(body),
      'holding detail leaked onto the home'
    );
    const device = requirePwDevice(this, 'Matthew');
    const link = device.page.locator('[data-testid="epr-home-network-detail"]');
    assert.equal((await link.textContent())?.trim(), label);
  }
);

Given(
  '{string} is not held by any peer doorway {string} can ask',
  async function (this: E2EWorld, id: string, doorwayName: string) {
    const doorway = this.getDoorway(doorwayName);
    const res = await fetch(`${doorway.url}/db/content/${encodeURIComponent(id)}`);
    assert.equal(
      res.status,
      404,
      `${id} is held on ${doorwayName} (status ${res.status}); pick an unheld fixture`
    );
  }
);

// --- arrival chip ---

Then('the arrival chip names {string}', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  const text = await page.arrivalText();
  assert.ok(text?.includes(label), `arrival chip: ${text}`);
});

Then('the atom home shows no arrival chip', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.equal(await page.arrivalText(), null);
});

// --- the gate ---

Then('the out-of-reach gate is shown for {string}', async function (this: E2EWorld, id: string) {
  const { page } = home(this, 'Matthew');
  const text = await page.gateText();
  assert.ok(text.includes("We can't reach this one from here"));
  assert.ok(text.includes(id));
});

Then(
  'the gate names {string} as the referring resource',
  async function (this: E2EWorld, label: string) {
    const { page } = home(this, 'Matthew');
    assert.ok((await page.gateText()).includes(label));
  }
);

Then('the gate offers to go back to {string}', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.gateText()).includes(`Back to ${label}`));
  assert.ok(await page.gateBackHref());
});

Then('the atom home shows no edit, affinity, or invite controls', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok(!(await page.has(EPR_HOME.YOUR_MARK)));
  assert.ok(!(await page.has('epr-home-invite-household')));
  assert.ok(!(await page.bodyText()).includes('Edit'));
});

// --- the commons (steps only — behaviour stays @wip until the commons plan) ---

Then('the conversation reads {string}', async function (this: E2EWorld, label: string) {
  const { page } = home(this, 'Matthew');
  assert.ok((await page.bodyText()).includes(label));
});

Then('the reply box is present', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok(await page.has('epr-home-reply-box'));
});

Then(
  'the tender says a message will reach {string}',
  async function (this: E2EWorld, reach: string) {
    const { page } = home(this, 'Matthew');
    assert.ok((await page.bodyText()).includes(reach));
  }
);

When(
  '{word} says {string} to the commons',
  async function (this: E2EWorld, humanName: string, message: string) {
    const device = requirePwDevice(this, humanName);
    await device.page.fill('[data-testid="epr-home-reply-box"]', message);
    await device.page.locator('[data-testid="epr-home-reply-submit"]').click();
  }
);

Then(
  'the conversation shows a message by {string} with a standing ring',
  async function (this: E2EWorld, author: string) {
    const { page } = home(this, 'Matthew');
    const body = await page.bodyText();
    assert.ok(body.includes(author));
    assert.ok(await page.has('epr-home-standing-ring'));
  }
);

Then('the message shows no upvote count', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  assert.ok(!(await page.bodyText()).includes('upvote'));
});

Given(
  'the statement {string} exists for {string} and is bridging',
  function (this: E2EWorld, _statement: string, _id: string) {
    // Fixture setup for "Where people stand" — pending the commons plan.
    return 'pending';
  }
);

Then(
  'the section {string} shows that statement tagged {string}',
  async function (this: E2EWorld, section: string, tag: string) {
    const { page } = home(this, 'Matthew');
    const body = await page.bodyText();
    assert.ok(body.includes(section));
    assert.ok(body.includes(tag));
  }
);

Then(
  '{word} can agree, disagree, or pass on it',
  async function (this: E2EWorld, humanName: string) {
    const device = requirePwDevice(this, humanName);
    assert.ok(await device.page.locator('[data-testid="epr-home-stance-agree"]').isVisible());
  }
);

// --- the learning lens ---

Then('the atom home offers {string}', async function (this: E2EWorld, label: string) {
  const device = requirePwDevice(this, 'Matthew');
  const text = await device.page
    .locator(`[data-testid="${EPR_HOME.OPEN_IN_BUNDLE}"]`)
    .textContent();
  assert.ok(text?.includes(label), `lens reads: ${text}`);
});

Then(
  "following it lands in the learning app's path view for {string}",
  async function (this: E2EWorld, id: string) {
    const { page } = home(this, 'Matthew');
    await page.clickOpenInBundle();
    const device = requirePwDevice(this, 'Matthew');
    assert.ok(device.page.url().includes(`/lamad/path/${id}`), `landed on ${device.page.url()}`);
  }
);
