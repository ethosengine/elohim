/**
 * Seed REA Storage Commitments
 *
 * Seeds storage capacity commitments for test peers.
 * Each peer declares how much storage they commit to the network.
 *
 * These are REA (Resource-Event-Agent) commitments: binding promises of future
 * economic activity. Here, each peer commits to providing storage capacity.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-commitments.ts
 *   DOORWAY_URL=https://doorway-alpha.elohim.host DOORWAY_API_KEY=xxx npx tsx src/seed-commitments.ts
 */

import { DoorwayClient } from './doorway-client.js';

// =============================================================================
// Types
// =============================================================================

interface PeerCommitment {
  id: string;
  provider: string;
  displayName: string;
  storageGb: number;
  region: string;
}

// =============================================================================
// Configuration
// =============================================================================

const PEER_COMMITMENTS: PeerCommitment[] = [
  {
    id: 'commitment-matthew-storage',
    provider: 'human-matthew-manager',
    displayName: 'Matthew',
    storageGb: 4000,
    region: 'home-lab',
  },
  {
    id: 'commitment-jessica-storage',
    provider: 'human-jessica-spouse',
    displayName: 'Jessica',
    storageGb: 2000,
    region: 'home-lab',
  },
  {
    id: 'commitment-pete-storage',
    provider: 'human-pete-pastor',
    displayName: 'Pete',
    storageGb: 1000,
    region: 'church-office',
  },
  {
    id: 'commitment-timothy-storage',
    provider: 'human-timothy-tutor',
    displayName: 'Timothy',
    storageGb: 500,
    region: 'university',
  },
  {
    id: 'commitment-frank-storage',
    provider: 'human-frank-farmer',
    displayName: 'Frank',
    storageGb: 2000,
    region: 'farm',
  },
];

// =============================================================================
// Client (extends DoorwayClient to access protected fetch)
// =============================================================================

class CommitmentClient extends DoorwayClient {
  async createCommitment(body: Record<string, unknown>): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

// =============================================================================
// Seeding Logic
// =============================================================================

/**
 * Seed storage commitments for all configured test peers.
 *
 * Each commitment follows the CreateReaCommitmentInputView shape (camelCase):
 *   - action: "provide"
 *   - provider: agent ID
 *   - receiver: "network"
 *   - resourceConformsTo: "storage-capacity"
 *   - resourceQuantity: { hasNumericalValue, hasUnit }
 *   - note: human-readable description
 */
export async function seedStorageCommitments(client: CommitmentClient): Promise<void> {
  console.log(`[seed-commitments] Seeding ${PEER_COMMITMENTS.length} storage commitments...`);

  for (const peer of PEER_COMMITMENTS) {
    try {
      const body = {
        id: peer.id,
        action: 'provide',
        provider: peer.provider,
        receiver: 'network',
        resourceConformsTo: 'storage-capacity',
        resourceQuantity: {
          hasNumericalValue: peer.storageGb,
          hasUnit: 'GB',
        },
        note: `${peer.displayName} storage node — ${peer.storageGb}GB committed (${peer.region})`,
        metadata: {
          region: peer.region,
          seedGeneration: 'genesis',
        },
      };

      const response = await client.createCommitment(body);

      if (response.ok) {
        console.log(`  [+] ${peer.displayName}: ${peer.storageGb}GB`);
      } else {
        const text = await response.text();
        if (text.includes('UNIQUE') || text.includes('already exists') || response.status === 409) {
          console.log(`  [=] ${peer.displayName}: already exists`);
        } else {
          console.warn(`  [!] ${peer.displayName}: HTTP ${response.status} — ${text}`);
        }
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('UNIQUE') || msg.includes('already exists') || msg.includes('409')) {
        console.log(`  [=] ${peer.displayName}: already exists`);
      } else {
        console.warn(`  [!] ${peer.displayName}: ${msg}`);
      }
    }
  }

  console.log('[seed-commitments] Done.');
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || process.env.STORAGE_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const client = new CommitmentClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('REA Storage Commitment Seeder');
  console.log(`  Target: ${doorwayUrl}`);
  console.log('='.repeat(60));
  console.log();

  // Pre-flight health check
  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }
  console.log('Doorway healthy, seeding commitments...');
  console.log();

  seedStorageCommitments(client)
    .then(() => process.exit(0))
    .catch((err) => {
      console.error('Seeding failed:', err);
      process.exit(1);
    });
}
