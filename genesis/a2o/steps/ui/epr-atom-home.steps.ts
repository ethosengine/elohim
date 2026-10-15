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
 *
 * "The legs unfold" (spec §11, Slice 4) reads the §11.3 test-id contract. Its
 * fixture Givens probe the doorway first and return 'pending' when the
 * precondition is absent (no piece map, no stewards, no relationships), so a
 * thin fixture holds the scenario instead of failing it.
 */

import { strict as assert } from 'node:assert';

import { Given, Then, When } from '@cucumber/cucumber';

import { PlaywrightDevice } from '../../src/framework/devices/playwright-device.js';
import { Human } from '../../src/framework/human.js';
import { EPR_HOME, EprHomePage, type EprHomeLeg } from '../../src/framework/pages/index.js';
import { FEEDBACK_GATE } from '../../src/framework/pages/selectors.js';
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
    // The atom home now records itself on the session nav stack when it
    // loads (spec §12.2 — a real navigation, cold or in-app, builds real
    // history), so no synthetic sessionStorage seeding is needed here
    // anymore: the PRIOR page already recorded itself for real when it
    // loaded, and this navigation's destination will too.
    const { page, appUrl } = home(this, humanName);
    await page.goto(appUrl, id);
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
  'the leg "Who holds it" shows the household floor the doorway reports for {string}',
  async function (this: E2EWorld, id: string) {
    const { page } = home(this, 'Matthew');
    const doorway = this.getDoorway('alpha');
    const res = await fetch(`${doorway.url}/api/v1/resilience/${encodeURIComponent(id)}/household`);
    assert.equal(res.status, 200);
    const body = (await res.json()) as {
      feltStatus?: { floor?: { wantsHouseholds: number; hasHouseholds: number } };
    };
    const floor = body.feltStatus?.floor;
    assert.ok(floor, 'no feltStatus.floor on the household snapshot');
    assert.ok(
      (await page.legText('holds')).includes(
        `${floor.hasHouseholds} of ${floor.wantsHouseholds} households`
      )
    );
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

// ---------------------------------------------------------------------------
// The legs unfold (spec §11, Slice 4)
// ---------------------------------------------------------------------------

const LEG_BY_LABEL: Record<string, EprHomeLeg> = {
  'Who holds it': 'holds',
  'Where this lives': 'lives',
  "How it's governed": 'governed',
  'Where it came from': 'from',
};

function legOf(label: string): EprHomeLeg {
  const leg = LEG_BY_LABEL[label];
  assert.ok(leg, `unknown leg "${label}" (known: ${Object.keys(LEG_BY_LABEL).join(', ')})`);
  return leg;
}

/** GET a doorway read and parse JSON; null when it does not answer 200. */
async function readJson<T>(url: string): Promise<T | null> {
  const res = await fetch(url).catch(() => null);
  if (res?.status !== 200) return null;
  return (await res.json().catch(() => null)) as T | null;
}

/** Path + fragment of the page's current URL. */
function pathAndHash(device: PlaywrightDevice): string {
  const url = new URL(device.page.url());
  return `${url.pathname}${url.hash}`;
}

/**
 * Visible text that could be a trust percentage: every "N%" left after the
 * stewardship share ("40% share", spec §11.2) is removed.
 */
function percentagesOutsideShares(text: string): string[] {
  return (
    text.replace(/\d{1,3}(?:\.\d{1,2})?\s?%\s*share/gi, '').match(/\d{1,3}(?:\.\d{1,2})?\s?%/g) ??
    []
  );
}

// --- fixture Givens ---

Given(
  'doorway {string} knows how {string} is split into pieces',
  async function (this: E2EWorld, doorwayName: string, id: string) {
    const doorway = this.getDoorway(doorwayName);
    const view = await readJson<{ encoding?: { totalShards?: number } }>(
      `${doorway.url}/api/v1/resilience/${encodeURIComponent(id)}`
    );
    if (!view?.encoding) return 'pending'; // no piece map served for this atom
    return undefined;
  }
);

Given(
  'doorway {string} knows who stewards {string}',
  async function (this: E2EWorld, doorwayName: string, id: string) {
    const doorway = this.getDoorway(doorwayName);
    const view = await readJson<{ allocations?: unknown[] }>(
      `${doorway.url}/db/allocations/content/${encodeURIComponent(id)}`
    );
    if (!view?.allocations?.length) return 'pending'; // nobody stewards it on this doorway
    return undefined;
  }
);

Given(
  '{string} names {string}, {string} and {string} as related',
  async function (this: E2EWorld, focalId: string, a: string, b: string, c: string) {
    const doorway = this.getDoorway('alpha');
    const node = await fetch(`${doorway.url}/db/content/${encodeURIComponent(focalId)}`).catch(
      () => null
    );
    if (!node?.ok) return 'pending'; // the doctrinal articles are not seeded here
    const graph = await readJson<{ related?: { contentId: string }[] }>(
      `${doorway.url}/db/relationships/graph/${encodeURIComponent(focalId)}?computed=false`
    );
    if (!graph) return 'pending'; // the relationship resolver is not served here
    const got = new Set((graph.related ?? []).map(r => r.contentId));
    for (const id of [a, b, c]) {
      assert.ok(
        got.has(id),
        `"${focalId}" does not name "${id}" as related (has: ${[...got].join(', ')})`
      );
    }
    return undefined;
  }
);

Given(
  'doorway {string} finds at least one more neighbour of {string} by shared topics',
  async function (this: E2EWorld, doorwayName: string, focalId: string) {
    const doorway = this.getDoorway(doorwayName);
    const graph = await readJson<{ related?: { inferenceSource?: string }[] }>(
      `${doorway.url}/db/relationships/graph/${encodeURIComponent(focalId)}?computed=true`
    );
    if (!graph) return 'pending'; // the relationship resolver is not served here
    const discovered = (graph.related ?? []).filter(r => r.inferenceSource === 'tag');
    assert.ok(discovered.length >= 1, `the doorway discovers no tag neighbour for "${focalId}"`);
    return undefined;
  }
);

// --- folds ---

When(
  '{word} opens {string} under {string}',
  async function (this: E2EWorld, humanName: string, foldName: string, legLabel: string) {
    const { page } = home(this, humanName);
    const leg = legOf(legLabel);
    if (!(await page.foldOpen(leg))) {
      const label = await page.foldToggleText(leg);
      assert.ok(
        label.toLowerCase().includes(foldName.toLowerCase()),
        `the fold toggle under "${legLabel}" reads "${label}", not "${foldName}"`
      );
    }
    await page.openFold(leg);
  }
);

When(
  '{word} opens the fold under {string}',
  async function (this: E2EWorld, humanName: string, legLabel: string) {
    const { page } = home(this, humanName);
    await page.openFold(legOf(legLabel));
  }
);

Then(
  '{string} is open under {string}',
  async function (this: E2EWorld, _fold: string, legLabel: string) {
    const { page } = home(this, 'Matthew');
    const leg = legOf(legLabel);
    await requirePwDevice(this, 'Matthew')
      .page.locator(`[data-testid="${EPR_HOME.FOLD(leg)}"]`)
      .first()
      .waitFor({ state: 'visible', timeout: 10_000 });
    assert.ok(await page.foldOpen(leg), `the fold under "${legLabel}" is closed`);
  }
);

// --- Who holds it → Network detail ---

Then(
  'no piece counts are on the page while {string} is folded',
  async function (this: E2EWorld, _fold: string) {
    const { page } = home(this, 'Matthew');
    assert.ok(!(await page.foldOpen('holds')), 'the "Who holds it" fold is already open');
    assert.ok(
      !(await page.isVisible(EPR_HOME.PIECES)),
      'the pieces grid shows before the fold opens'
    );
    assert.ok(
      !(await page.isVisible(EPR_HOME.HEALTH)),
      'the health line shows before the fold opens'
    );
    const body = await page.bodyText();
    assert.ok(
      !/\bshards?\b|pieces distributed|\d{1,6} ?\/ ?\d{1,6} pieces/i.test(body),
      'piece counts are visible on the page before "Network detail" was opened'
    );
  }
);

Then(
  '{string} starts with the health of the copies in words, above the pieces',
  async function (this: E2EWorld, _fold: string) {
    const { page } = home(this, 'Matthew');
    const health = await page.visibleText(EPR_HOME.HEALTH);
    assert.ok(health, 'no health line in Network detail');
    assert.match(health, /^[A-Za-z]/, `the health line leads with a number: "${health}"`);
    assert.ok(!health.includes('%'), `the health line carries a percentage: "${health}"`);
    const [healthTop, piecesTop] = [
      await page.top(EPR_HOME.HEALTH),
      await page.top(EPR_HOME.PIECES),
    ];
    assert.ok(
      healthTop !== null && piecesTop !== null && healthTop < piecesTop,
      'the health line does not sit above the pieces'
    );
  }
);

Then(
  '{string} shows where the pieces are, one cell for each piece',
  async function (this: E2EWorld, _fold: string) {
    const { page } = home(this, 'Matthew');
    assert.ok(await page.isVisible(EPR_HOME.PIECES), 'no "Where the pieces are" grid');
    assert.ok((await page.pieceCellCount()) >= 1, 'the pieces grid has no cells');
  }
);

When(
  '{word} presses {string} in {string}',
  async function (this: E2EWorld, humanName: string, label: string, _fold: string) {
    const { page } = home(this, humanName);
    const device = requirePwDevice(this, humanName);
    const button = device.page.locator(`[data-testid="${EPR_HOME.VERIFY}"]`).first();
    assert.ok(
      ((await button.innerText()) ?? '').includes(label),
      `the button does not read "${label}"`
    );
    (this as unknown as Record<string, unknown>).eprHomeVerifyResult = await page.verifyWhole();
  }
);

Then('one line answers whether {string} is whole', function (this: E2EWorld, _id: string) {
  const line = (this as unknown as Record<string, unknown>).eprHomeVerifyResult as
    | string
    | undefined;
  assert.ok(line, 'no answer to "Check it\'s still whole" was captured');
  assert.ok(!line.includes('\n'), `the answer is more than one line: "${line}"`);
  assert.match(line, /^(Whole|Not whole)\b/, `the answer does not say whole or not: "${line}"`);
});

// --- the old address ---

When(
  '{word} opens the old address {string}',
  async function (this: E2EWorld, humanName: string, path: string) {
    const device = requirePwDevice(this, humanName);
    await device.page.goto(`${doorwayToAppUrl(device.client.url)}${path}`, {
      waitUntil: 'networkidle',
    });
    await new EprHomePage(device.page).waitForReady();
  }
);

Then(
  'the browser shows the atom home address {string}',
  async function (this: E2EWorld, expected: string) {
    const device = requirePwDevice(this, 'Matthew');
    await device.page.waitForURL(url => `${url.pathname}${url.hash}` === expected, {
      timeout: 15_000,
    });
    assert.equal(pathAndHash(device), expected);
    assert.ok(await new EprHomePage(device.page).has(EPR_HOME.ROOT), 'not on the atom home');
  }
);

// --- Where it came from ---

Then(
  'every steward under "Where it came from" is named with a share and a recognition',
  async function (this: E2EWorld) {
    const { page } = home(this, 'Matthew');
    const rows = await page.stewardRows();
    assert.ok(rows.length >= 1, 'no steward rows under "Where it came from"');
    for (const row of rows) {
      assert.ok(/share/i.test(row), `steward row names no share: "${row}"`);
      assert.ok(
        /recogni[sz]ed|recognition/i.test(row),
        `steward row names no recognition: "${row}"`
      );
    }
  }
);

Then(
  '"Attestations" lists each attestation the doorway reports for {string}, or reads {string}',
  async function (this: E2EWorld, id: string, emptyLine: string) {
    const { page } = home(this, 'Matthew');
    const doorway = this.getDoorway('alpha');
    const reported = await readJson<unknown[]>(
      `${doorway.url}/api/v1/attestations/unified?subjectCid=${encodeURIComponent(id)}`
    );
    const shown = await page.attestationCount();
    const text = await page.visibleText(EPR_HOME.ATTESTATIONS);
    if (reported === null) {
      // The doorway would not say; the page must still say one or the other.
      assert.ok(shown > 0 || text.includes(emptyLine), `"Attestations" is blank: "${text}"`);
    } else if (reported.length === 0) {
      assert.ok(text.includes(emptyLine), `expected "${emptyLine}", read "${text}"`);
    } else {
      assert.equal(
        shown,
        reported.length,
        `doorway reports ${reported.length}, page lists ${shown}`
      );
    }
  }
);

Then('nothing under "Where it came from" is a trust percentage', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  const leg = await page.legText('from');
  const found = percentagesOutsideShares(leg);
  assert.equal(found.length, 0, `percentages under "Where it came from": ${found.join(', ')}`);
  assert.ok(!/trust (?:score|level)?:? ?\d/i.test(leg), 'a trust score is shown');
});

