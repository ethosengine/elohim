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
 * (matthew-desktop ↔ jessica-desktop, both directions, one blob — the
 * in-household pair; restored 2026-06-04 after the two-sweep divergent
 * rename RCA: the timothy→terrance seeder sweep and the timothy→jessica
 * Jenkinsfile sweep chose different successors, and no validation tied
 * the seeded pair to the documented story. The named-pair a2o scenario in
 * features/resilience/household-reciprocity.feature is the standing flag.)
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Suspended-persona guard
// =============================================================================

/**
 * Fail-fast when a custody pair references a persona suspended in
 * deployments.json. This is the flag that was MISSING during the 2026-06-04
 * terrance-drift RCA: the seeder silently seeded custody pairs for a persona
 * whose deployment was suspended (dead data, zero complaint), letting a
 * divergent persona-rename sweep sit unnoticed for three weeks.
 * deployments.json already gates deploy + seed-humans + a2o tests; custody-pair
 * seeding now respects it too.
 *
 * Mirrors loadSuspendedNames() in seed-humans.ts. Fail-open on an unreadable
 * registry (partial availability is the steady state); fail-FAST on a readable
 * registry that suspends a referenced persona.
 */
function loadSuspendedHumanPrefixes(): Set<string> {
  try {
    const here = dirname(fileURLToPath(import.meta.url));
    const jsonPath = resolve(here, '../../orchestrator/data/deployments.json');
    const raw = readFileSync(jsonPath, 'utf-8');
    const data = JSON.parse(raw) as { humans?: { name: string; suspended?: boolean }[] };
    const suspended = new Set<string>();
    for (const h of data.humans ?? []) {
      if (h.suspended) suspended.add(`human-${h.name.toLowerCase()}`);
    }
    return suspended;
  } catch {
    return new Set();
  }
}

/** Exit 1 when any pair references a suspended persona (exported for tests). */
export function assertPairsNotSuspended(
  pairs: CustodyPair[],
  suspended: Set<string>
): string[] {
  const offenders = [
    ...new Set(
      pairs
        .flatMap(p => [p.providerHumanId, p.receiverHumanId])
        .filter(id => [...suspended].some(prefix => id.startsWith(prefix)))
    ),
  ];
  if (offenders.length > 0) {
    console.error(
      `ERROR: custody pair(s) reference SUSPENDED persona(s): ${offenders.join(', ')}.\n` +
        `deployments.json suspends them — seeding custody for non-deployed peers is dead data.\n` +
        `Fix the pair set (or un-suspend the persona) before seeding.`
    );
    process.exit(1);
  }
  return offenders;
}

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
      receiverHumanId: 'human-jessica-spouse',
      receiverArchetype: 'desktop',
      blobHash: M1_DEFAULT_BLOB_HASH,
      blobSizeBytes: M1_DEFAULT_BLOB_SIZE,
    },
    {
      providerHumanId: 'human-jessica-spouse',
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

  // The terrance-drift flag: never seed custody for suspended personas.
  assertPairsNotSuspended(pairs, loadSuspendedHumanPrefixes());

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
