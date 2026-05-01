#!/usr/bin/env node
/**
 * Tests that manifest-epr.schema.json enforces constitutional floor sub-schemas
 * (§2.8 trust-compute gradient brainstorm) for standing-policy and tending-policy
 * manifestKind values.
 *
 * See: elohim/sdk/schemas/v1/manifest/standing-policy-floor.schema.json
 *      elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json
 */
import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const MANIFEST_DIR = resolve(__dirname, '../v1/manifest');
const ENUM_DIR = resolve(__dirname, '../v1/enums');

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

// --- Fixture builders ---

function minimalEnvelope(reach = 'commons') {
  return {
    kind: 'Manifest',
    reach,
    signerPubkey: 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=',
    createdAt: '2026-05-01T00:00:00Z',
    signature: 'AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
  };
}

function standingFloorClasses() {
  return [
    {
      class: 'local-relationship-reach',
      protection: 'Family and household propagation is never gated by standing gradient.',
    },
    {
      class: 'cid-targeted-lookup',
      protection: 'Anyone holding a CID can fetch the addressed content; standing modulates discovery, not retrievability.',
    },
    {
      class: 'constitutional-floor-signatures',
      protection: 'Mishpat decisions and imagodei-as-counsel signatures are always per-message verified regardless of author standing.',
    },
    {
      class: 'new-voice-baseline',
      protection: 'First-time authors receive a nonzero standing floor derived from their sponsor; no agent starts at zero.',
    },
    {
      class: 'vulnerable-class-elevation',
      protection: 'Children, refugees, persecution-targets, and displaced persons receive elevated baseline floor protections.',
      appliesWhen: 'Agent profile includes a recognized vulnerable-class attestation.',
    },
  ];
}

function tendingFloorClasses() {
  return [
    {
      class: 'accountability-information',
      protection: 'Corrections, peer-review, and restitution requests targeting the receiving agent bypass all attention-tending filters.',
    },
    {
      class: 'community-facts',
      protection: 'Civic emergencies, public-health information, and governance decisions are never filterable.',
    },
    {
      class: 'custodial-communications',
      protection: 'Under stewardship-graduated authority, ward-targeting messages bypass the steward filter entirely.',
      appliesWhen: 'Active stewardship contract exists between sender and ward.',
    },
    {
      class: 'constitutional-updates',
      protection: 'Mishpat rule changes, qahal ratifications, and charter amendments always reach their recipients.',
    },
    {
      class: 'elohim-as-counsel-notifications',
      protection: "An agent's own elohim has standing to override filters when its judgment requires surfacing information to the agent.",
    },
  ];
}

function validStandingPolicyPayload() {
  return {
    manifestKind: 'standing-policy',
    revision: 1,
    floor: { classes: standingFloorClasses() },
    debitWeights: {
      squelch: { advisory: 0, 'debit-soft': 1, 'debit-firm': 3 },
      correction: { advisory: 0, 'debit-soft': 10, 'debit-firm': 20 },
      retraction: { advisory: 0, 'debit-soft': -5, 'debit-firm': -10 },
      quarantine: { advisory: 0, 'debit-soft': 12, 'debit-firm': 30 },
      vouch: { advisory: 0, 'debit-soft': -3, 'debit-firm': -8 },
    },
  };
}

