/**
 * Human fixtures — pre-seeded test personas from genesis/data/humans/humans.json.
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

import { parseResourceAvailability } from './substrate-scope.js';

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
  relationshipType: string;
  intimacyLevel: string;
  context?: string;
}

interface HumansJson {
  humans: HumansJsonHuman[];
  relationships: HumansJsonRelationship[];
}

// ---------------------------------------------------------------------------
// Credential derivation
// ---------------------------------------------------------------------------

// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- test fixture credentials, not production
const DEFAULT_PASSWORD = 'Test2026!';
const ADMIN_EMAIL = 'matthew.dowell@alpha.elohim.host';
// eslint-disable-next-line sonarjs/no-hardcoded-passwords -- test fixture credentials, not production
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

let _cache: {
  fixtures: Map<string, HumanFixture>;
  relationships: HumansJsonRelationship[];
} | null = null;

function loadHumansJson(): HumansJson {
  const __dirname = dirname(fileURLToPath(import.meta.url));
  // genesis/a2o/src/framework/fixtures/ → genesis/data/humans/humans.json
  const jsonPath = resolve(__dirname, '../../../../data/humans/humans.json');
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

// ---------------------------------------------------------------------------
// Deployment topology awareness (reads deployments.json)
// ---------------------------------------------------------------------------
//
// IMPORTANT BOUNDARY: this module is genesis test-bench tooling. It knows
// about k8s node pools ("remote" = shem, "edge"/"performance"/"operations" =
// other nodes) because the test harness models peers as k8s pods on a real
// cluster. Elohim protocol code MUST NOT know about k8s — peers in
// production have no k8s pool concept. This file is the gating layer that
// keeps the test bench separate from the substrate.
//
// Some fixture humans (matthew, jessica, terrance, adam, pete, frank) have
// k8s pods declared in genesis/orchestrator/data/deployments.json. Others
// (Tommy, Georgina, Maria, etc.) are doorway-seeded personas without a
// k8s pod. When a node fails (e.g. shem 2026-05-04), affected humans get
// "suspended": true in deployments.json — OR the operator declares shem
// unavailable for this run via ELOHIM_REMOTE_COMPUTE_STATUS=unavailable. Both paths
// make isHumanDeployed() return false for the affected humans, and a2o
// step definitions auto-skip those scenarios.
//
// Scale-tier separation: shem is the heavy-lift k8s node where most of
// the peer-modeling load lives. The dev clusters (edge/performance/
// operations) are deliberately kept lean because dev work runs directly
// on them. When shem is down, we prefer to scale DOWN the test fixture
// set (skip remote-only humans) rather than fall back onto the dev
// clusters and crowd them out. The pipeline marks the build UNSTABLE
// rather than FAILED when this auto-scale-down happens.

/** k8s node pools known to the test bench. "remote" = shem; others are
 * dev-cluster pools (intel-nuc, ethosengine performance, etc.). */
export type NodePool = 'remote' | 'edge' | 'performance' | 'operations';

interface DeploymentRecord {
  name: string;
  suspended?: boolean;
  nodeTypes?: NodePool[];
}

interface DeploymentsCache {
  /** lowercased name → suspended boolean. Empty when file is unreadable (fail-open). */
  byName: Map<string, boolean>;
  /** lowercased name → nodeTypes array (in scheduler-preference order). */
  nodeTypesByName: Map<string, NodePool[]>;
}

let _deploymentsCache: DeploymentsCache | null = null;

function loadDeployments(): DeploymentsCache {
  try {
    // Allow tests to point at a fixture deployments.json without touching the real one.
    const override = process.env.ELOHIM_DEPLOYMENTS_PATH_OVERRIDE;
    let jsonPath: string;
    if (override) {
      jsonPath = override;
    } else {
      const __dirname = dirname(fileURLToPath(import.meta.url));
      // genesis/a2o/src/framework/fixtures/ → genesis/orchestrator/data/deployments.json
      jsonPath = resolve(__dirname, '../../../../orchestrator/data/deployments.json');
    }
    const raw = readFileSync(jsonPath, 'utf-8');
    const data = JSON.parse(raw) as { humans?: DeploymentRecord[] };
    const byName = new Map<string, boolean>();
    const nodeTypesByName = new Map<string, NodePool[]>();
    for (const h of data.humans ?? []) {
      const key = h.name.toLowerCase();
      byName.set(key, Boolean(h.suspended));
      if (Array.isArray(h.nodeTypes) && h.nodeTypes.length > 0) {
        nodeTypesByName.set(key, [...h.nodeTypes]);
      }
    }
    return { byName, nodeTypesByName };
  } catch {
    // Fail-open: tests should still run if deployments.json is unreadable.
    return { byName: new Map(), nodeTypesByName: new Map() };
  }
}