// --- How it's governed → Raise a concern ---

When(
  '{word} raises a concern and chooses to challenge the resource',
  async function (this: E2EWorld, humanName: string) {
    const { page } = home(this, humanName);
    const device = requirePwDevice(this, humanName);
    (this as unknown as Record<string, unknown>).eprHomeUrlBeforeConcern = device.page.url();
    await page.openConcernMenu();
    await device.page
      .locator(`[data-testid="${FEEDBACK_GATE.MENU_ITEM_CHALLENGE}"]`)
      .first()
      .click();
  }
);

Then('the concern form is open on the atom home', async function (this: E2EWorld) {
  const device = requirePwDevice(this, 'Matthew');
  await device.page
    .locator(`[data-testid="${FEEDBACK_GATE.MODAL_PANEL}"]`)
    .first()
    .waitFor({ state: 'visible', timeout: 5_000 });
  assert.ok(await new EprHomePage(device.page).has(EPR_HOME.ROOT), 'the atom home is gone');
});

Then('the browser still shows the atom home for {string}', function (this: E2EWorld, id: string) {
  const device = requirePwDevice(this, 'Matthew');
  const before = (this as unknown as Record<string, unknown>).eprHomeUrlBeforeConcern;
  const now = new URL(device.page.url());
  assert.equal(now.pathname, `/epr/${id}`, `the page moved to ${now.pathname}`);
  if (typeof before === 'string') {
    assert.equal(new URL(before).pathname, now.pathname, 'raising a concern changed the address');
  }
});

