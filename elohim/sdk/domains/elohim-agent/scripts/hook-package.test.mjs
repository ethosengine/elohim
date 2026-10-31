import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import Ajv2020 from 'ajv/dist/2020.js';
import {
  hookPackageFromSource,
  projectHook,
  registrationFromSettings,
  registrationsCoherent,
  registrationsFromSettings,
  verifyHookPackage,
} from './hook-package.mjs';

const root = fileURLToPath(new URL('../../../../../', import.meta.url));
const readJson = async (path) => JSON.parse(await readFile(resolve(root, path), 'utf8'));
const native = await readJson('.epr-meta/elohim/packages/hooks/native-seam-guidance.json');
const settings = await readJson('.claude/settings.json');
const codexSettings = await readJson('.codex/hooks.json');

async function verify(pkg, configs = { settings, codexSettings }) {
  const failures = [];
  const findings = [];
  const before = structuredClone(configs);
  await verifyHookPackage(pkg, {
    ...configs,
    assert: (condition, message) => {
      if (!condition) failures.push(message);
    },
    lodge: async (finding) => {
      findings.push(finding);
    },
  });
  assert.deepEqual(configs, before, 'registration verification must never rewrite runtime config');
  return { failures, findings };
}

test('native seam package and both projections retain the current hook bytes', async () => {
  const body = projectHook(native);
  assert.match(body, /epr_client\.resolve_binary\(\)/);
  for (const [runtime, projection] of Object.entries(native.projections)) {
    assert.equal(await readFile(resolve(root, projection.path), 'utf8'), body);
    assert.equal(
      await readFile(
        resolve(root, `.epr-meta/elohim/projections/${runtime}/hooks/native-seam-guidance.py`),
        'utf8',
      ),
      body,
    );
    assert.match(native.registrations[runtime].command, /\|\| exit 0$/);
  }
  assert(
    codexSettings.hooks.PreToolUse.some((group) =>
      group.hooks.some((hook) => hook.command.includes('/.claude/hooks/rust-style.py')),
    ),
  );
  assert.deepEqual(await verify(native), { failures: [], findings: [] });
});

test('hook schema accepts both recorded runtimes and rejects undeclared runtime keys', async () => {
  const schema = await readJson('elohim/sdk/domains/elohim-agent/schemas/hook-package.schema.json');
  const validate = new Ajv2020({ allErrors: true }).compile(schema);
  assert(validate(native), JSON.stringify(validate.errors));
  for (const field of ['projections', 'registrations']) {
    const invalid = structuredClone(native);
    invalid[field].unknown = structuredClone(invalid[field].claude);
    assert.equal(validate(invalid), false, field);
  }
});

test('legacy Claude-only hook packages still verify without a registrations map', async () => {
  const pkg = await readJson('.epr-meta/elohim/packages/hooks/gate-live-inject.json');
  assert.equal(pkg.registrations, undefined);
  assert.deepEqual(await verify(pkg), { failures: [], findings: [] });
});

test('each runtime registration is checked independently and drift is reported read-only', async () => {
  for (const runtime of ['claude', 'codex']) {
    const configs = structuredClone({ settings, codexSettings });
    const config = runtime === 'claude' ? configs.settings : configs.codexSettings;
    const path = native.projections[runtime].path;
    for (const group of config.hooks.PreToolUse) {
      for (const hook of group.hooks) {
        if (hook.command.includes(path)) hook.timeout += 1;
      }
    }
    const result = await verify(native, configs);
    assert.equal(result.failures.length, 1);
    assert.match(result.failures[0], new RegExp(`recorded ${runtime} registration`));
    assert.equal(result.findings.length, 1);
    assert.equal(
      result.findings[0].assertionClass,
      runtime === 'claude' ? 'registration-drift' : 'codex-registration-drift',
    );
  }
});

test('missing runtime records, mismatched targets and divergent legacy aliases refuse', async () => {
  const missing = structuredClone(native);
  delete missing.registrations.codex;
  assert((await verify(missing)).failures.some((message) => message.includes('cover every')));
  const targets = structuredClone(native);
  targets.metadata.runtimeTargets = ['claude'];
  assert((await verify(targets)).failures.some((message) => message.includes('match runtimeTargets')));
  const alias = structuredClone(native);
  alias.registration.timeout += 1;
  assert((await verify(alias)).failures.some((message) => message.includes('legacy Claude')));
});

const fixturePath = '.claude/hooks/synthetic-dialogue-stasis.py';
const fixtureCommand = `python3 ${fixturePath}`;
const fixtureBody = '#!/usr/bin/env python3\n"""Synthetic registration fixture."""\n';
const fixtureSettings = {
  hooks: {
    PostToolUse: [{ hooks: [{ type: 'command', command: fixtureCommand, timeout: 5 }] }],
    Stop: [{ hooks: [{ type: 'command', command: fixtureCommand, timeout: 120, async: true, asyncRewake: true }] }],
  },
};
const importFixture = (config) => hookPackageFromSource(resolve(root, fixturePath), fixtureBody, config, {
  repoRoot: root,
  governance: {},
});

