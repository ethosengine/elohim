/**
 * Entity Seeder Functions
 *
 * Seeding functions for the new entity types:
 * - Contributor Presences (from content author metadata)
 * - Content Mastery (initial state for a human)
 * - Economic Events (historical data for analytics)
 *
 * These functions can be called from the main seeder or used standalone.
 *
 * Usage:
 *   import { seedContributorPresences, seedInitialMastery } from './seed-entities.js';
 *   await seedContributorPresences(client, contentItems);
 *   await seedInitialMastery(client, 'test-human-001', contentIds);
 */

import type { MasteryLevel } from './generated/schema-enums.js';

/** Minimal content shape used by entity seeder (metadata + id) */
interface Content {
  id: string;
  metadata?: Record<string, unknown>;
}
import type {
  BulkPresenceResult,
  BulkMasteryResult,
  BulkEventResult,
} from './doorway-client.js';
import { DoorwayClient } from './doorway-client.js';

// =============================================================================
// Types
// =============================================================================

/** Input for creating a contributor presence — matches Rust CreateContributorPresenceInputView */
export interface CreatePresenceInput {
  schemaVersion?: number;
  displayName: string;
  presenceState?: string;
  externalIdentifiers?: unknown;
  establishingContentIds: string[];
  affinityTotal?: number;
  uniqueEngagers?: number;
  citationCount?: number;
  recognitionScore?: number;
}

/** Input for creating content mastery */
export interface CreateMasteryInput {
  schemaVersion?: number;
  humanId: string;
  contentId: string;
  masteryLevel?: MasteryLevel;
  masteryLevelIndex?: number;
  freshnessScore?: number;
  engagementCount?: number;
}

/** Input for creating an economic event — matches Rust CreateEconomicEventInputView */
export interface CreateEventInput {
  schemaVersion?: number;
  action: string;
  provider: string;
  receiver: string;
  resourceConformsTo?: string;
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  lamadEventType?: string;
  contentId?: string;
  contributorPresenceId?: string;
  pathId?: string;
  metadata?: Record<string, unknown>;
}

// =============================================================================
// Presence Extraction
// =============================================================================

/**
 * Extract author information from content metadata.
 *
 * Looks for author in various places:
 * - metadata.author
 * - metadata.creator
 * - metadata.contributors[]
 */
function extractAuthor(content: Content): string | null {
  if (!content.metadata) return null;

  const metadata = content.metadata;

  // Direct author field
  if (metadata.author) return metadata.author as string;
  if (metadata.creator) return metadata.creator as string;

  // First contributor
  const contributors = metadata.contributors;
  if (Array.isArray(contributors) && contributors.length > 0) {
    return contributors[0] as string;
  }

  return null;
}

/**
 * Build contributor presences from content author metadata.
 *
 * Groups content by author and creates presences in 'unclaimed' state.
 * Each presence tracks which content established it.
 */
export function buildContributorPresences(contentItems: Content[]): CreatePresenceInput[] {
  const authorMap = new Map<string, Set<string>>();

  // Group content by author
  for (const content of contentItems) {
    const author = extractAuthor(content);
    if (author) {
      if (!authorMap.has(author)) {
        authorMap.set(author, new Set());
      }
      authorMap.get(author)!.add(content.id);
    }
  }

  // Build presence inputs
  const presences: CreatePresenceInput[] = [];
  for (const [author, contentIds] of authorMap) {
    presences.push({
      schemaVersion: 1,
      displayName: author,
      presenceState: 'unclaimed',
      establishingContentIds: [...contentIds],
      affinityTotal: 0.0,
      uniqueEngagers: 0,
      citationCount: contentIds.size,
      recognitionScore: 0.0,
    });
  }

  return presences;
}

/**
 * Seed contributor presences from content author metadata.
 *
 * @param client - DoorwayClient or storage URL
 * @param contentItems - Content items to extract authors from
 * @returns BulkPresenceResult with created count
 */
export async function seedContributorPresences(
  client: DoorwayClient | string,
  contentItems: Content[]
): Promise<BulkPresenceResult> {
  const doorwayClient = typeof client === 'string'
    ? new DoorwayClient({ baseUrl: client })
    : client;

  const presences = buildContributorPresences(contentItems);

  if (presences.length === 0) {
    console.log('   No authors found in content metadata, skipping presences');
    return { created: 0, errors: [] };
  }

  console.log(`   Seeding ${presences.length} contributor presences...`);
  return doorwayClient.bulkCreatePresences(presences);
}

// =============================================================================
// Mastery Initialization
// =============================================================================

/**
 * Build initial mastery records for a human.
 *
 * All content starts at 'not_started' level with full freshness.
 */
export function buildInitialMastery(humanId: string, contentIds: string[]): CreateMasteryInput[] {
  return contentIds.map(contentId => ({
    schemaVersion: 1,
    humanId: humanId,
    contentId: contentId,
    masteryLevel: 'not_started',
    masteryLevelIndex: 0,
    freshnessScore: 1.0,
    engagementCount: 0,
  }));
}

