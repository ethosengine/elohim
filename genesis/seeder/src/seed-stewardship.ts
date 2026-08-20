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

// Storage InputView is #[serde(rename_all = "camelCase")] (elohim-views/src/shefa.rs
// StewardshipAllocationInputView) — snake_case never crosses the storage boundary, so
// the wire payload MUST be camelCase or storage rejects with "missing field contentId".
interface CreateAllocationInput {
  contentId: string;
  stewardPresenceId: string;
  allocationRatio: number;
  allocationMethod: string;
  contributionType: string;
  note?: string;
}

interface BulkAllocationResult {
  created: number;
  failed: number;
  errors: string[];
}

/**
 * The subset of a stored allocation row this seeder reconciles against.
 *
 * Read back from `GET /db/allocations` so the run can tell a MISSING
 * (content, steward) pair from a pair that exists with a STALE ratio. The
 * governance columns are carried so the reconciler can refuse to touch a row
 * the governance plane owns (disputed / ratified / non-active) — a seeder must
 * never stomp a negotiated allocation.
 */
export interface StoredAllocation {
  id: string;
  contentId: string;
  stewardPresenceId: string;
  allocationRatio: number;
  allocationMethod: string;
  contributionType: string;
  governanceState?: string | null;
  disputeId?: string | null;
  elohimRatifiedAt?: string | null;
  note?: string | null;
}

/** A ratio/method/type repair for one already-existing allocation row. */
export interface AllocationRepair {
  allocationId: string;
  contentId: string;
  stewardPresenceId: string;
  fromRatio: number;
  toRatio: number;
  allocationMethod: string;
  contributionType: ContributionType;
  note: string;
}

/** What the reconciler decided for ONE content item. */
export interface ContentAllocationPlan {
  creates: CreateAllocationInput[];
  repairs: AllocationRepair[];
  /** Rows whose ratio drifted but which the governance plane owns — reported, never written. */
  governedSkips: StoredAllocation[];
}

/**
 * Ratio drift below this is float noise, not a real difference.
 *
 * Storage stores `allocation_ratio` as an f32, so a 2/3 written by this seeder
 * reads back as 0.6666666865348816. Anything at or under this epsilon is the
 * same number; anything above is a genuinely different allocation.
 */
export const RATIO_EPSILON = 1e-4;

interface StewardRatio {
  presenceId: string;
  ratio: number;
  /**
   * Contribution type for the storage wire enum. Only the AUTHORED path sets
   * this (derived from the content JSON's declared `role`); the category and
   * default paths leave it undefined and the caller keeps the historical
   * `curator` value the a2o scenarios assert on.
   */
  contributionType?: ContributionType;
}

/**
 * Storage validates `contribution_type` against a fixed enum (db/models.rs):
 * original_creator | editor | translator | curator | maintainer | inherited.
 */
type ContributionType =
  | 'original_creator'
  | 'editor'
  | 'translator'
  | 'curator'
  | 'maintainer'
  | 'inherited';

/** A `stewardedBy` entry as authored on a content JSON in genesis/data/lamad/content. */
export interface AuthoredStewardEntry {
  humanId: string;
  affinity?: number;
  role?: string;
}

/** Which source decided a content item's steward set. Recorded in the allocation note. */
export type StewardshipProvenance = 'authored' | 'category' | 'default';

export interface ResolvedStewardship {
  stewards: StewardRatio[];
  provenance: StewardshipProvenance;
}

/**
 * Composite key for a single (content, steward) allocation. Idempotency is
 * (content,steward)-granular — see StewardshipClient.getContentWithAllocations.
 * MUST match the key shape produced there (`${contentId}::${stewardPresenceId}`).
 */
function allocationKey(contentId: string, stewardPresenceId: string): string {
  return `${contentId}::${stewardPresenceId}`;
}

/** Shared empty set so the per-content miss path allocates nothing. */
const EMPTY_STEWARD_SET: ReadonlySet<string> = new Set<string>();

// =============================================================================
// Category-to-Steward Affinity Mapping
//
// Each category maps to an array of stewards with proportional ratios.
// Ratios represent relational affinity, not ownership. They sum to 1.0.
// =============================================================================

