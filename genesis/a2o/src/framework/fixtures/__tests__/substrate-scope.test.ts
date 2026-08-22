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
  actFromTags,
  actBaselineCaps,
  undeclaredCapsWarned,
  resetUndeclaredCapWarnings,
  destructiveAllowed,
  DESTRUCTIVE_ENV,
  OWNED_SUBSTRATE_CAP,
} from '../substrate-scope.js';

const TMP_PREFIX = 'substrate-scope-';
const CLUSTER_STATE_FILE = 'cluster-state.yaml';
const PATH_ENV = 'ELOHIM_CLUSTER_STATE_PATH_OVERRIDE';
const SHEM = 'shem';
const ALPHA = 'alpha-cluster-6peer';
const HOUSEHOLD = 'household-nodes';
const AVAILABLE = 'available';
const UNAVAILABLE = 'unavailable';
const TAG_DOORWAY = '@requires:doorway';
const RESOURCES = 'resources:';
const AVAIL_TRUE = '    available: true';
const AVAIL_FALSE = '    available: false';
const INVENTED = '@requires:invented-cap';

const CAP_ENV = [
  'ELOHIM_REMOTE_COMPUTE_STATUS',
  'ELOHIM_CAP_SHEM_STATUS',
  'ELOHIM_CAP_ALPHA_CLUSTER_6PEER_STATUS',
  'ELOHIM_CAP_OWNED_SUBSTRATE_STATUS',
  'A2O_ALLOW_DESTRUCTIVE',
];

function clearCapEnv(): void {
  for (const v of CAP_ENV) delete process.env[v];
}

/** A realistic multi-resource cluster-state.yaml fixture (household up, shem down, alpha degraded,
 * harbor up) — with role/note multi-line scalars like the real file, to exercise the parser. */
