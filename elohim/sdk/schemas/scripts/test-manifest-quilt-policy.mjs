#!/usr/bin/env node
/**
 * Tests the vocabulary.quiltPolicies extension (tiered-quilt §4, amended 2026-06-04):
 * named declarative storage-policy classes + per-contentType references.
 * Schema validates SHAPE; referential integrity is loader-enforced
 * (see lib/manifest-quilt-refs.mjs, tested below in the REF CHECKS section).
 */
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
let failures = 0;
let passes = 0;

function assert(condition, message) {
  if (!condition) {
    console.error(`FAIL: ${message}`);
    failures++;
  } else {
    console.log(`PASS: ${message}`);
    passes++;
  }
}

async function loadJson(filepath) {
  return JSON.parse(await readFile(filepath, 'utf8'));
}

function minimalCoupling() {
  return {
    value: {
      onConsume: { action: 'use' },
    },
    governance: {
      defaultReach: 'commons',
      minimumReach: 'community',
      governanceModel: 'steward-consent',
    },
    claims: [
      { asserts: 'comprehension', contradictedBy: 'comprehension-failure', validityHorizon: 'P30D' },
    ],
  };
}

function minimalObservations() {
  return {
    comprehension: { description: 'Learner demonstrated comprehension', instrument: 'retention-check', polarity: 'positive' },
    'comprehension-failure': { description: 'Learner failed to demonstrate comprehension', instrument: 'retention-check', polarity: 'negative' },
  };
}

/** Spec §4 example policies — the streaming class exercises every field. */
function quiltPolicies() {
  return {
    'long-term-personal': {
      defaultTierFloor: 'stocked',
      shelveAfter: '30d',
      holdWarmMin: '7d',
      preferDestinations: [
        'federated-dwelling://family/{family-id}',
        'peer-cellar://household/{any}',
      ],
    },
    'streaming-media-library': {
      defaultTierFloor: 'shelved',
      holdWarmMin: '2h',
      shelveAfter: '7d',
      drawLatencyBudget: '2s',
      draw: 'streamed',
      preferDestinations: ['peer-cellar://household/{any}'],
    },
  };
}

function manifestWithQuiltPolicies() {
  return {
    id: 'bafkreiquiltexample',
    name: 'Quilt Policy Test App',
    version: '1.0.0',
    vocabulary: {
      quiltPolicies: quiltPolicies(),
      quiltPolicyDefault: 'long-term-personal',
      contentTypes: {
        'photo-album': {
          description: 'A family photo album',
          coupling: minimalCoupling(),
          quiltPolicy: 'long-term-personal',
        },
        'family-video': {
          description: 'A family video',
          coupling: minimalCoupling(),
          quiltPolicy: 'streaming-media-library',
        },
      },
      observations: minimalObservations(),
    },
  };
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load referenced schemas so AJV can resolve $ref — mirror the addSchema
  // block from test-manifest-schema.mjs main() exactly (run
  // `grep -n addSchema elohim/sdk/schemas/scripts/test-manifest-schema.mjs`
  // and replicate every line found there).
  const substrateSignalSchema = await loadJson(
    resolve(__dirname, '../v1/enums/substrate-signal.schema.json'),
  );
  ajv.addSchema(substrateSignalSchema, 'epr:enums/substrate-signal.schema.json');

  const instrumentArchetypeSchema = await loadJson(
    resolve(__dirname, '../v1/enums/instrument-archetype.schema.json'),
  );
  ajv.addSchema(instrumentArchetypeSchema, 'epr:enums/instrument-archetype.schema.json');

  const observationPolaritySchema = await loadJson(
    resolve(__dirname, '../v1/enums/observation-polarity.schema.json'),
  );
  ajv.addSchema(observationPolaritySchema, 'epr:enums/observation-polarity.schema.json');

  const eprKindSchema = await loadJson(
    resolve(__dirname, '../v1/enums/epr-kind.schema.json'),
  );
  ajv.addSchema(eprKindSchema, 'epr:enums/epr-kind.schema.json');

  const pillarProjectionSchema = await loadJson(
    resolve(__dirname, '../v1/manifest/pillar-projection.schema.json'),
  );
  ajv.addSchema(pillarProjectionSchema, 'epr:pillar-projection.schema.json');

  const observationKindSchema = await loadJson(
    resolve(__dirname, '../v1/manifest/observation-kind.schema.json'),
  );
  ajv.addSchema(observationKindSchema, 'epr:observation-kind.schema.json');

  const sessionLifecycleStateSchema = await loadJson(
    resolve(__dirname, '../v1/enums/session-lifecycle-state.schema.json'),
  );
  ajv.addSchema(sessionLifecycleStateSchema, 'epr:enums/session-lifecycle-state.schema.json');

  const schema = await loadJson(resolve(__dirname, '../v1/manifest/app-manifest.schema.json'));
  const validate = ajv.compile(schema);

  // --- ACCEPTANCE ---
  {
    const valid = validate(manifestWithQuiltPolicies());
    if (!valid) console.error(JSON.stringify(validate.errors, null, 2));
    assert(valid, 'Accepts manifest with named quiltPolicies + per-type references + default');
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies;
    delete m.vocabulary.quiltPolicyDefault;
    delete m.vocabulary.contentTypes['photo-album'].quiltPolicy;
    delete m.vocabulary.contentTypes['family-video'].quiltPolicy;
    assert(validate(m), 'quiltPolicies is fully optional — existing manifests stay valid');
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
