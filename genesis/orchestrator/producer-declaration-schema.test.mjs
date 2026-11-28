import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import test from 'node:test';
import Ajv2020 from 'ajv/dist/2020.js';

const root = resolve(import.meta.dirname, '../..');
const sprint = join(root, 'genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc2');
const fixturePath = (group, name) => join(sprint, 'fixtures', group, name);
const names = group => readdirSync(fixturePath(group, '')).filter(n => n.endsWith('.json')).sort();
const load = path => JSON.parse(readFileSync(path, 'utf8'));
const compile = () => new Ajv2020({ strict: true, allErrors: true }).compile(
  load(join(sprint, 'producer-declaration.draft.schema.json')));

function uniqueClaimIds(data) {
  const seen = new Set();
  return data.claims.flatMap((claim, i) => {
    const duplicate = seen.has(claim.id); seen.add(claim.id);
    return duplicate ? [{ keyword: 'uniqueClaimIds', instancePath: `/claims/${i}/id`, message: 'duplicate claim id' }] : [];
  });
}
function declaredSupersedes(data) {
  const ids = new Set(data.claims.map(c => c.id));
  return data.claims.flatMap((c, i) => c.supersedes.flatMap((id, j) => ids.has(id) ? [] :
    [{ keyword: 'declaredSupersedes', instancePath: `/claims/${i}/supersedes/${j}`, message: 'undeclared predecessor' }]));
}
function declaredCureClaims(data) {
  const ids = new Set(data.claims.map(c => c.id));
  return data.cures.flatMap((c, i) => ids.has(c.claim) ? [] :
    [{ keyword: 'declaredCureClaims', instancePath: `/cures/${i}/claim`, message: 'undeclared cure claim' }]);
}
function uniqueCureEntries(data) {
  const seen = new Set();
  return data.cures.flatMap((c, i) => {
    const key = JSON.stringify([c.claim, c.release]);
    const duplicate = seen.has(key); seen.add(key);
    return duplicate ? [{ keyword: 'uniqueCureEntries', instancePath: `/cures/${i}`, message: 'duplicate claim and release' }] : [];
  });
}
const expected = {
  "cure-missing-release.json": {"keyword": "required", "instancePath": "/cures/0", "params": {"missingProperty": "release"}},
  "cure-missing-state.json": {"keyword": "required", "instancePath": "/cures/0", "params": {"missingProperty": "state"}},
  "cure-state-reopened.json": {"keyword": "enum", "instancePath": "/cures/0/state"},
  "cure-release-not-a-version.json": {"keyword": "pattern", "instancePath": "/cures/0/release"},
  "cure-release-build-metadata.json": {"keyword": "pattern", "instancePath": "/cures/0/release"},
  "cure-empty-cite.json": {"keyword": "minLength", "instancePath": "/cures/0/cite"},
  "cure-empty-summary.json": {"keyword": "minLength", "instancePath": "/cures/0/summary"},
  "cure-duplicate-claim-release.json": {"keyword": "uniqueCureEntries", "instancePath": "/cures/1"},

  "unknown-claim-field.json": {"keyword": "additionalProperties", "instancePath": "/claims/0", "params": {"additionalProperty": "unknown"}},
  "unknown-top-level-field.json": {"keyword": "additionalProperties", "instancePath": "", "params": {"additionalProperty": "unknown"}},
  "severity-outside-list.json": {"keyword": "enum", "instancePath": "/claims/0/severity"},
  "polarity-outside-list.json": {"keyword": "enum", "instancePath": "/claims/0/polarity"},
  "ceiling-outside-list.json": {"keyword": "enum", "instancePath": "/claims/0/ceiling"},
  "not-duration.json": {"keyword": "pattern", "instancePath": "/claims/0/validityHorizon"},
  "duration-bare-p.json": {"keyword": "pattern", "instancePath": "/claims/0/validityHorizon"},
  "duration-garbage.json": {"keyword": "pattern", "instancePath": "/claims/0/validityHorizon"},
  "claim-id-grammar.json": {"keyword": "pattern", "instancePath": "/claims/1/id"},
  "claim-id-over-128-bytes.json": {"keyword": "maxLength", "instancePath": "/claims/1/id"},
  "clears-outside-list.json": {"keyword": "pattern", "instancePath": "/claims/1/clears/0"},
  "clears-missing-parameter.json": {"keyword": "pattern", "instancePath": "/claims/1/clears/0"},
  "negative-weight.json": {"keyword": "minimum", "instancePath": "/claims/0/weight"},
  "weight-over-u8.json": {"keyword": "maximum", "instancePath": "/claims/0/weight"},
  "cure-missing-summary.json": {"keyword": "required", "instancePath": "/cures/0", "params": {"missingProperty": "summary"}},
  "unknown-cure-field.json": {"keyword": "additionalProperties", "instancePath": "/cures/0", "params": {"additionalProperty": "unknown"}},
  "duplicate-id.json": {"keyword": "uniqueClaimIds", "instancePath": "/claims/1/id"},
  "supersedes-undeclared-id.json": {"keyword": "declaredSupersedes", "instancePath": "/claims/1/supersedes/0"},
  "cure-undeclared-id.json": {"keyword": "declaredCureClaims", "instancePath": "/cures/0/claim"},
  "cure-id-grammar.json": {"keyword": "pattern", "instancePath": "/cures/0/claim"},
  "supersedes-id-grammar.json": {"keyword": "pattern", "instancePath": "/claims/1/supersedes/0"},
  "retire-when-invalid.json": {"keyword": "pattern", "instancePath": "/claims/0/retireWhen"},
  "retire-when-omitted.json": {"keyword": "required", "instancePath": "/claims/0", "params": {"missingProperty": "retireWhen"}},
  "cure-null.json": {"keyword": "type", "instancePath": "/claims/0/cure"},
};