function writeClusterState(dir: string): string {
  const path = join(dir, CLUSTER_STATE_FILE);
  writeFileSync(
    path,
    [
      '# comment header',
      'schema_version: 1',
      'updated: 2026-06-03',
      '',
      RESOURCES,
      '  household-nodes:',
      '    role: local household cluster',
      AVAIL_TRUE,
      '  shem:',
      '    role: multi-tenant live P2P canvas',
      AVAIL_FALSE,
      '    note: offline (operator-declared). Cross-node scenarios OUT OF SCOPE',
      '          until it returns — held, NOT regressed.',
      '  alpha-cluster-6peer:',
      '    role: 6-peer alpha soak cluster',
      '    available: degraded',
      '  harbor-registry:',
      '    role: CI image registry',
      AVAIL_TRUE,
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

// ---------------------------------------------------------------------------
// ACT LAYERING — @act:<i|ii|iii|host> resolution.
//
// The act tag carries the act's BASELINE caps (read from that act's cluster-state file); the ACTIVE
// LANE is whatever ELOHIM_CLUSTER_STATE_PATH_OVERRIDE points at. A scenario runs iff its act's
// baseline ⊆ the lane's available caps AND every explicit @requires cap is available.
// ---------------------------------------------------------------------------

const MANIFESTS_ENV = 'ELOHIM_MANIFESTS_DIR_OVERRIDE';
const OWNED = 'owned-substrate';
const DHT = 'dht-anchored-content';
const SSR = 'ssr-bundle';
const ACT1_FILE = 'cluster-state.act1-household.yaml';
const ACT2_FILE = 'cluster-state.act2-neighbourhood.yaml';
const LIVE_FILE = 'cluster-state.yaml';
const ACT_I = '@act:i';
const ACT_II = '@act:ii';

/** Synthetic three-file manifests dir: deliberately small, so these tests assert the RESOLUTION
 * rule and never drift when the real lane contracts gain a cap. */
function writeManifestsDir(dir: string): void {
  writeFileSync(
    join(dir, ACT1_FILE),
    [
      RESOURCES,
      `  ${HOUSEHOLD}:`,
      AVAIL_TRUE,
      `  ${OWNED}:`,
      AVAIL_TRUE,
      `  ${SSR}:`,
      AVAIL_TRUE,
      `  ${DHT}:`,
      AVAIL_FALSE,
      `  ${SHEM}:`,
      AVAIL_FALSE,
      '',
    ].join('\n')
  );
  writeFileSync(
    join(dir, ACT2_FILE),
    [
      RESOURCES,
      `  ${HOUSEHOLD}:`,
      AVAIL_TRUE,
      `  ${OWNED}:`,
      AVAIL_FALSE,
      `  ${SSR}:`,
      AVAIL_TRUE,
      `  ${DHT}:`,
      AVAIL_TRUE,
      `  ${SHEM}:`,
      AVAIL_FALSE,
      '',
    ].join('\n')
  );
  writeFileSync(
    join(dir, LIVE_FILE),
    [
      RESOURCES,
      `  ${HOUSEHOLD}:`,
      AVAIL_TRUE,
      `  ${SHEM}:`,
      AVAIL_TRUE,
      `  ${ALPHA}:`,
      AVAIL_TRUE,
      '',
    ].join('\n')
  );
}

void describe('actFromTags', () => {
  void it('reads @act:<i|ii|iii|host> and nothing else', () => {
    assert.equal(actFromTags(['@e2e', ACT_I, '@requires:doorway']), 'i');
    assert.equal(actFromTags([ACT_II]), 'ii');
    assert.equal(actFromTags(['@act:iii']), 'iii');
    assert.equal(actFromTags(['@act:host']), 'host');
  });
  void it('returns null when the scenario declares no act', () => {
    assert.equal(actFromTags(['@e2e', '@dataplane']), null);
    assert.equal(actFromTags(['@act:iv', '@act:1']), null);
  });
});

void describe('actBaselineCaps', () => {
  let workDir: string;
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    writeManifestsDir(workDir);
    process.env[MANIFESTS_ENV] = workDir;
  });
  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
    delete process.env[MANIFESTS_ENV];
  });

  void it('resolves act i to the caps its own lane contract declares available', () => {
    assert.deepEqual(actBaselineCaps('i'), [HOUSEHOLD, OWNED, SSR]);
  });
  void it('resolves act ii from the neighbourhood contract — and it DROPS owned-substrate', () => {
    assert.deepEqual(actBaselineCaps('ii'), [DHT, HOUSEHOLD, SSR]);
  });
  void it('resolves act iii from the live cluster-state, with shem', () => {
    const caps = actBaselineCaps('iii');
    assert.ok(caps.includes(SHEM), 'act iii baseline must carry shem');
    assert.ok(caps.includes(ALPHA));
  });
  void it('resolves act host to NO caps — it needs no substrate at all', () => {
    assert.deepEqual(actBaselineCaps('host'), []);
  });
  void it('fails open (empty baseline) when the act contract is unreadable', () => {
    process.env[MANIFESTS_ENV] = join(workDir, 'nope');
    assert.deepEqual(actBaselineCaps('i'), []);
  });
});

