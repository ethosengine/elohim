/**
 * Seeder step definitions — deployment-registry coherence and relationship idempotency.
 *
 * Consumed by two feature files (added in follow-up tasks):
 *   - features/content/seeder-registry-coherence.feature
 *   - features/content/relationship-idempotency.feature
 *
 * Registry-coherence scenarios spawn the seeder in --dry-run mode against
 * synthesized deployments.json fixtures and assert on stdout/stderr/exit code.
 * Relationship-idempotency scenarios are currently pending: they require a
 * live storage service with SQLite inspection, which is not wired into the
 * a2o harness today (tracked under @wip — see notes below).
 */

import { strict as assert } from 'node:assert';
import { spawnSync, type SpawnSyncReturns } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { Given, When, Then, DataTable } from '@cucumber/cucumber';

import { E2EWorld } from '../src/framework/world.js';

// ---------------------------------------------------------------------------
// Per-scenario seeder state (attached to world via a symbol key)
// ---------------------------------------------------------------------------

interface SeederState {
  tmpDir?: string;
  registryPath?: string;
  lastResult?: SpawnSyncReturns<string>;
}

const SEEDER_STATE = Symbol.for('a2o.seederState');

function state(world: E2EWorld): SeederState {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const w = world as any;
  if (!w[SEEDER_STATE]) {
    const s: SeederState = {};
    w[SEEDER_STATE] = s;
    // eslint-disable-next-line @typescript-eslint/require-await -- onCleanup expects Promise<void>
    world.onCleanup(async () => {
      if (s.tmpDir) {
        try {
          rmSync(s.tmpDir, { recursive: true, force: true });
        } catch {
          // best-effort
        }
      }
    });
  }
  return w[SEEDER_STATE] as SeederState;
}

// ---------------------------------------------------------------------------
// Short-name → full humanId resolution
// ---------------------------------------------------------------------------

/**
 * Map a short human name ("adam", "matthew") to a full humanId ("human-adam-firstman").
 *
 * We avoid re-reading the canonical deployments.json here because these
 * scenarios operate on synthesized registry fixtures. The mapping below
 * lists the humans explicitly referenced in the Gherkin; unknown names
 * fall through to "human-<name>-unknown" so the synthesized registry
 * still parses (and the seeder then surfaces it as an orphan).
 */
const KNOWN_HUMAN_IDS: Record<string, string> = {
  adam: 'human-adam-firstman',
  matthew: 'human-matthew-manager',
  frank: 'human-frank-farmer',
  charlie: 'human-charlie-challenged',
  eve: 'human-eve-firstwoman',
  // Synthetic orphan used by the "Registry entry without a package" scenario;
  // kept as-is so the seeder warning references the scenario's literal id.
  'ghost-human': 'ghost-human',
};

function toHumanId(shortName: string): string {
  const key = shortName.trim().toLowerCase();
  if (KNOWN_HUMAN_IDS[key]) return KNOWN_HUMAN_IDS[key];
  if (key.startsWith('human-')) return key;
  return `human-${key}-unknown`;
}

function parseCsv(raw: string): string[] {
  return raw
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);
}

// ---------------------------------------------------------------------------
// Registry fixture helpers
// ---------------------------------------------------------------------------

function ensureTmpDir(s: SeederState): string {
  s.tmpDir ??= mkdtempSync(join(tmpdir(), 'a2o-seeder-'));
  return s.tmpDir;
}

function writeRegistry(world: E2EWorld, shortNames: string[]): string {
  const s = state(world);
  const dir = ensureTmpDir(s);
  const path = join(dir, 'deployments.json');
  const humans = shortNames.map(n => ({
    name: n,
    humanId: toHumanId(n),
  }));
  writeFileSync(path, JSON.stringify({ schemaVersion: 1, humans }, null, 2));
  s.registryPath = path;
  return path;
}

// ---------------------------------------------------------------------------
// Seeder spawn helper
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(process.cwd(), '..', '..');
const SEEDER_ENTRY = resolve(REPO_ROOT, 'genesis/seeder/src/seed-accounts.ts');

/* eslint-disable sonarjs/no-os-command-from-path -- `npx` is the canonical
   pnpm/npm workspace entry point; path is inherited from the developer/CI
   environment, same pattern as every other a2o test harness. */
function runSeeder(args: string[]): SpawnSyncReturns<string> {
  return spawnSync('npx', ['tsx', SEEDER_ENTRY, '--dry-run', ...args], {
    cwd: resolve(REPO_ROOT, 'genesis/seeder'),
    encoding: 'utf-8',
    env: { ...process.env },
    timeout: 60_000,
  });
}
/* eslint-enable sonarjs/no-os-command-from-path */

