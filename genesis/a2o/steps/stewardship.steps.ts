/**
 * Stewardship allocation step definitions — API-level validation.
 *
 * Verifies that affinity-based stewardship allocation distributes content
 * to stewards with appropriate ratios based on their relational affinity.
 *
 * These steps query the doorway API after seeding — no browser required.
 */

import { strict as assert } from 'node:assert';

import { Given, When, Then } from '@cucumber/cucumber';

import { E2EWorld } from '../src/framework/world.js';

import type { AllocationView } from '../src/framework/api/doorway-client.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Map from display name (used in Gherkin) to presence ID (used in allocations). */
const DISPLAY_NAME_TO_PRESENCE: Record<string, string> = {
  Adam: 'adam-firstman',
  Eve: 'eve-firstwoman',
  Susan: 'susan-spouse',
  Matthew: 'matthew-dowell',
  Pete: 'pete-pastor',
  'Pastor Pete': 'pete-pastor',
  Frank: 'frank-farmer',
  Dan: 'dan-developer',
  Nancy: 'nancy-neighbor',
  Meriadoc: 'meriadoc-moneybags',
};

function presenceIdFor(displayName: string): string {
  const id = DISPLAY_NAME_TO_PRESENCE[displayName];
  assert.ok(id, `No presence ID mapping for "${displayName}"`);
  return id;
}

function getClient(world: E2EWorld) {
  // Use the first doorway's client (set up in Background)
  const doorway = world.getDoorway('alpha');
  return doorway.client;
}

function getStoredAllocations(world: E2EWorld): AllocationView[] {
  const raw = world.contentIds.get('lastAllocations');
  assert.ok(raw, 'No allocations stored — was a query step executed?');
  return JSON.parse(raw) as AllocationView[];
}

// ---------------------------------------------------------------------------
// Background
// ---------------------------------------------------------------------------

Given(
  'content has been seeded with affinity-based stewardship allocations',
  async function (this: E2EWorld) {
    const client = getClient(this);
    const allocations = await client.listAllocations();
    assert.ok(
      allocations.length > 0,
      'No stewardship allocations found — has the seeder been run?'
    );
    this.contentIds.set('totalAllocationCount', String(allocations.length));
  }
);

// Alternate wording used in the philosophy scenario
Given('content has been seeded with affinity-based allocations', async function (this: E2EWorld) {
  const client = getClient(this);
  const allocations = await client.listAllocations();
  assert.ok(allocations.length > 0, 'No stewardship allocations found — has the seeder been run?');
  this.contentIds.set('totalAllocationCount', String(allocations.length));
});

// ---------------------------------------------------------------------------
// Query steps
// ---------------------------------------------------------------------------

When(
  'I query stewardship allocations for {word} content',
  async function (this: E2EWorld, category: string) {
    const client = getClient(this);

    // Find content with this category tag, then get allocations for each
    const allContent = await client.searchContent([category]);

    assert.ok(allContent.length > 0, `No content found with tag "${category}"`);

    // Get allocations for the first matching content item
    const contentId = allContent[0].id as string;
    const allocations = await client.getAllocationsForContent(contentId);

    this.contentIds.set('lastAllocations', JSON.stringify(allocations));
    this.contentIds.set('lastQueryCategory', category);
    this.contentIds.set('lastQueryContentId', contentId);
  }
);

When('I query stewardship allocations for any content category', async function (this: E2EWorld) {
  const client = getClient(this);
  const allAllocations = await client.listAllocations();
  assert.ok(allAllocations.length > 0, 'No allocations found');

  this.contentIds.set('allAllocations', JSON.stringify(allAllocations));
});

When(
  'I query stewardship allocations for content with no matching category',
  async function (this: E2EWorld) {
    const client = getClient(this);

    // Find content that gets the fallback (matthew-only) allocation.
    // Look for landing-page-concept content which maps to matthew at 1.0
    const allContent = await client.searchContent(['landing-page-concept']);

    if (allContent.length > 0) {
      const contentId = allContent[0].id as string;
      const allocations = await client.getAllocationsForContent(contentId);
      this.contentIds.set('lastAllocations', JSON.stringify(allocations));
      this.contentIds.set('lastQueryContentId', contentId);
    } else {
      // Fallback: find any content with a single matthew allocation
      const allAllocations = await client.listAllocations();
      const byContent = new Map<string, AllocationView[]>();
      for (const a of allAllocations) {
        const existing = byContent.get(a.contentId) ?? [];
        existing.push(a);
        byContent.set(a.contentId, existing);
      }

      for (const [contentId, allocations] of byContent) {
        if (
          allocations.length === 1 &&
          allocations[0].stewardPresenceId === 'matthew-dowell' &&
          allocations[0].allocationRatio === 1
        ) {
          this.contentIds.set('lastAllocations', JSON.stringify(allocations));
          this.contentIds.set('lastQueryContentId', contentId);
          return;
        }
      }

      assert.fail('No fallback-allocated content found');
    }
  }
);