void describe('unavailableRequiredCaps — act gating', () => {
  let workDir: string;
  const lane = (file: string): void => {
    process.env[PATH_ENV] = join(workDir, file);
  };
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    writeManifestsDir(workDir);
    process.env[MANIFESTS_ENV] = workDir;
    clearCapEnv();
    resetUndeclaredCapWarnings();
  });
  afterEach(() => {
    rmSync(workDir, { recursive: true, force: true });
    delete process.env[MANIFESTS_ENV];
    delete process.env[PATH_ENV];
    clearCapEnv();
    resetUndeclaredCapWarnings();
  });

  void it('runs an act i scenario on the act i lane', () => {
    lane(ACT1_FILE);
    assert.deepEqual(unavailableRequiredCaps([ACT_I, '@e2e']), []);
  });
  void it('HOLDS an act i scenario on the act ii lane, naming the act in the reason', () => {
    lane(ACT2_FILE);
    assert.deepEqual(unavailableRequiredCaps([ACT_I]), [`${OWNED} (act i baseline)`]);
  });
  void it('HOLDS an act ii scenario on the act i lane, naming the act in the reason', () => {
    lane(ACT1_FILE);
    assert.deepEqual(unavailableRequiredCaps([ACT_II]), [`${DHT} (act ii baseline)`]);
  });
  void it('HOLDS an act iii scenario on the act i lane (the commons is not the household)', () => {
    lane(ACT1_FILE);
    assert.deepEqual(unavailableRequiredCaps(['@act:iii']), [`${SHEM} (act iii baseline)`]);
  });
  void it('NEVER holds an act host scenario — on any lane', () => {
    for (const file of [ACT1_FILE, ACT2_FILE, LIVE_FILE]) {
      lane(file);
      assert.deepEqual(unavailableRequiredCaps(['@act:host', '@e2e']), [], `held on ${file}`);
    }
  });
  void it('still gates an explicit @requires ON TOP of a satisfied act baseline', () => {
    lane(ACT1_FILE);
    assert.deepEqual(unavailableRequiredCaps([ACT_I, `@requires:${DHT}`]), [DHT]);
  });
  void it('reports act-baseline holds before explicit @requires holds', () => {
    lane(ACT2_FILE);
    assert.deepEqual(unavailableRequiredCaps([ACT_I, `@requires:${SHEM}`]), [
      `${OWNED} (act i baseline)`,
      SHEM,
    ]);
  });
  void it('lets a per-cap env override rescue an act baseline cap', () => {
    lane(ACT2_FILE);
    process.env.ELOHIM_CAP_OWNED_SUBSTRATE_STATUS = AVAILABLE;
    assert.deepEqual(unavailableRequiredCaps([ACT_I]), []);
    delete process.env.ELOHIM_CAP_OWNED_SUBSTRATE_STATUS;
  });
  void it('fails open on a baseline cap the LANE does not declare (never invents a gate)', () => {
    // The live lane declares neither owned-substrate nor ssr-bundle; act i must not hold there.
    lane(LIVE_FILE);
    assert.deepEqual(unavailableRequiredCaps([ACT_I]), []);
  });
});

void describe('undeclared @requires caps WARN loudly, once per run', () => {
  let workDir: string;
  let warned: string[];
  const realWarn = console.warn;
  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    writeManifestsDir(workDir);
    process.env[MANIFESTS_ENV] = workDir;
    process.env[PATH_ENV] = join(workDir, ACT1_FILE);
    clearCapEnv();
    resetUndeclaredCapWarnings();
    warned = [];
    console.warn = (msg: unknown) => warned.push(String(msg));
  });
  afterEach(() => {
    console.warn = realWarn;
    rmSync(workDir, { recursive: true, force: true });
    delete process.env[MANIFESTS_ENV];
    delete process.env[PATH_ENV];
    clearCapEnv();
    resetUndeclaredCapWarnings();
  });

  void it('warns once for a cap the lane does not declare (it gates NOTHING)', () => {
    unavailableRequiredCaps([INVENTED]);
    unavailableRequiredCaps([INVENTED]);
    assert.equal(warned.length, 1);
    assert.match(warned[0], /UNDECLARED CAP/);
    assert.match(warned[0], /invented-cap/);
    assert.deepEqual(undeclaredCapsWarned(), ['invented-cap']);
  });
  void it('does not warn for a declared cap', () => {
    unavailableRequiredCaps([`@requires:${HOUSEHOLD}`, `@requires:${SHEM}`]);
    assert.deepEqual(warned, []);
    assert.deepEqual(undeclaredCapsWarned(), []);
  });
  void it('warns when a scenario declares two different acts (an authoring error)', () => {
    unavailableRequiredCaps([ACT_I, ACT_II]);
    assert.ok(
      warned.some(w => w.includes('MULTIPLE ACT TAGS')),
      `expected a multiple-act warning, got ${JSON.stringify(warned)}`
    );
  });
  void it('resetSubstrateSkips clears the once-per-run warning ledger', () => {
    unavailableRequiredCaps([INVENTED]);
    resetSubstrateSkips();
    unavailableRequiredCaps([INVENTED]);
    assert.equal(warned.length, 2);
  });
});

