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
  action: string;
  provider: string;
  receiver: string;
  /**
   * JSON-encoded array string: e.g. `'["doorway:alpha-elohim-host"]'`.
   *
   * The Rust `CreateReaCommitmentInput` (POST /api/v1/commitments) accepts
   * `in_scope_of: Option<String>` — a single string that gets stored verbatim
   * in the DB column. The read-side view (`ReaCommitmentView`) parses the
   * stored value back to `Vec<String>` via `serde_json::from_str`, so the DB
   * value MUST be a valid JSON array string. Sending a raw `string[]` array
   * causes HTTP 500 "invalid type: sequence, expected a string".
   */
  inScopeOf: string;
  /**
   * JSON-encoded array string: e.g. `'["*"]'`.
   *
   * Same wire-shape rule as inScopeOf: the Rust struct takes `Option<String>`
   * and stores verbatim; the view layer parses it back to `Vec<String>`.
   */
  resourceClassifiedAs: string;
  note: string;
  /**
   * Pre-serialized JSON string for the metadata column.
   *
   * The Rust struct field is `metadata_json: Option<String>` (camelCase key:
   * `metadataJson`). Sending `metadata: {...}` (an object) would be ignored
   * because the key does not match. Always JSON.stringify the metadata object
   * and send it under the `metadataJson` key.
   */
  metadataJson: string;
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

  // Digest input is STABLE: changing this string would change the id for
  // existing rows and cause idempotent re-runs to insert duplicates. The scope
  // value here is a plain string (not JSON), intentionally matching the original
  // digest input before the wire-shape fix.
  const idDigest = createHash('sha256')
    .update(`${peerId}|operate-doorway|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  const metadataObject = {
    schemaVersion: 1,
    successionRole: binding.successionRole,
    reachScope: binding.reachScope ?? 'operator-private',
    seedGeneration: 'genesis',
    operatorHumanId: binding.operatorHumanId,
  };

  return {
    id: `operate-doorway-${idDigest}`,
    action: 'operate-doorway',
    provider: peerId,
    receiver: peerId,
    // JSON-encoded array string — CreateReaCommitmentInput.resource_classified_as
    // is Option<String>; the view layer parses it back to Vec<String>.
    resourceClassifiedAs: JSON.stringify(binding.capabilities),
    // JSON-encoded array string — same wire-shape contract as resourceClassifiedAs.
    inScopeOf: JSON.stringify([scope]),
    note: `${binding.operatorHumanId} as ${binding.successionRole} operator of ${binding.doorwayId}`,
    // metadataJson (not metadata) — maps to CreateReaCommitmentInput.metadata_json.
    metadataJson: JSON.stringify(metadataObject),
  };
}

// =============================================================================
// Default M5 binding set
//
// One human (Matthew) registered as primary operator of both alpha-cluster
// doorways. Each doorway projects the same elohim-host-landing EPR; the
// operator binding is per-doorway but the human is the same.
// =============================================================================

export function defaultM5Bindings(): OperatorBinding[] {
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
// Hosting-Agreement Commitments
//
// In-kind REA Commitments declaring compute hosting agreements. These are
// queried by the landing-page-dogfood a2o scenario:
//
//   When I list active REA commitments where provider is "matthew"
//   Then at least one commitment has inScopeOf containing "host:alpha.elohim.host"
//   And that commitment has inScopeOf containing "epr_root:elohim-host-landing"
//   And that commitment's metadata signalKind is "compute-allocation"
//   And that commitment's metadata triggerKind is "subscription"
//
// Unlike operator bindings (where provider is a deterministic peer_id),
// hosting commitments use the human's short name literally as the provider
// so the /api/v1/commitments?provider=matthew query can locate them without
// knowing the peer_id.
// =============================================================================

export interface HostingAgreementSpec {
  /** Literal provider value — queried directly via ?provider=<value>. */
  provider: string;
  /** Scopes listed in the commitment's inScopeOf array. */
  scopes: string[];
  /** signalKind metadata field (e.g. "compute-allocation"). */
  signalKind: string;
  /** triggerKind metadata field (e.g. "subscription"). */
  triggerKind: string;
  /** Human-readable note. */
  note: string;
}

/**
 * Build a CreateReaCommitmentInput body for a hosting-agreement commitment.
 *
 * `id` is content-addressed over (provider, action, scopes-joined) so
 * idempotent re-runs collapse to 409. Changing the provider/scopes produces
 * a new id and a new row — intentional for hosting-agreement migrations.
 *
 * `inScopeOf` is a JSON-encoded array of scope strings, e.g.
 * `'["host:alpha.elohim.host","epr_root:elohim-host-landing"]'`, so both
 * scopes are individually findable by the step definition's `.includes()` check.
 *
 * `metadataJson` carries signalKind + triggerKind as a pre-serialized JSON
 * string (maps to CreateReaCommitmentInput.metadata_json).
 */
export function buildHostingAgreementBody(spec: HostingAgreementSpec): CommitmentBody {
  const scopeKey = spec.scopes.join('|');
  const idDigest = createHash('sha256')
    .update(`${spec.provider}|hosting-agreement|${scopeKey}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  const metadataObject = {
    signalKind: spec.signalKind,
    triggerKind: spec.triggerKind,
    seedGeneration: 'genesis',
  };

  return {
    id: `hosting-agreement-${idDigest}`,
    action: 'hosting-agreement',
    provider: spec.provider,
    receiver: spec.provider,
    resourceClassifiedAs: JSON.stringify(['compute']),
    inScopeOf: JSON.stringify(spec.scopes),
    note: spec.note,
    metadataJson: JSON.stringify(metadataObject),
  };
}

/**
 * Default hosting-agreement commitment set.
 *
 * Declares Matthew's in-kind hosting agreement for the alpha cluster landing
 * surface. The provider value "matthew" is the short-form literal queried by
 * the landing-page-dogfood a2o scenario.
 */
export function defaultHostingAgreements(): HostingAgreementSpec[] {
  return [
    {
      provider: 'matthew',
      scopes: ['host:alpha.elohim.host', 'epr_root:elohim-host-landing'],
      signalKind: 'compute-allocation',
      triggerKind: 'subscription',
      note: "matthew's in-kind compute hosting for elohim-host-landing on alpha.elohim.host",
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

/**
 * Factory — lets callers in seed.ts (or integration tests) construct an
 * OperatorBindingClient without importing the private class directly.
 */
export function createOperatorBindingClient(
  baseUrl: string,
  apiKey?: string,
): OperatorBindingClient {
  return new OperatorBindingClient({ baseUrl, apiKey });
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

export async function seedHostingAgreements(
  client: OperatorBindingClient,
  specs: HostingAgreementSpec[],
): Promise<void> {
  console.log(`[seed-operator-bindings] Seeding ${specs.length} hosting-agreement commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const spec of specs) {
    const body = buildHostingAgreementBody(spec);
    const label = `${spec.provider} hosting [${spec.scopes.join(', ')}]`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label}  signalKind=${spec.signalKind}`);
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
    `[seed-operator-bindings] Done. created=${created} already-exists=${alreadyExists} total=${specs.length}`,
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

  const hostingAgreements = defaultHostingAgreements();

  const client = new OperatorBindingClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('Doorway-Operator Binding Seeder');
  console.log(`  Target:           ${doorwayUrl}`);
  console.log(`  Operator bindings: ${bindings.length}`);
  console.log(`  Hosting agreements: ${hostingAgreements.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedOperatorBindings(client, bindings);
  await seedHostingAgreements(client, hostingAgreements);
  process.exit(0);
}
