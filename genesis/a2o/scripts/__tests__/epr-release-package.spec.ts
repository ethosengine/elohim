/**
 * Release manifest schema — the `happ-lineage` artifactClass (epic Task 3).
 *
 * Validates manifest objects directly against
 * `elohim/rakia/schemas/v1/release-manifest.schema.json` with the same Ajv
 * setup `epr-release-package.ts --validate` uses (dist/2020.js, strict:
 * false, allErrors: true). This test does NOT import the packager module —
 * it is mid-edit in another session tonight (a discipline-inheritance
 * change) and importing it here would couple this spec to that in-flight
 * shape. The schema file is the shared contract; that's what's under test.
 */

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
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

type JsonObject = Record<string, unknown>;

function loadSchema(): JsonObject {
  return JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')) as JsonObject;
}

function validateManifest(manifest: unknown): { ok: boolean; errors: string[] } {
  const ajv = new AjvCtor({ strict: false, allErrors: true });
  const validate = ajv.compile(loadSchema());
  const ok = validate(manifest) as boolean;
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
