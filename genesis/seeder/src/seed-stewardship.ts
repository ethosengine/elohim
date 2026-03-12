/**
 * Seed Stewardship - Distribute Content Across Stewards by Affinity
 *
 * From the Manifesto (Part IV-C):
 * "Content isn't ever owned by who might create it, it's stewarded by whoever
 * has the most relational connection to the content itself."
 *
 * This script:
 * 1. Loads all contributor presences from genesis/data/lamad/presences/
 * 2. Creates/verifies all presences in doorway
 * 3. Reads content categories from genesis/data/lamad/content/ seed files
 * 4. Allocates content to stewards based on category-affinity mapping
 * 5. Each content item gets multiple stewards with proportional ratios
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-stewardship.ts
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-stewardship.ts --dry-run
 *   DOORWAY_URL=https://doorway-alpha.elohim.host DOORWAY_API_KEY=xxx npx tsx src/seed-stewardship.ts
 */

import { DoorwayClient } from './doorway-client.js';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

// =============================================================================
// Configuration
// =============================================================================

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const DOORWAY_URL = process.env.DOORWAY_URL || process.env.STORAGE_URL || 'http://localhost:8888';
const API_KEY = process.env.DOORWAY_API_KEY;
const PRESENCES_DIR = path.join(__dirname, '../../data/lamad/presences');
const CONTENT_DIR = path.join(__dirname, '../../data/lamad/content');
const DRY_RUN = process.argv.includes('--dry-run');

// =============================================================================
// Types
// =============================================================================

interface PresenceData {
  id: string;
  displayName: string;
  presenceState: string;
  externalIdentifiers?: Array<{ platform: string; identifier: string }>;
  establishingContentIds: string[];
  claimedAgentId?: string;
  note?: string;
  metadata?: Record<string, unknown>;
}

interface CreateAllocationInput {
  content_id: string;
  steward_presence_id: string;
  allocation_ratio: number;
  allocation_method: string;
  contribution_type: string;
  note?: string;
}

interface BulkAllocationResult {
  created: number;
  failed: number;
  errors: string[];
}

interface StewardRatio {
  presenceId: string;
  ratio: number;
}

// =============================================================================
// Category-to-Steward Affinity Mapping
//
// Each category maps to an array of stewards with proportional ratios.
// Ratios represent relational affinity, not ownership. They sum to 1.0.
// =============================================================================

