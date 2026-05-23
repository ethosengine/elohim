/**
 * Seed Doorway-Operator Bindings (REA operate-doorway commitments)
 *
 * Each binding is an REA Commitment with action='operate-doorway' that names
 * a human as an operator of a specific doorway. The doorway's auth layer
 * reads these via `find_active_operator_binding` and embeds the capability
 * snapshot into JWTs issued to the operator.
 *
 * Substrate references:
 *   - elohim/sdk/schemas/v1/objects/operator-classification.schema.json
 *   - elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
 *   - elohim/elohim-storage/src/db/rea_commitments.rs (OPERATE_DOORWAY_ACTION)
 *   - genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
 *
 * Sister to seed-commitments.ts (custody-blob) — same POST endpoint, different
 * action discriminator. Same idempotency story: id is content-addressed over
 * (operator_peer_id, action, scope), re-runs collapse to 409.
 *
 * Default M5 binding set:
 *   - human-matthew-manager  →  doorway:alpha-elohim-host  (primary, *)
 *   - human-matthew-manager  →  doorway:apex-elohim-host   (primary, *)
 *
 * One human, two doorways, one capability scope. Federation pair
 * alpha.elohim.host ↔ elohim.host both list Matthew as primary operator.
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-operator-bindings.ts
 *   DOORWAY_URL=https://alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     OPERATOR_BINDINGS_JSON=./bindings.json npx tsx src/seed-operator-bindings.ts
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Types
// =============================================================================

export type SuccessionRole = 'primary' | 'deputy' | 'recovery-witness';
export type ReachScope = 'operator-private' | 'stewards-only' | 'public';

export interface OperatorBinding {
  operatorHumanId: string;
  operatorArchetype: Archetype;
  doorwayId: string;
  capabilities: string[];
  successionRole: SuccessionRole;
  reachScope?: ReachScope;
}

interface CommitmentBody {
  id: string;
  action: 'operate-doorway';
  provider: string;
  receiver: string;
  resourceClassifiedAs: string[];
  inScopeOf: string[];
  note: string;
  metadata: Record<string, unknown>;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

/**
 * Build a CreateReaCommitmentInputView body for one operator binding.
 *
 * `id` is content-addressed over (operator_peer_id, action, scope) so re-runs
 * are idempotent — POST returns 409 on the second invocation. Distinct
 * (operator, doorway) pairs produce distinct ids; the same operator on a
 * different doorway is a separate binding row.
 *
 * `provider` is the operator's deterministic peer_id (Stage 1 opaque hash,
 * see peer-id.ts). `receiver` is the same peer_id by convention — REA's
 * provider/receiver semantics don't apply cleanly to operate-doorway (the
 * "service" is doorway operation, not a transfer between agents). The auth
 * resolver filters by provider + scope; receiver is informational.
 */
export function buildOperatorCommitmentBody(binding: OperatorBinding): CommitmentBody {
  const peerId = deterministicPeerId(binding.operatorHumanId, binding.operatorArchetype);
  const scope = `doorway:${binding.doorwayId}`;

  const idDigest = createHash('sha256')
    .update(`${peerId}|operate-doorway|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  return {
    id: `operate-doorway-${idDigest}`,
    action: 'operate-doorway',
    provider: peerId,
    receiver: peerId,
    resourceClassifiedAs: binding.capabilities,
    inScopeOf: [scope],
    note: `${binding.operatorHumanId} as ${binding.successionRole} operator of ${binding.doorwayId}`,
    metadata: {
      schemaVersion: 1,
      successionRole: binding.successionRole,
      reachScope: binding.reachScope ?? 'operator-private',
      seedGeneration: 'genesis',
      operatorHumanId: binding.operatorHumanId,
    },
  };
}

// =============================================================================
// Default M5 binding set
//
// One human (Matthew) registered as primary operator of both alpha-cluster
// doorways. Each doorway projects the same elohim-host-landing EPR; the
// operator binding is per-doorway but the human is the same.
// =============================================================================

function defaultM5Bindings(): OperatorBinding[] {
  return [
    {
      operatorHumanId: 'human-matthew-manager',
      operatorArchetype: 'desktop',
      doorwayId: 'alpha-elohim-host',
      capabilities: ['*'],
      successionRole: 'primary',
      reachScope: 'stewards-only',
    },
    {
      operatorHumanId: 'human-matthew-manager',
      operatorArchetype: 'desktop',
      doorwayId: 'apex-elohim-host',
      capabilities: ['*'],
      successionRole: 'primary',
      reachScope: 'stewards-only',
    },
  ];
}

// =============================================================================
// Client
// =============================================================================

class OperatorBindingClient extends DoorwayClient {
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

export async function seedOperatorBindings(
  client: OperatorBindingClient,
  bindings: OperatorBinding[],
): Promise<void> {
  console.log(`[seed-operator-bindings] Seeding ${bindings.length} operate-doorway commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const binding of bindings) {
    const body = buildOperatorCommitmentBody(binding);
    const label = `${binding.operatorHumanId.replace(/^human-/, '')}→${binding.doorwayId} (${binding.successionRole})`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label}  caps=[${binding.capabilities.join(',')}]`);
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
    `[seed-operator-bindings] Done. created=${created} already-exists=${alreadyExists} total=${bindings.length}`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const bindingsJsonPath = process.env.OPERATOR_BINDINGS_JSON;
  const bindings: OperatorBinding[] = bindingsJsonPath
    ? (JSON.parse(readFileSync(bindingsJsonPath, 'utf-8')) as OperatorBinding[])
    : defaultM5Bindings();

  const client = new OperatorBindingClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('Doorway-Operator Binding Seeder');
  console.log(`  Target:   ${doorwayUrl}`);
  console.log(`  Bindings: ${bindings.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedOperatorBindings(client, bindings);
  process.exit(0);
}
