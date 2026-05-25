/**
 * Seed EPR-Projection Commitments (REA project-epr action)
 *
 * Each projection is an REA Commitment with action='project-epr' that
 * notarizes "doorway D will project EPR E at urlPath U under terms T."
 *
 * Substrate references:
 *   - elohim/sdk/schemas/v1/views/epr-projection-view.schema.json
 *   - elohim/elohim-storage/src/db/rea_commitments.rs (PROJECT_EPR_ACTION + validator)
 *   - genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
 *
 * Sister to seed-operator-bindings.ts — same POST endpoint, different
 * action discriminator. Same idempotency story: id is content-addressed
 * over (steward_peer_id, action, scope), re-runs collapse to 409.
 *
 * Default MVP projection set:
 *   - elohim-host-landing @ doorway:alpha-elohim-host  urlPath: "/"
 *   - elohim-host-landing @ doorway:elohim-host        urlPath: "/"
 *   - lamad-spa            @ doorway:alpha-elohim-host  urlPath: "/lamad"
 *   - lamad-spa            @ doorway:elohim-host        urlPath: "/lamad"
 *   - imagodei-portal      @ doorway:alpha-elohim-host  urlPath: "/auth/portal"
 *   - imagodei-portal      @ doorway:elohim-host        urlPath: "/auth/portal"
 *
 * Usage:
 *   DOORWAY_URL=http://localhost:8888 npx tsx src/seed-projections.ts
 *   DOORWAY_URL=https://alpha.elohim.host DOORWAY_API_KEY=xxx \
 *     PROJECTIONS_JSON=./projections.json npx tsx src/seed-projections.ts
 */

import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { DoorwayClient } from './doorway-client.js';
import { deterministicPeerId, type Archetype } from './peer-id.js';

// =============================================================================
// Types
// =============================================================================

export type ProjectionMode = 'cached' | 'stewardDirect';

export interface GateHintRef {
  eprRef: string;
  label: string | null;
  relation:
    | 'personWhoCanGrant'
    | 'membershipPrerequisite'
    | 'contentToSync'
    | 'placeToVisit'
    | 'capabilityToEarn'
    | 'paymentToOffer'
    | 'witnessToInvolve';
}

export interface StewardDirectEndpoint {
  peerId: string;
  altHost: string | null;
  tlsCertSan: string;
  acceptsProjectionFor: string[];
}

export interface ProjectionSpec {
  stewardHumanId: string;
  stewardArchetype: Archetype;
  doorwayId: string;
  eprId: string;
  urlPath: string;
  mode: ProjectionMode;
  reach: string;
  baseHref: string;
  entryFile: string;
  redirectsFrom: string[];
  previewEprRef: string | null;
  gateHints: GateHintRef[];
  deadEnd: boolean;
  stewardDirectEndpoint: StewardDirectEndpoint | null;
}

