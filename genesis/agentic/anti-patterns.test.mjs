import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const data = JSON.parse(
  readFileSync('genesis/agentic/data/anti-patterns.json', 'utf8'),
);

test('catalog is an array', () => {
  assert.ok(Array.isArray(data.patterns));
  assert.ok(data.patterns.length >= 5, 'at least 5 seed patterns');
});

test('each pattern has required fields', () => {
  for (const p of data.patterns) {
    assert.equal(typeof p.id, 'string');
    assert.match(p.id, /^AP-[0-9]{3}$/, `${p.id} matches AP-NNN`);
    assert.equal(typeof p.name, 'string');
    assert.equal(typeof p.description, 'string');
    assert.ok(Array.isArray(p.detection_hints));
    assert.ok(p.detection_hints.length >= 1);
    assert.equal(typeof p.attestation_maps_to, 'string');
  }
});

test('ids unique', () => {
  const ids = data.patterns.map((p) => p.id);
  assert.equal(new Set(ids).size, ids.length, 'pattern ids must be unique');
});