// --- a visitor who has not signed in (a second, separate browser) ---

const VISITOR = '_epr-home-visitor';

async function visitorDevice(world: E2EWorld): Promise<PlaywrightDevice> {
  let human: Human;
  try {
    human = world.getHuman(VISITOR);
  } catch {
    human = new Human(VISITOR, { identifier: '', password: '', displayName: VISITOR });
    world.addHuman(VISITOR, human);
  }
  const existing = human.devices.find(d => d.type === 'playwright');
  if (existing) return existing as PlaywrightDevice;
  const doorway = world.getDoorway('alpha');
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  const browser = await world.getBrowser();
  const device = new PlaywrightDevice(
    `${VISITOR}-pw`,
    doorwayToAppUrl(doorway.url),
    doorway.url,
    // eslint-disable-next-line @typescript-eslint/no-unsafe-argument
    browser
  );
  await device.init();
  human.addDevice(device);
  world.onCleanup(async () => device.close());
  return device;
}

When(
  'a visitor who has not signed in opens the atom home for {string}',
  async function (this: E2EWorld, id: string) {
    if (this.deviceMode !== 'playwright') return 'pending';
    const device = await visitorDevice(this);
    await new EprHomePage(device.page).goto(doorwayToAppUrl(device.client.url), id);
    return undefined;
  }
);