interface CommitmentBody {
  id: string;
  action: 'project-epr';
  provider: string;
  receiver: string;
  /**
   * Pipe-separated scope string: `"doorway:{id}|epr:{id}"`.
   *
   * The Rust `CreateReaCommitmentInput` accepts `Option<String>` for
   * `in_scope_of` (storage layer is single-string, see
   * elohim/elohim-storage/src/db/rea_commitments.rs:53). The A7 resolver
   * (`parse_projection_scope` in the same file) parses this exact format
   * to reconstruct (doorwayId, eprId) when projecting commitments back
   * out as `EprProjectionView`. Round-trip safe.
   *
   * NOTE: seed-operator-bindings.ts currently sends `inScopeOf: [scope]`
   * (an array) which is a pre-existing wire-shape mismatch with the Rust
   * struct — it deserializes via `CreateReaCommitmentInputView`'s `From`
   * impl (views_convert/inputs.rs:280) only on paths that use the View,
   * not the direct DB-layer struct. The /api/v1/commitments handler uses
   * the direct struct, so the single-string form here is the correct one
   * for that path.
   */
  inScopeOf: string;
  note: string;
  metadataJson: string;
  metadata: Record<string, unknown>;
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

/**
 * Build a CreateReaCommitmentInputView body for one EPR projection.
 *
 * `id` is content-addressed over (steward_peer_id, action, scope) so re-runs
 * are idempotent — POST returns 409 on the second invocation. Distinct
 * (doorway, epr) pairs produce distinct ids.
 *
 * `inScopeOf` contains both the doorway ref and the EPR ref so the Rust
 * validator can locate the EPR record and enforce projection constraints.
 *
 * `metadataJson` is a pre-serialized string that carries projection config
 * (urlPath, mode, reach, gateHints, etc.) through the commitment row.
 * `metadata` exposes the same payload as a parsed object for callers that
 * prefer structured access without re-parsing.
 */
export function buildProjectionCommitmentBody(spec: ProjectionSpec): CommitmentBody {
  const stewardPeerId = deterministicPeerId(spec.stewardHumanId, spec.stewardArchetype);
  const scope = `doorway:${spec.doorwayId}|epr:${spec.eprId}`;

  const idDigest = createHash('sha256')
    .update(`${stewardPeerId}|project-epr|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);

  const metadataObject = {
    urlPath: spec.urlPath,
    mode: spec.mode,
    reach: spec.reach,
    baseHref: spec.baseHref,
    entryFile: spec.entryFile,
    redirectsFrom: spec.redirectsFrom,
    previewEprRef: spec.previewEprRef,
    gateHints: spec.gateHints,
    deadEnd: spec.deadEnd,
    stewardDirectEndpoint: spec.stewardDirectEndpoint,
  };

  return {
    id: `project-epr-${idDigest}`,
    action: 'project-epr',
    provider: stewardPeerId,
    receiver: stewardPeerId,
    inScopeOf: `doorway:${spec.doorwayId}|epr:${spec.eprId}`,
    note: `Project ${spec.eprId} at ${spec.urlPath} on ${spec.doorwayId}`,
    metadataJson: JSON.stringify(metadataObject),
    metadata: metadataObject,
  };
}

// =============================================================================
// Default MVP projection set
//
// Matthew is the operator of both alpha-elohim-host and elohim-host.
// Three EPRs are projected on each doorway: the landing page, the lamad SPA,
// and the imagodei-portal auth surface (spec §6.1).
// =============================================================================

export function defaultProjectionSeeds(): ProjectionSpec[] {
  const base = {
    stewardHumanId: 'human-matthew-manager',
    stewardArchetype: 'desktop' as Archetype,
    mode: 'cached' as ProjectionMode,
    reach: 'commons',
    entryFile: 'index.html',
    redirectsFrom: [] as string[],
    previewEprRef: null,
    gateHints: [] as GateHintRef[],
    deadEnd: false,
    stewardDirectEndpoint: null,
  };

  const landingAt = (doorwayId: string): ProjectionSpec => ({
    ...base,
    doorwayId,
    eprId: 'elohim-host-landing',
    urlPath: '/',
    baseHref: '/',
  });

  const lamadAt = (doorwayId: string): ProjectionSpec => ({
    ...base,
    doorwayId,
    eprId: 'lamad-spa',
    urlPath: '/lamad',
    baseHref: '/lamad/',
  });

  const imagodeiPortalAt = (doorwayId: string): ProjectionSpec => ({
    ...base,
    doorwayId,
    eprId: 'imagodei-portal',
    urlPath: '/auth/portal',
    baseHref: '/auth/portal/',
  });

  return [
    landingAt('alpha-elohim-host'),
    landingAt('elohim-host'),
    lamadAt('alpha-elohim-host'),
    lamadAt('elohim-host'),
    imagodeiPortalAt('alpha-elohim-host'),
    imagodeiPortalAt('elohim-host'),
  ];
}

// =============================================================================
// Client
// =============================================================================

class ProjectionClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }
}

/**
 * Factory — lets callers in seed.ts (or integration tests) construct a
 * ProjectionClient without importing the private class directly.
 */
export function createProjectionClient(baseUrl: string, apiKey?: string): ProjectionClient {
  return new ProjectionClient({ baseUrl, apiKey });
}

// =============================================================================
// Seeding (fail-fast on non-409 errors)
// =============================================================================

export async function seedProjections(
  client: ProjectionClient,
  specs: ProjectionSpec[],
): Promise<void> {
  console.log(`[seed-projections] Seeding ${specs.length} project-epr commitments...`);

  let created = 0;
  let alreadyExists = 0;

  for (const spec of specs) {
    const body = buildProjectionCommitmentBody(spec);
    const label = `${spec.eprId} @ ${spec.urlPath} on ${spec.doorwayId}`;

    const response = await client.createCommitment(body);

    if (response.ok) {
      console.log(`  [+] ${label}  mode=${spec.mode} reach=${spec.reach}`);
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
    `[seed-projections] Done. created=${created} already-exists=${alreadyExists} total=${specs.length}`,
  );
}

// =============================================================================
// Standalone execution
// =============================================================================

const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  const doorwayUrl = process.env.DOORWAY_URL || 'http://localhost:8888';
  const apiKey = process.env.DOORWAY_API_KEY;

  const projectionsJsonPath = process.env.PROJECTIONS_JSON;
  const specs: ProjectionSpec[] = projectionsJsonPath
    ? (JSON.parse(readFileSync(projectionsJsonPath, 'utf-8')) as ProjectionSpec[])
    : defaultProjectionSeeds();

  const client = new ProjectionClient({ baseUrl: doorwayUrl, apiKey });

  console.log('='.repeat(60));
  console.log('EPR-Projection Seeder');
  console.log(`  Target:      ${doorwayUrl}`);
  console.log(`  Projections: ${specs.length}`);
  console.log('='.repeat(60));
  console.log();

  const health = await client.checkHealth();
  if (!health.healthy) {
    console.error(`ERROR: Doorway not healthy — ${health.error}`);
    process.exit(1);
  }

  await seedProjections(client, specs);
  process.exit(0);
}
