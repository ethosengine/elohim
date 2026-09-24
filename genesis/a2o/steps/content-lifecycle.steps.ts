/**
 * Content lifecycle step definitions — create, read, discover, cross-human.
 *
 * Hosted Human agency phase: what can you do with content on a doorway?
 * Follows the pattern from federation.steps.ts but for single-doorway use.
 */

import { strict as assert } from 'node:assert';
import { randomUUID } from 'node:crypto';

import { Given, When, Then } from '@cucumber/cucumber';

import {
  PROVENANCE_VISIBILITY_BUDGET,
  waitForContent,
  waitForContentByTags,
} from '../src/framework/assertions/content-sync.js';
import { BrowserDevice } from '../src/framework/devices/browser-device.js';
import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Create content
// ---------------------------------------------------------------------------

/**
 * Options the run-unique / reach-bearing Given variants add to the base create.
 *
 * `unique` appends the scenario's own run tag to the TITLE (not only to the tag
 * list), so a title-matching search — content-search.feature — can assert that
 * the row it found is the row this run just wrote, never a leftover of an
 * earlier run on the same household. `reach` declares how widely the row may
 * travel; absent, the substrate's own default stands (no reach is invented
 * here).
 */
interface CreateContentOptions {
  unique?: boolean;
  reach?: string;
}

/**
 * Shared create logic — used by both When and Given variants.
 */
async function createContent(
  world: E2EWorld,
  humanName: string,
  title: string,
  tagsCsv: string,
  options: CreateContentOptions = {}
): Promise<void> {
  const human = world.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  const runTag = `e2e-run-${randomUUID().slice(0, 8)}`;
  const tags = tagsCsv
    .split(',')
    .map(t => t.trim())
    .filter(Boolean);

  const resolvedTitle = options.unique ? `${title} ${runTag}` : title;

  const contentId = `e2e-${randomUUID()}`;
  const content = await device.client.createContent({
    id: contentId,
    contentType: 'article',
    title: resolvedTitle,
    description: `E2E content created by ${humanName}`,
    contentBody: `Automated test content for lifecycle validation.`,
    contentFormat: 'text',
    tags: ['e2e', runTag, ...tags],
    ...(options.reach ? { reach: options.reach } : {}),
  });

  const id = content.id as string;
  world.contentIds.set('lastContentId', id);
  world.contentIds.set('lastContentTitle', resolvedTitle);
  world.contentIds.set('lastContentAuthor', humanName);
  world.contentIds.set('lastContentRunTag', runTag);
  if (options.reach) world.contentIds.set('lastContentReach', options.reach);
  // Store per-title for multi-content scenarios
  world.contentIds.set(`content:${resolvedTitle}:id`, id);
  world.contentIds.set(`content:${resolvedTitle}:runTag`, runTag);
  world.contentIds.set(`content:${resolvedTitle}:tags`, JSON.stringify(['e2e', runTag, ...tags]));

  // Delete the e2e-<uuid> content when the scenario ends. Without this,
  // EVERY content-lifecycle scenario permanently leaks a content row on the
  // target doorway's primary conductor (main-branch CI targets doorway-B →
  // adam). Those rows land NULL-anchor and are re-selected + re-thrashed by
  // the reanchor backfill on every boot, saturating the Holochain Cache-DB
  // read pool. Mirrors the world.onCleanup pattern in fixture-humans /
  // seeder step files. Best-effort: a drained/absent row on delete is fine.
  world.onCleanup(async () => {
    try {
      await device.client.deleteContent(id);
    } catch {
      // best-effort cleanup
    }
  });
}

When(
  '{word} creates content titled {string} with tags {string}',
  async function (this: E2EWorld, humanName: string, title: string, tagsCsv: string) {
    await createContent(this, humanName, title, tagsCsv);
  }
);

Given(
  '{word} has created content titled {string} with tags {string}',
  async function (this: E2EWorld, humanName: string, title: string, tagsCsv: string) {
    await createContent(this, humanName, title, tagsCsv);
  }
);