function validTendingPolicyPayload() {
  return {
    manifestKind: 'tending-policy',
    revision: 1,
    floor: { classes: tendingFloorClasses() },
  };
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load and register all enum schemas (needed for reach $ref)
  const { readdir } = await import('node:fs/promises');
  const enumFiles = (await readdir(ENUM_DIR)).filter((f) => f.endsWith('.schema.json'));
  for (const file of enumFiles) {
    const schema = await loadJson(resolve(ENUM_DIR, file));
    if (schema.$id) ajv.addSchema(schema);
  }

  // Load and register floor sub-schemas
  const standingFloorSchema = await loadJson(resolve(MANIFEST_DIR, 'standing-policy-floor.schema.json'));
  ajv.addSchema(standingFloorSchema);

  const tendingFloorSchema = await loadJson(resolve(MANIFEST_DIR, 'tending-policy-floor.schema.json'));
  ajv.addSchema(tendingFloorSchema);

  // Load manifest-epr schema with $ref resolution
  const manifestEprRaw = await loadJson(resolve(MANIFEST_DIR, 'manifest-epr.schema.json'));

  // Resolve relative $ref values to $id URIs for AJV
  function resolveRefs(schema, baseDir) {
    if (typeof schema !== 'object' || schema === null) return schema;
    if (Array.isArray(schema)) return schema.map((item) => resolveRefs(item, baseDir));
    const result = {};
    for (const [key, value] of Object.entries(schema)) {
      if (key === '$ref' && typeof value === 'string' && !value.startsWith('#') && !value.startsWith('epr:')) {
        // Map relative file paths to $id URIs
        const refMap = {
          '../enums/reach.schema.json': 'epr:schema:enum:reach',
          'standing-policy-floor.schema.json': 'epr:schema:manifest:standing-policy-floor',
          'tending-policy-floor.schema.json': 'epr:schema:manifest:tending-policy-floor',
        };
        result[key] = refMap[value] ?? value;
      } else {
        result[key] = resolveRefs(value, baseDir);
      }
    }
    return result;
  }

  const manifestEprSchema = resolveRefs(manifestEprRaw, MANIFEST_DIR);
  const validate = ajv.compile(manifestEprSchema);

  // 1. Valid standing-policy envelope with full floor
  assert(
    validate({ envelope: minimalEnvelope(), payload: validStandingPolicyPayload() }),
    'accepts valid standing-policy envelope with full floor',
  );

  // 2. Valid tending-policy envelope with full floor
  assert(
    validate({ envelope: minimalEnvelope(), payload: validTendingPolicyPayload() }),
    'accepts valid tending-policy envelope with full floor',
  );

  // 3. Invalid: standing-policy payload missing floor field
  assert(
    !validate({
      envelope: minimalEnvelope(),
      payload: { manifestKind: 'standing-policy', revision: 1 },
    }),
    'rejects standing-policy payload missing floor field',
  );

  // 4. Invalid: tending-policy payload missing floor field
  assert(
    !validate({
      envelope: minimalEnvelope(),
      payload: { manifestKind: 'tending-policy', revision: 1 },
    }),
    'rejects tending-policy payload missing floor field',
  );

  // 5. Invalid: standing-policy floor with only 4 classes (minItems: 5 violated)
  {
    const payload = validStandingPolicyPayload();
    payload.floor.classes = payload.floor.classes.slice(0, 4);
    assert(
      !validate({ envelope: minimalEnvelope(), payload }),
      'rejects standing-policy floor with only 4 classes (minItems: 5 required)',
    );
  }

  // 6. Invalid: tending-policy floor with only 4 classes
  {
    const payload = validTendingPolicyPayload();
    payload.floor.classes = payload.floor.classes.slice(0, 4);
    assert(
      !validate({ envelope: minimalEnvelope(), payload }),
      'rejects tending-policy floor with only 4 classes (minItems: 5 required)',
    );
  }

  // 7. Invalid: standing-policy floor with unknown class value
  {
    const payload = validStandingPolicyPayload();
    payload.floor.classes[0] = { class: 'not-a-valid-class', protection: 'Bad class' };
    assert(
      !validate({ envelope: minimalEnvelope(), payload }),
      'rejects standing-policy floor with unknown class value',
    );
  }

  // 8. Invalid: tending-policy floor with unknown class value
  {
    const payload = validTendingPolicyPayload();
    payload.floor.classes[0] = { class: 'not-a-valid-class', protection: 'Bad class' };
    assert(
      !validate({ envelope: minimalEnvelope(), payload }),
      'rejects tending-policy floor with unknown class value',
    );
  }

  // 9. Invalid: standing-policy floor class missing protection field
  {
    const payload = validStandingPolicyPayload();
    // Remove protection from first class
    const { protection: _p, ...classWithoutProtection } = payload.floor.classes[0];
    payload.floor.classes[0] = classWithoutProtection;
    assert(
      !validate({ envelope: minimalEnvelope(), payload }),
      'rejects standing-policy floor class missing protection field',
    );
  }

  // 10. Other manifestKind values (app, pillar-projection, onboarding) do NOT require floor
  assert(
    validate({
      envelope: minimalEnvelope(),
      payload: { manifestKind: 'app', revision: 1 },
    }),
    'accepts app manifestKind without floor field',
  );

  assert(
    validate({
      envelope: minimalEnvelope(),
      payload: {
        manifestKind: 'pillar-projection',
        revision: 1,
        pillar: 'lamad',
        kinds: ['EconomicEvent'],
      },
    }),
    'accepts pillar-projection manifestKind without floor field',
  );

  assert(
    validate({
      envelope: minimalEnvelope(),
      payload: { manifestKind: 'onboarding', revision: 1 },
    }),
    'accepts onboarding manifestKind without floor field',
  );

  // 11. Invalid: duplicate classes in standing-policy floor (uniqueItems violated)
  {
    const payload = validStandingPolicyPayload();
    // Duplicate the first class to create a 5-item array with a duplicate
    payload.floor.classes[4] = { ...payload.floor.classes[0] };
    assert(
      !validate({ envelope: minimalEnvelope(), payload }),
      'rejects standing-policy floor with duplicate class entries (uniqueItems)',
    );
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