const CATEGORY_STEWARD_MAP: Record<string, StewardRatio[]> = {
  // Care economy: Adam (gardener-steward) primary, Jessica (family), Matthew (founder), Frank (ecology)
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

  // Digital relationships: Eve (courage, family-systems), Jessica (community-building), Matthew
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
// Category-to-Steward Affinity Scores
//
// Affinity represents earned stewardship standing through curation.
// Higher affinity = deeper relationship with the content domain.
// These are initial seeds — real activity will update them over time.
// =============================================================================

interface StewardAffinityEntry {
  stewardId: string;
  affinityScore: number;
}

const CATEGORY_AFFINITY_MAP: Record<string, StewardAffinityEntry[]> = {
  'public-observer': [
    { stewardId: 'eve-firstwoman', affinityScore: 0.85 },
    { stewardId: 'nancy-neighbor', affinityScore: 0.60 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'scripture': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-media': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-practice': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-narrative': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'fct-activity': [
    { stewardId: 'pete-pastor', affinityScore: 0.50 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'value-scanner': [
    { stewardId: 'adam-firstman', affinityScore: 0.75 },
    { stewardId: 'jessica-spouse', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'frank-farmer', affinityScore: 0.45 },
  ],
  'governance': [
    { stewardId: 'nancy-neighbor', affinityScore: 0.70 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'eve-firstwoman', affinityScore: 0.55 },
  ],
  'social-medium': [
    { stewardId: 'eve-firstwoman', affinityScore: 0.80 },
    { stewardId: 'jessica-spouse', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'autonomous-entity': [
    { stewardId: 'meriadoc-moneybags', affinityScore: 0.65 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'frank-farmer', affinityScore: 0.45 },
  ],
  'economic-coordination': [
    { stewardId: 'meriadoc-moneybags', affinityScore: 0.65 },
    { stewardId: 'frank-farmer', affinityScore: 0.60 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'community': [
    { stewardId: 'nancy-neighbor', affinityScore: 0.70 },
    { stewardId: 'adam-firstman', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'local-economy': [
    { stewardId: 'frank-farmer', affinityScore: 0.70 },
    { stewardId: 'meriadoc-moneybags', affinityScore: 0.55 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'foundation': [
    { stewardId: 'dan-developer', affinityScore: 0.75 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'contributor': [
    { stewardId: 'dan-developer', affinityScore: 0.75 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'general': [
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
    { stewardId: 'dan-developer', affinityScore: 0.50 },
  ],
  'landing-page-concept': [
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
  'algorithmic-bias': [
    { stewardId: 'eve-firstwoman', affinityScore: 0.75 },
    { stewardId: 'matthew-dowell', affinityScore: 0.70 },
  ],
};

// =============================================================================
// Doorway Client Extensions
// =============================================================================

class StewardshipClient extends DoorwayClient {
  async getAllContentIds(): Promise<string[]> {
    const response = await this.fetch('/db/content?limit=10000', {
      method: 'GET',
    });

    if (!response.ok) {
      throw new Error(`Failed to get content: ${response.status}`);
    }

    // The doorway's /db/content list returns an envelope {items: [...]} (the
    // legacy storage-direct path returned a bare array — genesis #1081 failed
    // here with "content.map is not a function"). Accept both shapes.
    const body = (await response.json()) as unknown;
    const content = (
      Array.isArray(body) ? body : ((body as { items?: unknown[] }).items ?? [])
    ) as Array<{ id: string }>;
    return content.map((c) => c.id);
  }

  /**
   * Returns the EXISTING (content, steward) allocations, keyed by the
   * `${contentId}::${stewardPresenceId}` composite. Per-steward granularity
   * (not content-granular) is required because a content item can be
   * partially allocated — some of its category-expected stewards present,
   * others missing. The storage bulk handler counts an already-existing
   * (content,steward) pair as `failed` (uniqueness violation surfaces as Err;
   * http.rs handle_bulk_create_allocations), so re-POSTing existing pairs both
   * pollutes the failure count and wastes round-trips. The caller diffs
   * against this map to POST only the genuinely-missing pairs.
   *
   * The VALUE (not just the key) is carried because membership is not the only
   * thing that can drift: a pair that exists can hold a stale RATIO. See
   * `planContentAllocations` — the map is what lets the run tell "missing" from
   * "present but wrong".
   */
  async getContentWithAllocations(): Promise<Map<string, StoredAllocation>> {
    // PAGINATED — a single `limit=10000` read SILENTLY TRUNCATES once the
    // (persistent-PVC) allocations table exceeds the page size. A truncated
    // existing-set makes the caller's idempotency diff INCOMPLETE: genuinely-
    // present pairs beyond the first page read as "missing" → re-POSTed → the
    // storage bulk handler returns UNIQUE-constraint `failed`, and (worse) the
    // run's net-new creates collapse to ~0 while affinity stewards for already-
    // partially-seeded content are never repaired. That is exactly the failure
    // observed on elohim-genesis #1100/#1102/#1104 (`Found 10000 existing`,
    // `Created: 3, Failed: 1725`, value-scanner/fct/public-observer items left
    // matthew-only). Page through `limit`/`offset` (DB layer wires both —
    // db/stewardship_allocations.rs AllocationQuery) until a short/empty page.
    const PAGE_SIZE = 10000;
    const existing = new Map<string, StoredAllocation>();
    let offset = 0;

    for (;;) {
      const response = await this.fetch(
        `/db/allocations?activeOnly=true&limit=${PAGE_SIZE}&offset=${offset}`,
        { method: 'GET' }
      );

      if (!response.ok) {
        if (response.status === 404) {
          console.log('   Allocations endpoint not available, assuming no existing allocations');
          return new Map();
        }
        throw new Error(`Failed to get allocations: ${response.status}`);
      }

      const allocBody = (await response.json()) as unknown;
      const page = (
        Array.isArray(allocBody) ? allocBody : ((allocBody as { items?: unknown[] }).items ?? [])
      ) as StoredAllocation[];

      for (const a of page) {
        existing.set(`${a.contentId}::${a.stewardPresenceId}`, a);
      }

      // A short page (fewer rows than requested) is the last page. An exactly-
      // full page means there may be more — advance the offset and continue.
      if (page.length < PAGE_SIZE) {
        break;
      }
      offset += PAGE_SIZE;
    }

    return existing;
  }

  /**
   * Repair ONE existing allocation row's ratio (plus the method /
   * contributionType / note that must travel with it).
   *
   * ## Why `/api/v1/stewardship/allocations/{id}` and not `/db/allocations/{id}`
   *
   * Storage implements PUT on BOTH paths (they share
   * `handle_allocation_by_id` → `stewardship_allocations::update_allocation`),
   * but the doorway's route registry only declares GET and DELETE for
   * `/db/allocations/{id}`. A PUT there is not a route the doorway knows, so it
   * falls through to the 404 catch-all:
   *
   *   PUT http://localhost:8888/db/allocations/{id}
   *     -> 404 {"error":"Not Found","hint":"Use WebSocket connection to /admin or /app/:port"}
   *   PUT http://localhost:8888/api/v1/stewardship/allocations/{id}
   *     -> 200 (row updated)
   *
   * This seeder always runs THROUGH a doorway (genesis/scripts/ci/seed-stewardship.sh
   * passes RESOLVED_DOORWAY_HOST), so the `/api/v1/stewardship` path is the only
   * one that reaches the handler. Do not "simplify" it back to `/db`.
   */
  async repairAllocation(repair: AllocationRepair): Promise<void> {
    const response = await this.fetch(`/api/v1/stewardship/allocations/${repair.allocationId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify({
        schemaVersion: 1,
        allocationRatio: repair.toRatio,
        allocationMethod: repair.allocationMethod,
        contributionType: repair.contributionType,
        note: repair.note,
      }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(
        `Failed to repair allocation ${repair.allocationId} ` +
          `(${repair.contentId} / ${repair.stewardPresenceId}): ${response.status} ${error}`
      );
    }
  }

  async presenceExists(presenceId: string): Promise<boolean> {
    const response = await this.fetch(`/db/presences/${presenceId}`, {
      method: 'GET',
    });
    return response.ok;
  }

  async createPresence(data: PresenceData): Promise<void> {
    // /db/presences (CreateContributorPresenceInputView) is camelCase + PARSED
    // objects — NOT snake_case `_json` strings. Mirror seed-presences.ts (the
    // canonical presence seeder) or storage rejects with "missing field displayName".
    const body = {
      id: data.id,
      schemaVersion: 1,
      displayName: data.displayName,
      presenceState: data.presenceState,
      externalIdentifiers: data.externalIdentifiers ?? [],
      establishingContentIds: data.establishingContentIds ?? [],
      claimedAgentId: data.claimedAgentId ?? null,
      note: data.note ?? null,
      metadata: data.metadata ?? null,
    };

    const response = await this.fetch('/db/presences', {
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
    const response = await this.fetch('/db/allocations/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Schema-Version': '1' },
      body: JSON.stringify(inputs),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to bulk create allocations: ${error}`);
    }

    return response.json();
  }

  async bulkCreateAffinities(
    affinities: Array<{ stewardId: string; contentId: string; affinityScore: number; source: string }>
  ): Promise<{ created: number; errors: string[] }> {
    const response = await this.fetch('/api/v1/steward-affinity/bulk', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ affinities }),
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to bulk create affinities: ${error}`);
    }

    return response.json();
  }
}

// =============================================================================
// Content Category Reader
// =============================================================================

/** What the seeder reads off each content JSON to decide its stewardship. */
export interface ContentStewardshipFacts {
  category?: string;
  stewardedBy?: AuthoredStewardEntry[];
}

/**
 * Build a map of content ID -> {category, stewardedBy} from the seed data
 * files on disk. This avoids needing to parse metadata from the API.
 *
 * `stewardedBy` is read here (and honored in `resolveStewardship`) because the
 * field was previously INERT: it is authored on 3428 content files and no
 * seeder ever read it, so every declaration fell through to the category map
 * or the bootstrap default.
 */
export function buildContentIndex(contentDir: string = CONTENT_DIR): Map<
  string,
  ContentStewardshipFacts
> {
  const index = new Map<string, ContentStewardshipFacts>();

  if (!fs.existsSync(contentDir)) {
    console.log(`   WARNING: Content directory not found: ${contentDir}`);
    return index;
  }

  const files = fs.readdirSync(contentDir).filter((f) => f.endsWith('.json'));

  for (const file of files) {
    try {
      const data = JSON.parse(fs.readFileSync(path.join(contentDir, file), 'utf-8'));
      if (data.id) {
        index.set(data.id, {
          category: data.metadata?.category,
          stewardedBy: Array.isArray(data.stewardedBy) ? data.stewardedBy : undefined,
        });
      }
    } catch {
      // Skip malformed files
    }
  }

  return index;
}

// =============================================================================
// Allocation Logic
// =============================================================================

/**
 * Map a content JSON's declared steward `role` onto the storage
 * `contribution_type` enum. Unknown/absent roles fall to `curator` (a steward
 * curates) — the same value the category and default paths use.
 */
export function contributionTypeForRole(role: string | undefined): ContributionType {
  switch (role) {
    case 'author':
      return 'original_creator';
    case 'steward':
      return 'maintainer';
    case 'editor':
      return 'editor';
    case 'translator':
      return 'translator';
    // 'endorser', 'curator', anything else — an endorsement is a curation act.
    default:
      return 'curator';
  }
}

/**
 * Build the humanId → presenceId index from the contributor presence files.
 *
 * The mapping is DECLARATIVE, never derived from the slug: each presence JSON
 * carries `metadata.humanId` naming the canonical human it belongs to
 * (`human-matthew-manager` → `matthew-dowell`). A content JSON's
 * `stewardedBy[].humanId` is a humans.json id; allocations are keyed by
 * presenceId — this index is the only bridge between the two namespaces.
 */
export function buildPresenceIdByHumanId(presences: PresenceData[]): Map<string, string> {
  const index = new Map<string, string>();
  for (const p of presences) {
    const humanId = p.metadata?.humanId;
    if (typeof humanId === 'string' && humanId.length > 0) {
      index.set(humanId, p.id);
    }
  }
  return index;
}

/**
 * Resolve an authored `stewardedBy` array to steward ratios, or `null` when it
 * cannot be honored.
 *
 * FAIL-CLOSED on unresolvable humanIds. A `stewardedBy` entry naming a human
 * with no contributor presence (e.g. `human-georgina-grocer`, which appears on
 * 1870 content files with no presence file behind it) would otherwise mint
 * allocations pointing at a presence that does not exist — dead rows the
 * stewardship views silently drop. One unresolvable entry disqualifies the
 * WHOLE array: a partial honor would silently redistribute the missing
 * steward's affinity to the others, which is a different (and unauthored)
 * allocation.
 *
 * Ratios are the declared affinities normalized to sum 1.0 (the a2o
 * `@integrity` scenario asserts the per-content sum). Duplicate humanIds are
 * summed before normalization.
 */
export function resolveAuthoredStewards(
  authored: AuthoredStewardEntry[] | undefined,
  presenceIdByHumanId: Map<string, string>
): StewardRatio[] | null {
  if (!authored || authored.length === 0) return null;

  const affinityByPresence = new Map<string, number>();
  const roleByPresence = new Map<string, string | undefined>();

  for (const entry of authored) {
    const presenceId = presenceIdByHumanId.get(entry.humanId);
    if (!presenceId) return null; // fail-closed — see doc comment
    const affinity = typeof entry.affinity === 'number' ? entry.affinity : 0;
    if (!(affinity > 0)) return null; // a zero/negative/absent affinity can't be normalized
    affinityByPresence.set(presenceId, (affinityByPresence.get(presenceId) ?? 0) + affinity);
    // First declared role for a presence wins (the strongest relationship is
    // declared first in the annotated files).
    if (!roleByPresence.has(presenceId)) roleByPresence.set(presenceId, entry.role);
  }

  const total = [...affinityByPresence.values()].reduce((a, b) => a + b, 0);
  if (!(total > 0)) return null;

  return [...affinityByPresence.entries()].map(([presenceId, affinity]) => ({
    presenceId,
    ratio: affinity / total,
    contributionType: contributionTypeForRole(roleByPresence.get(presenceId)),
  }));
}

/**
 * Decide the steward set for one content item, and record WHICH source decided
 * it (the provenance lands in the allocation note).
 *
 * ## Precedence: category → authored → default
 *
 * The curated `CATEGORY_STEWARD_MAP` keeps precedence over an authored
 * `stewardedBy`. That ordering is deliberate and measured, NOT an oversight:
 *
 *   - 3428 of 3431 content files carry a `stewardedBy` array, and essentially
 *     all of them were machine-written by `genesis/scripts/annotate-stewardship.py`
 *     from a tag→human rule table. They are an annotation pass, not an
 *     authorial declaration.
 *   - Letting them win would rewrite the steward set of 3157 content items
 *     (1276 even after discarding the unresolvable ones) and would silently
 *     replace the hand-curated affinity graph the six a2o
 *     `content/stewardship-allocation.feature` scenarios assert on.
 *
 * So authored stewardship displaces only the BOOTSTRAP DEFAULT — the
 * `matthew-dowell @ 1.0` fallback that means "we know nothing about this
 * item". Where the seeder previously knew nothing, the content's own
 * declaration is strictly better information. Measured blast radius of that
 * rule: 50 content items (all currently default-allocated), each gaining a
 * genuine second steward. `elohim-host-landing` itself keeps the same steward
 * set (matthew-dowell @ 1.0) and gains `authored` provenance plus the
 * `original_creator` contribution type its `role: author` declares.
 *
 * Promoting authored above the category map is a separate, deliberate
 * decision about the whole stewardship graph — it needs presence files for
 * georgina-grocer/terrance-tutor first, and an allocation-supersede pass.
 */
export function resolveStewardship(
  category: string | undefined,
  authored: AuthoredStewardEntry[] | undefined,
  presenceIdByHumanId: Map<string, string>
): ResolvedStewardship {
  if (category && CATEGORY_STEWARD_MAP[category]) {
    return { stewards: CATEGORY_STEWARD_MAP[category], provenance: 'category' };
  }

  const authoredStewards = resolveAuthoredStewards(authored, presenceIdByHumanId);
  if (authoredStewards) {
    return { stewards: authoredStewards, provenance: 'authored' };
  }

  // Default: Matthew as bootstrap steward
  return { stewards: [{ presenceId: 'matthew-dowell', ratio: 1.0 }], provenance: 'default' };
}

/** Human-readable provenance note stored on each allocation row. */
export function allocationNote(
  provenance: StewardshipProvenance,
  category: string | undefined
): string {
  switch (provenance) {
    case 'authored':
      return `Authored stewardship declared on the content's stewardedBy field${
        category ? ` (category: ${category})` : ''
      }`;
    case 'category':
      return `Affinity-based stewardship for ${category} content`;
    default:
      return 'Bootstrap steward assignment - uncategorized content';
  }
}

/**
 * True when the governance plane owns this row and the seeder must not write it.
 *
 * An allocation that has been disputed, ratified by an Elohim council, or moved
 * out of `active` is a governance artifact — its ratio is a negotiated outcome,
 * not a derived one. The seeder reports such drift and leaves the row alone.
 */
function isGovernanceOwned(stored: StoredAllocation): boolean {
  if (stored.disputeId) return true;
  if (stored.elohimRatifiedAt) return true;
  const state = stored.governanceState ?? 'active';
  return state !== 'active';
}

/**
 * Decide what must change for ONE content item so its stored allocations match
 * the resolved steward set.
 *
 * ## Why this exists: insert-only idempotency leaves ratios permanently stale
 *
 * The seeder's idempotency diff is (content, steward)-granular: it POSTs the
 * pairs that don't exist yet and skips the ones that do. That is correct for
 * MEMBERSHIP and silently wrong for RATIOS. When a content item's resolved
 * steward set changes shape — which is exactly what happened when the authored
 * `stewardedBy` path started displacing the bootstrap default — the NEW steward
 * is inserted at its normalized ratio while the pre-existing steward keeps the
 * ratio it was seeded with. `unit-appeals-process` is the canonical case:
 *
 *     matthew-dowell @ 1.00   (seeded 2026-06-04, provenance `default`)
 *   + pete-pastor    @ 0.333  (seeded 2026-07-31, provenance `authored`)
 *   = 1.333
 *
 * where the resolver says matthew 0.667 / pete 0.333. Fifty content items on
 * alpha carry exactly this shape (sums 1.333 / 1.579 / 1.636 / 1.70), which is
 * what the a2o `@integrity` scenario reports as
 * "Allocation ratios for unit-appeals-process sum to 1.333, expected ~1.0".
 * A ratio that does not sum to 1.0 corrupts every downstream share, so the
 * seeder must RECONCILE existing rows, not only insert missing ones.
 *
 * Repairs carry the method/contributionType/note along with the ratio: a row
 * whose ratio comes from the authored declaration but whose note still reads
 * "Bootstrap steward assignment - uncategorized content" is a lie about its own
 * provenance.
 *
 * Stewards stored but ABSENT from the resolved set are deliberately left
 * alone — retiring a steward is a supersede decision (it can carry accumulated
 * recognition), not a seeding one. They are counted by the caller so the drift
 * stays visible.
 */
export function planContentAllocations(
  contentId: string,
  resolved: ResolvedStewardship,
  category: string | undefined,
  storedBySteward: Map<string, StoredAllocation>
): ContentAllocationPlan {
  const note = allocationNote(resolved.provenance, category);
  const plan: ContentAllocationPlan = { creates: [], repairs: [], governedSkips: [] };

  for (const steward of resolved.stewards) {
    const contributionType: ContributionType = steward.contributionType ?? 'curator';
    const stored = storedBySteward.get(steward.presenceId);

    if (!stored) {
      plan.creates.push({
        contentId,
        stewardPresenceId: steward.presenceId,
        allocationRatio: steward.ratio,
        // Storage validates these against fixed enums (db/models.rs):
        //   allocation_methods = manual|computed|negotiated  → 'computed' (ratios ARE
        //     affinity-computed by this seeder); 'affinity' is rejected.
        //   contribution_types = original_creator|editor|translator|curator|maintainer|
        //     inherited → 'curator' (a steward curates); 'steward' is rejected.
        allocationMethod: 'computed',
        contributionType,
        note,
      });
      continue;
    }

    // A row this seeder did not author is not this seeder's to reconcile.
    // `allocation_methods` is manual|computed|negotiated (db/models.rs), `manual`
    // is BOTH the column default (migrations/2026-01-08-000000_initial/up.sql)
    // and what the app writes for every UI-created split
    // (app/lamad/src/app/services/stewardship-allocation.service.ts). Only
    // `computed` rows are this seeder's own output.
    //
    // Counting a non-computed method as "shape drift" and repairing it to
    // `computed` is precisely the stomp the StoredAllocation contract above
    // forbids — and genesis runs this unattended against the live fleet, so a
    // negotiated split would be silently overwritten with the affinity ratio
    // and the seeder's note. isGovernanceOwned() cannot catch it: a manual row
    // that is active and undisputed is invisible to all three of its checks.
    if (stored.allocationMethod !== 'computed') {
      plan.governedSkips.push(stored);
      continue;
    }

    const ratioDrifted = Math.abs(stored.allocationRatio - steward.ratio) > RATIO_EPSILON;
    const shapeDrifted = stored.contributionType !== contributionType;
    if (!ratioDrifted && !shapeDrifted) continue;

    if (isGovernanceOwned(stored)) {
      plan.governedSkips.push(stored);
      continue;
    }

    plan.repairs.push({
      allocationId: stored.id,
      contentId,
      stewardPresenceId: steward.presenceId,
      fromRatio: stored.allocationRatio,
      toRatio: steward.ratio,
      allocationMethod: 'computed',
      contributionType,
      note,
    });
  }

  return plan;
}

/**
 * Invert the flat `${contentId}::${stewardPresenceId}` allocation map into
 * content -> stored steward ids.
 *
 * ## Why this exists
 *
 * The reconciler needs, per content item, the stewards ALREADY stored against
 * it (to count the ones no longer in any resolved set). Asking that question of
 * the flat map by prefix — `for (const key of existing.keys()) if
 * (key.startsWith(`${contentId}::`))` — is O(content x allocations): on alpha
 * that is ~10k content ids x ~10k allocations = 10^8 iterations, each one
 * re-allocating the `${contentId}::` template literal, to produce a single
 * number that is only ever logged. This builds the answer once, in one pass.
 *
 * The steward id is read from the VALUE's `stewardPresenceId`, not parsed back
 * out of the key. The two are identical by construction — the key is built as
 * `${a.contentId}::${a.stewardPresenceId}` in
 * `StewardshipClient.getContentWithAllocations` — and reading the field cannot
 * mis-split an id that happens to contain the `::` separator.
 */
export function indexStoredStewardsByContent(
  existingAllocations: ReadonlyMap<string, StoredAllocation>
): Map<string, Set<string>> {
  const byContent = new Map<string, Set<string>>();
  for (const stored of existingAllocations.values()) {
    let stewards = byContent.get(stored.contentId);
    if (!stewards) {
      stewards = new Set<string>();
      byContent.set(stored.contentId, stewards);
    }
    stewards.add(stored.stewardPresenceId);
  }
  return byContent;
}

/**
 * The run's one report of stored-but-unresolved stewards, or null when there
 * are none. Extracted so the exact wording is pinned by a test alongside the
 * counter it reports — the count and the sentence are one claim.
 */
export function unresolvedStoredStewardsNotice(count: number): string | null {
  if (count <= 0) return null;
  return (
    `   Stored stewards no longer in any resolved set: ${count} ` +
    `(retirement is a supersede decision — not written here)`
  );
}

// =============================================================================
// Presence Loader
// =============================================================================

export function loadAllPresences(): PresenceData[] {
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
  const contentIndex = buildContentIndex();
  const presenceIdByHumanId = buildPresenceIdByHumanId(presences);
  console.log(`   Mapped ${contentIndex.size} content items to categories`);
  console.log(`   Resolvable humanId -> presenceId bindings: ${presenceIdByHumanId.size}`);

  /** Resolve one content item's steward set from the on-disk facts. */
  const stewardshipFor = (contentId: string): ResolvedStewardship => {
    const facts = contentIndex.get(contentId);
    return resolveStewardship(facts?.category, facts?.stewardedBy, presenceIdByHumanId);
  };

  // Show category distribution
  const categoryStats = new Map<string, number>();
  for (const facts of contentIndex.values()) {
    if (!facts.category) continue;
    categoryStats.set(facts.category, (categoryStats.get(facts.category) || 0) + 1);
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
    const provenanceCounts = new Map<StewardshipProvenance, number>();

    for (const [contentId] of contentIndex) {
      const { stewards: allocations, provenance } = stewardshipFor(contentId);
      provenanceCounts.set(provenance, (provenanceCounts.get(provenance) || 0) + 1);
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
    console.log(
      `   Average stewards per item: ${(totalAllocations / contentIndex.size).toFixed(1)}`
    );

    // Provenance breakdown — the audit surface for the authored-stewardship
    // path. `authored` counts the items whose steward set now comes from the
    // content's own stewardedBy declaration instead of the bootstrap default.
    console.log();
    console.log('   Provenance breakdown:');
    for (const p of ['category', 'authored', 'default'] as StewardshipProvenance[]) {
      console.log(`     ${p.padEnd(9)}: ${provenanceCounts.get(p) ?? 0} content items`);
    }
    const authoredIds = [...contentIndex.keys()].filter(
      (id) => stewardshipFor(id).provenance === 'authored'
    );
    if (authoredIds.length > 0) {
      console.log();
      console.log('   Content allocated from its authored stewardedBy:');
      for (const id of authoredIds) {
        const { stewards } = stewardshipFor(id);
        const shape = stewards
          .map((s) => `${s.presenceId}@${s.ratio.toFixed(2)}/${s.contributionType}`)
          .join(', ');
        console.log(`     ${id}: ${shape}`);
      }
    }
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

  // Step 5: Get existing (content, steward) allocation pairs. Idempotency is
  // per-steward, not per-content: a content item may be partially allocated.
  console.log('Checking existing allocations...');
  let existingAllocations: Map<string, StoredAllocation>;
  try {
    existingAllocations = await client.getContentWithAllocations();
    console.log(`   Found ${existingAllocations.size} existing (content, steward) allocations`);
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

  // Step 7: Reconcile every content item against its resolved steward set.
  //
  // TWO kinds of drift, not one:
  //
  //   MEMBERSHIP — a resolved steward with no row yet. POST it. (Per-steward
  //     idempotency: a content item is re-visited whenever ANY of its expected
  //     stewards lacks an allocation. The storage bulk handler counts an
  //     already-existing pair as `failed` (uniqueness Err → http.rs:5793-5796),
  //     so we diff against `existingAllocations` and send only the missing.)
  //
  //   RATIO — a row that exists but holds a stale share. PUT it. Insert-only
  //     idempotency cannot see this class at all: when the authored
  //     `stewardedBy` path started displacing the bootstrap default, the new
  //     steward was inserted at its normalized ratio while the pre-existing
  //     matthew-dowell row kept the 1.0 it was seeded with — 50 content items
  //     on alpha whose ratios sum to 1.333 / 1.579 / 1.636 / 1.70 instead of
  //     1.0. See `planContentAllocations` for the full account.
  //
  // Both are computed per content item by `planContentAllocations`, which also
  // refuses to touch a row the governance plane owns.
  const allocations: CreateAllocationInput[] = [];
  const repairs: AllocationRepair[] = [];
  const governedSkips: StoredAllocation[] = [];
  // Content items that gained at least one new (content, steward) pair — drives
  // the Step 9 affinity seeding (mirrors the per-steward diff, see below).
  const contentNeedingAllocations: string[] = [];
  const contentNeedingRepair = new Set<string>();
  // Stewards stored against a content item but absent from its resolved set.
  // Retiring one is a supersede decision, not a seeding one — counted, never written.
  let unresolvedStoredStewards = 0;

  // Provenance per content id — reused by the Step 9 affinity seeding.
  const provenanceByContent = new Map<string, StewardshipProvenance>();

  // content -> stewards already stored against it. Built ONCE (one pass over
  // the allocations) instead of re-scanned per content item inside the loop
  // below; see `indexStoredStewardsByContent` for the cost that removes.
  const storedStewardsByContent = indexStoredStewardsByContent(existingAllocations);

  for (const contentId of allContentIds) {
    const facts = contentIndex.get(contentId);
    const category = facts?.category;
    const resolved = stewardshipFor(contentId);
    provenanceByContent.set(contentId, resolved.provenance);

    const storedBySteward = new Map<string, StoredAllocation>();
    for (const steward of resolved.stewards) {
      const stored = existingAllocations.get(allocationKey(contentId, steward.presenceId));
      if (stored) storedBySteward.set(steward.presenceId, stored);
    }

    const plan = planContentAllocations(contentId, resolved, category, storedBySteward);

    if (plan.creates.length > 0) {
      contentNeedingAllocations.push(contentId);
      allocations.push(...plan.creates);
    }
    if (plan.repairs.length > 0) {
      contentNeedingRepair.add(contentId);
      repairs.push(...plan.repairs);
    }
    governedSkips.push(...plan.governedSkips);

    const resolvedIds = new Set(resolved.stewards.map((s) => s.presenceId));
    for (const stewardId of storedStewardsByContent.get(contentId) ?? EMPTY_STEWARD_SET) {
      if (!resolvedIds.has(stewardId)) unresolvedStoredStewards++;
    }
  }

  console.log(
    `Building allocations for ${contentNeedingAllocations.length} content items ` +
      `(${allContentIds.length} total; ${allContentIds.length - contentNeedingAllocations.length} fully allocated)...`
  );
  console.log(
    `   Ratio drift to repair: ${repairs.length} rows across ${contentNeedingRepair.size} content items`
  );
  if (governedSkips.length > 0) {
    console.log(
      `   Rows left untouched (human-authored method, or disputed/ratified/non-active): ${governedSkips.length}`
    );
    for (const s of governedSkips.slice(0, 10)) {
      console.log(
        `     ${s.contentId} / ${s.stewardPresenceId}: state=${s.governanceState ?? 'active'}` +
          `${s.disputeId ? ` dispute=${s.disputeId}` : ''}`
      );
    }
  }
  const unresolvedNotice = unresolvedStoredStewardsNotice(unresolvedStoredStewards);
  if (unresolvedNotice) {
    console.log(unresolvedNotice);
  }

  if (allocations.length === 0 && repairs.length === 0) {
    console.log('   All content already has every expected steward at its resolved ratio');
    console.log();
    console.log('Done!');
    return;
  }

  if (allocations.length > 0) {
    console.log(`   Generated ${allocations.length} missing allocation records`);
    console.log(
      `   Average new stewards per item: ${(allocations.length / contentNeedingAllocations.length).toFixed(1)}`
    );
  }
  console.log();

  // Step 8: Bulk create allocations (in batches to avoid overwhelming the API)
  const BATCH_SIZE = 100;
  const BATCH_DELAY_MS = 200; // Let SQLite breathe between batches
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
    // Brief pause between batches to avoid SQLite "database is locked" errors
    if (i + BATCH_SIZE < allocations.length) {
      await new Promise(resolve => setTimeout(resolve, BATCH_DELAY_MS));
    }
  }

  // Step 8b: Repair the stale ratios on rows that already exist.
  //
  // One PUT per row (there is no bulk update endpoint), paced the same way the
  // creates are so SQLite is not hammered. A repair failure is reported and
  // counted, never fatal: a partially-repaired table is strictly closer to
  // correct than an unrepaired one, and the next run re-plans from live state.
  let totalRepaired = 0;
  let totalRepairFailed = 0;

  if (repairs.length > 0) {
    console.log();
    console.log(`Repairing ${repairs.length} drifted allocation ratios...`);
    for (let i = 0; i < repairs.length; i++) {
      const repair = repairs[i];
      try {
        await client.repairAllocation(repair);
        totalRepaired++;
        if (totalRepaired <= 10) {
          console.log(
            `   ${repair.contentId} / ${repair.stewardPresenceId}: ` +
              `${repair.fromRatio.toFixed(4)} -> ${repair.toRatio.toFixed(4)}`
          );
        }
      } catch (error) {
        totalRepairFailed++;
        allErrors.push(String(error));
      }
      // Same pacing as the create batches — one PUT per row can still lock SQLite.
      if ((i + 1) % BATCH_SIZE === 0 && i + 1 < repairs.length) {
        await new Promise((resolve) => setTimeout(resolve, BATCH_DELAY_MS));
      }
    }
    if (totalRepaired > 10) {
      console.log(`   ... and ${totalRepaired - 10} more repaired`);
    }
  }

  console.log();
  console.log('='.repeat(60));
  console.log('Stewardship allocation complete!');
  console.log(`   Created: ${totalCreated}`);
  console.log(`   Failed: ${totalFailed}`);
  console.log(`   Ratios repaired: ${totalRepaired}`);
  console.log(`   Ratio repairs failed: ${totalRepairFailed}`);
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
      alloc.stewardPresenceId,
      (stewardSummary.get(alloc.stewardPresenceId) || 0) + 1
    );
  }
  console.log();
  console.log('   Steward distribution:');
  const sorted = [...stewardSummary.entries()].sort((a, b) => b[1] - a[1]);
  for (const [steward, count] of sorted) {
    const name = presences.find((p) => p.id === steward)?.displayName || steward;
    console.log(`     ${name}: ${count} allocations`);
  }

  // Step 9: Seed steward affinities
  console.log();
  console.log('Seeding steward affinities...');

  const affinityInputs: Array<{
    stewardId: string;
    contentId: string;
    affinityScore: number;
    source: string;
  }> = [];

  for (const contentId of contentNeedingAllocations) {
    const facts = contentIndex.get(contentId);
    const category = facts?.category;
    // Authored allocations carry the DECLARED affinity scores from the content
    // JSON — the honest source for "how close is this steward to this content".
    // Everything else keeps the curated category affinity table, then the
    // bootstrap 0.7.
    const authoredAffinities =
      provenanceByContent.get(contentId) === 'authored'
        ? (facts?.stewardedBy ?? [])
            .map((s) => ({
              stewardId: presenceIdByHumanId.get(s.humanId),
              affinityScore: typeof s.affinity === 'number' ? s.affinity : 0,
            }))
            .filter(
              (e): e is StewardAffinityEntry => typeof e.stewardId === 'string' && e.affinityScore > 0
            )
        : null;

    const affinityEntries =
      authoredAffinities && authoredAffinities.length > 0
        ? authoredAffinities
        : category && CATEGORY_AFFINITY_MAP[category]
          ? CATEGORY_AFFINITY_MAP[category]
          : [{ stewardId: 'matthew-dowell', affinityScore: 0.7 }];

    for (const entry of affinityEntries) {
      affinityInputs.push({
        stewardId: entry.stewardId,
        contentId,
        affinityScore: entry.affinityScore,
        source: 'genesis_seed',
      });
    }
  }

  console.log(`   Generated ${affinityInputs.length} affinity records`);

  for (let i = 0; i < affinityInputs.length; i += BATCH_SIZE) {
    const batch = affinityInputs.slice(i, i + BATCH_SIZE);
    const batchNum = Math.floor(i / BATCH_SIZE) + 1;
    const totalBatches = Math.ceil(affinityInputs.length / BATCH_SIZE);

    try {
      const result = await client.bulkCreateAffinities(batch);
      console.log(
        `   Affinity batch ${batchNum}/${totalBatches}: ${result.created} created, ${result.errors.length} errors`
      );
    } catch (error) {
      console.error(`   ERROR in affinity batch ${batchNum}: ${error}`);
    }
    // Brief pause between batches to avoid SQLite "database is locked" errors
    if (i + BATCH_SIZE < affinityInputs.length) {
      await new Promise(resolve => setTimeout(resolve, BATCH_DELAY_MS));
    }
  }

  console.log('='.repeat(60));
}

// Standalone execution only — guard so importing this module (seed-epr-atom.ts
// imports the stewardship resolver to build the landing atom's stewardship
// claim; the unit tests import it too) does NOT run the seeder. Mirrors the
// isMain pattern in seed-commitments.ts / seed-sqlite.ts.
const isMain = import.meta.url === `file://${process.argv[1]}`;
if (isMain) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}
