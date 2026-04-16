import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clusterAndPropose } from './generalize.mjs';
import { readFileSync } from 'node:fs';

const taxonomy = JSON.parse(
  readFileSync('genesis/agentic/data/safety-taxonomy.json', 'utf8'),
);

test('collapses RUSTFLAGS cargo variants into one pattern', () => {
  const entries = [
    'Bash(RUSTFLAGS="" cargo check)',
    'Bash(RUSTFLAGS="" cargo build)',
    'Bash(RUSTFLAGS="" cargo test --workspace)',
    'Bash(RUSTFLAGS="" cargo clippy -p brit-epr)',
    'Bash(RUSTFLAGS="" cargo clippy -p brit-verify)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  // Should propose one pattern covering all 5.
  const cargoCluster = proposals.find((p) =>
    p.proposed.includes('RUSTFLAGS') && p.proposed.includes('cargo'),
  );
  assert.ok(cargoCluster, 'cargo cluster produced');
  assert.equal(cargoCluster.absorbs.length, 5);
  assert.equal(cargoCluster.safety, 'broadly_safe');
});

test('does not generalize across never-wildcard commands', () => {
  const entries = [
    'Bash(rm tmp/a.log)',
    'Bash(rm tmp/b.log)',
    'Bash(rm tmp/c.log)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  // rm is never-wildcard — no generalization should be proposed.
  const rmCluster = proposals.find((p) => p.proposed.startsWith('Bash(rm'));
  assert.equal(rmCluster, undefined, 'rm not generalized');
});

test('respects subcommand-scope boundaries for git', () => {
  const entries = [
    'Bash(git add .claude/shifts/x)',
    'Bash(git add .claude/shifts/y)',
    'Bash(git add genesis/a2o/foo.feature)',
    'Bash(git push origin dev)',
    'Bash(git push origin main)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  // git add is safe → one generalized pattern proposed.
  const addCluster = proposals.find(
    (p) => p.proposed.includes('git add') && p.proposed.endsWith(' *)'),
  );
  assert.ok(addCluster, 'git add generalized');
  // git push is prompt-only → should NOT be auto-generalized at this tier.
  const pushCluster = proposals.find(
    (p) => p.proposed.includes('git push') && p.proposed.endsWith(' *)'),
  );
  assert.equal(pushCluster, undefined, 'git push not auto-generalized');
});

test('singletons pass through unchanged (no collapse opportunity)', () => {
  const entries = ['Bash(echo hello)'];
  const proposals = clusterAndPropose(entries, taxonomy);
  // Singletons produce no generalization proposal.
  assert.equal(proposals.length, 0);
});

test('proposal includes absorbed entries for user review', () => {
  const entries = [
    'Bash(pnpm run test)',
    'Bash(pnpm run test:unit)',
    'Bash(pnpm run test:e2e)',
  ];
  const proposals = clusterAndPropose(entries, taxonomy);
  const pnpmCluster = proposals.find((p) => p.proposed.includes('pnpm'));
  assert.ok(pnpmCluster);
  assert.ok(pnpmCluster.absorbs.includes('Bash(pnpm run test)'));
  assert.ok(pnpmCluster.absorbs.includes('Bash(pnpm run test:unit)'));
});
