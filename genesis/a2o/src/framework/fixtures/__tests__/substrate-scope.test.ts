/**
 * Tests for substrate-scope.ts — the generic substrate-availability primitive (runtime arm of the
 * cybernetic scope reconciler). Nothing here is shem-specific: the same primitive gates any cap
 * declared in cluster-state.yaml (shem, alpha-cluster-6peer, harbor-registry, …).
 *
 * Run: tsx --test src/framework/fixtures/__tests__/substrate-scope.test.ts
 *
 * We inject a fixture cluster-state.yaml via ELOHIM_CLUSTER_STATE_PATH_OVERRIDE and exercise the
 * env-override channel (ELOHIM_REMOTE_COMPUTE_STATUS for shem; ELOHIM_CAP_<CAP>_STATUS generic).
 */

import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, it } from 'node:test';

import {
  parseResourceAvailability,
  knownResources,
  isCapAvailable,
  requiredCapsFromTags,
  unavailableRequiredCaps,
  noteSubstrateSkip,
  substrateSkippedScenarios,
  resetSubstrateSkips,
} from '../substrate-scope.js';

const TMP_PREFIX = 'substrate-scope-';
const PATH_ENV = 'ELOHIM_CLUSTER_STATE_PATH_OVERRIDE';
const SHEM = 'shem';
const ALPHA = 'alpha-cluster-6peer';
const HOUSEHOLD = 'household-nodes';
const AVAILABLE = 'available';
const UNAVAILABLE = 'unavailable';
const TAG_DOORWAY = '@requires:doorway';

const CAP_ENV = [
  'ELOHIM_REMOTE_COMPUTE_STATUS',
  'ELOHIM_CAP_SHEM_STATUS',
  'ELOHIM_CAP_ALPHA_CLUSTER_6PEER_STATUS',
];

function clearCapEnv(): void {
  for (const v of CAP_ENV) delete process.env[v];
}

/** A realistic multi-resource cluster-state.yaml fixture (household up, shem down, alpha degraded,
 * harbor up) — with role/note multi-line scalars like the real file, to exercise the parser. */
function writeClusterState(dir: string): string {
  const path = join(dir, 'cluster-state.yaml');
  writeFileSync(
    path,
    [
      '# comment header',
      'schema_version: 1',
      'updated: 2026-06-03',
      '',
      'resources:',
      '  household-nodes:',
      '    role: local household cluster',
      '    available: true',
      '  shem:',
      '    role: multi-tenant live P2P canvas',
      '    available: false',
      '    note: offline (operator-declared). Cross-node scenarios OUT OF SCOPE',
      '          until it returns — held, NOT regressed.',
      '  alpha-cluster-6peer:',
      '    role: 6-peer alpha soak cluster',
      '    available: degraded',
      '  harbor-registry:',
      '    role: CI image registry',
      '    available: true',
      '',
      '# requires_env vocabulary used by tests/stories/gates',
      '#   shem — needs the shem multi-tenant canvas',
      '',
    ].join('\n')
  );
  return path;
}

const sortedStrings = (xs: string[]): string[] => [...xs].sort((a, b) => a.localeCompare(b));

void describe('parseResourceAvailability', () => {
  let workDir: string;
  let raw: string;
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    raw = readFileSync(writeClusterState(workDir), 'utf-8');
  });
  afterEach(() => rmSync(workDir, { recursive: true, force: true }));

  void it('reads available: true as available', () => {
    assert.equal(parseResourceAvailability(raw, HOUSEHOLD), AVAILABLE);
    assert.equal(parseResourceAvailability(raw, 'harbor-registry'), AVAILABLE);
  });
  void it('reads available: false as unavailable', () => {
    assert.equal(parseResourceAvailability(raw, SHEM), UNAVAILABLE);
  });
  void it('reads available: degraded as unavailable (conservative)', () => {
    assert.equal(parseResourceAvailability(raw, ALPHA), UNAVAILABLE);
  });
  void it('returns unknown for a resource absent from cluster-state', () => {
    assert.equal(parseResourceAvailability(raw, 'iroh'), 'unknown');
  });
  void it('is not confused by multi-line role/note scalars around available:', () => {
    // shem's `available: false` sits between role: and a wrapped note: — must still resolve.
    assert.equal(parseResourceAvailability(raw, SHEM), UNAVAILABLE);
  });
});

