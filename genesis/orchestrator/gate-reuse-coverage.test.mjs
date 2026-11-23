import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { loadGateRegistry } from './pipeline-registry.mjs';

const ROOT = resolve(import.meta.dirname, '../..');
const POLICY = JSON.parse(readFileSync(resolve(ROOT, 'genesis/agentic/pool-policy.json'), 'utf8'));

test('every gate project is either reuse-enabled or withheld with one written reason', () => {
  const registry = loadGateRegistry(ROOT);
  const projects = POLICY.gate_reuse?.projects;
  const withheld = POLICY.gate_reuse?.withheld;

  assert.ok(Array.isArray(projects), 'gate_reuse.projects must be an array');
  assert.ok(withheld && typeof withheld === 'object' && !Array.isArray(withheld),
    'gate_reuse.withheld must be an object');
  assert.equal(new Set(projects).size, projects.length, 'gate_reuse.projects must not contain duplicates');

  const enabled = new Set(projects);
  const held = new Set(Object.keys(withheld));
  for (const name of registry.keys()) {
    assert.notEqual(enabled.has(name), held.has(name),
      `${name} must appear in exactly one of gate_reuse.projects and gate_reuse.withheld`);
  }

  for (const [name, reason] of Object.entries(withheld)) {
    assert.equal(typeof reason, 'string', `${name} withheld reason must be a string`);
    assert.equal(reason, reason.trim(), `${name} withheld reason must not have surrounding whitespace`);
    assert.ok(reason.length > 0, `${name} withheld reason must not be empty`);
    assert.doesNotMatch(reason, /[\r\n]/, `${name} withheld reason must be one line`);
  }

  for (const name of [...enabled, ...held]) {
    assert.ok(registry.has(name), `${name} is named by gate_reuse but is not a registry gate project`);
  }
});