When('I query all stewardship allocations', async function (this: E2EWorld) {
  const client = getClient(this);
  const allAllocations = await client.listAllocations();
  assert.ok(allAllocations.length > 0, 'No allocations found');

  this.contentIds.set('allAllocations', JSON.stringify(allAllocations));
});

// ---------------------------------------------------------------------------
// Assertion steps — steward presence
// ---------------------------------------------------------------------------

Then(
  /^(.+) should be listed as a steward with the highest ratio$/,
  function (this: E2EWorld, displayName: string) {
    const allocations = getStoredAllocations(this);
    const presenceId = presenceIdFor(displayName);

    const steward = allocations.find(a => a.stewardPresenceId === presenceId);
    assert.ok(
      steward,
      `${displayName} (${presenceId}) not found in allocations: ${allocations.map(a => a.stewardPresenceId).join(', ')}`
    );

    const maxRatio = Math.max(...allocations.map(a => a.allocationRatio));
    assert.strictEqual(
      steward.allocationRatio,
      maxRatio,
      `Expected ${displayName} to have highest ratio (${maxRatio}), got ${steward.allocationRatio}`
    );

    // Track "last mentioned steward" for follow-up pronoun steps
    this.contentIds.set('lastMentionedSteward', presenceId);
    this.contentIds.set('lastMentionedStewardGender', displayName);
  }
);

Then(/^(.+) should be listed as a steward$/, function (this: E2EWorld, displayName: string) {
  const allocations = getStoredAllocations(this);
  const presenceId = presenceIdFor(displayName);

  const steward = allocations.find(a => a.stewardPresenceId === presenceId);
  assert.ok(
    steward,
    `${displayName} (${presenceId}) not found in allocations: ${allocations.map(a => a.stewardPresenceId).join(', ')}`
  );

  this.contentIds.set('lastMentionedSteward', presenceId);
  this.contentIds.set('lastMentionedStewardGender', displayName);
});

Then(
  /^(.+) should have the highest allocation ratio$/,
  function (this: E2EWorld, displayName: string) {
    const allocations = getStoredAllocations(this);
    const presenceId = presenceIdFor(displayName);

    const steward = allocations.find(a => a.stewardPresenceId === presenceId);
    assert.ok(steward, `${displayName} not found in allocations`);

    const maxRatio = Math.max(...allocations.map(a => a.allocationRatio));
    assert.strictEqual(
      steward.allocationRatio,
      maxRatio,
      `Expected ${displayName} to have highest ratio (${maxRatio}), got ${steward.allocationRatio}`
    );

    this.contentIds.set('lastMentionedSteward', presenceId);
    this.contentIds.set('lastMentionedStewardGender', displayName);
  }
);

// ---------------------------------------------------------------------------
// Assertion steps — allocation properties
// ---------------------------------------------------------------------------

Then('no single steward should have 100% allocation ratio', function (this: E2EWorld) {
  const allocations = getStoredAllocations(this);
  for (const a of allocations) {
    assert.ok(
      a.allocationRatio < 1,
      `Steward ${a.stewardPresenceId} has 100% allocation (ratio=${a.allocationRatio})`
    );
  }
});

Then(
  'her/his allocation method should be {string}',
  function (this: E2EWorld, expectedMethod: string) {
    const allocations = getStoredAllocations(this);
    const presenceId = this.contentIds.get('lastMentionedSteward');
    assert.ok(presenceId, 'No steward was mentioned in a prior step');

    const steward = allocations.find(a => a.stewardPresenceId === presenceId);
    assert.ok(steward, `Steward ${presenceId} not found`);
    assert.strictEqual(
      steward.allocationMethod,
      expectedMethod,
      `Expected allocation method "${expectedMethod}", got "${steward.allocationMethod}"`
    );
  }
);

