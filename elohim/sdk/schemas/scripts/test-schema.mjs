#!/usr/bin/env node
/**
 * Tests that protocol schemas accept valid data and reject invalid data.
 */
import { readdir, readFile } from 'node:fs/promises';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../current');
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

/**
 * Recursively resolve relative-path $ref values to their $id URIs.
 * Schemas use relative file paths (e.g. "../enums/content-type.schema.json")
 * but AJV resolves $ref against the $id namespace. This function maps
 * file-path refs to the $id of the target schema.
 */
function resolveRefs(schema, schemaDir, idMap) {
  if (typeof schema !== 'object' || schema === null) return schema;
  if (Array.isArray(schema)) return schema.map((item) => resolveRefs(item, schemaDir, idMap));

  const result = {};
  for (const [key, value] of Object.entries(schema)) {
    if (key === '$ref' && typeof value === 'string' && !value.startsWith('#')) {
      // Resolve relative file path to absolute, then look up the $id
      const absPath = resolve(schemaDir, value);
      const id = idMap.get(absPath);
      if (id) {
        result[key] = id;
      } else {
        result[key] = value; // Keep original if not found
      }
    } else {
      result[key] = resolveRefs(value, schemaDir, idMap);
    }
  }
  return result;
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load all enum schemas and build a map of absolute-path -> $id
  const idMap = new Map();
  const enumDir = join(SCHEMA_DIR, 'enums');
  const enumFiles = (await readdir(enumDir)).filter((f) => f.endsWith('.schema.json'));
  for (const file of enumFiles) {
    const absPath = join(enumDir, file);
    const schema = await loadJson(absPath);
    if (schema.$id) {
      idMap.set(absPath, schema.$id);
    }
    ajv.addSchema(schema);
  }

  // --- Enum Tests ---

  const ctValidate = ajv.getSchema('epr:schema:enum:content-type');
  assert(ctValidate('epic'), 'content-type accepts "epic"');
  assert(ctValidate('scenario'), 'content-type accepts "scenario"');
  assert(!ctValidate('invalid-type'), 'content-type rejects "invalid-type"');
  assert(!ctValidate(''), 'content-type rejects empty string');
  assert(!ctValidate(42), 'content-type rejects number');

  const reachValidate = ajv.getSchema('epr:schema:enum:reach');
  assert(reachValidate('commons'), 'reach accepts "commons"');
  assert(reachValidate('private'), 'reach accepts "private"');
  assert(!reachValidate('public-all'), 'reach rejects "public-all"');
  assert(!reachValidate('invited'), 'reach rejects "invited" (not a DNA value)');

  const masteryValidate = ajv.getSchema('epr:schema:enum:mastery-level');
  assert(masteryValidate('create'), 'mastery-level accepts "create"');
  assert(!masteryValidate('expert'), 'mastery-level rejects "expert"');

  const vsValidate = ajv.getSchema('epr:schema:enum:validation-status');
  assert(vsValidate('valid'), 'validation-status accepts "valid"');
  assert(vsValidate('healing'), 'validation-status accepts "healing"');
  assert(!vsValidate('broken'), 'validation-status rejects "broken"');

  // --- CreateContentInput Tests ---

  const inputSchemaRaw = await loadJson(
    join(SCHEMA_DIR, 'inputs', 'create-content-input.schema.json'),
  );
  const inputSchema = resolveRefs(
    inputSchemaRaw,
    join(SCHEMA_DIR, 'inputs'),
    idMap,
  );
  const inputValidate = ajv.compile(inputSchema);

  // Valid minimal
  assert(
    inputValidate({ id: 'test-1', title: 'Test' }),
    'CreateContentInput accepts minimal valid input',
  );

  // Valid full
  assert(
    inputValidate({
      id: 'test-2',
      title: 'Full Test',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach: 'commons',
      tags: ['test'],
      description: 'A test',
      contentBody: '# Hello',
      metadata: { key: 'val' },
    }),
    'CreateContentInput accepts full valid input',
  );

  // Missing required: id
  assert(
    !inputValidate({ title: 'Test' }),
    'CreateContentInput rejects missing id',
  );

  // Missing required: title
  assert(
    !inputValidate({ id: 'test' }),
    'CreateContentInput rejects missing title',
  );

  // Invalid content type (not in any tier)
  assert(
    !inputValidate({ id: 'test', title: 'Test', contentType: 'squirrel' }),
    'CreateContentInput rejects unregistered content type "squirrel"',
  );

  // Invalid reach
  assert(
    !inputValidate({ id: 'test', title: 'Test', reach: 'invited' }),
    'CreateContentInput rejects invalid reach "invited"',
  );

  // Additional properties rejected
  assert(
    !inputValidate({ id: 'test', title: 'Test', unknownField: true }),
    'CreateContentInput rejects unknown properties',
  );

  // --- ContentView Tests ---

  const viewSchemaRaw = await loadJson(
    join(SCHEMA_DIR, 'views', 'content-view.schema.json'),
  );
  const viewSchema = resolveRefs(
    viewSchemaRaw,
    join(SCHEMA_DIR, 'views'),
    idMap,
  );
  const viewValidate = ajv.compile(viewSchema);

  // Valid minimal
  assert(
    viewValidate({
      id: 'test',
      hAppId: 'app1',
      title: 'Test',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach: 'commons',
      validationStatus: 'valid',
      createdAt: '2026-03-17T00:00:00Z',
      updatedAt: '2026-03-17T00:00:00Z',
    }),
    'ContentView accepts valid record',
  );

  // Missing required field
  assert(
    !viewValidate({
      id: 'test',
      title: 'Test',
    }),
    'ContentView rejects missing required fields',
  );

  // Nullable fields accept null
  assert(
    viewValidate({
      id: 'test',
      hAppId: 'app1',
      title: 'Test',
      contentType: 'concept',
      contentFormat: 'markdown',
      reach: 'commons',
      validationStatus: 'valid',
      createdAt: '2026-03-17',
      updatedAt: '2026-03-17',
      description: null,
      blobHash: null,
      dhtAnchorHash: null,
    }),
    'ContentView accepts null for nullable fields',
  );

  // Test: SessionLifecycleState enum schema
  {
    const enumPath = resolve(__dirname, '../v1/enums/session-lifecycle-state.schema.json');
    let lifecycleSchema;
    try {
      lifecycleSchema = await loadJson(enumPath);
    } catch {
      lifecycleSchema = null;
    }
    assert(
      lifecycleSchema !== null,
      'SessionLifecycleState enum schema exists at v1/enums/session-lifecycle-state.schema.json'
    );
    assert(
      lifecycleSchema?.title === 'SessionLifecycleState',
      'SessionLifecycleState enum schema declares title: "SessionLifecycleState"'
    );
    assert(
      Array.isArray(lifecycleSchema?.enum) &&
        lifecycleSchema.enum.length === 4 &&
        lifecycleSchema.enum.includes('Anonymous') &&
        lifecycleSchema.enum.includes('OauthIdentified') &&
        lifecycleSchema.enum.includes('PeerNativeSampling') &&
        lifecycleSchema.enum.includes('PeerNativeMember'),
      'SessionLifecycleState enum contains all four lifecycle values'
    );
  }

  // Test: app-manifest $defs additions for staged-intents substrate
  {
    const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
    const manifestSchema = await loadJson(manifestSchemaPath);
    assert(
      manifestSchema.$defs?.StagedIntentDeclaration !== undefined,
      '$defs/StagedIntentDeclaration is defined on app-manifest.schema.json'
    );
    assert(
      manifestSchema.$defs?.GraduationPolicy !== undefined,
      '$defs/GraduationPolicy is defined on app-manifest.schema.json'
    );
    const sid = manifestSchema.$defs?.StagedIntentDeclaration;
    const sidRequired = Array.isArray(sid?.required) ? sid.required : [];
    assert(
      sidRequired.includes('description') &&
        sidRequired.includes('intentSchema') &&
        sidRequired.includes('graduatesTo') &&
        sidRequired.includes('actionableFrom') &&
        sidRequired.includes('resolutionMode') &&
        sidRequired.includes('coupling'),
      'StagedIntentDeclaration requires all six declared fields (description / intentSchema / graduatesTo / actionableFrom / resolutionMode / coupling)'
    );
    const gp = manifestSchema.$defs?.GraduationPolicy;
    const gpRequired = Array.isArray(gp?.required) ? gp.required : [];
    assert(
      gpRequired.includes('deterministicCeremony'),
      'GraduationPolicy requires deterministicCeremony'
    );
  }

  // Test: Task 2 review fixes — intentSchema closed shape + framingCid pattern guard
  {
    const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
    const manifestSchema = await loadJson(manifestSchemaPath);

    const intentSchemaDef = manifestSchema.$defs?.StagedIntentDeclaration?.properties?.intentSchema;
    assert(
      intentSchemaDef?.additionalProperties === false,
      'StagedIntentDeclaration.intentSchema declares additionalProperties: false (closed shape)'
    );

    const framingCidDef = manifestSchema.$defs?.GraduationPolicy?.properties?.framingCid;
    assert(
      typeof framingCidDef?.pattern === 'string' && framingCidDef.pattern.includes('epr:'),
      'GraduationPolicy.framingCid declares an EPR-CID pattern guard'
    );
  }

  // Test: vocabulary.stagedIntents + top-level graduation + dependentSchemas (Task 3)
  {
    const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
    const manifestSchema = await loadJson(manifestSchemaPath);

    // The Vocabulary $def carries the contentTypes/contentFormats/relationships/signals/observations
    // properties. We're adding stagedIntents as a sibling.
    const vocabProps = manifestSchema.$defs?.Vocabulary?.properties;
    assert(
      vocabProps?.stagedIntents !== undefined,
      'vocabulary.stagedIntents property is declared (under $defs/Vocabulary)'
    );
    assert(
      vocabProps?.stagedIntents?.additionalProperties?.$ref === '#/$defs/StagedIntentDeclaration',
      'vocabulary.stagedIntents.additionalProperties references $defs/StagedIntentDeclaration'
    );

    assert(
      manifestSchema.properties?.graduation?.$ref === '#/$defs/GraduationPolicy',
      'top-level graduation property references $defs/GraduationPolicy'
    );

    // Conditional: if vocabulary.stagedIntents is non-empty, top-level graduation must be present.
    // JSON Schema 2020-12 expresses this via "dependentSchemas" on vocabulary, OR a top-level
    // allOf with if/then. Either is acceptable — assert one or the other shape is present.
    const hasDependentSchemas = manifestSchema.dependentSchemas?.vocabulary !== undefined;
    const hasAllOfIfThen = Array.isArray(manifestSchema.allOf) &&
      manifestSchema.allOf.some(clause => clause.if && clause.then);
    assert(
      hasDependentSchemas || hasAllOfIfThen,
      'Conditional rule present: stagedIntents non-empty implies top-level graduation required'
    );
  }

  // Test: Task 3 review fix — stagedIntents requires non-empty content
  {
    const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
    const manifestSchema = await loadJson(manifestSchemaPath);

    const stagedIntentsDef = manifestSchema.$defs?.Vocabulary?.properties?.stagedIntents;
    assert(
      stagedIntentsDef?.minProperties === 1,
      'vocabulary.stagedIntents declares minProperties: 1 (rejects empty {})'
    );
  }

  // Test: manifest fixtures validate per stagedIntents + graduation rules (Task 4 — spec §11.1)
  {
    const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
    const manifestSchema = await loadJson(manifestSchemaPath);

    // Register the enums referenced by the manifest schema so $refs resolve.
    // AJV resolves "../enums/foo.schema.json" against the manifest's $id
    // (epr:schema:manifest:app-manifest), which produces "epr:enums/foo.schema.json".
    // We register under that resolved URI AND under the $id for robustness.
    const fixtureAjv = new Ajv2020({ strict: false, allErrors: true });

    const lifecycleSchemaPath = resolve(__dirname, '../v1/enums/session-lifecycle-state.schema.json');
    const lifecycleSchema = await loadJson(lifecycleSchemaPath);
    fixtureAjv.addSchema(lifecycleSchema, 'epr:enums/session-lifecycle-state.schema.json');

    const instrumentSchema = await loadJson(
      resolve(__dirname, '../v1/enums/instrument-archetype.schema.json')
    );
    fixtureAjv.addSchema(instrumentSchema, 'epr:enums/instrument-archetype.schema.json');

    const polaritySchema = await loadJson(
      resolve(__dirname, '../v1/enums/observation-polarity.schema.json')
    );
    fixtureAjv.addSchema(polaritySchema, 'epr:enums/observation-polarity.schema.json');

    const substrateSignalSchema = await loadJson(
      resolve(__dirname, '../v1/enums/substrate-signal.schema.json')
    );
    fixtureAjv.addSchema(substrateSignalSchema, 'epr:enums/substrate-signal.schema.json');

    const eprKindSchema = await loadJson(
      resolve(__dirname, '../v1/enums/epr-kind.schema.json')
    );
    fixtureAjv.addSchema(eprKindSchema, 'epr:enums/epr-kind.schema.json');

    const pillarProjectionSchema = await loadJson(
      resolve(__dirname, '../v1/manifest/pillar-projection.schema.json')
    );
    fixtureAjv.addSchema(pillarProjectionSchema, 'epr:pillar-projection.schema.json');

    const observationKindSchema = await loadJson(
      resolve(__dirname, '../v1/manifest/observation-kind.schema.json')
    );
    fixtureAjv.addSchema(observationKindSchema, 'epr:observation-kind.schema.json');

    const validate = fixtureAjv.compile(manifestSchema);

    // ThreeLegCoupling-conforming fixture. Schema requires value, governance, and claims;
    // GovernanceLeg requires defaultReach, minimumReach, governanceModel;
    // ValueFlowEvent requires action; ClaimDeclaration requires asserts/contradictedBy/validityHorizon.
    const couplingFixture = {
      knowledge: { relationships: { REFERENCES: ['concept'] } },
      value: { onConsume: { action: 'use', resourceConformsTo: 'test', recognition: 'test' } },
      governance: {
        defaultReach: 'commons',
        minimumReach: 'community',
        governanceModel: 'steward-consent',
        signalTypes: ['test-signal']
      },
      claims: [
        { asserts: 'test-claim', contradictedBy: 'test-contradiction', validityHorizon: 'P90D', leg: 'value' }
      ]
    };

    // Vocabulary requires contentTypes (minProperties: 1) and observations (minProperties: 1).
    // Build a shared baseManifest with one valid content type and the two observations referenced
    // by the coupling fixture's claims.
    const baseObservations = {
      'test-claim': {
        description: 'Positive observation referenced by the fixture claim.',
        instrument: 'retention-check',
        polarity: 'positive'
      },
      'test-contradiction': {
        description: 'Negative observation referenced by the fixture claim.',
        instrument: 'retention-check',
        polarity: 'negative'
      }
    };

    const baseContentTypes = {
      'test-content-type': {
        description: 'Fixture content type satisfying ThreeLegCoupling.',
        coupling: couplingFixture
      }
    };

    const buildBaseManifest = () => ({
      id: 'manifest-test-fixture',
      name: 'test-fixture',
      version: '1.0.0',
      vocabulary: {
        contentTypes: { ...baseContentTypes },
        observations: { ...baseObservations }
      }
    });

    // Assertion 1: manifest with stagedIntents + graduation validates clean
    const fixtureWithStaged = buildBaseManifest();
    fixtureWithStaged.vocabulary.stagedIntents = {
      'staged-test-intent': {
        description: 'Test staged intent for schema validation.',
        intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
        graduatesTo: 'TestEntry',
        actionableFrom: ['OauthIdentified', 'PeerNativeMember'],
        resolutionMode: 'deterministic',
        coupling: couplingFixture
      }
    };
    fixtureWithStaged.graduation = {
      deterministicCeremony: 'test::DeterministicCeremony'
    };
    const ok1 = validate(fixtureWithStaged);
    if (!ok1) {
      console.error('Assertion 1 errors:', JSON.stringify(validate.errors, null, 2));
    }
    assert(
      ok1,
      'Manifest with stagedIntents + graduation validates clean (Task 4 §11.1 assertion 1)'
    );

    // Assertion 2: manifest without stagedIntents validates clean (backward compat)
    const fixtureBaseline = buildBaseManifest();
    const ok2 = validate(fixtureBaseline);
    if (!ok2) {
      console.error('Assertion 2 errors:', JSON.stringify(validate.errors, null, 2));
    }
    assert(
      ok2,
      'Manifest without stagedIntents validates clean (Task 4 §11.1 assertion 2 — backward compatibility)'
    );

    // Assertion 3: manifest with stagedIntents but missing top-level graduation fails
    const fixtureMissingGraduation = buildBaseManifest();
    fixtureMissingGraduation.vocabulary.stagedIntents = {
      'staged-test-intent': {
        description: 'Test staged intent.',
        intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
        graduatesTo: 'TestEntry',
        actionableFrom: ['OauthIdentified'],
        resolutionMode: 'deterministic',
        coupling: couplingFixture
      }
    };
    assert(
      !validate(fixtureMissingGraduation),
      'Manifest with non-empty stagedIntents but missing graduation fails (Task 4 §11.1 assertion 3 — dependentSchemas conditional)'
    );

    // Assertion 4: stagedIntents entry missing graduatesTo fails
    const fixtureMissingGraduatesTo = buildBaseManifest();
    fixtureMissingGraduatesTo.vocabulary.stagedIntents = {
      'staged-test-intent': {
        description: 'Test staged intent missing graduatesTo.',
        intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
        actionableFrom: ['OauthIdentified'],
        resolutionMode: 'deterministic',
        coupling: couplingFixture
      }
    };
    fixtureMissingGraduatesTo.graduation = { deterministicCeremony: 'test::DeterministicCeremony' };
    assert(
      !validate(fixtureMissingGraduatesTo),
      'stagedIntents entry missing graduatesTo fails validation (Task 4 §11.1 assertion 4)'
    );

    // Assertion 5: actionableFrom with invalid lifecycle value fails
    const fixtureInvalidLifecycle = buildBaseManifest();
    fixtureInvalidLifecycle.vocabulary.stagedIntents = {
      'staged-test-intent': {
        description: 'Test staged intent with invalid actionableFrom.',
        intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
        graduatesTo: 'TestEntry',
        actionableFrom: ['NotAValidLifecycleValue'],
        resolutionMode: 'deterministic',
        coupling: couplingFixture
      }
    };
    fixtureInvalidLifecycle.graduation = { deterministicCeremony: 'test::DeterministicCeremony' };
    assert(
      !validate(fixtureInvalidLifecycle),
      'actionableFrom array with invalid lifecycle value fails validation (Task 4 §11.1 assertion 5)'
    );

    // Assertion 6: graduation.deterministicCeremony empty string fails
    const fixtureEmptyCeremony = buildBaseManifest();
    fixtureEmptyCeremony.vocabulary.stagedIntents = {
      'staged-test-intent': {
        description: 'Test staged intent.',
        intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
        graduatesTo: 'TestEntry',
        actionableFrom: ['OauthIdentified'],
        resolutionMode: 'deterministic',
        coupling: couplingFixture
      }
    };
    fixtureEmptyCeremony.graduation = { deterministicCeremony: '' };
    assert(
      !validate(fixtureEmptyCeremony),
      'graduation.deterministicCeremony empty string fails validation (Task 4 §11.1 assertion 6)'
    );

    // Assertion 7: resolutionMode outside enum fails
    const fixtureInvalidResolution = buildBaseManifest();
    fixtureInvalidResolution.vocabulary.stagedIntents = {
      'staged-test-intent': {
        description: 'Test staged intent.',
        intentSchema: { $ref: './schemas/staged-test-intent.schema.json' },
        graduatesTo: 'TestEntry',
        actionableFrom: ['OauthIdentified'],
        resolutionMode: 'instantaneous',
        coupling: couplingFixture
      }
    };
    fixtureInvalidResolution.graduation = { deterministicCeremony: 'test::DeterministicCeremony' };
    assert(
      !validate(fixtureInvalidResolution),
      'resolutionMode outside the enum (deterministic | negotiated | either) fails validation (Task 4 §11.1 assertion 7)'
    );
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