void describe('the REAL act contracts (guards the lane files themselves)', () => {
  beforeEach(() => {
    delete process.env[MANIFESTS_ENV];
    clearCapEnv();
  });

  void it('act i baseline carries every cap LAYERS.md names for the household', () => {
    const caps = new Set(actBaselineCaps('i'));
    for (const cap of [
      HOUSEHOLD,
      'doorway',
      'doorway-pair',
      'multi-node',
      'seeded-content',
      'seeded-humans',
      'mongo-archive',
      OWNED,
      'epr-cli',
      SSR,
    ]) {
      assert.ok(caps.has(cap), `act i baseline is missing ${cap}`);
    }
  });
  void it('act ii baseline adds the fleet caps and DROPS owned-substrate', () => {
    const caps = new Set(actBaselineCaps('ii'));
    for (const cap of [ALPHA, DHT, 'per-human-conductor', 'apex-dns', 'tls', 'deploy-churn']) {
      assert.ok(caps.has(cap), `act ii baseline is missing ${cap}`);
    }
    assert.ok(!caps.has(OWNED), 'act ii must NOT own its substrate');
  });
  void it('act iii baseline carries shem', () => {
    assert.ok(actBaselineCaps('iii').includes(SHEM));
  });
});

/** The destructive gate — ONE answer for every kill/restart/write step, read from the declared
 * `owned-substrate` cap with the env var as operator override. Never fail-open. */
void describe('destructiveAllowed', () => {
  let workDir: string;

  function writeOwnedSubstrate(available: boolean): string {
    const path = join(workDir, CLUSTER_STATE_FILE);
    writeFileSync(
      path,
      [
        RESOURCES,
        `  ${OWNED_SUBSTRATE_CAP}:`,
        '    role: the lane OWNS its substrate',
        available ? AVAIL_TRUE : AVAIL_FALSE,
        `  ${HOUSEHOLD}:`,
        AVAIL_TRUE,
        '',
      ].join('\n')
    );
    return path;
  }

  beforeEach(() => {
    workDir = mkdtempSync(join(tmpdir(), TMP_PREFIX));
    clearCapEnv();
    delete process.env[PATH_ENV];
  });

  afterEach(() => {
    clearCapEnv();
    delete process.env[PATH_ENV];
    rmSync(workDir, { recursive: true, force: true });
  });

  void it('is ON when the active lane declares owned-substrate available (no env needed)', () => {
    process.env[PATH_ENV] = writeOwnedSubstrate(true);
    assert.equal(destructiveAllowed(), true);
  });

  void it('is OFF when the active lane declares owned-substrate unavailable', () => {
    process.env[PATH_ENV] = writeOwnedSubstrate(false);
    assert.equal(destructiveAllowed(), false);
  });

  void it('A2O_ALLOW_DESTRUCTIVE=1 forces ON even on a lane that declares the cap false', () => {
    process.env[PATH_ENV] = writeOwnedSubstrate(false);
    process.env[DESTRUCTIVE_ENV] = '1';
    assert.equal(destructiveAllowed(), true);
  });

  void it('A2O_ALLOW_DESTRUCTIVE=0 forces OFF even on a lane that owns its substrate', () => {
    process.env[PATH_ENV] = writeOwnedSubstrate(true);
    process.env[DESTRUCTIVE_ENV] = '0';
    assert.equal(destructiveAllowed(), false);
  });

  void it('the per-cap runtime override channel wins over the durable declaration', () => {
    process.env[PATH_ENV] = writeOwnedSubstrate(false);
    process.env['ELOHIM_CAP_OWNED_SUBSTRATE_STATUS'] = AVAILABLE;
    assert.equal(destructiveAllowed(), true);
  });

  void it('never fails open: an unreadable cluster-state holds destructive steps', () => {
    process.env[PATH_ENV] = join(workDir, 'does-not-exist.yaml');
    assert.equal(destructiveAllowed(), false);
  });

  void it('never fails open: a lane whose cluster-state is silent on the cap holds them', () => {
    const path = join(workDir, CLUSTER_STATE_FILE);
    writeFileSync(path, [RESOURCES, `  ${HOUSEHOLD}:`, AVAIL_TRUE, ''].join('\n'));
    process.env[PATH_ENV] = path;
    assert.equal(destructiveAllowed(), false);
  });
});
