/**
 * Release manifest schema + packager — the `happ-lineage` artifactClass
 * (epic Task 3).
 *
 * Part 1 (schema-only tests) validates manifest objects directly against
 * `elohim/rakia/schemas/v1/release-manifest.schema.json` with the same Ajv
 * setup `epr-release-package.ts --validate` uses (dist/2020.js, strict:
 * false, allErrors: true), without importing the packager module.
 *
 * Part 2 (packager tests) drives the real `epr-release-package.ts` CLI as a
 * subprocess (via `tsx`, `--no-put` so nothing touches the network) rather
 * than importing it — the module exports nothing beyond content-addressing
 * helpers, and reaching into its internals would couple this spec to
 * implementation shape rather than the CLI contract these flags are part of.
 */

import { strict as assert } from 'node:assert';
import { execFile, execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import * as AjvNs from 'ajv/dist/2020.js';

import type { AddressInfo } from 'node:net';

interface AjvOptions {
  strict?: boolean;
  allErrors?: boolean;
}

const AjvCtor: new (opts: AjvOptions) => AjvNs.default =
  (AjvNs as unknown as { default: new (opts: AjvOptions) => AjvNs.default }).default ??
  (AjvNs as unknown as new (opts: AjvOptions) => AjvNs.default);

const SCHEMA_PATH = fileURLToPath(
  new URL('../../../../elohim/rakia/schemas/v1/release-manifest.schema.json', import.meta.url)
);
const REPO_ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
const TSX_BIN = path.join(REPO_ROOT, 'node_modules/.bin/tsx');
const PACKAGER_SCRIPT = path.join(REPO_ROOT, 'genesis/a2o/scripts/epr-release-package.ts');
const MIGRATE_FROM_HASH = 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH';
const PATH_COMMITMENT_CID = 'uhCEk' + 'A'.repeat(48);

// Flags + artifactClass names repeated across many CLI invocations below —
// named once each so `sonarjs/no-duplicate-string` stays quiet as cases are
// added (same reasoning as `NOTARIZED_EMIT_ARGS` further down).
const FLAG_ARTIFACT = '--artifact';
const FLAG_ARTIFACT_CLASS = '--artifact-class';
const FLAG_APPLIES_TO = '--applies-to';
const FLAG_SOAK_SECS = '--soak-secs';
const FLAG_ATTESTATION_THRESHOLD = '--attestation-threshold';
const ARTIFACT_CLASS_HAPP_LINEAGE = 'happ-lineage';
const ARTIFACT_CLASS_COORDINATOR_BUNDLE = 'coordinator-bundle';

type JsonObject = Record<string, unknown>;

function loadSchema(): JsonObject {
  return JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')) as JsonObject;
}

function validateManifest(manifest: unknown): { ok: boolean; errors: string[] } {
  const ajv = new AjvCtor({ strict: false, allErrors: true });
  const validate = ajv.compile(loadSchema());
  const ok = validate(manifest);
  const errors = (validate.errors ?? []).map(
    error => `${error.instancePath || '/'} ${error.message ?? 'invalid'}`
  );
  return { ok, errors };
}

/** A minimal, otherwise-valid manifest — every required field present. */
function baseManifest(overrides: JsonObject = {}): JsonObject {
  return {
    kind: 'release-manifest',
    channelId: 'runtime:happ:elohim:commons',
    artifactClass: ARTIFACT_CLASS_COORDINATOR_BUNDLE,
    artifacts: [
      {
        blobCid: 'bafkrei' + 'a'.repeat(52),
        bytes: 100,
        sha256: 'a'.repeat(64),
        filename: 'artifact.wasm',
      },
    ],
    appliesTo: {
      roles: {
        node_registry: {
          dnaHash: 'uhC0k' + 'A'.repeat(48),
          coordinatorWasmHashes: [],
        },
      },
    },
    envelope: {
      wireEpochs: [1],
      lineageParentCid: null,
      additiveOnly: true,
    },
    provenance: {
      builderAgent: 'did:elohim:test',
      toolchain: 'rustc 1.90.0 (stable)',
      buildInfo: {},
      builtFrom: { gitCommit: 'abc1234' },
    },
    declaredReach: 'commons',
    adoptionDiscipline: {
      soakSecs: 900,
      attestationThreshold: 2,
      canaryOrder: [],
    },
    ...overrides,
  };
}

void describe('release-manifest schema — happ-lineage artifactClass', () => {
  void it('accepts a coordinator-bundle manifest with no migrateFrom/lineage/path', () => {
    const result = validateManifest(baseManifest());
    assert.equal(result.ok, true, result.errors.join('\n'));
  });

  void it('happ-lineage requires migrateFrom, lineage and path', () => {
    const m = baseManifest({ artifactClass: ARTIFACT_CLASS_HAPP_LINEAGE });
    const before = validateManifest(m);
    assert.equal(before.ok, false, 'missing adoptionDiscipline.path must fail validation');
    const roles = (m['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    nodeRegistry['migrateFrom'] = 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH';
    nodeRegistry['lineage'] = [nodeRegistry['migrateFrom']];
    (m['adoptionDiscipline'] as JsonObject)['path'] = { commitmentCid: 'uhCEk' + 'A'.repeat(48) };
    const result = validateManifest(m);
    assert.equal(result.ok, true, result.errors.join('\n'));
  });

  void it('rejects an artifactClass the enum does not name', () => {
    const result = validateManifest(baseManifest({ artifactClass: 'not-a-real-class' }));
    assert.equal(result.ok, false);
  });

  void it('accepts roleBinding.migrateFrom and .lineage as additive fields on any class', () => {
    const m = baseManifest();
    const roles = (m['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    nodeRegistry['migrateFrom'] = 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH';
    nodeRegistry['lineage'] = [nodeRegistry['migrateFrom']];
    const result = validateManifest(m);
    assert.equal(result.ok, true, result.errors.join('\n'));
  });

  void it('rejects a commitmentCid that does not match the uhCEk pattern', () => {
    const m = baseManifest({ artifactClass: ARTIFACT_CLASS_HAPP_LINEAGE });
    const roles = (m['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    nodeRegistry['migrateFrom'] = 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH';
    nodeRegistry['lineage'] = [nodeRegistry['migrateFrom']];
    (m['adoptionDiscipline'] as JsonObject)['path'] = { commitmentCid: 'not-a-valid-cid' };
    const result = validateManifest(m);
    assert.equal(result.ok, false);
  });

  void it('rejects an empty roleBinding.constitutionRoot', () => {
    // `minLength: 1` on purpose: an empty root is not "undeclared", it is a
    // root nothing can equal, which would refuse every crossing under it.
    const m = baseManifest();
    const roles = (m['appliesTo'] as JsonObject)['roles'] as JsonObject;
    (roles['node_registry'] as JsonObject)['constitutionRoot'] = '';
    assert.equal(validateManifest(m).ok, false);
  });

  void it('rejects roleBinding.lineage with zero items', () => {
    const m = baseManifest();
    const roles = (m['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    nodeRegistry['migrateFrom'] = 'uhC0kyvKwO2J5u3mf52tjASWe0ryhdpNYalrSeMGJODF3OpUxyeoH';
    nodeRegistry['lineage'] = [];
    const result = validateManifest(m);
    assert.equal(result.ok, false);
  });
});

// ---------------------------------------------------------------------------
// Part 2 — the packager CLI (--migrate-from / --lineage / --path-commitment)
// ---------------------------------------------------------------------------

function tempArtifact(): string {
  const dir = mkdtempSync(path.join(tmpdir(), 'epr-release-package-spec-'));
  const file = path.join(dir, 'artifact.bin');
  writeFileSync(file, 'happ-lineage-fixture-bytes');
  return file;
}

/** Shells out to the real CLI so `parseArgs` + the manifest literal are under test, not a mock. */
function runPackager(args: string[]): string {
  return execFileSync(TSX_BIN, [PACKAGER_SCRIPT, ...args], {
    encoding: 'utf8',
    cwd: REPO_ROOT,
  });
}

const execFileAsync = promisify(execFile);

/**
 * The async twin of `runPackager`, for the ONE case here that needs it:
 * `--applies-to-from` makes the CLI fetch a fixture server hosted in THIS
 * test process. `execFileSync` blocks this process's event loop until the
 * child exits, so the fixture server's request handler — which runs on that
 * same loop — would never fire and the child would hang until its own fetch
 * timeout. Awaiting `execFile` keeps the loop free to answer the child.
 */
async function runPackagerAsync(args: string[]): Promise<string> {
  const { stdout } = await execFileAsync(TSX_BIN, [PACKAGER_SCRIPT, ...args], {
    encoding: 'utf8',
    cwd: REPO_ROOT,
  });
  return stdout;
}

function appliesToLiteral(): string {
  return JSON.stringify({
    roles: {
      node_registry: { dnaHash: 'uhC0k' + 'B'.repeat(48), coordinatorWasmHashes: [] },
    },
  });
}

function happLineageArgs(artifactPath: string, extra: string[] = []): string[] {
  return [
    FLAG_ARTIFACT,
    artifactPath,
    FLAG_ARTIFACT_CLASS,
    ARTIFACT_CLASS_HAPP_LINEAGE,
    FLAG_APPLIES_TO,
    appliesToLiteral(),
    '--migrate-from',
    `node_registry=${MIGRATE_FROM_HASH}`,
    '--lineage',
    MIGRATE_FROM_HASH,
    FLAG_SOAK_SECS,
    '900',
    FLAG_ATTESTATION_THRESHOLD,
    '2',
    '--no-put',
    ...extra,
  ];
}

const CONSTITUTION_ROOT = 'bafyLineageConstitutionRootForThePackagerSpec';

/**
 * The notarized-path + emit flags every packager case here shares. Hoisted so
 * the three call sites name one array rather than repeating the same four
 * literals — which is also what keeps `sonarjs/no-duplicate-string` quiet as
 * cases are added.
 */
const NOTARIZED_EMIT_ARGS = ['--path-commitment', PATH_COMMITMENT_CID, '--compact', '--strict'];

void describe('epr-release-package.ts CLI — happ-lineage flags', () => {
  void it('assembles a manifest with migrateFrom/lineage/path that passes schema + --strict', () => {
    const stdout = runPackager(happLineageArgs(tempArtifact(), NOTARIZED_EMIT_ARGS));
    const manifest = JSON.parse(stdout) as JsonObject;
    const result = validateManifest(manifest);
    assert.equal(result.ok, true, result.errors.join('\n'));

    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const nodeRegistry = roles['node_registry'] as JsonObject;
    assert.equal(nodeRegistry['migrateFrom'], MIGRATE_FROM_HASH);
    assert.deepEqual(nodeRegistry['lineage'], [MIGRATE_FROM_HASH]);
    assert.deepEqual((manifest['adoptionDiscipline'] as JsonObject)['path'], {
      commitmentCid: PATH_COMMITMENT_CID,
    });
  });

  void it('--constitution-root lands on every crossing role and passes schema + --strict', () => {
    const stdout = runPackager(
      happLineageArgs(tempArtifact(), [
        ...NOTARIZED_EMIT_ARGS,
        '--constitution-root',
        CONSTITUTION_ROOT,
      ])
    );
    const manifest = JSON.parse(stdout) as JsonObject;
    const result = validateManifest(manifest);
    assert.equal(result.ok, true, result.errors.join('\n'));

    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    assert.equal((roles['node_registry'] as JsonObject)['constitutionRoot'], CONSTITUTION_ROOT);
  });

  void it('omits constitutionRoot entirely when the flag is not given', () => {
    // Undeclared is a real state, not an empty string: with no installed root
    // either, verify_path skips the root check and says `root: undeclared`.
    // An empty-string field would be a root nothing can equal.
    const stdout = runPackager(happLineageArgs(tempArtifact(), NOTARIZED_EMIT_ARGS));
    const manifest = JSON.parse(stdout) as JsonObject;
    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    assert.equal('constitutionRoot' in (roles['node_registry'] as JsonObject), false);
  });

  void it('refuses to package a happ-lineage release without --path-commitment', () => {
    assert.throws(
      () => runPackager(happLineageArgs(tempArtifact())),
      (error: unknown) => {
        const status = (error as { status?: number }).status;
        assert.equal(status, 64, `expected a usage-error exit (64), got ${String(status)}`);
        return true;
      }
    );
  });

  void it('refuses --migrate-from for a role --applies-to did not resolve', () => {
    assert.throws(
      () =>
        runPackager([
          FLAG_ARTIFACT,
          tempArtifact(),
          FLAG_ARTIFACT_CLASS,
          ARTIFACT_CLASS_HAPP_LINEAGE,
          FLAG_APPLIES_TO,
          appliesToLiteral(),
          '--migrate-from',
          `mishpat=${MIGRATE_FROM_HASH}`,
          '--path-commitment',
          PATH_COMMITMENT_CID,
          FLAG_SOAK_SECS,
          '900',
          FLAG_ATTESTATION_THRESHOLD,
          '2',
          '--no-put',
        ]),
      (error: unknown) => {
        const status = (error as { status?: number }).status;
        assert.equal(status, 64, `expected a usage-error exit (64), got ${String(status)}`);
        return true;
      }
    );
  });

  void it('a coordinator-bundle manifest never gains a path field', () => {
    const stdout = runPackager([
      FLAG_ARTIFACT,
      tempArtifact(),
      FLAG_ARTIFACT_CLASS,
      ARTIFACT_CLASS_COORDINATOR_BUNDLE,
      FLAG_APPLIES_TO,
      appliesToLiteral(),
      FLAG_SOAK_SECS,
      '900',
      FLAG_ATTESTATION_THRESHOLD,
      '2',
      '--no-put',
      '--compact',
      '--strict',
    ]);
    const manifest = JSON.parse(stdout) as JsonObject;
    const result = validateManifest(manifest);
    assert.equal(result.ok, true, result.errors.join('\n'));
    assert.equal((manifest['adoptionDiscipline'] as JsonObject)['path'], undefined);
  });
});

// ---------------------------------------------------------------------------
// Part 3 — `--applies-to-from` reads the AUTHORING cell of a crossed role
// (rung5-workspace Task 1; mirrors
// `InstalledReality::from_happ_passport` in
// elohim/elohim-storage/src/services/release_adoption/verify.rs)
// ---------------------------------------------------------------------------

const UNCROSSED_DNA_HASH = 'uhC0k' + 'E'.repeat(48);
const UNCROSSED_ZOME_HASH = 'uhCok' + 'F'.repeat(48);
const CROSSED_BASE_DNA_HASH = 'uhC0k' + 'D'.repeat(48);
const CROSSED_BASE_ZOME_HASH = 'uhCok' + 'G'.repeat(48);
const CROSSED_AUTHORING_DNA_HASH = 'uhC0k' + 'C'.repeat(48);
const CROSSED_AUTHORING_ZOME_HASH = 'uhCok' + 'H'.repeat(48);
const SUNSET_AUTHORING_DNA_HASH = 'uhC0k' + 'J'.repeat(48);

/**
 * A `GET /version` passport carrying three roles: `node_registry` never
 * crossed (no `lineage` view at all — the pre-Task-8 shape); `imagodei`
 * crossed with an OPEN window (`authoringAppId !== readingAppId`, `closed:
 * false`); `mishpat` crossed by SUNSET alone (`closed: true` while
 * `authoringAppId === readingAppId`, exercising the `closed` arm of the
 * crossed predicate independent of the app-id compare).
 */
function lineagePassportFixture(): JsonObject {
  return {
    passport: {
      happ: {
        appId: 'elohim',
        roles: [
          {
            role: 'node_registry',
            dnaHash: UNCROSSED_DNA_HASH,
            coordinatorWasmHashes: { node_registry_coordinator: UNCROSSED_ZOME_HASH },
          },
          {
            role: 'imagodei',
            dnaHash: CROSSED_BASE_DNA_HASH,
            coordinatorWasmHashes: { imagodei_coordinator: CROSSED_BASE_ZOME_HASH },
            lineage: {
              readingAppId: 'elohim',
              authoringAppId: 'elohim@lineage-2026-09-05',
              readingDnaHash: CROSSED_BASE_DNA_HASH,
              authoringDnaHash: CROSSED_AUTHORING_DNA_HASH,
              authoringCoordinatorWasmHashes: { imagodei_coordinator: CROSSED_AUTHORING_ZOME_HASH },
              closed: false,
            },
          },
          {
            role: 'mishpat',
            dnaHash: CROSSED_BASE_DNA_HASH,
            coordinatorWasmHashes: {},
            lineage: {
              readingAppId: 'elohim',
              authoringAppId: 'elohim',
              readingDnaHash: CROSSED_BASE_DNA_HASH,
              authoringDnaHash: SUNSET_AUTHORING_DNA_HASH,
              authoringCoordinatorWasmHashes: {},
              closed: true,
            },
          },
        ],
      },
    },
  };
}

/** Serves one fixed JSON body at `GET /version` on an ephemeral local port. */
async function servePassport(
  body: JsonObject
): Promise<{ url: string; close: () => Promise<void> }> {
  return new Promise((resolve, reject) => {
    const server = createServer((req, res) => {
      if (req.url === '/version') {
        res.writeHead(200, { 'content-type': 'application/json' });
        res.end(JSON.stringify(body));
        return;
      }
      res.writeHead(404);
      res.end();
    });
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address() as AddressInfo | null;
      if (address === null) {
        reject(new Error('fixture passport server did not bind to a port'));
        return;
      }
      resolve({
        url: `http://127.0.0.1:${address.port}`,
        close: async () => new Promise<void>(res2 => server.close(() => res2())),
      });
    });
  });
}

async function packageFromPassport(passport: JsonObject): Promise<JsonObject> {
  const { url, close } = await servePassport(passport);
  try {
    const stdout = await runPackagerAsync([
      FLAG_ARTIFACT,
      tempArtifact(),
      FLAG_ARTIFACT_CLASS,
      ARTIFACT_CLASS_COORDINATOR_BUNDLE,
      '--applies-to-from',
      url,
      FLAG_SOAK_SECS,
      '900',
      FLAG_ATTESTATION_THRESHOLD,
      '2',
      '--no-put',
      '--compact',
      '--strict',
    ]);
    return JSON.parse(stdout) as JsonObject;
  } finally {
    await close();
  }
}

void describe('epr-release-package.ts CLI — --applies-to-from lineage-aware derivation', () => {
  void it('an un-crossed role reads base values, byte-identical to the pre-lineage shape', async () => {
    const manifest = await packageFromPassport(lineagePassportFixture());
    const result = validateManifest(manifest);
    assert.equal(result.ok, true, result.errors.join('\n'));
    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    assert.equal(
      JSON.stringify(roles['node_registry']),
      JSON.stringify({
        dnaHash: UNCROSSED_DNA_HASH,
        coordinatorWasmHashes: [UNCROSSED_ZOME_HASH],
        coordinatorZomes: { node_registry_coordinator: UNCROSSED_ZOME_HASH },
      })
    );
  });

  void it('an OPEN-window crossed role reads the authoring cell, not the base cell', async () => {
    const manifest = await packageFromPassport(lineagePassportFixture());
    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    const imagodei = roles['imagodei'] as JsonObject;
    assert.equal(imagodei['dnaHash'], CROSSED_AUTHORING_DNA_HASH);
    assert.notEqual(imagodei['dnaHash'], CROSSED_BASE_DNA_HASH);
    assert.deepEqual(imagodei['coordinatorWasmHashes'], [CROSSED_AUTHORING_ZOME_HASH]);
  });

  void it('a CLOSED-after-sunset role reads the authoring cell even with a matching app id', async () => {
    return packageFromPassport(lineagePassportFixture()).then(manifest => {
      const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
      const mishpat = roles['mishpat'] as JsonObject;
      assert.equal(mishpat['dnaHash'], SUNSET_AUTHORING_DNA_HASH);
      assert.notEqual(mishpat['dnaHash'], CROSSED_BASE_DNA_HASH);
    });
  });

  void it('demotes a crossed role whose authoring cell could not be read ("unknown")', async () => {
    const passport = lineagePassportFixture();
    const happRoles = ((passport['passport'] as JsonObject)['happ'] as JsonObject)[
      'roles'
    ] as JsonObject[];
    const imagodei = happRoles.find(r => r['role'] === 'imagodei') as JsonObject;
    (imagodei['lineage'] as JsonObject)['authoringDnaHash'] = 'unknown';
    const manifest = await packageFromPassport(passport);
    const roles = (manifest['appliesTo'] as JsonObject)['roles'] as JsonObject;
    assert.equal(
      'imagodei' in roles,
      false,
      'a role whose authoring cell is unreadable is demoted'
    );
    // The peer's other roles still package — one unreadable role does not
    // refuse the whole derivation.
    assert.equal('node_registry' in roles, true);
  });
});