test('all matching registrations are captured in settings order, including async flags', () => {
  const config = structuredClone(fixtureSettings);
  config.hooks.PostToolUse.push({ matcher: 'Bash', hooks: [
    { command: 'echo unrelated' },
    { command: fixtureCommand, async: false, asyncRewake: false },
  ] });
  const matches = registrationsFromSettings(config, fixturePath);
  assert.deepEqual(matches.map(({ event, matcher }) => [event, matcher]), [
    ['PostToolUse', null], ['PostToolUse', 'Bash'], ['Stop', null],
  ]);
  assert.deepEqual(matches.map(({ async, asyncRewake }) => [async, asyncRewake]), [
    [undefined, undefined], [false, false], [true, true],
  ]);
  assert.deepEqual(registrationFromSettings(config, fixturePath), matches[0]);
  assert.equal(registrationsFromSettings(config, '.claude/hooks/absent.py'), null);
  assert.equal(registrationFromSettings({}, fixturePath), null);
  assert.equal(importFixture({}).registration, null);
});

test('a two-registration package is coherent in either order for each runtime', async () => {
  const pkg = importFixture(fixtureSettings);
  assert.equal(pkg.registration.length, 2);
  assert.equal(pkg.metadata.event, 'PostToolUse');
  pkg.metadata.runtimeTargets.push('codex');
  pkg.projections.codex = { path: fixturePath };
  pkg.registrations = {
    claude: [...pkg.registration].reverse(),
    codex: [...pkg.registration].reverse(),
  };
  assert.deepEqual(await verify(pkg, { settings: fixtureSettings, codexSettings: fixtureSettings }), {
    failures: [], findings: [],
  });
  assert.equal(registrationsCoherent(pkg.registration, pkg.registration.slice(0, 1)), false);
  assert.equal(registrationsCoherent(null, pkg.registration), false);
  assert.equal(registrationsCoherent(null, null), true);
});

test('drift in either registration fails, including the SECOND registration', async () => {
  const pkg = importFixture(fixtureSettings);
  for (const event of ['PostToolUse', 'Stop']) {
    const config = structuredClone(fixtureSettings);
    config.hooks[event][0].hooks[0].timeout += 1;
    const result = await verify(pkg, { settings: config });
    assert.equal(result.failures.length, 1, event);
    assert.match(result.failures[0], /recorded claude registration/);
    assert.equal(result.findings[0].assertionClass, 'registration-drift');
  }
});

test('async and asyncRewake differences in the second registration fail', async () => {
  const pkg = importFixture(fixtureSettings);
  for (const flag of ['async', 'asyncRewake']) {
    const config = structuredClone(fixtureSettings);
    config.hooks.Stop[0].hooks[0][flag] = false;
    const result = await verify(pkg, { settings: config });
    assert.equal(result.failures.length, 1, flag);
    assert.equal(result.findings[0].assertionClass, 'registration-drift');
  }
});

test('a legacy single-object recording retains its bytes and matches one live entry', async () => {
  const config = { hooks: { PostToolUse: fixtureSettings.hooks.PostToolUse } };
  const pkg = importFixture(config);
  const legacy = { event: 'PostToolUse', matcher: null, command: fixtureCommand, timeout: 5, type: 'command' };
  assert.equal(JSON.stringify(pkg.registration), JSON.stringify(legacy));
  assert.deepEqual(await verify(pkg, { settings: config }), { failures: [], findings: [] });
  assert(registrationsCoherent(legacy, [{ ...legacy, async: false, asyncRewake: false }]));
  assert(registrationsCoherent([legacy], legacy));
});

test('hook schema accepts object, non-empty array and null in each registration slot', async () => {
  const schema = await readJson('elohim/sdk/domains/elohim-agent/schemas/hook-package.schema.json');
  const validate = new Ajv2020({ allErrors: true }).compile(schema);
  const fixture = importFixture(fixtureSettings);
  for (const value of [fixture.registration[0], fixture.registration, null]) {
    const pkg = structuredClone(native);
    pkg.registration = value;
    pkg.registrations = { claude: value, codex: value };
    assert(validate(pkg), JSON.stringify(validate.errors));
  }
  for (const value of [[], [null], { async: 'true' }, [{ asyncRewake: 1 }]]) {
    for (const slot of ['registration', 'claude', 'codex']) {
      const pkg = structuredClone(native);
      if (slot === 'registration') pkg.registration = value;
      else pkg.registrations[slot] = value;
      assert.equal(validate(pkg), false, `${slot}: ${JSON.stringify(value)}`);
    }
  }
});
