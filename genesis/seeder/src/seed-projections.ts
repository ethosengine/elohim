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

export interface RouteClaimTemplate {
  contentType: string;
  template: string;
  fragments?: Record<string, string>;
}

export interface RouteClaimGrant {
  schemaVersion: number;
  claimsManifestCid: string | null;
  claims: RouteClaimTemplate[];
}

export interface RedirectTemplate {
  from: string;
  to: string;
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
  routeClaims: RouteClaimGrant | null;
  redirectTemplates: RedirectTemplate[];
}

interface CommitmentBody {
  id: string;
  action: 'project-epr';
  provider: string;
  receiver: string;
  /**
   * Predecessor commitment id this body supersedes (spec §3.2/§3.3 re-grant
   * path). Only set on a superseding (drift) re-seed; omitted on a first seed.
   * The storage layer reads this off the direct `CreateReaCommitmentInput`
   * (camelCase `supersedes`) and, for project-epr, runs the supersession
   * ceremony: mark the predecessor `superseded`, insert this successor. Also
   * mirrored into `metadata.supersedes` so the chain is walkable via
   * `GET /api/v1/commitments/{id}`.
   */
  supersedes?: string;
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
// Projection-relevant metadata — the deep-compare field set
// =============================================================================

/**
 * The projection-relevant metadata fields, normalized for drift comparison.
 *
 * These are EXACTLY the fields the doorway EprRouter reads off an
 * `EprProjectionView` to dispatch — a change to any one of them is operative
 * routing-law drift that warrants a re-grant. Fields NOT here (commitmentId,
 * eprId, doorwayId, seededAt, seededBy) are identity/provenance, not routing law.
 *
 * Normalization (critical for null-vs-missing equivalence): the storage
 * projection (`commitment_to_projection_view`) substitutes defaults for absent
 * keys — `spaFallback` defaults `true`, `redirectsFrom`/`redirectTemplates`/
 * `gateHints` default `[]`, `routeClaims`/`previewEprRef`/`stewardDirectEndpoint`
 * default `null`. So a desired body that OMITS a key and an existing row that
 * MATERIALIZED its default must compare EQUAL. We materialize the same defaults
 * here on both sides before stringifying, so `undefined` (missing) and the
 * default value are indistinguishable — no false-positive drift, no needless
 * supersede churn.
 */
export interface ProjectionRelevantMetadata {
  urlPath: string;
  mode: ProjectionMode;
  reach: string;
  baseHref: string;
  entryFile: string;
  spaFallback: boolean;
  redirectsFrom: string[];
  redirectTemplates: RedirectTemplate[];
  routeClaims: RouteClaimGrant | null;
  previewEprRef: string | null;
  gateHints: GateHintRef[];
  deadEnd: boolean;
  stewardDirectEndpoint: StewardDirectEndpoint | null;
}

/** The ordered field list documented in the Jenkinsfile seed-stage comment. */
export const PROJECTION_RELEVANT_FIELDS = [
  'urlPath',
  'mode',
  'reach',
  'baseHref',
  'entryFile',
  'spaFallback',
  'redirectsFrom',
  'redirectTemplates',
  'routeClaims',
  'previewEprRef',
  'gateHints',
  'deadEnd',
  'stewardDirectEndpoint',
] as const;

/**
 * Normalize a (possibly partial) projection metadata bag into the canonical
 * comparison shape, materializing the SAME defaults the storage projection does
 * (see `commitment_to_projection_view` in rea_commitments.rs). `null` and
 * `undefined` (missing key) collapse to the same default — so a seed that omits
 * `spaFallback` and a row that materialized `true` compare equal.
 */
export function projectionRelevantMetadata(
  m: Partial<ProjectionRelevantMetadata>,
): ProjectionRelevantMetadata {
  return {
    urlPath: m.urlPath ?? '/',
    mode: m.mode ?? 'cached',
    reach: m.reach ?? 'commons',
    baseHref: m.baseHref ?? '/',
    entryFile: m.entryFile ?? 'index.html',
    // Storage defaults spaFallback=true when absent; treat null as the default too.
    spaFallback: m.spaFallback ?? true,
    redirectsFrom: m.redirectsFrom ?? [],
    redirectTemplates: m.redirectTemplates ?? [],
    routeClaims: m.routeClaims ?? null,
    previewEprRef: m.previewEprRef ?? null,
    gateHints: m.gateHints ?? [],
    deadEnd: m.deadEnd ?? false,
    stewardDirectEndpoint: m.stewardDirectEndpoint ?? null,
  };
}

/**
 * Stable JSON for a normalized metadata bag (object keys emitted in the fixed
 * `PROJECTION_RELEVANT_FIELDS` order) so two equal bags stringify identically
 * regardless of source key order.
 */
function stableMetadataJson(m: ProjectionRelevantMetadata): string {
  const ordered: Record<string, unknown> = {};
  for (const k of PROJECTION_RELEVANT_FIELDS) {
    ordered[k] = (m as unknown as Record<string, unknown>)[k];
  }
  return JSON.stringify(ordered);
}

/**
 * Deep-compare the desired projection metadata against an existing row's
 * metadata. Returns the list of drifted field names (empty ⇒ identical, a
 * re-seed is a quiet idempotent 409). Both sides are normalized first so
 * null-vs-missing never registers as drift.
 */
export function metadataDrift(
  desired: Partial<ProjectionRelevantMetadata>,
  existing: Partial<ProjectionRelevantMetadata>,
): string[] {
  const d = projectionRelevantMetadata(desired);
  const e = projectionRelevantMetadata(existing);
  const drifted: string[] = [];
  for (const k of PROJECTION_RELEVANT_FIELDS) {
    const dv = JSON.stringify((d as unknown as Record<string, unknown>)[k]);
    const ev = JSON.stringify((e as unknown as Record<string, unknown>)[k]);
    if (dv !== ev) drifted.push(k);
  }
  return drifted;
}

/**
 * 8-hex re-grant fingerprint over the DESIRED projection-relevant metadata.
 *
 * Deterministic over the metadata that drifted: re-running the same re-grant
 * (same desired metadata) always derives the same successor id, so a second
 * re-seed of an already-applied re-grant is itself idempotent (it 409s against
 * the successor, not the superseded predecessor). Distinct drifts → distinct
 * suffixes.
 */
export function regrantFingerprint(desired: Partial<ProjectionRelevantMetadata>): string {
  return createHash('sha256')
    .update(stableMetadataJson(projectionRelevantMetadata(desired)), 'utf8')
    .digest('hex')
    .slice(0, 8);
}

// =============================================================================
// Body builder (testable in isolation)
// =============================================================================

/**
 * Base content-addressed projection id over (steward_peer_id, action, scope).
 * Distinct (doorway, epr) pairs produce distinct ids; re-runs are idempotent.
 *
 * ID DERIVATION FORMULA (documented here AND in genesis/Jenkinsfile seed stage):
 *   base       = `project-epr-${sha256(stewardPeerId|project-epr|scope)[:16]}`
 *   superseder = `${base}-r${sha256(stableJson(projectionRelevantMetadata))[:8]}`
 * where scope = `doorway:{doorwayId}|epr:{eprId}`.
 */
export function baseProjectionId(spec: ProjectionSpec): string {
  const stewardPeerId = deterministicPeerId(spec.stewardHumanId, spec.stewardArchetype);
  const scope = `doorway:${spec.doorwayId}|epr:${spec.eprId}`;
  const idDigest = createHash('sha256')
    .update(`${stewardPeerId}|project-epr|${scope}`, 'utf8')
    .digest('hex')
    .slice(0, 16);
  return `project-epr-${idDigest}`;
}

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
 *
 * When `supersedePredecessorId` is set (the re-grant path), the id is the
 * fingerprint-suffixed superseder id, `supersedes` carries the predecessor id
 * for the storage supersession ceremony, and `metadata.supersedes` mirrors it
 * for chain walkability.
 */
export function buildProjectionCommitmentBody(
  spec: ProjectionSpec,
  supersedePredecessorId?: string,
): CommitmentBody {
  const stewardPeerId = deterministicPeerId(spec.stewardHumanId, spec.stewardArchetype);

  const baseMetadata = {
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
    routeClaims: spec.routeClaims,
    redirectTemplates: spec.redirectTemplates,
  };

  const base = baseProjectionId(spec);
  const id = supersedePredecessorId
    ? `${base}-r${regrantFingerprint(specToMetadata(spec))}`
    : base;

  // On a supersede, mirror the predecessor pointer into the metadata so the
  // chain is walkable via GET /api/v1/commitments/{id}.
  const metadataObject = supersedePredecessorId
    ? { ...baseMetadata, supersedes: supersedePredecessorId }
    : baseMetadata;

  return {
    id,
    action: 'project-epr',
    provider: stewardPeerId,
    receiver: stewardPeerId,
    ...(supersedePredecessorId ? { supersedes: supersedePredecessorId } : {}),
    inScopeOf: `doorway:${spec.doorwayId}|epr:${spec.eprId}`,
    note: `Project ${spec.eprId} at ${spec.urlPath} on ${spec.doorwayId}`,
    metadataJson: JSON.stringify(metadataObject),
    metadata: metadataObject,
  };
}

/** Project a ProjectionSpec down to its projection-relevant metadata bag. */
export function specToMetadata(spec: ProjectionSpec): ProjectionRelevantMetadata {
  return projectionRelevantMetadata({
    urlPath: spec.urlPath,
    mode: spec.mode,
    reach: spec.reach,
    baseHref: spec.baseHref,
    entryFile: spec.entryFile,
    redirectsFrom: spec.redirectsFrom,
    redirectTemplates: spec.redirectTemplates,
    routeClaims: spec.routeClaims,
    previewEprRef: spec.previewEprRef,
    gateHints: spec.gateHints,
    deadEnd: spec.deadEnd,
    stewardDirectEndpoint: spec.stewardDirectEndpoint,
  });
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
    routeClaims: null as RouteClaimGrant | null,
    redirectTemplates: [] as RedirectTemplate[],
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
    routeClaims: {
      schemaVersion: 1,
      claimsManifestCid: null,
      claims: [
        { contentType: 'path', template: 'path/{id}', fragments: { step: 'path/{id}/step/{n}' } },
      ],
    },
    redirectTemplates: [{ from: '/lamad/resource/{id}', to: '/epr/{id}' }],
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
    landingAt('apex-elohim-host'),
    lamadAt('alpha-elohim-host'),
    lamadAt('apex-elohim-host'),
    imagodeiPortalAt('alpha-elohim-host'),
    imagodeiPortalAt('apex-elohim-host'),
  ];
}

