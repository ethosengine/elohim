import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const data = JSON.parse(
  readFileSync('genesis/agentic/data/safety-taxonomy.json', 'utf8'),
);

test('has three tiers', () => {
  assert.ok(Array.isArray(data.broadly_safe), 'broadly_safe is array');
  assert.ok(Array.isArray(data.subcommand_scoped), 'subcommand_scoped is array');
  assert.ok(Array.isArray(data.never_wildcard), 'never_wildcard is array');
});

test('broadly_safe entries are command names', () => {
  for (const entry of data.broadly_safe) {
    assert.equal(typeof entry, 'string');
    assert.match(entry, /^[a-z][a-z0-9-]*$/, `${entry} is a command name`);
  }
});

test('subcommand_scoped entries have safe/prompt lists', () => {
  for (const entry of data.subcommand_scoped) {
    assert.equal(typeof entry.command, 'string');
    assert.ok(Array.isArray(entry.safe_subcommands));
    assert.ok(Array.isArray(entry.prompt_subcommands));
  }
});

test('never_wildcard has rationale per entry', () => {
  for (const entry of data.never_wildcard) {
    assert.equal(typeof entry.command, 'string');
    assert.equal(typeof entry.reason, 'string');
    assert.ok(entry.reason.length > 10);
  }
});

test('required commands present', () => {
  // Spec-mandated entries
  assert.ok(data.broadly_safe.includes('cargo'), 'cargo broadly safe');
  assert.ok(data.broadly_safe.includes('pnpm'), 'pnpm broadly safe');
  assert.ok(data.broadly_safe.includes('vitest'), 'vitest broadly safe');
  assert.ok(
    data.subcommand_scoped.find((e) => e.command === 'git'),
    'git is subcommand-scoped',
  );
  assert.ok(
    data.never_wildcard.find((e) => e.command === 'rm'),
    'rm is never wildcard',
  );
});