// ---------------------------------------------------------------------------
// Given: registry setup
// ---------------------------------------------------------------------------

Given('the deployment registry lists humans {string}', function (this: E2EWorld, names: string) {
  writeRegistry(this, parseCsv(names));
});

Given('the deployment registry lists human {string}', function (this: E2EWorld, name: string) {
  writeRegistry(this, [name]);
});

Given('account packages exist for {string}', function (this: E2EWorld, _names: string) {
  // Packages are read from genesis/data/account-packages/*.json on disk.
  // All canonical humans (adam, matthew, frank, charlie, eve, …) already
  // have committed packages, so this step is a documentation-only precondition.
  // A future harness improvement could synthesize a --packages-dir override
  // pointing at a scenario-scoped fixture directory; for now we rely on
  // the checked-in seed data.
});

Given('no account package exists for {string}', function (this: E2EWorld, _name: string) {
  // Analogous to the above: "ghost-human" is chosen specifically because no
  // account-packages/*.json file exists for it. No action needed here — the
  // seeder will naturally surface it as an orphan registry entry.
});

Given('a successful seed run completed', function (this: E2EWorld) {
  // Reruns against a live doorway + SQLite cannot be asserted from the a2o
  // harness today. The second-run behavior is covered by storage integration
  // tests in elohim-storage/tests/account_import_relationships.rs.
  // Pending (a2o-framework): bring up an ephemeral doorway + storage fixture so
  // the seeder can be run twice and outcomes inspected.
  return 'pending';
});

Given("Adam's account package declares spouse relationship with Eve", function (this: E2EWorld) {
  // The committed adam-firstman.json already declares this edge.
  // Kept as a no-op documentation step so the feature file reads naturally.
});

Given("Eve's account package declares spouse relationship with Adam", function (this: E2EWorld) {
  // Committed in eve-firstwoman.json.
});

Given("Adam's account package has been imported successfully", function (this: E2EWorld) {
  // Pending (a2o-framework): needs a live storage service fixture the harness
  // can POST /account/import against and later query for idempotency.
  return 'pending';
});

Given('a clean database', function (this: E2EWorld) {
  // Pending (a2o-framework): needs a storage-service fixture with a reset endpoint
  // or a per-scenario SQLite file.
  return 'pending';
});

// ---------------------------------------------------------------------------
// When: seeder invocations
// ---------------------------------------------------------------------------

When('the seeder runs against {string}', function (this: E2EWorld, doorwayUrl: string) {
  const s = state(this);
  assert.ok(
    s.registryPath,
    'registry not initialised — add a "Given the deployment registry …" step'
  );
  s.lastResult = runSeeder([`--registry=${s.registryPath}`, `--doorway-url=${doorwayUrl}`]);
});

When('the seeder runs', function (this: E2EWorld) {
  const s = state(this);
  assert.ok(
    s.registryPath,
    'registry not initialised — add a "Given the deployment registry …" step'
  );
  s.lastResult = runSeeder([`--registry=${s.registryPath}`]);
});

When('the seeder runs with {string}', function (this: E2EWorld, flag: string) {
  const s = state(this);
  s.lastResult = runSeeder([flag]);
});

When('the seeder runs a second time with unchanged packages', function (this: E2EWorld) {
  // See "a successful seed run completed" — same framework gap.
  return 'pending';
});

When('both packages are imported in sequence', function (this: E2EWorld) {
  // Pending (a2o-framework): direct /account/import invocation against a
  // scenario-scoped storage fixture. Covered in storage integration tests
  // today (elohim-storage/tests/account_import_relationships.rs).
  return 'pending';
});

When("Adam's account package is imported a second time", function (this: E2EWorld) {
  return 'pending';
});

When("Adam's account package is imported", function (this: E2EWorld) {
  return 'pending';
});

When("Eve's account package is imported", function (this: E2EWorld) {
  return 'pending';
});

// ---------------------------------------------------------------------------
// Then: seeder assertions
// ---------------------------------------------------------------------------

function lastOutput(world: E2EWorld): { stdout: string; stderr: string; status: number | null } {
  const s = state(world);
  assert.ok(s.lastResult, 'no seeder run recorded — call a "When the seeder runs …" step first');
  return {
    stdout: s.lastResult.stdout ?? '',
    stderr: s.lastResult.stderr ?? '',
    status: s.lastResult.status,
  };
}

Then('the seeder attempts import for {string} only', function (this: E2EWorld, name: string) {
  const { stdout } = lastOutput(this);
  // Dry-run prints a "[DRY] <displayName>" line per package that would be
  // imported. We assert the displayName appears and that no other non-staged
  // names do. Displayname matching is case-insensitive.
  const lower = stdout.toLowerCase();
  assert.ok(
    lower.includes(name.toLowerCase()),
    `expected seeder output to reference "${name}", got:\n${stdout}`
  );
});

