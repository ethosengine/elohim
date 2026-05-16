#!/usr/bin/env node
/**
 * Tests that the app-manifest schema correctly validates the "graph" extension section
 * (per 2026-05-16 graph-native-projection-substrate spec, Tasks 15-18).
 *
 * Follows the pattern of test-manifest-schema.mjs: plain Node.js + AJV 2020, no Vitest.
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

// Minimal valid manifest (must satisfy id/name/version/vocabulary required fields)
function minimalManifest() {
  return {
    id: 'bafkreiexample',
    name: 'test-domain',
    version: '1.0.0',
    vocabulary: {
      contentTypes: {
        concept: {
          description: 'A learning concept',
          coupling: {
            value: { onConsume: { action: 'use' } },
            governance: { defaultReach: 'commons', minimumReach: 'community', governanceModel: 'steward-consent' },
            claims: [{ asserts: 'comprehension', contradictedBy: 'comprehension-failure', validityHorizon: 'P30D' }],
          },
        },
      },
      observations: {
        comprehension: { description: 'Learner demonstrated comprehension', instrument: 'retention-check', polarity: 'positive' },
        'comprehension-failure': { description: 'Learner failed to demonstrate comprehension', instrument: 'retention-check', polarity: 'negative' },
      },
    },
  };
}

async function main() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  // Load referenced schemas so AJV can resolve $ref (same pattern as test-manifest-schema.mjs)
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

  const schema = await loadJson(
    resolve(__dirname, '../v1/manifest/app-manifest.schema.json'),
  );
  const validate = ajv.compile(schema);

  // 1. Accepts manifest without graph section (graph is optional)
  {
    const m = minimalManifest();
    assert(
      validate(m),
      `accepts manifest without graph section: ${JSON.stringify(validate.errors)}`,
    );
  }

  // 2. Accepts manifest with a valid full graph section
  {
    const m = minimalManifest();
    m.graph = {
      edges: [
        {
          type: 'PREREQUISITE',
          from: 'EprHead',
          to: 'EprHead',
          indexed: true,
          directional: true,
          description: 'Content prerequisite relationship',
        },
      ],
      nodes: [
        {
          type: 'LearningPath',
          properties: { sequence_index: 'Int', completion_rate: 'Float' },
        },
      ],
      indexes: [
        {
          name: 'prereq_forward',
          on: 'epr_edge',
          where: "rel_type = 'PREREQUISITE'",
        },
      ],
      rules: [
        {
          name: 'prerequisite_chain',
          description: 'Recursive prerequisite traversal',
          datalog:
            "prerequisite_chain[to, hops] := *epr_edge{from_cid: $start, to_cid: to, rel_type: 'PREREQUISITE'}, hops = 1",
        },
      ],
    };
    assert(
      validate(m),
      `accepts manifest with valid full graph section: ${JSON.stringify(validate.errors)}`,
    );
  }

  // 3. Accepts manifest with empty graph arrays (all fields optional within graph)
  {
    const m = minimalManifest();
    m.graph = { edges: [], nodes: [], indexes: [], rules: [] };
    assert(
      validate(m),
      `accepts manifest with empty graph arrays: ${JSON.stringify(validate.errors)}`,
    );
  }

  // 4. Accepts manifest with only graph.edges (partial graph section)
  {
    const m = minimalManifest();
    m.graph = {
      edges: [{ type: 'RELATED_TO', from: 'EprHead', to: 'EprHead' }],
    };
    assert(
      validate(m),
      `accepts manifest with only graph.edges: ${JSON.stringify(validate.errors)}`,
    );
  }

  // 5. Rejects edge missing required "type" field
  {
    const m = minimalManifest();
    m.graph = {
      edges: [{ from: 'EprHead', to: 'EprHead' }],
    };
    assert(
      !validate(m),
      'rejects edge missing required "type" field',
    );
  }

  // 6. Rejects edge missing required "from" field
  {
    const m = minimalManifest();
    m.graph = {
      edges: [{ type: 'PREREQUISITE', to: 'EprHead' }],
    };
    assert(
      !validate(m),
      'rejects edge missing required "from" field',
    );
  }

  // 7. Rejects edge missing required "to" field
  {
    const m = minimalManifest();
    m.graph = {
      edges: [{ type: 'PREREQUISITE', from: 'EprHead' }],
    };
    assert(
      !validate(m),
      'rejects edge missing required "to" field',
    );
  }

  // 8. Rejects edge with unknown additional property
  {
    const m = minimalManifest();
    m.graph = {
      edges: [{ type: 'PREREQUISITE', from: 'EprHead', to: 'EprHead', bogusField: true }],
    };
    assert(
      !validate(m),
      'rejects edge with unknown additional property',
    );
  }

  // 9. Rejects node missing required "type" field
  {
    const m = minimalManifest();
    m.graph = {
      nodes: [{ properties: { count: 'Int' } }],
    };
    assert(
      !validate(m),
      'rejects node missing required "type" field',
    );
  }

  // 10. Rejects node missing required "properties" field
  {
    const m = minimalManifest();
    m.graph = {
      nodes: [{ type: 'LearningPath' }],
    };
    assert(
      !validate(m),
      'rejects node missing required "properties" field',
    );
  }

  // 11. Rejects index missing required "name" field
  {
    const m = minimalManifest();
    m.graph = {
      indexes: [{ on: 'epr_edge' }],
    };
    assert(
      !validate(m),
      'rejects index missing required "name" field',
    );
  }

  // 12. Rejects index missing required "on" field
  {
    const m = minimalManifest();
    m.graph = {
      indexes: [{ name: 'my_index' }],
    };
    assert(
      !validate(m),
      'rejects index missing required "on" field',
    );
  }

  // 13. Rejects rule missing required "name" field
  {
    const m = minimalManifest();
    m.graph = {
      rules: [{ datalog: 'rule_body[x] := *epr_node{cid: x}' }],
    };
    assert(
      !validate(m),
      'rejects rule missing required "name" field',
    );
  }

  // 14. Rejects rule missing required "datalog" field
  {
    const m = minimalManifest();
    m.graph = {
      rules: [{ name: 'my_rule' }],
    };
    assert(
      !validate(m),
      'rejects rule missing required "datalog" field',
    );
  }

  // 15. Rejects graph with unknown top-level property
  {
    const m = minimalManifest();
    m.graph = {
      edges: [],
      unknownSection: {},
    };
    assert(
      !validate(m),
      'rejects graph with unknown top-level property',
    );
  }

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
