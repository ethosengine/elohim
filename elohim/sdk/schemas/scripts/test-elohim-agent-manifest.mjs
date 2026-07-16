#!/usr/bin/env node
/**
 * Validates the elohim-agent app manifest and its metadata schemas.
 */
import { readFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SDK_ROOT = resolve(__dirname, '../..');
const DOMAIN_DIR = resolve(SDK_ROOT, 'domains/elohim-agent');

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

async function loadAppManifestValidator() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  const referencedSchemas = [
    ['../v1/enums/substrate-signal.schema.json', 'epr:enums/substrate-signal.schema.json'],
    ['../v1/enums/instrument-archetype.schema.json', 'epr:enums/instrument-archetype.schema.json'],
    ['../v1/enums/observation-polarity.schema.json', 'epr:enums/observation-polarity.schema.json'],
    ['../v1/enums/epr-kind.schema.json', 'epr:enums/epr-kind.schema.json'],
    ['../v1/manifest/pillar-projection.schema.json', 'epr:pillar-projection.schema.json'],
    ['../v1/manifest/observation-kind.schema.json', 'epr:observation-kind.schema.json'],
    [
      '../v1/enums/session-lifecycle-state.schema.json',
      'epr:enums/session-lifecycle-state.schema.json',
    ],
  ];

  for (const [relativePath, schemaId] of referencedSchemas) {
    ajv.addSchema(await loadJson(resolve(__dirname, relativePath)), schemaId);
  }

  const schema = await loadJson(resolve(__dirname, '../v1/manifest/app-manifest.schema.json'));
  return ajv.compile(schema);
}

async function resolveContentTypes(manifest) {
  const resolved = structuredClone(manifest);
  const contentTypes = resolved.vocabulary?.contentTypes ?? {};

  for (const [name, declaration] of Object.entries(contentTypes)) {
    if (
      declaration &&
      typeof declaration === 'object' &&
      Object.keys(declaration).length === 1 &&
      typeof declaration.$ref === 'string'
    ) {
      contentTypes[name] = await loadJson(resolve(DOMAIN_DIR, declaration.$ref));
    }
  }

  return resolved;
}

const expectedContentTypes = [
  'skill',
  'agent',
  'agent-spec',
  'hook',
  'event',
  'signal',
  'counter',
  'policy',
  'validator',
  'mcp-server',
  'mcp-profile',
  'projection-binding',
];

const metadataFixtures = {
  skill: {
    schema: 'skill-metadata.schema.json',
    missing: 'triggerDescription',
    fixture: {
      id: 'skill.review',
      version: '1.0.0',
      description: 'Review local changes.',
      triggerDescription: 'Use when asked to review a patch.',
    },
  },
  agent: {
    schema: 'agent-metadata.schema.json',
    missing: 'role',
    fixture: {
      id: 'agent.reviewer',
      version: '1.0.0',
      description: 'Reviews changes for correctness.',
      role: 'code-reviewer',
    },
  },
  'agent-spec': {
    schema: 'agent-spec-metadata.schema.json',
    missing: 'appliesTo',
    fixture: {
      id: 'agent-spec.repo',
      version: '1.0.0',
      description: 'Repository standing instructions.',
      scope: 'repository',
      appliesTo: ['.'],
    },
  },
  hook: {
    schema: 'hook-metadata.schema.json',
    missing: 'authorityClass',
    fixture: {
      id: 'hook.pre-write',
      version: '1.0.0',
      description: 'Checks a file before writing.',
      phase: 'pre-write',
      matcher: '**/*.ts',
      authorityClass: 'local-write',
    },
  },
  event: {
    schema: 'event-metadata.schema.json',
    missing: 'subjectRef',
    fixture: {
      id: 'event.file-write',
      version: '1.0.0',
      description: 'A file write occurred.',
      eventType: 'file-write',
      subjectRef: 'epr:example',
    },
  },
  signal: {
    schema: 'signal-metadata.schema.json',
    missing: 'sourceRefs',
    fixture: {
      id: 'signal.policy-risk',
      version: '1.0.0',
      description: 'Interprets event evidence as policy risk.',
      signalType: 'policy-risk',
      sourceRefs: ['event.file-write'],
    },
  },
  counter: {
    schema: 'counter-metadata.schema.json',
    missing: 'unit',
    fixture: {
      id: 'counter.tool-calls',
      version: '1.0.0',
      description: 'Counts tool invocations.',
      counterType: 'increment',
      unit: 'call',
    },
  },
  policy: {
    schema: 'policy-metadata.schema.json',
    missing: 'status',
    fixture: {
      id: 'policy.no-secrets',
      version: '1.0.0',
      description: 'Prevents secret exposure.',
      bindingClass: 'artifact',
      status: 'active',
    },
  },
  validator: {
    schema: 'validator-metadata.schema.json',
    missing: 'fuelDefault',
    fixture: {
      id: 'validator.path-match',
      version: '1.0.0',
      description: 'Matches paths deterministically.',
      validatorType: 'json-rule',
      fuelDefault: 1000,
    },
  },
  'mcp-server': {
    schema: 'mcp-server-metadata.schema.json',
    missing: 'transport',
    fixture: {
      id: 'jenkins',
      version: '1.0.0',
      description: 'Read-only Jenkins MCP server.',
      transport: 'streamable-http',
    },
  },
  'mcp-profile': {
    schema: 'mcp-profile-metadata.schema.json',
    missing: 'serverRefs',
    fixture: {
      id: 'elohim-project',
      version: '1.0.0',
      description: 'Project MCP activation profile.',
      scope: 'project',
      serverRefs: ['jenkins'],
    },
  },
  'projection-binding': {
    schema: 'projection-binding-metadata.schema.json',
    missing: 'sourceType',
    fixture: {
      id: 'projection.codex-skill',
      version: '1.0.0',
      description: 'Projects a skill into a Codex skill directory.',
      runtime: 'codex',
      targetPattern: '.codex/skills/{id}',
      sourceType: 'skill',
    },
  },
};

async function testManifest() {
  const manifest = await loadJson(resolve(DOMAIN_DIR, 'manifest.json'));
  const contentTypeNames = Object.keys(manifest.vocabulary.contentTypes).sort();
  assert(
    JSON.stringify(contentTypeNames) === JSON.stringify([...expectedContentTypes].sort()),
    'elohim-agent manifest declares the V1 content type vocabulary',
  );

  const resolvedManifest = await resolveContentTypes(manifest);
  const validate = await loadAppManifestValidator();
  assert(
    validate(resolvedManifest),
    `resolved elohim-agent manifest validates against app-manifest.schema.json: ${JSON.stringify(validate.errors)}`,
  );
}

async function testMetadataSchemas() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });

  for (const [name, { schema, missing, fixture }] of Object.entries(metadataFixtures)) {
    const schemaPath = resolve(DOMAIN_DIR, 'schemas', schema);
    const validate = ajv.compile(await loadJson(schemaPath));

    assert(
      validate(fixture),
      `${name} metadata schema accepts minimal fixture: ${JSON.stringify(validate.errors)}`,
    );

    const invalid = structuredClone(fixture);
    delete invalid[missing];
    assert(!validate(invalid), `${name} metadata schema rejects missing ${missing}`);
  }
}

async function main() {
  await testManifest();
  await testMetadataSchemas();

  console.log(`\n${passes} passed, ${failures} failed`);
  process.exit(failures > 0 ? 1 : 0);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
