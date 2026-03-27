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

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
