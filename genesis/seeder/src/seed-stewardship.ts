/**
 * Seed Stewardship - Bootstrap Matthew Dowell as Initial Content Steward
 *
 * From the Manifesto (Part IV-C):
 * "Content isn't ever owned by who might create it, it's stewarded by whoever
 * has the most relational connection to the content itself."
 *
 * This script:
 * 1. Creates Matthew Dowell's contributor presence (if not exists)
 * 2. Queries all content in the database
 * 3. Creates stewardship allocations for all content with no existing allocations
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-stewardship.ts
 *   DOORWAY_URL=https://doorway-dev.elohim.host DOORWAY_API_KEY=xxx npx tsx src/seed-stewardship.ts
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
const PRESENCE_FILE = path.join(__dirname, '../../data/lamad/presences/matthew-dowell.json');

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

// =============================================================================
// Doorway Client Extensions
// =============================================================================

/**
 * Extended DoorwayClient with stewardship allocation methods.
 */
class StewardshipClient extends DoorwayClient {
  /**
   * Get all content IDs from the database.
   */
  async getAllContentIds(): Promise<string[]> {
    const response = await this.fetch('/api/db/content?limit=10000', {
      method: 'GET',
    });

    if (!response.ok) {
      throw new Error(`Failed to get content: ${response.status}`);
    }

    const content = await response.json() as Array<{ id: string }>;
    return content.map(c => c.id);
  }

  /**
   * Get content IDs that already have allocations.
   */
  async getContentWithAllocations(): Promise<Set<string>> {
    const response = await this.fetch('/api/db/allocations?active_only=true&limit=10000', {
      method: 'GET',
    });

    if (!response.ok) {
      // If endpoint doesn't exist yet, return empty set
      if (response.status === 404) {
        console.log('   Allocations endpoint not available, assuming no existing allocations');
        return new Set();
      }
      throw new Error(`Failed to get allocations: ${response.status}`);
    }

    const allocations = await response.json() as Array<{ content_id: string }>;
    return new Set(allocations.map(a => a.content_id));
  }

  /**
   * Check if a presence exists.
   */
  async presenceExists(presenceId: string): Promise<boolean> {
    const response = await this.fetch(`/api/db/presences/${presenceId}`, {
      method: 'GET',
    });

    return response.ok;
  }

  /**
   * Create a contributor presence.
   */
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

  /**
   * Bulk create stewardship allocations.
   */
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
// Main Script
// =============================================================================

async function main() {
  console.log('='.repeat(60));
  console.log('Stewardship Bootstrap Seeder');
  console.log('='.repeat(60));
  console.log();

  // Load presence data
  console.log('📄 Loading Matthew Dowell presence data...');
  if (!fs.existsSync(PRESENCE_FILE)) {
    console.error(`   ERROR: Presence file not found: ${PRESENCE_FILE}`);
    process.exit(1);
  }

  const presenceData: PresenceData = JSON.parse(fs.readFileSync(PRESENCE_FILE, 'utf-8'));
  console.log(`   Loaded: ${presenceData.displayName} (${presenceData.id})`);
  console.log();

  // Create client
  console.log('🔌 Connecting to doorway...');
  console.log(`   URL: ${DOORWAY_URL}`);

  const client = new StewardshipClient({
    baseUrl: DOORWAY_URL,
    apiKey: API_KEY,
  });

  // Check health
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

  // Step 1: Create/verify Matthew Dowell presence
  console.log('👤 Creating contributor presence...');
  const presenceExists = await client.presenceExists(presenceData.id);
  if (presenceExists) {
    console.log(`   Presence "${presenceData.id}" already exists, skipping creation`);
  } else {
    try {
      await client.createPresence(presenceData);
      console.log(`   Created presence: ${presenceData.displayName}`);
    } catch (error) {
      console.error(`   ERROR: Failed to create presence: ${error}`);
      process.exit(1);
    }
  }
  console.log();

  // Step 2: Get all content IDs
  console.log('📚 Getting all content...');
  let allContentIds: string[];
  try {
    allContentIds = await client.getAllContentIds();
    console.log(`   Found ${allContentIds.length} content items`);
  } catch (error) {
    console.error(`   ERROR: Failed to get content: ${error}`);
    process.exit(1);
  }
  console.log();

  // Step 3: Get content that already has allocations
  console.log('🔍 Checking existing allocations...');
  let contentWithAllocations: Set<string>;
  try {
    contentWithAllocations = await client.getContentWithAllocations();
    console.log(`   Found ${contentWithAllocations.size} content items with existing allocations`);
  } catch (error) {
    console.error(`   ERROR: Failed to get allocations: ${error}`);
    process.exit(1);
  }
  console.log();

  // Step 4: Create allocations for content without
  const contentNeedingAllocations = allContentIds.filter(id => !contentWithAllocations.has(id));
  console.log(`📝 Creating allocations for ${contentNeedingAllocations.length} content items...`);

  if (contentNeedingAllocations.length === 0) {
    console.log('   All content already has allocations, nothing to do');
    console.log();
    console.log('✅ Bootstrap complete!');
    return;
  }

  const allocations: CreateAllocationInput[] = contentNeedingAllocations.map(contentId => ({
    content_id: contentId,
    steward_presence_id: presenceData.id,
    allocation_ratio: 1.0,
    allocation_method: 'manual',
    contribution_type: 'inherited',
    note: 'Bootstrap steward assignment - initial protocol content',
  }));

  try {
    const result = await client.bulkCreateAllocations(allocations);
    console.log(`   Created: ${result.created}`);
    console.log(`   Failed: ${result.failed}`);
    if (result.errors.length > 0) {
      console.log('   Errors:');
      result.errors.slice(0, 10).forEach(e => console.log(`     - ${e}`));
      if (result.errors.length > 10) {
        console.log(`     ... and ${result.errors.length - 10} more`);
      }
    }
  } catch (error) {
    console.error(`   ERROR: Failed to create allocations: ${error}`);
    process.exit(1);
  }

  console.log();
  console.log('='.repeat(60));
  console.log('✅ Bootstrap complete!');
  console.log(`   ${presenceData.displayName} is now steward of all content`);
  console.log('='.repeat(60));
}

// Run
main().catch(error => {
  console.error('Fatal error:', error);
  process.exit(1);
});