// ---------------------------------------------------------------------------
// Shem / node-pool availability (test bench only — never read by elohim code)
// ---------------------------------------------------------------------------

/** Operator declarations for the test bench:
 *   - "available"   → shem is up; remote-resident humans are deployable
 *   - "unavailable" → shem is down; remote-resident humans auto-skip
 *   - unset / "unknown" / unrecognized → DERIVE from the durable home
 *       (genesis/manifests/cluster-state.yaml); fail-open only if that file is
 *       itself unreadable.
 *
 * The two signal homes MUST stay coherent: cluster-state.yaml (durable,
 * planning-layer) and ELOHIM_REMOTE_COMPUTE_STATUS (runtime, a2o/CI). CI's
 * Probe Substrate stage / the operator sets the env var explicitly, and an
 * explicit available|unavailable always WINS (the CI override). When the env
 * is unset — a bare local run — we derive the status from cluster-state so the
 * runtime can't silently claim shem is up while the durable home says it is
 * down (no need to `eval "$(scope-reconcile.py --env)"` first). Mirrors
 * scope-reconcile.py's derive_remote_compute_status(): shem available iff its
 * `available:` value is exactly `true`.
 */
export type RemoteComputeStatus = 'available' | 'unavailable' | 'unknown';

/** The cluster resource whose availability drives the remote-compute signal.
 * MUST match REMOTE_COMPUTE_RESOURCE in
 * .claude/scripts/memory-kit/scope-reconcile.py. */
const REMOTE_COMPUTE_RESOURCE = 'shem';

/** shem availability from cluster-state.yaml text, as a RemoteComputeStatus. Delegates the generic
 * parse to substrate-scope (one parser for every cap); shem is available iff its `available:` value
 * is exactly `true` — `false`/`degraded`/absent all read as 'unavailable' (the conservative read for
 * a cross-node canvas). */
function parseShemAvailability(raw: string): RemoteComputeStatus {
  // Delegate the generic line-based parse to substrate-scope (one parser for every cap). shem is
  // available iff its `available:` value is exactly `true`; `false`/`degraded`/absent all read as
  // 'unavailable' — the conservative read for a cross-node canvas.
  return parseResourceAvailability(raw, REMOTE_COMPUTE_RESOURCE) === 'available'
    ? 'available'
    : 'unavailable';
}

function deriveRemoteComputeFromClusterState(): RemoteComputeStatus | null {
  try {
    const override = process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE;
    const yamlPath =
      override ??
      // genesis/a2o/src/framework/fixtures/ → genesis/manifests/cluster-state.yaml
      resolve(dirname(fileURLToPath(import.meta.url)), '../../../../manifests/cluster-state.yaml');
    return parseShemAvailability(readFileSync(yamlPath, 'utf-8'));
  } catch {
    return null; // unreadable durable home (e.g. published consumer) → fail-open
  }
}

function readRemoteComputeStatus(): RemoteComputeStatus {
  const raw = (process.env.ELOHIM_REMOTE_COMPUTE_STATUS ?? '').toLowerCase().trim();
  // An explicit available|unavailable always wins — the CI/operator override.
  if (raw === 'available' || raw === 'unavailable') return raw;
  // Otherwise (unset / 'unknown' / unrecognized) derive from the durable home so
  // the runtime can't disagree with cluster-state; fail-open only if unreadable.
  return deriveRemoteComputeFromClusterState() ?? 'unknown';
}

/** Returns true if shem is considered reachable for this test run. Reflects an
 * explicit ELOHIM_REMOTE_COMPUTE_STATUS when set, otherwise the value derived
 * from cluster-state.yaml; fail-open ("unknown" → available) only when neither
 * the env var nor the durable home is available. */
export function isRemoteComputeAvailable(): boolean {
  return readRemoteComputeStatus() !== 'unavailable';
}

/** Returns true if at least one of `pools` is reachable on the test bench
 * right now. Today only the `remote` pool is gated (via shem-status); the
 * dev-cluster pools are assumed always available because dev is running on
 * them. When new gated pools appear, add their probes here. */
