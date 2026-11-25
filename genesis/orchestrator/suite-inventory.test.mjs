import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { closureDigest } from './suite-inventory.mjs';

test('package digest traversal stops before doing work after its deadline check fails', t => {
  const root = mkdtempSync(join(tmpdir(), 'suite-inventory-budget-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const pkg = join(root, 'package');
  mkdirSync(pkg);
  writeFileSync(join(pkg, 'a.js'), 'a');
  writeFileSync(join(pkg, 'b.js'), 'b');
  writeFileSync(join(pkg, 'c.js'), 'c');

  let checks = 0;
  const read = [];
  const deadline = new Error('budget 1 ms exhausted in read-shadow traversal');
  assert.throws(() => closureDigest({ packages: ['package'], links: [] }, root, {
    check: () => {
      checks++;
      if (checks === 3) throw deadline;
    },
    readFile: path => {
      read.push(path);
      return readFileSync(path);
    },
  }), error => error === deadline);

  assert.equal(checks, 3);
  assert.deepEqual(read.map(path => path.slice(pkg.length + 1)), ['a.js'],
    'the entry whose deadline check failed and every later entry do no hashing work');
});