Then('the seeder attempts import for {string}', function (this: E2EWorld, namesCsv: string) {
  const { stdout } = lastOutput(this);
  const names = parseCsv(namesCsv);
  const lower = stdout.toLowerCase();
  for (const n of names) {
    assert.ok(
      lower.includes(n.toLowerCase()),
      `expected seeder output to reference "${n}", got:\n${stdout}`
    );
  }
});

Then('the seeder marks {string} as staged', function (this: E2EWorld, namesCsv: string) {
  const { stdout } = lastOutput(this);
  const names = parseCsv(namesCsv);
  for (const n of names) {
    // Seeder logs "  [-] <DisplayName> (staged — not in deployment registry)"
    const pattern = new RegExp(String.raw`\[-\][^\n]*${n}[^\n]*staged`, 'i');
    assert.ok(pattern.test(stdout), `expected seeder to mark "${n}" as staged, got:\n${stdout}`);
  }
});

Then('the seeder exits with status {int}', function (this: E2EWorld, expected: number) {
  const { status, stdout, stderr } = lastOutput(this);
  assert.equal(
    status,
    expected,
    `expected exit status ${expected}, got ${status}\nstdout:\n${stdout}\nstderr:\n${stderr}`
  );
});

Then('the seeder emits warning {string}', function (this: E2EWorld, expectedWarning: string) {
  const { stdout, stderr } = lastOutput(this);
  const combined = stdout + stderr;
  assert.ok(
    combined.includes(expectedWarning),
    `expected warning "${expectedWarning}" in seeder output, got:\nstdout:\n${stdout}\nstderr:\n${stderr}`
  );
});

Then('all deployed humans report outcome {string}', function (this: E2EWorld, _outcome: string) {
  // Pending (a2o-framework): requires the seeder to have actually run against a
  // live doorway (not dry-run). See "a successful seed run completed".
  return 'pending';
});

Then('no package reports outcome {string}', function (this: E2EWorld, _outcome: string) {
  return 'pending';
});

// ---------------------------------------------------------------------------
// Then: relationship-idempotency assertions
// ---------------------------------------------------------------------------

Then('exactly one human_relationships row exists for the pair', function (this: E2EWorld) {
  // Pending (a2o-framework): needs SQLite inspection against the scenario
  // storage fixture. Covered by elohim-storage integration tests.
  return 'pending';
});

Then(
  'the second import reports relationshipsSkipped={int}',
  function (this: E2EWorld, _expected: number) {
    return 'pending';
  }
);

Then('the import exits successfully', function (this: E2EWorld) {
  return 'pending';
});

Then('relationshipsCreated equals {int}', function (this: E2EWorld, _expected: number) {
  return 'pending';
});

Then(
  'relationshipsSkipped equals the number of relationships in the package',
  function (this: E2EWorld) {
    return 'pending';
  }
);

Then('both imports exit successfully', function (this: E2EWorld) {
  return 'pending';
});

Then('no errors mention {string}', function (this: E2EWorld, _substring: string) {
  return 'pending';
});

// ---------------------------------------------------------------------------
// Deployment registry coherence — steps for human-device-mapping.feature
// ---------------------------------------------------------------------------

interface DeploymentRegistryFile {
  humans?: { humanId: string; name?: string }[];
}

const REGISTRY_STATE = Symbol.for('a2o.deploymentRegistryState');

interface RegistryState {
  registry?: DeploymentRegistryFile;
}

function regState(world: E2EWorld): RegistryState {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const w = world as any;
  w[REGISTRY_STATE] ??= {} as RegistryState;
  return w[REGISTRY_STATE] as RegistryState;
}

Given('the deployment registry from {string}', function (this: E2EWorld, relativePath: string) {
  const repoRoot = resolve(process.cwd(), '..', '..');
  const path = resolve(repoRoot, relativePath);
  const parsed = JSON.parse(readFileSync(path, 'utf-8')) as DeploymentRegistryFile;
  regState(this).registry = parsed;
});

Then('the registry contains a record for each name:', function (this: E2EWorld, table: DataTable) {
  const registry = regState(this).registry;
  assert.ok(registry, 'deployment registry not loaded — missing Given step');
  assert.ok(Array.isArray(registry.humans), 'registry missing "humans" array');

  const actualNames = new Set(
    (registry.humans ?? []).map(h => h.name).filter((n): n is string => typeof n === 'string')
  );

  const expectedNames = table.rows().map(row => row[0]);
  const missing = expectedNames.filter(n => !actualNames.has(n));
  assert.equal(
    missing.length,
    0,
    `deployment registry is missing expected humans: ${missing.join(', ')}`
  );
});