export function isAnyNodePoolAvailable(pools: NodePool[]): boolean {
  for (const p of pools) {
    if (p === 'remote') {
      if (isRemoteComputeAvailable()) return true;
    } else {
      // edge / performance / operations — dev clusters, always available.
      return true;
    }
  }
  return false;
}

/** Counter of humans auto-skipped this run due to node-pool unavailability.
 * Test runner can read this at end-of-run and mark the build UNSTABLE if > 0
 * (i.e. some scenarios skipped because of test-bench scaling, not failures). */
let _autoSkippedHumans = new Set<string>();

/** Snapshot of which humans were auto-skipped by node-pool gating. Returns a
 * copy so callers can't mutate internal state. */
export function autoSkippedHumans(): string[] {
  return [..._autoSkippedHumans].sort((a, b) => a.localeCompare(b));
}

/** Reset auto-skip tracking. Test runners call this at start-of-run. */
export function resetAutoSkippedHumans(): void {
  _autoSkippedHumans = new Set();
}

function ensureDeploymentsLoaded(): DeploymentsCache {
  _deploymentsCache ??= loadDeployments();
  return _deploymentsCache;
}

/**
 * Returns true if the named human is currently deployable for live testing:
 *   - matthew/jessica/terrance → true (in deployments.json, not suspended)
 *   - adam/pete/frank (suspended) → false (conductor pod won't run)
 *   - daniel/emma/susan/caleb/eve/gertrude/frank/pete/terrance/adam
 *     (nodeTypes starts with "remote") → false when shem is unavailable
 *   - Susan/Tommy/Georgina/etc. → true (doorway-only personas, no k8s pod)
 *
 * Two gating paths:
 *   1. Hard suspension via deployments.json "suspended": true (operator
 *      decision; persists across runs).
 *   2. Soft auto-skip via ELOHIM_REMOTE_COMPUTE_STATUS=unavailable (per-run probe;
 *      no deployments.json edit needed). Pipeline probes shem at start
 *      and sets the env; tests scale themselves to the available compute.
 *
 * Either path causes step definitions that name the affected human to
 * return 'pending' from Cucumber, so the scenario auto-skips without any
 * feature-file edit. Auto-skipped humans accumulate in autoSkippedHumans();
 * the test runner can mark the build UNSTABLE (not FAILED) at end-of-run
 * when that set is non-empty.
 */
export function isHumanDeployed(displayName: string): boolean {
  const cache = ensureDeploymentsLoaded();
  // Empty registry = fail-open (file unreadable, missing, or no humans declared).
  if (cache.byName.size === 0) return true;
  const key = displayName.toLowerCase();
  const suspended = cache.byName.get(key);
  // Not in registry = doorway-only persona; treat as deployed.
  if (suspended === undefined) return true;
  // Hard-suspended in deployments.json — definitive false.
  if (suspended) return false;
  // Soft auto-skip: human is not suspended in deployments.json, but their
  // nodeTypes require a pool that's currently unavailable on the test bench.
  const nodeTypes = cache.nodeTypesByName.get(key);
  if (nodeTypes !== undefined && !isAnyNodePoolAvailable(nodeTypes)) {
    _autoSkippedHumans.add(key);
    return false;
  }
  return true;
}

/**
 * True if the named persona has NO on-prem fallback — every declared nodeType
 * is the "remote" pool (shem). These are exactly the personas that auto-skip
 * when the remote pool is unavailable. Household personas (with operations/
 * edge/performance pools) return false; unknown or doorway-only personas (not
 * in deployments.json, or with no nodeTypes) return false. Used by the
 * substrate-reconciliation a2o scenarios to assert the invariant that the run
 * only ever scales DOWN remote-only personas, never the household.
 */
export function isRemoteOnlyPersona(displayName: string): boolean {
  const cache = ensureDeploymentsLoaded();
  const nodeTypes = cache.nodeTypesByName.get(displayName.toLowerCase());
  return Array.isArray(nodeTypes) && nodeTypes.length > 0 && nodeTypes.every(p => p === 'remote');
}

/**
 * Reset the deployments.json cache. For unit tests only — production code
 * should never call this. Lets a test write a fixture deployments.json,
 * reset the cache, then assert behavior on the next isHumanDeployed call.
 */
export function _resetDeploymentsCacheForTests(): void {
  _deploymentsCache = null;
  _autoSkippedHumans = new Set();
}