// Run-unique variants — see CreateContentOptions. content-search.feature needs a
// title no other run can have written, and one scenario needs the row to declare
// a reach so a non-holder's search can be shown withholding it.

Given(
  '{word} has created content titled {string} made unique to this run, with tags {string}',
  async function (this: E2EWorld, humanName: string, title: string, tagsCsv: string) {
    await createContent(this, humanName, title, tagsCsv, { unique: true });
  }
);

Given(
  '{word} has created content titled {string} made unique to this run, with tags {string} and reach {string}',
  async function (
    this: E2EWorld,
    humanName: string,
    title: string,
    tagsCsv: string,
    reach: string
  ) {
    await createContent(this, humanName, title, tagsCsv, { unique: true, reach });
  }
);

// ---------------------------------------------------------------------------
// Assert creation
// ---------------------------------------------------------------------------

Then('the content should be created successfully', function (this: E2EWorld) {
  const id = this.contentIds.get('lastContentId');
  assert.ok(id, 'No content was created');
});

Then('the content should have an id', function (this: E2EWorld) {
  const id = this.contentIds.get('lastContentId');
  assert.ok(id, 'Content has no id');
  assert.ok(id.length > 0, 'Content id is empty');
});

// ---------------------------------------------------------------------------
// Read content
// ---------------------------------------------------------------------------

When('{word} reads the content by id', async function (this: E2EWorld, humanName: string) {
  const human = this.getHuman(humanName);
  const device = human.devices[0] as BrowserDevice;
  assert.ok(device, `${humanName} has no device`);

  const id = this.contentIds.get('lastContentId');
  assert.ok(id, 'No content id to read');

  // A fresh POST /db/content becomes externally visible only after the
  // provenance-publish drain loop marks the row (≈DRAIN_INTERVAL_SECS=15s on
  // storage). A read-by-id immediately after create races that drain and 404s,
  // so poll until the content is visible — mirroring the waitForContentByTags
  // pattern the sibling tag-search step already uses. The budget must span the
  // whole drain window, not half of it: see PROVENANCE_VISIBILITY_BUDGET.
  const content = await waitForContent(device.client, id, PROVENANCE_VISIBILITY_BUDGET);
  this.contentIds.set('lastReadContent', JSON.stringify(content));
});

Then('the content title should be {string}', function (this: E2EWorld, expectedTitle: string) {
  const content = JSON.parse(this.contentIds.get('lastReadContent')!) as Record<string, unknown>;
  assert.strictEqual(content.title, expectedTitle, `Content title mismatch`);
});

// ---------------------------------------------------------------------------
// Search / discover content
// ---------------------------------------------------------------------------

When(
  '{word} searches for content with tag {string}',
  async function (this: E2EWorld, humanName: string, tag: string) {
    const human = this.getHuman(humanName);
    const device = human.devices[0] as BrowserDevice;
    assert.ok(device, `${humanName} has no device`);

    // Use the run tag that was stored when content was created
    // to scope the search to this test run
    const runTag = this.contentIds.get('lastContentRunTag');
    assert.ok(runTag, 'No run tag — was content created in this scenario?');

    // Same provenance-publish window as the read-by-id step above: a tag
    // search is a gated list read, so it cannot see the row until the drain
    // loop has stamped `p2p_published_at`.
    const results = await waitForContentByTags(
      device.client,
      [tag, runTag],
      PROVENANCE_VISIBILITY_BUDGET
    );

    this.contentIds.set('lastSearchResults', JSON.stringify(results));
  }
);

Then(
  'the search results should include {string}',
  function (this: E2EWorld, expectedTitle: string) {
    const results = JSON.parse(this.contentIds.get('lastSearchResults')!) as Record<
      string,
      unknown
    >[];
    const match = results.find(r => r.title === expectedTitle);
    assert.ok(
      match,
      `Content "${expectedTitle}" not found in search results. Got: ${results.map(r => r.title).join(', ')}`
    );
  }
);