const rules = [uniqueClaimIds, declaredSupersedes, declaredCureClaims, uniqueCureEntries];
function check(data, validate, checks = rules) {
  if (!validate(data)) return validate.errors;
  return checks.flatMap(rule => rule(data));
}

test('the declaration schema compiles in strict mode', () => {
  assert.doesNotThrow(compile, `schema must exist and compile at ${sprint}`);
});

test('every accepted fixture passes the schema and script rules', () => {
  const validate = compile();
  const files = names('accepted');
  assert.ok(files.length > 0, `no accepted fixtures at ${sprint}`);
  for (const name of files) {
    const errors = check(load(fixturePath('accepted', name)), validate);
    assert.deepEqual(errors, [], `${name}: ${JSON.stringify(errors)}`);
  }
});

test('brit output passes and retains its recorded SHA-256', () => {
  const name = 'brit-0.3.10-a99143327.claims.json';
  const bytes = readFileSync(fixturePath('accepted', name));
  const errors = check(JSON.parse(bytes), compile());
  assert.deepEqual(errors, [], `${name}: ${JSON.stringify(errors)}`);
  assert.equal(createHash('sha256').update(bytes).digest('hex'),
    'b30fd3017fb24bf8eedd602d17b93a21f7309a387aee87f40315d5845308d5ea', name);
});

test('every refused fixture fails once for its named reason', () => {
  const validate = compile();
  for (const name of names('refused')) {
    const errors = check(load(fixturePath('refused', name)), validate);
    const message = `${name}: ${JSON.stringify(errors)}`;
    const want = expected[name];
    assert.ok(want, `no expected reason for ${name}`);
    assert.equal(errors.length, 1, message);
    assert.equal(errors[0].keyword, want.keyword, message);
    assert.equal(errors[0].instancePath, want.instancePath, message);
    for (const [key, value] of Object.entries(want.params ?? {})) {
      assert.deepEqual(errors[0].params?.[key], value, message);
    }
  }
});

test('refused fixtures and expected reasons name exactly the same files', () => {
  assert.deepEqual(names('refused'), Object.keys(expected).sort(), `refused fixtures at ${sprint}`);
});

test('each script rule refuses its fixture and runs only after schema acceptance', () => {
  const validate = compile();
  const ownFixtures = ['duplicate-id.json', 'supersedes-undeclared-id.json', 'cure-undeclared-id.json', 'cure-duplicate-claim-release.json'];
  for (const [i, rule] of rules.entries()) {
    const name = ownFixtures[i];
    const data = load(fixturePath('refused', name));
    assert.equal(validate(data), true, `${name}: ${JSON.stringify(validate.errors)}`);
    assert.equal(rule(data).length, 1, name);
    assert.equal(rule(data)[0].keyword, rule.name, name);
    for (const other of rules.filter(candidate => candidate !== rule)) {
      assert.deepEqual(other(data), [], `${name}: must not fail ${other.name}`);
    }
  }
  let schemaRefusals = 0;
  for (const name of names('refused')) {
    const data = load(fixturePath('refused', name));
    if (validate(data)) continue;
    schemaRefusals++;
    const calls = [];
    const errors = check(data, validate, rules.map(rule => value => {
      calls.push(rule.name);
      return rule(value);
    }));
    assert.ok(errors.length > 0, name);
    assert.deepEqual(calls, [], `${name}: schema rejection must stop script rules`);
  }
  assert.ok(schemaRefusals > 0, 'at least one fixture must exercise schema rejection');
});