Then(
  'her/his contribution type should be {string}',
  function (this: E2EWorld, expectedType: string) {
    const allocations = getStoredAllocations(this);
    const presenceId = this.contentIds.get('lastMentionedSteward');
    assert.ok(presenceId, 'No steward was mentioned in a prior step');

    const steward = allocations.find(a => a.stewardPresenceId === presenceId);
    assert.ok(steward, `Steward ${presenceId} not found`);
    assert.strictEqual(
      steward.contributionType,
      expectedType,
      `Expected contribution type "${expectedType}", got "${steward.contributionType}"`
    );
  }
);

Then('the allocation method should be {string}', function (this: E2EWorld, expectedMethod: string) {
  const allocations = getStoredAllocations(this);
  for (const a of allocations) {
    assert.strictEqual(
      a.allocationMethod,
      expectedMethod,
      `Expected method "${expectedMethod}", got "${a.allocationMethod}" for ${a.stewardPresenceId}`
    );
  }
});

Then('the contribution type should be {string}', function (this: E2EWorld, expectedType: string) {
  const allocations = getStoredAllocations(this);
  for (const a of allocations) {
    assert.strictEqual(
      a.contributionType,
      expectedType,
      `Expected type "${expectedType}", got "${a.contributionType}" for ${a.stewardPresenceId}`
    );
  }
});

// ---------------------------------------------------------------------------
// Assertion steps — ratios
// ---------------------------------------------------------------------------

Then(
  "{word}'s allocation ratio should be approximately {float}",
  function (this: E2EWorld, displayName: string, expectedRatio: number) {
    const allocations = getStoredAllocations(this);
    const presenceId = presenceIdFor(displayName);

    const steward = allocations.find(a => a.stewardPresenceId === presenceId);
    assert.ok(steward, `${displayName} not found in allocations`);

    const tolerance = 0.05;
    assert.ok(
      Math.abs(steward.allocationRatio - expectedRatio) <= tolerance,
      `Expected ${displayName}'s ratio ~${expectedRatio}, got ${steward.allocationRatio} (tolerance ${tolerance})`
    );
  }
);

Then(
  'the sum of allocation ratios for each content item should be approximately 1.0',
  function (this: E2EWorld) {
    const raw = this.contentIds.get('allAllocations');
    assert.ok(raw, 'No allocations stored');
    const allAllocations = JSON.parse(raw) as AllocationView[];

    // Group by content ID
    const byContent = new Map<string, number>();
    for (const a of allAllocations) {
      byContent.set(a.contentId, (byContent.get(a.contentId) ?? 0) + a.allocationRatio);
    }

    const tolerance = 0.05;
    for (const [contentId, total] of byContent) {
      assert.ok(
        Math.abs(total - 1) <= tolerance,
        `Allocation ratios for ${contentId} sum to ${total.toFixed(3)}, expected ~1.0`
      );
    }
  }
);

Then('Matthew should be the sole steward with ratio 1.0', function (this: E2EWorld) {
  const allocations = getStoredAllocations(this);
  assert.strictEqual(allocations.length, 1, `Expected 1 allocation, got ${allocations.length}`);
  assert.strictEqual(allocations[0].stewardPresenceId, 'matthew-dowell');
  assert.strictEqual(allocations[0].allocationRatio, 1);
});

Then(
  'the average number of stewards per content item should be greater than 1',
  function (this: E2EWorld) {
    const raw = this.contentIds.get('allAllocations');
    assert.ok(raw, 'No allocations stored');
    const allAllocations = JSON.parse(raw) as AllocationView[];

    // Count unique content items and total allocation records
    const contentIds = new Set(allAllocations.map(a => a.contentId));
    const avg = allAllocations.length / contentIds.size;

    assert.ok(avg > 1, `Average stewards per content item is ${avg.toFixed(2)}, expected > 1`);
  }
);

Then('the manifesto principle holds: content is stewarded, not owned', function (this: E2EWorld) {
  // This is a philosophical assertion backed by the data checked in the prior step.
  // If avg stewards > 1, the principle holds. No additional verification needed.
  const raw = this.contentIds.get('allAllocations');
  assert.ok(raw, 'No allocations stored');
  const allAllocations = JSON.parse(raw) as AllocationView[];
  const contentIds = new Set(allAllocations.map(a => a.contentId));
  const avg = allAllocations.length / contentIds.size;
  assert.ok(avg > 1, 'Content should be stewarded by multiple humans, not owned by one');
});