When(
  'the visitor opens the fold under {string}',
  async function (this: E2EWorld, legLabel: string) {
    const device = await visitorDevice(this);
    await new EprHomePage(device.page).openFold(legOf(legLabel));
  }
);

Then(
  'the visitor reads {string} under {string}',
  async function (this: E2EWorld, line: string, legLabel: string) {
    const device = await visitorDevice(this);
    const text = await new EprHomePage(device.page).legText(legOf(legLabel));
    assert.ok(text.includes(line), `"${legLabel}" reads: ${text}`);
  }
);

Then('the visitor is offered no concern form', async function (this: E2EWorld) {
  const device = await visitorDevice(this);
  assert.equal(await new EprHomePage(device.page).concernTriggerCount(), 0);
});

// --- Who's here ---

Then(
  '"Who\'s here" says in words the attention the doorway reports for {string}',
  async function (this: E2EWorld, id: string) {
    const { page } = home(this, 'Matthew');
    const doorway = this.getDoorway('alpha');
    const stats = await readJson<{ views: number | string }>(
      `${doorway.url}/api/v1/lamad/content/${encodeURIComponent(id)}/engagement`
    );
    if (!stats) return 'pending'; // the engagement projection is not served here
    const views = Number(stats.views);
    const line = await page.visibleText(EPR_HOME.ATTENTION);
    if (views > 0) {
      assert.ok(
        new RegExp(String.raw`^Viewed ${views} times?\b`).test(line),
        `doorway counts ${views} views; "Who's here" reads "${line}"`
      );
    } else {
      assert.ok(line.includes('No one has been counted here yet.'), `"Who's here" reads "${line}"`);
    }
    return undefined;
  }
);