void describe('knownResources', () => {
  let workDir: string;
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    process.env[PATH_ENV] = writeClusterState(workDir);
  });
  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
    delete process.env[PATH_ENV];
  });

  void it('extracts every declared resource name, and nothing from the trailing comment block', () => {
    assert.deepEqual(sortedStrings([...knownResources()]), [
      ALPHA,
      'harbor-registry',
      HOUSEHOLD,
      SHEM,
    ]);
  });
});

void describe('isCapAvailable', () => {
  let workDir: string;
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    process.env[PATH_ENV] = writeClusterState(workDir);
    clearCapEnv();
  });
  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
    delete process.env[PATH_ENV];
    clearCapEnv();
  });

  void it('derives from cluster-state when no env override is set', () => {
    assert.equal(isCapAvailable(HOUSEHOLD), true);
    assert.equal(isCapAvailable(SHEM), false);
    assert.equal(isCapAvailable(ALPHA), false); // degraded → unavailable
  });
  void it('fails open for a cap not declared in cluster-state (never silently gates)', () => {
    assert.equal(isCapAvailable('iroh'), true);
  });
  void it('lets the shem env override (ELOHIM_REMOTE_COMPUTE_STATUS) win over the durable home', () => {
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = AVAILABLE;
    assert.equal(isCapAvailable(SHEM), true);
    process.env.ELOHIM_REMOTE_COMPUTE_STATUS = UNAVAILABLE;
    assert.equal(isCapAvailable(SHEM), false);
  });
  void it('lets a generic per-cap env override win (ELOHIM_CAP_<CAP>_STATUS)', () => {
    process.env.ELOHIM_CAP_ALPHA_CLUSTER_6PEER_STATUS = AVAILABLE;
    assert.equal(isCapAvailable(ALPHA), true);
  });
});

void describe('requiredCapsFromTags / unavailableRequiredCaps', () => {
  let workDir: string;
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    process.env[PATH_ENV] = writeClusterState(workDir);
    clearCapEnv();
  });
  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
    delete process.env[PATH_ENV];
    clearCapEnv();
  });

  void it('extracts cap names from @requires:<cap> tags only', () => {
    const caps = requiredCapsFromTags(['@e2e', '@auth', `@requires:${SHEM}`, TAG_DOORWAY, '@wip']);
    assert.deepEqual(sortedStrings(caps), ['doorway', SHEM]);
  });
  void it('reports only cluster-tracked caps that are unavailable (ignores fixture preconditions)', () => {
    // @requires:doorway and @requires:seeded-content are NOT cluster-state resources → ignored.
    const missing = unavailableRequiredCaps([
      `@requires:${SHEM}`,
      TAG_DOORWAY,
      '@requires:seeded-content',
    ]);
    assert.deepEqual(missing, [SHEM]);
  });
  void it('reports an in-scope scenario as empty (no held caps)', () => {
    assert.deepEqual(unavailableRequiredCaps(['@e2e', `@requires:${HOUSEHOLD}`, TAG_DOORWAY]), []);
  });
  void it('reports multiple unavailable caps (e.g. shem + alpha-cluster-6peer)', () => {
    const missing = unavailableRequiredCaps([`@requires:${SHEM}`, `@requires:${ALPHA}`]);
    assert.deepEqual(sortedStrings(missing), [ALPHA, SHEM]);
  });
});

void describe('substrate skip tracking', () => {
  beforeEach(() => resetSubstrateSkips());
  afterEach(() => resetSubstrateSkips());

  void it('records and snapshots held scenarios, sorted', () => {
    noteSubstrateSkip('Zebra scenario', [SHEM]);
    noteSubstrateSkip('Apple scenario', [SHEM, ALPHA]);
    assert.deepEqual(substrateSkippedScenarios(), [
      { scenario: 'Apple scenario', caps: [SHEM, ALPHA] },
      { scenario: 'Zebra scenario', caps: [SHEM] },
    ]);
  });
  void it('reset clears the tracker', () => {
    noteSubstrateSkip('x', [SHEM]);
    resetSubstrateSkips();
    assert.deepEqual(substrateSkippedScenarios(), []);
  });
});