const CATEGORY_STEWARD_MAP: Record<string, StewardRatio[]> = {
  // Care economy: Adam (gardener-steward) primary, Susan (family), Matthew (founder), Frank (ecology)
  'value-scanner': [
    { presenceId: 'adam-firstman', ratio: 0.35 },
    { presenceId: 'jessica-spouse', ratio: 0.25 },
    { presenceId: 'matthew-dowell', ratio: 0.20 },
    { presenceId: 'frank-farmer', ratio: 0.20 },
  ],

  // Truth-seeking: Eve (first to reach for knowledge) primary, Nancy (community organizer), Matthew
  'public-observer': [
    { presenceId: 'eve-firstwoman', ratio: 0.50 },
    { presenceId: 'nancy-neighbor', ratio: 0.30 },
    { presenceId: 'matthew-dowell', ratio: 0.20 },
  ],

  // Constitutional oversight: Nancy (community leader) primary, Matthew (founder), Eve (truth-seeking)
  'governance': [
    { presenceId: 'nancy-neighbor', ratio: 0.40 },
    { presenceId: 'matthew-dowell', ratio: 0.35 },
    { presenceId: 'eve-firstwoman', ratio: 0.25 },
  ],

  // Business roles: Meriadoc (investor) primary, Matthew, Frank (local economy)
  'autonomous-entity': [
    { presenceId: 'meriadoc-moneybags', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.30 },
    { presenceId: 'frank-farmer', ratio: 0.20 },
  ],

  // Digital relationships: Eve (courage, family-systems), Susan (community-building), Matthew
  'social-medium': [
    { presenceId: 'eve-firstwoman', ratio: 0.45 },
    { presenceId: 'jessica-spouse', ratio: 0.30 },
    { presenceId: 'matthew-dowell', ratio: 0.25 },
  ],

  // Scripture: Pete (pastor) primary, Matthew (theology)
  'scripture': [
    { presenceId: 'pete-pastor', ratio: 0.60 },
    { presenceId: 'matthew-dowell', ratio: 0.40 },
  ],

  // Foundations for Christian Technology
  'fct': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-media': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-practice': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-narrative': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-activity': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],

  // REA economics: Meriadoc (impact investing), Frank (local economy), Matthew
  'economic-coordination': [
    { presenceId: 'meriadoc-moneybags', ratio: 0.40 },
    { presenceId: 'frank-farmer', ratio: 0.35 },
    { presenceId: 'matthew-dowell', ratio: 0.25 },
  ],

  // Community: Nancy (block captain), Adam (garden club), Matthew
  'community': [
    { presenceId: 'nancy-neighbor', ratio: 0.40 },
    { presenceId: 'adam-firstman', ratio: 0.30 },
    { presenceId: 'matthew-dowell', ratio: 0.30 },
  ],

  // Local economy: Frank (farmer), Meriadoc (investor), Matthew
  'local-economy': [
    { presenceId: 'frank-farmer', ratio: 0.40 },
    { presenceId: 'meriadoc-moneybags', ratio: 0.35 },
    { presenceId: 'matthew-dowell', ratio: 0.25 },
  ],

  // Technical foundation: Dan (developer), Matthew (founder)
  'foundation': [
    { presenceId: 'dan-developer', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],

  // Contributor content: Dan (developer), Matthew
  'contributor': [
    { presenceId: 'dan-developer', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],

  // General/landing pages: Matthew as primary
  'general': [
    { presenceId: 'matthew-dowell', ratio: 0.60 },
    { presenceId: 'dan-developer', ratio: 0.40 },
  ],
  'landing-page-concept': [
    { presenceId: 'matthew-dowell', ratio: 1.0 },
  ],

  // Algorithmic bias: Eve (truth-seeking), Matthew
  'algorithmic-bias': [
    { presenceId: 'eve-firstwoman', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
};

// =============================================================================
// Doorway Client Extensions
// =============================================================================

class StewardshipClient extends DoorwayClient {
  async getAllContentIds(): Promise<string[]> {
    const response = await this.fetch('/api/db/content?limit=10000', {
      method: 'GET',
    });

    if (!response.ok) {
      throw new Error(`Failed to get content: ${response.status}`);
    }

    const content = (await response.json()) as Array<{ id: string }>;
    return content.map((c) => c.id);
  }

  async getContentWithAllocations(): Promise<Set<string>> {
    const response = await this.fetch('/api/db/allocations?active_only=true&limit=10000', {
      method: 'GET',
    });

    if (!response.ok) {
      if (response.status === 404) {
        console.log('   Allocations endpoint not available, assuming no existing allocations');
        return new Set();
      }
      throw new Error(`Failed to get allocations: ${response.status}`);
    }

    const allocations = (await response.json()) as Array<{ content_id: string }>;
    return new Set(allocations.map((a) => a.content_id));
  }

  async presenceExists(presenceId: string): Promise<boolean> {
    const response = await this.fetch(`/api/db/presences/${presenceId}`, {
      method: 'GET',
    });
    return response.ok;
  }

  async createPresence(data: PresenceData): Promise<void> {
    const body = {
      id: data.id,
      display_name: data.displayName,
      presence_state: data.presenceState,
      external_identifiers_json: data.externalIdentifiers
        ? JSON.stringify(data.externalIdentifiers)
        : null,
      establishing_content_ids_json: JSON.stringify(data.establishingContentIds),
      claimed_agent_id: data.claimedAgentId,
      note: data.note,
      metadata_json: data.metadata ? JSON.stringify(data.metadata) : null,
    };

    const response = await this.fetch('/api/db/presences', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to create presence: ${error}`);
    }
  }

  async bulkCreateAllocations(inputs: CreateAllocationInput[]): Promise<BulkAllocationResult> {
    const response = await this.fetch('/api/db/allocations/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(inputs),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to bulk create allocations: ${error}`);
    }

    return response.json();
  }
}

// =============================================================================
// Content Category Reader
// =============================================================================

/**
 * Build a map of content ID -> category from the seed data files on disk.
 * This avoids needing to parse metadata from the API.
 */
function buildContentCategoryMap(): Map<string, string> {
  const categoryMap = new Map<string, string>();

  if (!fs.existsSync(CONTENT_DIR)) {
    console.log(`   WARNING: Content directory not found: ${CONTENT_DIR}`);
    return categoryMap;
  }

  const files = fs.readdirSync(CONTENT_DIR).filter((f) => f.endsWith('.json'));

  for (const file of files) {
    try {
      const data = JSON.parse(fs.readFileSync(path.join(CONTENT_DIR, file), 'utf-8'));
      if (data.id && data.metadata?.category) {
        categoryMap.set(data.id, data.metadata.category);
      }
    } catch {
      // Skip malformed files
    }
  }

  return categoryMap;
}

// =============================================================================
// Allocation Logic
// =============================================================================

/**
 * Get the steward allocation for a content item based on its category.
 * Falls back to Matthew as sole steward for unknown categories.
 */
function getStewardAllocations(contentId: string, category: string | undefined): StewardRatio[] {
  if (category && CATEGORY_STEWARD_MAP[category]) {
    return CATEGORY_STEWARD_MAP[category];
  }

  // Default: Matthew as bootstrap steward
  return [{ presenceId: 'matthew-dowell', ratio: 1.0 }];
}

// =============================================================================
// Presence Loader
// =============================================================================

function loadAllPresences(): PresenceData[] {
  if (!fs.existsSync(PRESENCES_DIR)) {
    console.error(`   ERROR: Presences directory not found: ${PRESENCES_DIR}`);
    process.exit(1);
  }

  const files = fs.readdirSync(PRESENCES_DIR).filter((f) => f.endsWith('.json'));
  const presences: PresenceData[] = [];

  for (const file of files) {
    try {
      const data = JSON.parse(fs.readFileSync(path.join(PRESENCES_DIR, file), 'utf-8'));
      presences.push(data);
    } catch (err) {
      console.error(`   WARNING: Could not parse ${file}: ${err}`);
    }
  }

  return presences;
}

// =============================================================================
// Main Script
// =============================================================================

async function main() {
  console.log('='.repeat(60));
  console.log('Stewardship Allocation Seeder');
  console.log(DRY_RUN ? '  (DRY RUN - no changes will be made)' : '');
  console.log('='.repeat(60));
  console.log();

  // Step 1: Load all presences
  console.log('Loading contributor presences...');
  const presences = loadAllPresences();
  console.log(`   Loaded ${presences.length} presences:`);
  for (const p of presences) {
    const role = (p.metadata?.role as string) || 'unknown';
    console.log(`     - ${p.displayName} (${p.id}) [${role}]`);
  }
  console.log();

  // Step 2: Build content category map from seed data
  console.log('Building content category map from seed data...');
  const categoryMap = buildContentCategoryMap();
  console.log(`   Mapped ${categoryMap.size} content items to categories`);

  // Show category distribution
  const categoryStats = new Map<string, number>();
  for (const cat of categoryMap.values()) {
    categoryStats.set(cat, (categoryStats.get(cat) || 0) + 1);
  }
  const sortedCategories = [...categoryStats.entries()].sort((a, b) => b[1] - a[1]);
  for (const [cat, count] of sortedCategories.slice(0, 10)) {
    const mapped = CATEGORY_STEWARD_MAP[cat] ? 'mapped' : 'default->matthew';
    console.log(`     ${cat}: ${count} items (${mapped})`);
  }
  if (sortedCategories.length > 10) {
    console.log(`     ... and ${sortedCategories.length - 10} more categories`);
  }
  console.log();

  if (DRY_RUN) {
    // Show allocation preview
    console.log('Allocation preview (dry run):');
    const stewardCounts = new Map<string, number>();
    const stewardTotalRatio = new Map<string, number>();

    for (const [contentId] of categoryMap) {
      const category = categoryMap.get(contentId);
      const allocations = getStewardAllocations(contentId, category);
      for (const alloc of allocations) {
        stewardCounts.set(alloc.presenceId, (stewardCounts.get(alloc.presenceId) || 0) + 1);
        stewardTotalRatio.set(
          alloc.presenceId,
          (stewardTotalRatio.get(alloc.presenceId) || 0) + alloc.ratio
        );
      }
    }

    const sortedStewards = [...stewardCounts.entries()].sort((a, b) => b[1] - a[1]);
    console.log();
    console.log('   Steward                  | Content Items | Weighted Share');
    console.log('   ' + '-'.repeat(56));
    for (const [steward, count] of sortedStewards) {
      const weightedShare = stewardTotalRatio.get(steward) || 0;
      const name = presences.find((p) => p.id === steward)?.displayName || steward;
      console.log(
        `   ${name.padEnd(25)} | ${String(count).padStart(13)} | ${weightedShare.toFixed(1)}`
      );
    }

    const totalAllocations = [...stewardCounts.values()].reduce((a, b) => a + b, 0);
    console.log();
    console.log(`   Total allocation records: ${totalAllocations}`);
    console.log(`   Average stewards per item: ${(totalAllocations / categoryMap.size).toFixed(1)}`);
    console.log();
    console.log('Dry run complete. Run without --dry-run to apply.');
    return;
  }

  // Step 3: Connect to doorway
  console.log('Connecting to doorway...');
  console.log(`   URL: ${DOORWAY_URL}`);

  const client = new StewardshipClient({
    baseUrl: DOORWAY_URL,
    apiKey: API_KEY,
  });

  try {
    const health = await client.checkHealth();
    if (!health.healthy) {
      console.error('   ERROR: Doorway is not healthy');
      process.exit(1);
    }
    console.log('   Connected successfully');
  } catch (error) {
    console.error(`   ERROR: Could not connect to doorway: ${error}`);
    process.exit(1);
  }
  console.log();

  // Step 4: Create/verify all presences
  console.log('Creating contributor presences...');
  for (const presence of presences) {
    const exists = await client.presenceExists(presence.id);
    if (exists) {
      console.log(`   "${presence.id}" already exists, skipping`);
    } else {
      try {
        await client.createPresence(presence);
        console.log(`   Created: ${presence.displayName} (${presence.id})`);
      } catch (error) {
        console.error(`   ERROR creating ${presence.id}: ${error}`);
      }
    }
  }
  console.log();

  // Step 5: Get content that already has allocations
  console.log('Checking existing allocations...');
  let contentWithAllocations: Set<string>;
  try {
    contentWithAllocations = await client.getContentWithAllocations();
    console.log(`   Found ${contentWithAllocations.size} content items with existing allocations`);
  } catch (error) {
    console.error(`   ERROR: Failed to get allocations: ${error}`);
    process.exit(1);
  }
  console.log();

  // Step 6: Get all content IDs from the database
  console.log('Getting all content from doorway...');
  let allContentIds: string[];
  try {
    allContentIds = await client.getAllContentIds();
    console.log(`   Found ${allContentIds.length} content items in database`);
  } catch (error) {
    console.error(`   ERROR: Failed to get content: ${error}`);
    process.exit(1);
  }
  console.log();

  // Step 7: Build allocations for content without existing ones
  const contentNeedingAllocations = allContentIds.filter((id) => !contentWithAllocations.has(id));
  console.log(`Building allocations for ${contentNeedingAllocations.length} content items...`);

  if (contentNeedingAllocations.length === 0) {
    console.log('   All content already has allocations, nothing to do');
    console.log();
    console.log('Done!');
    return;
  }

  const allocations: CreateAllocationInput[] = [];

  for (const contentId of contentNeedingAllocations) {
    const category = categoryMap.get(contentId);
    const stewards = getStewardAllocations(contentId, category);

    for (const steward of stewards) {
      allocations.push({
        content_id: contentId,
        steward_presence_id: steward.presenceId,
        allocation_ratio: steward.ratio,
        allocation_method: 'affinity',
        contribution_type: 'steward',
        note: category
          ? `Affinity-based stewardship for ${category} content`
          : 'Bootstrap steward assignment - uncategorized content',
      });
    }
  }

  console.log(`   Generated ${allocations.length} allocation records`);
  console.log(
    `   Average stewards per item: ${(allocations.length / contentNeedingAllocations.length).toFixed(1)}`
  );
  console.log();

  // Step 8: Bulk create allocations (in batches to avoid overwhelming the API)
  const BATCH_SIZE = 500;
  let totalCreated = 0;
  let totalFailed = 0;
  const allErrors: string[] = [];

  console.log(`Seeding allocations in batches of ${BATCH_SIZE}...`);

  for (let i = 0; i < allocations.length; i += BATCH_SIZE) {
    const batch = allocations.slice(i, i + BATCH_SIZE);
    const batchNum = Math.floor(i / BATCH_SIZE) + 1;
    const totalBatches = Math.ceil(allocations.length / BATCH_SIZE);

    try {
      const result = await client.bulkCreateAllocations(batch);
      totalCreated += result.created;
      totalFailed += result.failed;
      allErrors.push(...result.errors);
      console.log(
        `   Batch ${batchNum}/${totalBatches}: ${result.created} created, ${result.failed} failed`
      );
    } catch (error) {
      console.error(`   ERROR in batch ${batchNum}: ${error}`);
      totalFailed += batch.length;
    }
  }

  console.log();
  console.log('='.repeat(60));
  console.log('Stewardship allocation complete!');
  console.log(`   Created: ${totalCreated}`);
  console.log(`   Failed: ${totalFailed}`);
  if (allErrors.length > 0) {
    console.log('   Errors:');
    allErrors.slice(0, 10).forEach((e) => console.log(`     - ${e}`));
    if (allErrors.length > 10) {
      console.log(`     ... and ${allErrors.length - 10} more`);
    }
  }

  // Show steward distribution summary
  const stewardSummary = new Map<string, number>();
  for (const alloc of allocations) {
    stewardSummary.set(
      alloc.steward_presence_id,
      (stewardSummary.get(alloc.steward_presence_id) || 0) + 1
    );
  }
  console.log();
  console.log('   Steward distribution:');
  const sorted = [...stewardSummary.entries()].sort((a, b) => b[1] - a[1]);
  for (const [steward, count] of sorted) {
    const name = presences.find((p) => p.id === steward)?.displayName || steward;
    console.log(`     ${name}: ${count} allocations`);
  }

  console.log('='.repeat(60));
}

// Run
main().catch((error) => {
  console.error('Fatal error:', error);
  process.exit(1);
});
