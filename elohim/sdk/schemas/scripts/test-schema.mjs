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

  const closureValidate = ajv.getSchema('epr:schema:enum:closure');
  assert(closureValidate('open'), 'closure accepts "open"');
  assert(closureValidate('closed'), 'closure accepts "closed"');
  assert(!closureValidate('partial'), 'closure rejects "partial" (two postures, not a gradient)');

  // --- Axis Card Registry Tests ---
  //
  // Axis cards (registries/axes/*.axis.json) declare what an axis means, which vocabulary it
  // classifies over, and — the load-bearing field — what SILENCE means on each of its two layers.
  // Without the closure declaration a counterparty that does not share our code cannot tell whether
  // our silence on an axis is an admission or a denial, which is the whole attack surface.
  // Enum schemas are already in `ajv` above, so the card's `vocabulary.$ref` and the two
  // `closure.*` $refs resolve by $id.

  const axisCardSchema = await loadJson(join(SCHEMA_DIR, 'registries', 'axis-card.schema.json'));
  ajv.addSchema(axisCardSchema);
  const axisCardValidate = ajv.getSchema('epr:registry:axis-card');
  assert(!!axisCardValidate, 'axis-card meta-schema compiles');

  const axesDir = join(SCHEMA_DIR, 'registries', 'axes');
  const axisFiles = (await readdir(axesDir)).filter((f) => f.endsWith('.axis.json'));
  assert(axisFiles.length > 0, 'registries/axes contains at least one axis card');

  for (const file of axisFiles) {
    const card = await loadJson(join(axesDir, file));
    const valid = axisCardValidate(card);
    if (!valid) {
      console.error(`  ${file}:`, JSON.stringify(axisCardValidate.errors, null, 2));
    }
    assert(valid, `axis card ${file} conforms to epr:registry:axis-card`);

    // Identity coherence: filename stem, `axis`, and the `$id` suffix must agree. Three names for
    // one axis is exactly how the reach vocabulary drifted five ways.
    const stem = file.replace(/\.axis\.json$/, '');
    assert(card.axis === stem, `axis card ${file}: \`axis\` matches filename stem`);
    assert(
      card.$id === `epr:registry:axis:${stem}`,
      `axis card ${file}: $id matches filename stem`,
    );

    // The vocabulary is REFERENCED, never inlined — a card that copies its value list has forked
    // the enum it was meant to point at.
    const vocabId = card.vocabulary?.$ref;
    assert(
      !!ajv.getSchema(vocabId),
      `axis card ${file}: vocabulary $ref "${vocabId}" resolves to a registered enum`,
    );

    // Totality law: a closed-verdict axis must name the classifier bridging its open fact layer to
    // its closed verdict layer, and that classifier must be able to express the `refer` ceiling.
    if (card.closure?.verdicts === 'closed') {
      assert(
        !!card.classifier,
        `axis card ${file}: closed-verdict axis declares a classifier`,
      );
    }
  }

  // Assert the NEGATIVE — the conditional must actually bite. A closed-verdict card with no
  // classifier has to fail, or the totality law is decorative.
  const cardMissingClassifier = {
    $id: 'epr:registry:axis:fixture',
    axis: 'fixture',
    version: 1,
    kind: 'derived',
    semantics: 'fixture',
    vocabulary: { $ref: 'epr:schema:enum:closure' },
    closure: { facts: 'open', verdicts: 'closed' },
    sourceOfRecord: { kind: 'derived-fold' },
  };
  assert(
    !axisCardValidate(cardMissingClassifier),
    'axis-card REJECTS a closed-verdict card with no classifier (totality law bites)',
  );
  assert(
    axisCardValidate({ ...cardMissingClassifier, closure: { facts: 'open', verdicts: 'open' } }),
    'axis-card allows an open-verdict card without a classifier (the conditional is conditional)',
  );
  assert(
    !axisCardValidate({ ...cardMissingClassifier, closure: { facts: 'open' } }),
    'axis-card REJECTS a card declaring only half its closure posture',
  );
  const { closure: _dropped, ...cardNoClosure } = cardMissingClassifier;
  assert(
    !axisCardValidate(cardNoClosure),
    'axis-card REJECTS a card with no closure declaration at all',
  );

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

    // AJV resolves "../enums/foo.schema.json" relative to the manifest's $id
    // (epr:schema:manifest:app-manifest) → "epr:enums/foo.schema.json".
    // We register under that resolved URI key — that is what the manifest's $refs
    // actually look up. AJV also registers the schema under its own $id automatically
    // as a side effect of addSchema, but the resolved URI is the primary key.

    // A fresh Ajv2020 instance is required because the outer `ajv` registers enum
    // schemas under their $id keys (epr:schema:enum:*), but the manifest's relative
    // $refs resolve to epr:enums/* keys — a different namespace. The outer instance
    // cannot satisfy both key shapes simultaneously, and its schemas are already
    // sealed by the time this block runs.
    const fixtureAjv = new Ajv2020({ strict: false, allErrors: true });

    // Register EVERY enum under its resolved URI, matching the regression block below.
    // This was previously a hand-picked list of five, which made any new enum $ref added
    // to app-manifest.schema.json fail at COMPILE time with a MissingRefError — a trap
    // that fires on the schema author, far from the list that caused it, and says nothing
    // about which list to edit. Globbing removes the maintenance step entirely.
    const fixtureEnumDir = resolve(__dirname, '../v1/enums');
    for (const file of (await readdir(fixtureEnumDir)).filter((f) => f.endsWith('.schema.json'))) {
      const resolvedUri = `epr:enums/${file}`;
      if (!fixtureAjv.getSchema(resolvedUri)) {
        fixtureAjv.addSchema(await loadJson(resolve(fixtureEnumDir, file)), resolvedUri);
      }
    }

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

  // Test: Tasks 1-3 substrate additions do not introduce new validation errors
  // for existing per-pillar manifests (Task 6 — non-regression invariant).
  //
  // Background: the 8 existing per-pillar manifests fail validation against
  // app-manifest.schema.json at BASE_SHA for pre-existing schema/manifest drift
  // (root-level keys like `attestations`, `gates`, missing required `observations`,
  // content-type entries with `status`, observation entries with `archetype`, etc.).
  // That drift is out of scope for B-MANIFEST.
  //
  // What Tasks 1-3 must NOT do: introduce new errors that reference our additions
  // (vocabulary.stagedIntents, top-level graduation, dependentSchemas/vocabulary,
  // SessionLifecycleState enum, $defs/StagedIntentDeclaration, $defs/GraduationPolicy).
  //
  // This block validates each manifest and asserts: ZERO of the error schemaPaths
  // reference any Task 1-3 substrate addition. If we ever introduce a change that
  // causes an existing manifest to fail for one of OUR new substrate reasons, this
  // test surfaces it.
  {
    const manifestSchemaPath = resolve(__dirname, '../v1/manifest/app-manifest.schema.json');
    const manifestSchema = await loadJson(manifestSchemaPath);

    const regressionAjv = new Ajv2020({ strict: false, allErrors: true });

    // Register all enum schemas under their resolved-URI keys so the manifest's
    // `../enums/...` $refs resolve. (Same pattern as the Task 4 fixture block —
    // see that block's comments for the namespace-collision rationale.)
    const enumDir = resolve(__dirname, '../v1/enums');
    const enumFiles = await readdir(enumDir);
    for (const file of enumFiles.filter(f => f.endsWith('.schema.json'))) {
      const schema = await loadJson(resolve(enumDir, file));
      const resolvedUri = `epr:enums/${file}`;
      if (!regressionAjv.getSchema(resolvedUri)) {
        regressionAjv.addSchema(schema, resolvedUri);
      }
    }

    // Register additional referenced schemas (mirrors Task 4 fixture setup).
    // The manifest schema's $id is epr:schema:manifest:app-manifest, so relative
    // $refs like "./pillar-projection.schema.json" resolve to "epr:<filename>".
    const manifestDir = resolve(__dirname, '../v1/manifest');
    for (const file of ['pillar-projection.schema.json', 'observation-kind.schema.json']) {
      try {
        const schema = await loadJson(resolve(manifestDir, file));
        const resolvedUri = `epr:${file}`;
        if (!regressionAjv.getSchema(resolvedUri)) {
          regressionAjv.addSchema(schema, resolvedUri);
        }
      } catch {
        // schema may not exist; skip
      }
    }

    const validate = regressionAjv.compile(manifestSchema);

    // The Task 1-3 substrate-addition surface that Task 6 guards against.
    // If any of these strings appear in an error's schemaPath, the test fails —
    // meaning our additions are now visible to an existing manifest's validation.
    //
    // MAINTENANCE: when future tasks (B-PILLAR-*, B-CRATE, B-APPRAISE Phase 2, etc.)
    // extend the substrate with new named additions ($defs, properties, enums, or
    // path fragments that can appear in Ajv error metadata), add their discriminating
    // tokens to this array. The non-regression invariant only catches errors that
    // reference a token here.
    const TASK_1_TO_3_SURFACE = [
      'stagedIntents',
      'graduation',                    // catches top-level graduation property errors
      'dependentSchemas/vocabulary',   // catches the conditional rule
      'StagedIntentDeclaration',       // catches $defs ref errors
      'GraduationPolicy',              // catches $defs ref errors
      'SessionLifecycleState',         // catches lifecycle enum ref errors
      'session-lifecycle-state',       // catches the enum file path
      // Third-party gate closure (2026-07-24). `gates` is the CONSUMER'S verdict
      // surface — where a downstream domain's own vocabulary reaches a decision.
      // These tokens are here so a gate that handles events without declaring what
      // its silence means fails loudly instead of joining the tolerated drift.
      // NB: `GateDeclaration` (the $defs name) is the discriminator, NOT a bare `gates`
      // token — `gates` also appears as a pre-existing undeclared key inside
      // GovernanceLeg (vocabulary.contentTypes.*.coupling.governance.gates), which is one
      // of the 35 counted drift items and unrelated to the top-level gate block.
      'GateDeclaration',
      'closure'
    ];

    // Resolve the modular manifest split BEFORE validating — the EXACT rule from
    // domains/lamad/scripts/codegen.mjs::resolveRefs: inline only `./manifest/` and
    // `../` refs, and deliberately leave `./schemas/…` pointers alone (those are JSON
    // Schema references for metadata payloads and must stay refs).
    //
    // Without this the validator sees `{"$ref": "./manifest/content-types/article.json"}`
    // where the schema expects a content-type object and reports three phantom errors per
    // content type (unexpected `$ref`, missing `description`, missing `coupling`). Measured
    // 2026-07-24: raw load = 120 errors across 8 pillars, of which 85 were phantom; with
    // this rule = 35 genuine. The error filter below was compensating for a loader that had
    // never been wired in, not merely for drift.
    const inlineManifestRefs = async (value, baseDir) => {
      if (value === null || typeof value !== 'object') return value;
      if (Array.isArray(value)) {
        return Promise.all(value.map((v) => inlineManifestRefs(v, baseDir)));
      }
      if (
        typeof value.$ref === 'string' &&
        (value.$ref.startsWith('./manifest/') || value.$ref.startsWith('../'))
      ) {
        const refPath = resolve(baseDir, value.$ref);
        return inlineManifestRefs(await loadJson(refPath), dirname(refPath));
      }
      const out = {};
      for (const [k, v] of Object.entries(value)) {
        out[k] = await inlineManifestRefs(v, baseDir);
      }
      return out;
    };

    const pillars = ['avodah', 'elohim', 'imagodei', 'infrastructure', 'lamad', 'mishpat', 'qahal', 'shefa'];
    let phantomRefErrors = 0;
    for (const pillar of pillars) {
      const pillarDir = resolve(__dirname, `../../domains/${pillar}`);
      const manifestPath = resolve(pillarDir, 'manifest.json');
      let manifest;
      try {
        manifest = await inlineManifestRefs(await loadJson(manifestPath), pillarDir);
      } catch (e) {
        assert(false, `Pillar manifest could not be loaded: ${pillar} (${e.message})`);
        continue;
      }

      validate(manifest);
      const errors = validate.errors || [];

      // The loader is wired iff no error blames a literal `$ref` key — that error can
      // only occur when the modular split was left unresolved.
      phantomRefErrors += errors.filter(
        (e) => e.keyword === 'additionalProperties' && e.params?.additionalProperty === '$ref',
      ).length;

      // Find any errors that reference our Task 1-3 substrate additions.
      const ourErrors = errors.filter(err => {
        const path = `${err.schemaPath || ''} ${err.instancePath || ''} ${err.message || ''}`;
        const paramsStr = JSON.stringify(err.params || '');
        return TASK_1_TO_3_SURFACE.some(token => path.includes(token) || paramsStr.includes(token));
      });

      if (ourErrors.length > 0) {
        console.error(`Pillar ${pillar} has errors referencing Task 1-3 substrate:`, JSON.stringify(ourErrors, null, 2));
      }
      assert(
        ourErrors.length === 0,
        `Tasks 1-3 substrate additions do not introduce errors for ${pillar} manifest (Task 6 non-regression)`
      );
    }

    assert(
      phantomRefErrors === 0,
      'modular manifest $refs are resolved before validation (no phantom "$ref" errors)'
    );

    // The closure conditional must BITE on the consumer's verdict surface. A gate that
    // handles an event and does not say what its silence means is the exact hole this
    // declaration exists to close, so prove the schema rejects it rather than trusting
    // that it would.
    // Validate against the GateDeclaration $def directly rather than wrapping each case in
    // a whole valid manifest — the unit under test is the gate, and a full-manifest fixture
    // would fail on unrelated required vocabulary and obscure which rule actually fired.
    // The $id deliberately mirrors the manifest's own shape
    // (`epr:schema:manifest:app-manifest`) so the def's relative `../enums/…` ref resolves
    // to the same `epr:enums/*` keys the enums are registered under.
    const gateOnly = regressionAjv.compile({
      $id: 'epr:schema:manifest:gate-declaration-only',
      ...manifestSchema.$defs.GateDeclaration,
    });
    const baseGate = {
      processCid: 'epr:gates:fixture',
      description: 'fixture',
      governanceReach: 'community',
    };
    assert(
      !gateOnly({ ...baseGate, handlesEvents: ['CapabilityInvoke'] }),
      'app-manifest REJECTS an event-handling gate with no closure declaration'
    );
    assert(
      gateOnly({ ...baseGate, handlesEvents: ['CapabilityInvoke'], closure: 'closed' }),
      'app-manifest accepts an event-handling gate that declares its closure'
    );
    assert(
      gateOnly({ ...baseGate, handlesEvents: [] }),
      'app-manifest allows a gate handling no events to omit closure (inert, not a verdict path)'
    );
    assert(
      !gateOnly({ ...baseGate, handlesEvents: ['CapabilityInvoke'], closure: 'maybe' }),
      'app-manifest REJECTS a gate closure outside the shared closure vocabulary'
    );
    assert(
      !gateOnly({ processCid: 'epr:gates:x', handlesEvents: [], governanceReach: 'community' }),
      'app-manifest REJECTS a gate with no description (a verdict surface must say what it does)'
    );
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