/**
 * Seed initial mastery records for a human.
 *
 * Creates 'not_started' mastery records for all provided content IDs.
 *
 * @param client - DoorwayClient or storage URL
 * @param humanId - Human/agent ID to create mastery for
 * @param contentIds - Content IDs to initialize mastery for
 * @returns BulkMasteryResult with created count
 */
export async function seedInitialMastery(
  client: DoorwayClient | string,
  humanId: string,
  contentIds: string[]
): Promise<BulkMasteryResult> {
  const doorwayClient = typeof client === 'string'
    ? new DoorwayClient({ baseUrl: client })
    : client;

  const masteryRecords = buildInitialMastery(humanId, contentIds);

  if (masteryRecords.length === 0) {
    console.log('   No content IDs provided, skipping mastery initialization');
    return { created: 0, updated: 0, errors: [] };
  }

  console.log(`   Initializing mastery for ${masteryRecords.length} content items...`);
  return doorwayClient.bulkUpsertMastery(masteryRecords);
}

// =============================================================================
// Event Generation (Sample Data)
// =============================================================================

/**
 * Generate sample economic events for analytics testing.
 *
 * Creates view and completion events distributed across content.
 */
export function generateSampleEvents(
  agentId: string,
  contentIds: string[],
  options: {
    viewsPerContent?: number;
    completionRate?: number;
  } = {}
): CreateEventInput[] {
  const {
    viewsPerContent = 3,
    completionRate = 0.7,
  } = options;

  const events: CreateEventInput[] = [];

  for (const contentId of contentIds) {
    // Generate view events
    for (let i = 0; i < viewsPerContent; i++) {
      events.push({
        schemaVersion: 1,
        action: 'use',
        provider: agentId,
        receiver: contentId,
        lamadEventType: 'content-view',
        contentId: contentId,
      });
    }

    // Maybe generate completion event
    if (Math.random() < completionRate) {
      events.push({
        schemaVersion: 1,
        action: 'produce',
        provider: agentId,
        receiver: agentId,
        lamadEventType: 'content-complete',
        contentId: contentId,
      });
    }
  }

  return events;
}

/**
 * Seed sample economic events for analytics testing.
 *
 * @param client - DoorwayClient or storage URL
 * @param agentId - Agent ID to attribute events to
 * @param contentIds - Content IDs to generate events for
 * @param options - Event generation options
 * @returns BulkEventResult with recorded count
 */
export async function seedSampleEvents(
  client: DoorwayClient | string,
  agentId: string,
  contentIds: string[],
  options?: { viewsPerContent?: number; completionRate?: number }
): Promise<BulkEventResult> {
  const doorwayClient = typeof client === 'string'
    ? new DoorwayClient({ baseUrl: client })
    : client;

  const events = generateSampleEvents(agentId, contentIds, options);

  if (events.length === 0) {
    console.log('   No events to seed');
    return { recorded: 0, errors: [] };
  }

  console.log(`   Seeding ${events.length} sample economic events...`);
  return doorwayClient.bulkRecordEvents(events);
}

// =============================================================================
// CLI Interface
// =============================================================================

/**
 * Seed all new entity types.
 *
 * This is a convenience function for seeding presences, mastery, and events
 * in a single call.
 *
 * @param client - DoorwayClient or storage URL
 * @param contentItems - Content items (for presence extraction)
 * @param options - Seeding options
 */
export async function seedAllEntities(
  client: DoorwayClient | string,
  contentItems: Content[],
  options: {
    humanId?: string;
    seedPresences?: boolean;
    seedMastery?: boolean;
    seedEvents?: boolean;
  } = {}
): Promise<{
  presences: BulkPresenceResult;
  mastery: BulkMasteryResult;
  events: BulkEventResult;
}> {
  const {
    humanId = 'test-human-001',
    seedPresences = true,
    seedMastery = true,
    seedEvents = false, // Events are opt-in since they're sample data
  } = options;

  const results = {
    presences: { created: 0, errors: [] as string[] },
    mastery: { created: 0, updated: 0, errors: [] as string[] },
    events: { recorded: 0, errors: [] as string[] },
  };

  // Seed contributor presences
  if (seedPresences) {
    console.log('\n📋 Seeding contributor presences...');
    results.presences = await seedContributorPresences(client, contentItems);
    console.log(`   Created: ${results.presences.created}`);
    if (results.presences.errors.length > 0) {
      console.log(`   Errors: ${results.presences.errors.length}`);
    }
  }

  // Initialize mastery for test human
  if (seedMastery) {
    console.log('\n🎯 Initializing content mastery...');
    const contentIds = contentItems.map(c => c.id);
    results.mastery = await seedInitialMastery(client, humanId, contentIds);
    console.log(`   Created: ${results.mastery.created}, Updated: ${results.mastery.updated}`);
    if (results.mastery.errors.length > 0) {
      console.log(`   Errors: ${results.mastery.errors.length}`);
    }
  }

  // Seed sample events (opt-in)
  if (seedEvents) {
    console.log('\n📊 Seeding sample economic events...');
    const contentIds = contentItems.map(c => c.id);
    results.events = await seedSampleEvents(client, humanId, contentIds);
    console.log(`   Recorded: ${results.events.recorded}`);
    if (results.events.errors.length > 0) {
      console.log(`   Errors: ${results.events.errors.length}`);
    }
  }

  return results;
}
