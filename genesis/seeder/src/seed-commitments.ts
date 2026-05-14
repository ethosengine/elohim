/**
 * Seed REA Custody-Blob Commitments
 *
 * Writes peer_id-keyed custody-blob commitments via the doorway REST POST
 * /api/v1/commitments handler. The shape exactly matches what
 * `reciprocity_view`, `cluster_view`, and `distribution_view` SQL filters
 * expect:
 *   action = "custody-blob"
 *   provider, receiver = 12D3KooW... peer_ids (NOT human-* agent_cids)
 *   resource_classified_as = "sha256-<blob_hash>"
 *   resource_quantity_value = bytes count
 *
 * Drift in any of these fields = views silently filter out the row.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-commitments.ts
 *   DOORWAY_URL=https://doorway-alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     CUSTODY_PAIRS_JSON=./pairs.json npx tsx src/seed-commitments.ts
 *
 * If CUSTODY_PAIRS_JSON is not set, falls back to the M1 default pair set
 * (matthew-desktop ↔ terrance-desktop, both directions, one blob).
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Types
// =============================================================================

export interface CustodyPair {
  providerHumanId: string;
  providerArchetype: Archetype;
  receiverHumanId: string;
  receiverArchetype: Archetype;
  blobHash: string;       // raw hex (no prefix), or full "sha256-<hex>"
  blobSizeBytes: number;
}

interface CommitmentBody {
  id: string;
  action: 'custody-blob';
  provider: string;
  receiver: string;
  resourceConformsTo: 'blob';
  resourceClassifiedAs: string;
  resourceQuantity: { hasNumericalValue: number; hasUnit: 'B' };
  note: string;
  metadata: Record<string, unknown>;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

function normalizeBlobHash(input: string): string {
  return input.startsWith('sha256-') ? input : `sha256-${input}`;
}

/**
 * Build a CreateReaCommitmentInputView body for a single custody-blob pair.
 *
 * `id` is a deterministic content-addressed hash of the (provider_peer,
 * receiver_peer, blob_hash) tuple — re-runs produce identical ids and the
 * doorway POST handler returns 409 (idempotent). Distinct tuples produce
 * distinct ids so genuine shape drift surfaces as 400 (fail-fast).
 */
export function buildCustodyCommitmentBody(pair: CustodyPair): CommitmentBody {
  const providerPeerId = deterministicPeerId(pair.providerHumanId, pair.providerArchetype);
  const receiverPeerId = deterministicPeerId(pair.receiverHumanId, pair.receiverArchetype);
  const blob = normalizeBlobHash(pair.blobHash);

  const idDigest = createHash('sha256')
    .update(`${providerPeerId}|${receiverPeerId}|${blob}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `custody-blob-${idDigest}`,
    action: 'custody-blob',
    provider: providerPeerId,
    receiver: receiverPeerId,
    resourceConformsTo: 'blob',
    resourceClassifiedAs: blob,
    resourceQuantity: { hasNumericalValue: pair.blobSizeBytes, hasUnit: 'B' },
    note: `${pair.providerHumanId} commits to host ${blob} for ${pair.receiverHumanId}`,
    metadata: {
      seedGeneration: 'genesis',
      blobHash: blob,
      providerHumanId: pair.providerHumanId,
      receiverHumanId: pair.receiverHumanId,
    },
  };
}

// =============================================================================
// Default M1 pair set
// =============================================================================

const M1_DEFAULT_BLOB_HASH = process.env.M1_BLOB_HASH || '';
const M1_DEFAULT_BLOB_SIZE = parseInt(process.env.M1_BLOB_SIZE_BYTES || '0', 10);

function defaultM1Pairs(): CustodyPair[] {
  if (!M1_DEFAULT_BLOB_HASH || M1_DEFAULT_BLOB_SIZE <= 0) {
    console.error(
      'ERROR: M1_BLOB_HASH and M1_BLOB_SIZE_BYTES must be set (or pass CUSTODY_PAIRS_JSON).',
    );
    process.exit(1);
  }
  return [
    {
      providerHumanId: 'human-matthew-manager',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-terrance-tutor',
      receiverArchetype: 'desktop',
      blobHash: M1_DEFAULT_BLOB_HASH,
      blobSizeBytes: M1_DEFAULT_BLOB_SIZE,
    },
    {
      providerHumanId: 'human-terrance-tutor',
      providerArchetype: 'desktop',
      receiverHumanId: 'human-matthew-manager',
      receiverArchetype: 'desktop',
      blobHash: M1_DEFAULT_BLOB_HASH,
      blobSizeBytes: M1_DEFAULT_BLOB_SIZE,
    },
  ];
}

// =============================================================================
// Client
// =============================================================================

class CommitmentClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

// =============================================================================
// Seeding (fail-fast on non-409 errors)
// =============================================================================

export async function seedCustodyCommitments(
  client: CommitmentClient,
  pairs: CustodyPair[],
): Promise<void> {
  console.log(`[seed-commitments] Seeding ${pairs.length} custody-blob commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const pair of pairs) {
    const body = buildCustodyCommitmentBody(pair);
    const label = `${pair.providerHumanId.replace(/^human-/, '')}→${pair.receiverHumanId.replace(/^human-/, '')}`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label} ${pair.blobHash.slice(0, 16)}...`);
      created += 1;
      continue;
    }

    const text = await response.text();
    if (response.status === 409 || text.includes('UNIQUE') || text.includes('already exists')) {
      console.log(`  [=] ${label} (idempotent re-run)`);
      alreadyExists += 1;
      continue;
    }

    // ANY other failure is a shape mismatch or doorway issue — fail fast.
    console.error(`  [X] ${label}: HTTP ${response.status}`);
    console.error(`      Body: ${text.slice(0, 500)}`);
    console.error(`      Sent: ${JSON.stringify(body, null, 2)}`);
    process.exit(1);
  }

  console.log(
    `[seed-commitments] Done. created=${created} already-exists=${alreadyExists} total=${pairs.length}`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const pairsJsonPath = process.env.CUSTODY_PAIRS_JSON;
  const pairs: CustodyPair[] = pairsJsonPath
    ? (JSON.parse(readFileSync(pairsJsonPath, 'utf-8')) as CustodyPair[])
    : defaultM1Pairs();

  const client = new CommitmentClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('REA Custody-Blob Commitment Seeder');
  console.log(`  Target: ${doorwayUrl}`);
  console.log(`  Pairs:  ${pairs.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedCustodyCommitments(client, pairs);
  process.exit(0);
}
