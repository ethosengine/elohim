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
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import * as AjvNs from 'ajv/dist/2020.js';

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
    artifactClass: 'coordinator-bundle',
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
    const m = baseManifest({ artifactClass: 'happ-lineage' });
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
    const m = baseManifest({ artifactClass: 'happ-lineage' });
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

function appliesToLiteral(): string {
  return JSON.stringify({
    roles: {
      node_registry: { dnaHash: 'uhC0k' + 'B'.repeat(48), coordinatorWasmHashes: [] },
    },
  });
}

function happLineageArgs(artifactPath: string, extra: string[] = []): string[] {
  return [
    '--artifact',
    artifactPath,
    '--artifact-class',
    'happ-lineage',
    '--applies-to',
    appliesToLiteral(),
    '--migrate-from',
    `node_registry=${MIGRATE_FROM_HASH}`,
    '--lineage',
    MIGRATE_FROM_HASH,
    '--soak-secs',
    '900',
    '--attestation-threshold',
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
          '--artifact',
          tempArtifact(),
          '--artifact-class',
          'happ-lineage',
          '--applies-to',
          appliesToLiteral(),
          '--migrate-from',
          `mishpat=${MIGRATE_FROM_HASH}`,
          '--path-commitment',
          PATH_COMMITMENT_CID,
          '--soak-secs',
          '900',
          '--attestation-threshold',
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
      '--artifact',
      tempArtifact(),
      '--artifact-class',
      'coordinator-bundle',
      '--applies-to',
      appliesToLiteral(),
      '--soak-secs',
      '900',
      '--attestation-threshold',
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
