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

import type { Content } from './generated/index.js';
import type {
  BulkPresenceResult,
  BulkMasteryResult,
  BulkEventResult,
} from './doorway-client.js';
import { DoorwayClient } from './doorway-client.js';

// =============================================================================
// Types
// =============================================================================

/** Input for creating a contributor presence */
export interface CreatePresenceInput {
  display_name: string;
  presence_state?: string;
  external_identifiers_json?: string;
  establishing_content_ids_json: string;
  affinity_total?: number;
  unique_engagers?: number;
  citation_count?: number;
  recognition_score?: number;
}

/** Input for creating content mastery */
export interface CreateMasteryInput {
  human_id: string;
  content_id: string;
  mastery_level?: string;
  mastery_level_index?: number;
  freshness_score?: number;
  engagement_count?: number;
}

/** Input for creating an economic event */
export interface CreateEventInput {
  action: string;
  provider: string;
  receiver: string;
  resource_conforms_to?: string;
  resource_quantity_value?: number;
  resource_quantity_unit?: string;
  lamad_event_type?: string;
  content_id?: string;
  contributor_presence_id?: string;
  path_id?: string;
  metadata_json?: string;
}

// =============================================================================
// Presence Extraction
// =============================================================================

/**
 * Extract author information from content metadata.
 *
 * Looks for author in various places:
 * - metadata_json.author
 * - metadata_json.creator
 * - metadata_json.contributors[]
 */
function extractAuthor(content: Content): string | null {
  if (!content.metadata_json) return null;

  try {
    const metadata = typeof content.metadata_json === 'string'
      ? JSON.parse(content.metadata_json)
      : content.metadata_json;

    // Direct author field
    if (metadata.author) return metadata.author;
    if (metadata.creator) return metadata.creator;

    // First contributor
    if (metadata.contributors && Array.isArray(metadata.contributors) && metadata.contributors.length > 0) {
      return metadata.contributors[0];
    }

    return null;
  } catch {
    return null;
  }
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
      display_name: author,
      presence_state: 'unclaimed',
      establishing_content_ids_json: JSON.stringify([...contentIds]),
      affinity_total: 0.0,
      unique_engagers: 0,
      citation_count: contentIds.size,
      recognition_score: 0.0,
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
    human_id: humanId,
    content_id: contentId,
    mastery_level: 'not_started',
    mastery_level_index: 0,
    freshness_score: 1.0,
    engagement_count: 0,
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
        action: 'use',
        provider: agentId,
        receiver: contentId,
        lamad_event_type: 'content-view',
        content_id: contentId,
      });
    }

    // Maybe generate completion event
    if (Math.random() < completionRate) {
      events.push({
        action: 'produce',
        provider: agentId,
        receiver: agentId,
        lamad_event_type: 'content-complete',
        content_id: contentId,
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
