import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import {
  extractCargoTargets,
  candidateTargetPaths,
  buildManifestMatchers,
  findUncoveredTargets,
} from './cargo-target-coverage.mjs';

// ── extractCargoTargets ──────────────────────────────────────────

describe('extractCargoTargets', () => {
  it('extracts a bench with default path', () => {
    const toml = `
[package]
name = "x"
version = "0.1.0"

[[bench]]
name = "graph_traversal"
harness = false
`;
    assert.deepEqual(extractCargoTargets(toml), [
      { kind: 'bench', name: 'graph_traversal', path: null },
    ]);
  });

  it('extracts a bin with explicit path', () => {
    const toml = `
[[bin]]
name = "elohim-storage"
path = "src/main.rs"
`;
    assert.deepEqual(extractCargoTargets(toml), [
      { kind: 'bin', name: 'elohim-storage', path: 'src/main.rs' },
    ]);
  });

  it('extracts multiple targets in one file', () => {
    const toml = `
[[bin]]
name = "doorway"
path = "src/main.rs"

[[bin]]
name = "doorway-worker"
path = "src/bin/worker.rs"

[[bench]]
name = "throughput"
`;
    const targets = extractCargoTargets(toml);
    assert.equal(targets.length, 3);
    assert.equal(targets[0].name, 'doorway');
    assert.equal(targets[1].name, 'doorway-worker');
    assert.equal(targets[2].kind, 'bench');
  });

  it('ignores [[test]] and [lib] sections', () => {
    const toml = `
[lib]
name = "foo"

[[test]]
name = "integration"

[[bench]]
name = "real"
`;
    const targets = extractCargoTargets(toml);
    assert.equal(targets.length, 1);
    assert.equal(targets[0].kind, 'bench');
  });

  it('skips malformed blocks (no name field)', () => {
    const toml = `
[[bench]]
harness = false

[[bin]]
name = "valid"
`;
    const targets = extractCargoTargets(toml);
    assert.equal(targets.length, 1);
    assert.equal(targets[0].name, 'valid');
  });

  it('handles dependencies sections between targets without confusing them', () => {
    const toml = `
[package]
name = "x"

[dependencies]
serde = "1"

[[bin]]
name = "a"
path = "src/a.rs"

[dev-dependencies]
foo = "1"

[[bench]]
name = "b"
`;
    const targets = extractCargoTargets(toml);
    assert.equal(targets.length, 2);
    assert.equal(targets[0].kind, 'bin');
    assert.equal(targets[1].kind, 'bench');
  });
});

// ── candidateTargetPaths ─────────────────────────────────────────

describe('candidateTargetPaths', () => {
  it('returns explicit path when present', () => {
    const paths = candidateTargetPaths(
      { kind: 'bin', name: 'doorway', path: 'src/main.rs' },
      'doorway/doorway-service',
    );
    assert.deepEqual(paths, ['doorway/doorway-service/src/main.rs']);
  });

  it('returns conventional paths for a bench with no explicit path', () => {
    const paths = candidateTargetPaths(
      { kind: 'bench', name: 'graph_traversal', path: null },
      'elohim/elohim-storage',
    );
    assert.deepEqual(paths, [
      'elohim/elohim-storage/benches/graph_traversal.rs',
      'elohim/elohim-storage/benches/graph_traversal/main.rs',
    ]);
  });

  it('returns conventional paths for an example', () => {
    const paths = candidateTargetPaths(
      { kind: 'example', name: 'demo', path: null },
      'crates/foo',
    );
    assert.deepEqual(paths, [
      'crates/foo/examples/demo.rs',
      'crates/foo/examples/demo/main.rs',
    ]);
  });

  it('returns multiple candidates for a bin (subdir + main.rs)', () => {
    const paths = candidateTargetPaths(
      { kind: 'bin', name: 'tool', path: null },
      'crates/foo',
    );
    assert.ok(paths.includes('crates/foo/src/bin/tool.rs'));
    assert.ok(paths.includes('crates/foo/src/main.rs'));
  });
});

// ── End-to-end: catches the sprint bug shape ─────────────────────

