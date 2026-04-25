#!/usr/bin/env node
/**
 * Tests that the app manifest schema accepts valid manifests and rejects invalid ones.
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

// --- Helpers to build test manifests ---

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

function minimalManifest() {
  return {
    id: 'bafkreiexample',
    name: 'Test App',
    version: '1.0.0',
    vocabulary: {
      contentTypes: {
        article: {
          description: 'A written article',
          coupling: minimalCoupling(),
        },
      },
      observations: minimalObservations(),
    },
  };
}

function fullManifest() {
  return {
    id: 'bafkreifullexample',
    name: 'Full Test App',
    version: '2.1.0',
    description: 'A full manifest with all sections populated.',
    vocabulary: {
      contentTypes: {
        article: {
          description: 'A written article',
          coupling: {
            knowledge: {
              relationships: {
                contains: ['section'],
              },
            },
            value: {
              onConsume: { action: 'use', recognition: 'mastery-credit' },
              onComplete: { action: 'produce', resourceConformsTo: 'completion-record' },
              onContribute: { action: 'produce', recognition: 'stewardship-standing' },
            },
            governance: {
              defaultReach: 'commons',
              minimumReach: 'community',
              governanceModel: 'steward-consent',
              signalTypes: ['view', 'bookmark'],
            },
            claims: [
              { asserts: 'comprehension', contradictedBy: 'comprehension-failure', validityHorizon: 'P30D', leg: 'knowledge' },
            ],
          },
        },
        section: {
          description: 'A section within an article',
          coupling: minimalCoupling(),
        },
      },
      contentFormats: {
        markdown: {
          description: 'Markdown text',
          renderer: 'markdown-renderer',
          mimeTypes: ['text/markdown'],
          extensions: ['md'],
          aliases: ['md'],
        },
      },
      relationships: {
        contains: {
          description: 'Parent contains child',
          sourceTypes: ['article'],
          targetTypes: ['section'],
          inverse: 'belongs_to',
        },
      },
      signals: {
        view: {
          description: 'Learner viewed the content',
          substrateSignal: 'attention',
          economicAction: 'use',
          resourceType: 'attention-minutes',
        },
        bookmark: {
          description: 'Learner bookmarked the content',
          substrateSignal: 'storage',
          economicAction: 'consume',
        },
      },
      observations: minimalObservations(),
    },
    rendering: {
      'markdown-renderer': {
        component: 'elohim-markdown',
        formats: ['markdown'],
        platform: 'web',
      },
    },
  };
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load referenced schemas so AJV can resolve $ref
  // AJV resolves $ref relative to the parent's $id (epr:schema:manifest:app-manifest),
  // so "../enums/substrate-signal.schema.json" becomes "epr:enums/substrate-signal.schema.json"
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

  const schema = await loadJson(
    resolve(__dirname, '../v1/manifest/app-manifest.schema.json'),
  );
  const validate = ajv.compile(schema);

  // 1. Accepts minimal valid manifest
  assert(
    validate(minimalManifest()),
    'accepts minimal valid manifest',
  );

  // 2. Accepts full manifest with all sections
  assert(
    validate(fullManifest()),
    'accepts full manifest with all sections',
  );

  // 3. Rejects manifest missing id
  {
    const m = minimalManifest();
    delete m.id;
    assert(!validate(m), 'rejects manifest missing id');
  }

  // 4. Rejects manifest missing vocabulary
  {
    const m = minimalManifest();
    delete m.vocabulary;
    assert(!validate(m), 'rejects manifest missing vocabulary');
  }

  // 5. Rejects content type without coupling
  {
    const m = minimalManifest();
    m.vocabulary.contentTypes.article = { description: 'No coupling' };
    assert(!validate(m), 'rejects content type without coupling');
  }

  // 6. Rejects coupling without value leg
  {
    const m = minimalManifest();
    m.vocabulary.contentTypes.article.coupling = {
      governance: {
        defaultReach: 'commons',
        minimumReach: 'community',
        governanceModel: 'steward-consent',
      },
    };
    assert(!validate(m), 'rejects coupling without value leg');
  }

  // 7. Rejects coupling without governance leg
  {
    const m = minimalManifest();
    m.vocabulary.contentTypes.article.coupling = {
      value: { onConsume: { action: 'use' } },
    };
    assert(!validate(m), 'rejects coupling without governance leg');
  }

  // 8. Rejects signal with invalid substrate
  {
    const m = minimalManifest();
    m.vocabulary.signals = {
      bad: {
        description: 'Bad signal',
        substrateSignal: 'electricity',
      },
    };
    assert(!validate(m), 'rejects signal with invalid substrate');
  }

  // 9. Should reject content type without claims
  {
    const noClaims = {
      id: 'test', name: 'test', version: '1.0.0',
      vocabulary: {
        contentTypes: {
          thing: {
            description: 'test',
            coupling: {
              value: { onConsume: { action: 'use' } },
              governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' }
            }
          }
        },
        observations: {
          'good-thing': { description: 'test', instrument: 'retention-check', polarity: 'positive' },
          'bad-thing': { description: 'test', instrument: 'retention-check', polarity: 'negative' }
        }
      }
    };
    const valid = validate(noClaims);
    assert(!valid, 'Should reject content type without claims');
  }

  // 10. Should accept content type with valid claims + observations
  {
    const validClaims = {
      id: 'test', name: 'test', version: '1.0.0',
      vocabulary: {
        contentTypes: {
          thing: {
            description: 'test',
            coupling: {
              value: { onConsume: { action: 'use' } },
              governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' },
              claims: [{ asserts: 'good-thing', contradictedBy: 'bad-thing', validityHorizon: 'P30D' }]
            }
          }
        },
        observations: {
          'good-thing': { description: 'test', instrument: 'retention-check', polarity: 'positive' },
          'bad-thing': { description: 'test', instrument: 'retention-check', polarity: 'negative' }
        }
      }
    };
    const valid = validate(validClaims);
    assert(valid, `Should accept content type with valid claims + observations: ${JSON.stringify(validate.errors)}`);
  }

  // 12. Accepts manifest with valid projections array (Batch B — projector mapping)
  {
    const m = minimalManifest();
    m.projections = [
      {
        kind: 'EconomicEvent',
        schemaKey: 'economic-event',
        targetTable: 'economic_events',
        columnMapping: {
          provider: 'payload.provider',
          quantity: 'payload.quantity.numericValue',
        },
      },
    ];
    assert(validate(m), `accepts manifest with valid projections: ${JSON.stringify(validate.errors)}`);
  }

  // 13. Rejects projection with kind not in EprKind enum
  {
    const m = minimalManifest();
    m.projections = [
      {
        kind: 'Squirrel',
        schemaKey: 'economic-event',
        targetTable: 'economic_events',
        columnMapping: { provider: 'payload.provider' },
      },
    ];
    assert(!validate(m), 'rejects projection with kind not in EprKind enum');
  }

  // 14. Rejects projection missing required fields
  {
    const m = minimalManifest();
    m.projections = [{ kind: 'EconomicEvent', schemaKey: 'economic-event' }];
    assert(!validate(m), 'rejects projection missing targetTable + columnMapping');
  }

  // 15. Rejects projection with empty columnMapping
  {
    const m = minimalManifest();
    m.projections = [
      {
        kind: 'EconomicEvent',
        schemaKey: 'economic-event',
        targetTable: 'economic_events',
        columnMapping: {},
      },
    ];
    assert(!validate(m), 'rejects projection with empty columnMapping');
  }

  // 11. Should reject manifest without observations
  {
    const noObservations = {
      id: 'test', name: 'test', version: '1.0.0',
      vocabulary: {
        contentTypes: {
          thing: {
            description: 'test',
            coupling: {
              value: { onConsume: { action: 'use' } },
              governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' },
              claims: [{ asserts: 'good', contradictedBy: 'bad', validityHorizon: 'P30D' }]
            }
          }
        }
      }
    };
    const valid = validate(noObservations);
    assert(!valid, 'Should reject manifest without observations');
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