// =============================================================================
// Client
// =============================================================================

/**
 * The projection-relevant subset of an EprProjectionView the seeder reads back
 * to detect drift. The storage endpoint returns the full view; we only consume
 * the fields in PROJECTION_RELEVANT_FIELDS plus the identity fields used to
 * locate the row.
 */
export interface EprProjectionViewLite extends Partial<ProjectionRelevantMetadata> {
  commitmentId: string;
  eprId: string;
  doorwayId: string;
}

class ProjectionClient extends DoorwayClient {
  async createCommitment(body: CommitmentBody): Promise<Response> {
    return this.fetch('/api/v1/commitments', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
  }

  /**
   * Fetch the ACTIVE project-epr projections for a doorway (the EprRouter
   * source). Superseded predecessors are already excluded by the storage layer
   * (`find_active_projections`), so the returned rows are the current grants.
   *
   * NOTE: `doorwayId` is the BARE id (no `doorway:` prefix) — the query param
   * the storage handler expects.
   */
  async fetchProjections(bareDoorwayId: string): Promise<EprProjectionViewLite[]> {
    const response = await this.fetch(
      `/db/rea_commitments?action=project-epr&doorwayId=${encodeURIComponent(bareDoorwayId)}`,
      { method: 'GET' },
    );
    if (!response.ok) {
      throw new Error(
        `fetchProjections(${bareDoorwayId}) failed: HTTP ${response.status}: ${await response.text()}`,
      );
    }
    // Bare JSON array (see handle_db_rea_commitments_inner — do NOT expect a wrapper).
    return (await response.json()) as EprProjectionViewLite[];
  }
}

/**
 * Locate the active projection row for a spec among the doorway's fetched rows.
 *
 * Matches on (eprId, doorwayId). The storage view's `doorwayId` is the LONG
 * form (`doorway:<id>`); the spec carries the BARE id — so we compare the bare
 * tail. Returns `undefined` when no active row matches (shouldn't happen on a
 * 409, but the caller treats it as "cannot supersede" and fails fast).
 */
export function findActiveRowForSpec(
  rows: EprProjectionViewLite[],
  spec: ProjectionSpec,
): EprProjectionViewLite | undefined {
  return rows.find(
    (r) =>
      r.eprId === spec.eprId &&
      (r.doorwayId === spec.doorwayId || r.doorwayId === `doorway:${spec.doorwayId}`),
  );
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
  let updated = 0;

  // Cache fetched active projections per doorway so a multi-spec run hits the
  // storage GET at most once per doorway.
  const projectionCache = new Map<string, EprProjectionViewLite[]>();
  const projectionsFor = async (bareDoorwayId: string): Promise<EprProjectionViewLite[]> => {
    if (!projectionCache.has(bareDoorwayId)) {
      projectionCache.set(bareDoorwayId, await client.fetchProjections(bareDoorwayId));
    }
    return projectionCache.get(bareDoorwayId)!;
  };

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
    const isConflict =
      response.status === 409 || text.includes('UNIQUE') || text.includes('already exists');

    if (isConflict) {
      // The content-addressed id already exists. Determine whether the existing
      // ACTIVE row's projection-relevant metadata matches the desired spec:
      //   identical → quiet idempotent re-run (current behavior, no write);
      //   drift     → re-grant via supersession (spec §3.2/§3.3).
      let rows: EprProjectionViewLite[];
      try {
        rows = await projectionsFor(spec.doorwayId);
      } catch (err) {
        console.error(`  [X] ${label}: could not read existing projections for drift check`);
        console.error(`      ${err instanceof Error ? err.message : String(err)}`);
        process.exit(1);
      }

      const existing = findActiveRowForSpec(rows, spec);
      if (!existing) {
        // 409 with no matching active row: the id collides but no current grant
        // exists to supersede (e.g. the only row is already superseded by a
        // DIFFERENT successor). This is an unexpected state — fail fast rather
        // than silently double-supersede.
        console.error(`  [X] ${label}: 409 but no active projection row to compare/supersede`);
        console.error(`      Existing rows: ${JSON.stringify(rows.map((r) => r.commitmentId))}`);
        process.exit(1);
      }

      const drifted = metadataDrift(specToMetadata(spec), existing);
      if (drifted.length === 0) {
        console.log(`  [=] ${label} (idempotent re-run)`);
        alreadyExists += 1;
        continue;
      }

      // Drift → re-grant. Supersede the CURRENT active row (existing.commitmentId).
      const supersedeBody = buildProjectionCommitmentBody(spec, existing.commitmentId);
      const regrantResponse = await client.createCommitment(supersedeBody);
      if (regrantResponse.ok) {
        console.log(
          `  [~] ${label} (re-grant: superseded ${existing.commitmentId} -> ${supersedeBody.id}; ` +
            `drifted=[${drifted.join(', ')}])`,
        );
        updated += 1;
        // Invalidate the cache for this doorway: the active set changed.
        projectionCache.delete(spec.doorwayId);
        continue;
      }

      // A 409 on the supersede itself means the successor id already exists —
      // this exact re-grant was already applied (the fingerprint is stable), so
      // it is idempotent. Any other failure is a real error.
      const regrantText = await regrantResponse.text();
      if (
        regrantResponse.status === 409 ||
        regrantText.includes('UNIQUE') ||
        regrantText.includes('already superseded') ||
        regrantText.includes('already exists')
      ) {
        console.log(`  [=] ${label} (re-grant already applied — idempotent)`);
        alreadyExists += 1;
        continue;
      }

      console.error(`  [X] ${label}: re-grant POST failed HTTP ${regrantResponse.status}`);
      console.error(`      Body: ${regrantText.slice(0, 500)}`);
      console.error(`      Sent: ${JSON.stringify(supersedeBody, null, 2)}`);
      process.exit(1);
    }

    // ANY other failure is a shape mismatch or doorway issue — fail fast.
    console.error(`  [X] ${label}: HTTP ${response.status}`);
    console.error(`      Body: ${text.slice(0, 500)}`);
    console.error(`      Sent: ${JSON.stringify(body, null, 2)}`);
    process.exit(1);
  }

  console.log(
    `[seed-projections] Done. created=${created} updated=${updated} ` +
      `already-exists=${alreadyExists} total=${specs.length}`,
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
