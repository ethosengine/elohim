/**
 * Human fixtures — pre-seeded test personas from genesis/docs/humans/humans.json.
 *
 * These humans are registered in the deployment by `genesis/seeder/src/seed-humans.ts`.
 * Tests can login as any of them without needing to register first.
 *
 * Credential derivation (MUST match genesis/seeder/src/seed-humans.ts):
 *   identifier: {displayName slugified}@test.elohim.host
 *   password:   Test2026!
 *
 * Matthew is the exception — he uses his admin credentials:
 *   identifier: matthew.dowell@alpha.elohim.host
 *   password:   TestAdmin2026!
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { HumanCredentials } from '../human.js';

// ---------------------------------------------------------------------------
// Types mirroring humans.json schema
// ---------------------------------------------------------------------------

export interface HumansJsonHuman {
  id: string;
  displayName: string;
  bio: string;
  category: string;
  profileReach: string;
  location?: { layer: string; name: string };
  organizations?: { id: string; name: string; role: string }[];
  communities?: string[];
  affinities?: string[];
  ageCategory?: string;
  guardianIds?: string[];
  accessibilityNeeds?: string[];
  languagePreferences?: { primary: string; secondary: string; learningLevel: string };
  attestations?: string[];
  claimedAttestations?: { claim: string; status: string; challengedAt?: string }[];
  flags?: { type: string; reason: string; count?: number; severity?: string }[];
  isPseudonymous?: boolean;
  acceptingConnections?: boolean;
  notes?: string;
}

export interface HumansJsonRelationship {
  source: string;
  target: string;
  type: string;
  intimacy: string;
  context?: string;
}

interface HumansJson {
  humans: HumansJsonHuman[];
  relationships: HumansJsonRelationship[];
}

// ---------------------------------------------------------------------------
// Credential derivation
// ---------------------------------------------------------------------------

const DEFAULT_PASSWORD = 'Test2026!';
const ADMIN_EMAIL = 'matthew.dowell@alpha.elohim.host';
const ADMIN_PASSWORD = 'TestAdmin2026!';
const ADMIN_HUMAN_ID = 'human-matthew-manager';

function slugify(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
}

/** Derive deterministic credentials for a human from humans.json. */
export function deriveCredentials(human: HumansJsonHuman): HumanCredentials {
  if (human.id === ADMIN_HUMAN_ID) {
    return {
      identifier: ADMIN_EMAIL,
      password: ADMIN_PASSWORD,
      displayName: human.displayName,
    };
  }
  return {
    identifier: `${slugify(human.displayName)}@test.elohim.host`,
    password: DEFAULT_PASSWORD,
    displayName: human.displayName,
  };
}

// ---------------------------------------------------------------------------
// Fixture: combines human metadata + credentials
// ---------------------------------------------------------------------------

export interface HumanFixture {
  id: string;
  credentials: HumanCredentials;
  category: string;
  bio: string;
  profileReach: string;
  affinities: string[];
  isAdmin: boolean;
  isMinor: boolean;
  isRedTeam: boolean;
  raw: HumansJsonHuman;
}

// ---------------------------------------------------------------------------
// Loader — reads humans.json once and caches
// ---------------------------------------------------------------------------

let _cache: { fixtures: Map<string, HumanFixture>; relationships: HumansJsonRelationship[] } | null =
  null;

function loadHumansJson(): HumansJson {
  const __dirname = dirname(fileURLToPath(import.meta.url));
  // genesis/a2o/src/framework/fixtures/ → genesis/docs/humans/humans.json
  const jsonPath = resolve(__dirname, '../../../../docs/humans/humans.json');
  const raw = readFileSync(jsonPath, 'utf-8');
  return JSON.parse(raw) as HumansJson;
}

function ensureLoaded() {
  if (_cache) return _cache;

  const json = loadHumansJson();
  const fixtures = new Map<string, HumanFixture>();

  for (const h of json.humans) {
    const fixture: HumanFixture = {
      id: h.id,
      credentials: deriveCredentials(h),
      category: h.category,
      bio: h.bio,
      profileReach: h.profileReach,
      affinities: h.affinities ?? [],
      isAdmin: h.id === ADMIN_HUMAN_ID,
      isMinor: h.ageCategory === 'minor',
      isRedTeam: h.category === 'red-team',
      raw: h,
    };
    // Key by displayName (what Cucumber steps use)
    fixtures.set(h.displayName, fixture);
  }

  _cache = { fixtures, relationships: json.relationships };
  return _cache;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/** All fixture humans, keyed by displayName. */
export function allFixtures(): Map<string, HumanFixture> {
  return ensureLoaded().fixtures;
}

/** All relationships from humans.json. */
export function allRelationships(): HumansJsonRelationship[] {
  return ensureLoaded().relationships;
}

/** Get a fixture by displayName. Throws if not found. */
export function getFixture(displayName: string): HumanFixture {
  const f = ensureLoaded().fixtures.get(displayName);
  if (!f) {
    const known = [...ensureLoaded().fixtures.keys()].join(', ');
    throw new Error(`Unknown human fixture: "${displayName}". Known: ${known}`);
  }
  return f;
}

/** Get credentials for a fixture human by displayName. */
export function fixtureCredentials(displayName: string): HumanCredentials {
  return getFixture(displayName).credentials;
}

/** Get fixture humans by category. */
export function fixturesByCategory(category: string): HumanFixture[] {
  return [...ensureLoaded().fixtures.values()].filter(f => f.category === category);
}

/** Get a fixture by human ID (e.g. "human-matthew-manager"). Throws if not found. */
export function getFixtureById(id: string): HumanFixture {
  const match = [...ensureLoaded().fixtures.values()].find(f => f.id === id);
  if (!match) {
    const known = [...ensureLoaded().fixtures.values()].map(f => f.id).join(', ');
    throw new Error(`Unknown human fixture ID: "${id}". Known: ${known}`);
  }
  return match;
}

/** All display names from humans.json (convenience for step definitions). */
export function allDisplayNames(): string[] {
  return [...ensureLoaded().fixtures.keys()];
}

/** Get relationships for a specific human id. */
export function relationshipsFor(humanId: string): HumansJsonRelationship[] {
  return ensureLoaded().relationships.filter(r => r.source === humanId || r.target === humanId);
}