Then('"Who\'s here" shows no charts, gauges or percentages', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  const device = requirePwDevice(this, 'Matthew');
  const leg = device.page.locator(`[data-testid="${EPR_HOME.LEG('here')}"]`).first();
  const widgets = await leg
    .locator('canvas, progress, meter, [role="progressbar"], [role="meter"], [class*="chart"]')
    .count();
  assert.equal(widgets, 0, `"Who's here" carries ${widgets} chart or gauge widgets`);
  assert.ok(!(await page.legText('here')).includes('%'), '"Who\'s here" shows a percentage');
});

// --- Where this lives → See also ---

Then(
  '"See also" lists {string}, {string} and {string} as authored neighbours',
  async function (this: E2EWorld, a: string, b: string, c: string) {
    const { page } = home(this, 'Matthew');
    const authored = (await page.seeAlsoRows())
      .filter(r => r.inference === 'explicit')
      .map(r => r.text.toLowerCase());
    for (const id of [a, b, c]) {
      assert.ok(
        authored.some(t => t.includes(id.toLowerCase())),
        `no authored "See also" row for "${id}" (authored: ${JSON.stringify(authored)})`
      );
    }
  }
);

Then(
  '"See also" shows at least one neighbour marked {string}',
  async function (this: E2EWorld, mark: string) {
    const { page } = home(this, 'Matthew');
    const discovered = (await page.seeAlsoRows()).filter(r => r.inference !== 'explicit');
    assert.ok(discovered.length >= 1, 'no discovered neighbour under "See also"');
    for (const row of discovered) {
      assert.ok(row.text.toLowerCase().includes(mark.toLowerCase()), `unmarked: "${row.text}"`);
    }
  }
);

Then('every "See also" neighbour opens at its own atom home', async function (this: E2EWorld) {
  const { page } = home(this, 'Matthew');
  const rows = await page.seeAlsoRows();
  assert.ok(rows.length >= 1, 'no "See also" rows');
  for (const row of rows) {
    assert.match(row.href ?? '', /\/epr\/[^/?#]+/, `"${row.text}" links to ${row.href}`);
  }
});