describe('findUncoveredTargets', () => {
  function setup({ cargoToml, sources }) {
    const root = mkdtempSync(join(tmpdir(), 'cargo-cov-'));
    mkdirSync(join(root, 'mycrate'), { recursive: true });
    writeFileSync(join(root, 'mycrate/Cargo.toml'), cargoToml);
    const manifests = [{
      path: 'pipeline/build-manifest.json',
      content: {
        manifestVersion: '1.0',
        pipeline: 'test-pipeline',
        description: 'Test',
        steps: {
          build: {
            description: 'build',
            inputs: { sources, buildProcess: [] },
            outputs: { artifacts: [], verify: null },
            depends: [],
            executor: { stage: 'Build', function: null },
          },
        },
      },
    }];
    return { root, manifests, cleanup: () => rmSync(root, { recursive: true, force: true }) };
  }

  it('passes when all targets are covered', () => {
    const { root, manifests, cleanup } = setup({
      cargoToml: '[package]\nname = "x"\nversion = "0.1.0"\n\n[[bench]]\nname = "graph_traversal"\n',
      sources: ['mycrate/Cargo.toml', 'mycrate/src/**', 'mycrate/benches/**'],
    });
    try {
      const result = findUncoveredTargets(root, manifests);
      assert.equal(result.gaps.length, 0, JSON.stringify(result.gaps, null, 2));
      assert.equal(result.checked.length, 1);
    } finally {
      cleanup();
    }
  });

  it('flags an uncovered bench (the sprint bug shape)', () => {
    // Reproduce the pre-sprint manifest state: Cargo.toml is tracked, but
    // `benches/**` is NOT in the source globs. Adding [[bench]] silently
    // makes the file invisible to the orchestrator + breaks Docker.
    const { root, manifests, cleanup } = setup({
      cargoToml: '[package]\nname = "x"\nversion = "0.1.0"\n\n[[bench]]\nname = "graph_traversal"\n',
      sources: ['mycrate/Cargo.toml', 'mycrate/src/**'], // no benches/**
    });
    try {
      const result = findUncoveredTargets(root, manifests);
      assert.equal(result.gaps.length, 1);
      assert.equal(result.gaps[0].target.kind, 'bench');
      assert.equal(result.gaps[0].target.name, 'graph_traversal');
      assert.ok(result.gaps[0].candidates.some(p => p.endsWith('benches/graph_traversal.rs')));
    } finally {
      cleanup();
    }
  });

  it('flags an uncovered bin with explicit path', () => {
    const { root, manifests, cleanup } = setup({
      cargoToml: '[package]\nname = "x"\n\n[[bin]]\nname = "tool"\npath = "extra/tool.rs"\n',
      sources: ['mycrate/Cargo.toml', 'mycrate/src/**'],
    });
    try {
      const result = findUncoveredTargets(root, manifests);
      assert.equal(result.gaps.length, 1);
      assert.deepEqual(result.gaps[0].candidates, ['mycrate/extra/tool.rs']);
    } finally {
      cleanup();
    }
  });

  it('skips Cargo.toml not covered by any manifest glob (vendored crates)', () => {
    const root = mkdtempSync(join(tmpdir(), 'cargo-cov-'));
    try {
      mkdirSync(join(root, 'vendored/lib'), { recursive: true });
      writeFileSync(
        join(root, 'vendored/lib/Cargo.toml'),
        '[package]\nname = "v"\n\n[[bench]]\nname = "missing"\n',
      );
      const manifests = [{
        path: 'pipeline/build-manifest.json',
        content: {
          manifestVersion: '1.0',
          pipeline: 'p',
          description: 'd',
          steps: {
            build: {
              description: 'b',
              inputs: { sources: ['other/**'], buildProcess: [] },
              outputs: { artifacts: [], verify: null },
              depends: [],
              executor: { stage: 'B', function: null },
            },
          },
        },
      }];
      const result = findUncoveredTargets(root, manifests);
      assert.equal(result.gaps.length, 0);
      assert.equal(result.checked.length, 0);
      assert.ok(result.skipped.some(s => s.cargoFile === 'vendored/lib/Cargo.toml'));
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  it('counts any candidate path covered as coverage (bin sole-main case)', () => {
    // [[bin]] with no path; cargo accepts src/main.rs as the source.
    // Manifest covers src/** — that's sufficient.
    const { root, manifests, cleanup } = setup({
      cargoToml: '[package]\nname = "x"\n\n[[bin]]\nname = "x"\n',
      sources: ['mycrate/Cargo.toml', 'mycrate/src/**'],
    });
    try {
      const result = findUncoveredTargets(root, manifests);
      assert.equal(result.gaps.length, 0);
    } finally {
      cleanup();
    }
  });
});

// ── buildManifestMatchers ────────────────────────────────────────

describe('buildManifestMatchers', () => {
  it('flattens source globs across all steps in all manifests', () => {
    const manifests = [
      {
        path: 'a/build-manifest.json',
        content: {
          steps: {
            one: { inputs: { sources: ['a/**', 'shared/**'] } },
            two: { inputs: { sources: ['b/**'] } },
          },
        },
      },
      {
        path: 'b/build-manifest.json',
        content: {
          steps: {
            three: { inputs: { sources: ['c/**'] } },
          },
        },
      },
    ];
    const matchers = buildManifestMatchers(manifests);
    assert.equal(matchers.length, 4);
    const globs = matchers.map(m => m.glob).sort();
    assert.deepEqual(globs, ['a/**', 'b/**', 'c/**', 'shared/**']);
  });

  it('tolerates manifests with no steps or no sources', () => {
    const matchers = buildManifestMatchers([
      { path: 'a/build-manifest.json', content: {} },
      { path: 'b/build-manifest.json', content: { steps: { one: { inputs: {} } } } },
    ]);
    assert.equal(matchers.length, 0);
  });
});
