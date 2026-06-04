#!/usr/bin/env node
/**
 * Tests the vocabulary.quiltPolicies extension (tiered-quilt §4, amended 2026-06-04):
 * named declarative storage-policy classes + per-contentType references.
 * Schema validates SHAPE; referential integrity is loader-enforced
 * (see lib/manifest-quilt-refs.mjs, tested below in the REF CHECKS section).
 */
import { readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
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

  // --- NEGATIVE: schema shape ---
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['streaming-media-library'].shelveAfter = '5 minutes';
    assert(!validate(m), 'Rejects non-compact duration ("5 minutes")');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['streaming-media-library'].defaultTierFloor = 'warm';
    assert(!validate(m), 'Rejects unknown tier name ("warm" is not a temperature class)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies = {};
    assert(!validate(m), 'Rejects empty quiltPolicies {} (not a meaningful declaration)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['streaming-media-library'].draw = 'progressive';
    assert(!validate(m), 'Rejects unknown draw mode ("progressive")');
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies['long-term-personal'].defaultTierFloor;
    assert(!validate(m), 'Rejects policy without defaultTierFloor (the one required field)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['long-term-personal'].costClassHint = 'long-term-personal';
    assert(!validate(m), 'Rejects retired costClassHint field (the policy name IS the cost class)');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicies['Bad_Name'] = { defaultTierFloor: 'stocked' };
    assert(!validate(m), 'Rejects non-kebab-case policy name (names become <pillar>/<name> cost classes)');
  }

  // --- REF CHECKS (loader-enforced; not expressible in JSON Schema) ---
  const { validateQuiltPolicyRefs } = await import('./lib/manifest-quilt-refs.mjs');
  {
    const errs = validateQuiltPolicyRefs(manifestWithQuiltPolicies());
    assert(errs.length === 0, 'Ref check: clean manifest has zero ref errors');
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.contentTypes['family-video'].quiltPolicy = 'streaming-media-libary'; // typo
    const errs = validateQuiltPolicyRefs(m);
    assert(
      errs.length === 1 && errs[0].includes('family-video') && errs[0].includes('streaming-media-libary'),
      "Ref check: typo'd contentType.quiltPolicy fails loud, naming type and ref",
    );
  }
  {
    const m = manifestWithQuiltPolicies();
    m.vocabulary.quiltPolicyDefault = 'does-not-exist';
    const errs = validateQuiltPolicyRefs(m);
    assert(
      errs.length === 1 && errs[0].includes('quiltPolicyDefault') && errs[0].includes('does-not-exist'),
      'Ref check: dangling quiltPolicyDefault fails loud',
    );
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies;
    const errs = validateQuiltPolicyRefs(m);
    assert(
      errs.length === 3,
      'Ref check: references with NO quiltPolicies section at all → every reference is dangling (2 types + default)',
    );
  }
  {
    const m = manifestWithQuiltPolicies();
    delete m.vocabulary.quiltPolicies;
    delete m.vocabulary.quiltPolicyDefault;
    delete m.vocabulary.contentTypes['photo-album'].quiltPolicy;
    delete m.vocabulary.contentTypes['family-video'].quiltPolicy;
    const errs = validateQuiltPolicyRefs(m);
    assert(errs.length === 0, 'Ref check: manifest without any quilt vocabulary is clean (fully optional)');
  }

  // --- WIRING CHECK: codegen resolves $ref content-type stubs before checking quiltPolicy refs ---
  // This exercises the full codegen-manifest.mjs path (resolveRefs → validateQuiltPolicyRefs).
  // The old shallow resolver left vocabulary.contentTypes.<type> = {$ref:...} objects unresolved,
  // so validateQuiltPolicyRefs silently skipped per-content-type checks on modular manifests.
  // This fixture MUST fail if the resolver regresses to shallow: a modular manifest whose
  // content-type $ref file declares a dangling quiltPolicy must exit codegen with code 1,
  // naming the type and the dangling ref.
  {
    const tmpDir = resolve(__dirname, '.quilt-gate-wiring-check');
    const contentTypesDir = resolve(tmpDir, 'manifest', 'content-types');
    try {
      await mkdir(contentTypesDir, { recursive: true });

      // Content-type stub file — the quiltPolicy typo is IN THE $ref file, not inline.
      // This is the exact shape that the old shallow resolver missed.
      const photoStub = {
        description: 'A family photo album',
        coupling: minimalCoupling(),
        quiltPolicy: 'long-term-personl', // typo: missing 'a'
      };
      await writeFile(
        resolve(contentTypesDir, 'photo.json'),
        JSON.stringify(photoStub),
      );

      // Manifest shell: quiltPolicies declares 'long-term-personal' (correct spelling);
      // the photo content-type references 'long-term-personl' (typo) via $ref.
      const manifestShell = {
        id: 'bafkreiwiringtest',
        name: 'wiring-test',
        version: '1.0.0',
        vocabulary: {
          quiltPolicies: {
            'long-term-personal': { defaultTierFloor: 'stocked' },
          },
          quiltPolicyDefault: 'long-term-personal',
          contentTypes: {
            photo: { $ref: './manifest/content-types/photo.json' },
          },
          observations: minimalObservations(),
        },
      };
      await writeFile(resolve(tmpDir, 'manifest.json'), JSON.stringify(manifestShell));

      const outPath = resolve(tmpDir, 'out.ts');
      const codegenScript = resolve(__dirname, 'codegen-manifest.mjs');
      const result = spawnSync(
        process.execPath,
        [codegenScript, resolve(tmpDir, 'manifest.json'), outPath],
        { encoding: 'utf8' },
      );

      const exitedWithError = result.status === 1;
      const stderrNames = (result.stderr ?? '').includes('photo') &&
        (result.stderr ?? '').includes('long-term-personl');
      assert(
        exitedWithError && stderrNames,
        'Wiring: codegen exits 1 on dangling quiltPolicy in $ref content-type stub, naming type and ref',
      );
      if (!exitedWithError || !stderrNames) {
        console.error(`  (codegen exit=${result.status}, stderr=${JSON.stringify(result.stderr)})`);
      }
    } finally {
      await rm(tmpDir, { recursive: true, force: true });
    }
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
