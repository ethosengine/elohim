/**
 * Orchestrator Strategy Tests
 *
 * Mirrors the pure functions from genesis/orchestrator/Jenkinsfile
 * to validate changeset routing, dependency propagation, cascade
 * behavior, commit message tags, and topological ordering — all
 * without needing Jenkins.
 *
 * Run: node --test orchestrator-strategy.test.mjs
 */

import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadManifests } from './manifest-utils.mjs';

// ══════════════════════════════════════════════════════════════════
// Algorithm under test — imported from the pure-function module so
// that CLI consumers (preview.mjs) can import it without side-effects
// from node:test's auto-discovered describe/it blocks.
// ══════════════════════════════════════════════════════════════════

import {
  PIPELINES,
  CI_ONLY_PATTERNS,
  CI_ONLY_FILES,
  analyzePipelineRequirements,
  propagateDependencies,
  orderByDependencies,
  parseCommitTags,
  parseSkipCi,
  simulate,
} from './orchestrator-strategy.mjs';

// ══════════════════════════════════════════════════════════════════
// TESTS
// ══════════════════════════════════════════════════════════════════

// ── Changeset Routing ───────────────────────────────────────────

describe('changeset routing', () => {
  it('DNA changes trigger elohim-holochain', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
  });

  it('doorway changes trigger elohim-edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['doorway/doorway-service/src/main.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
  });

  it('Angular app changes trigger elohim', () => {
    const { pipelines } = simulate({
      changedFiles: ['app/elohim-app/src/app/lamad/services/content.service.ts'],
    });
    assert.ok(pipelines.includes('elohim'));
  });

  it('sophia submodule pointer triggers elohim-sophia', () => {
    const { pipelines } = simulate({
      changedFiles: ['sophia'],
    });
    assert.ok(pipelines.includes('elohim-sophia'));
  });

  it('sophia directory changes trigger elohim-sophia', () => {
    const { pipelines } = simulate({
      changedFiles: ['sophia/packages/sophia-core/src/index.ts'],
    });
    assert.ok(pipelines.includes('elohim-sophia'));
  });

  it('genesis changes trigger elohim-genesis', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/a2o/features/auth/auth-lifecycle.feature'],
    });
    assert.ok(pipelines.includes('elohim-genesis'));
  });

  it('storage changes trigger elohim-edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/elohim-storage/src/main.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
  });

  it('VERSION triggers holochain, edge, and app', () => {
    const { pipelines } = simulate({
      changedFiles: ['VERSION'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim'));
  });

  it('pipeline Jenkinsfile changes route to their owning pipeline', () => {
    // Post-Adam fix: a pipeline's own Jenkinsfile is a real trigger for that
    // pipeline (and only that pipeline). Root Jenkinsfile owns elohim (app);
    // elohim/holochain/dna/Jenkinsfile owns elohim-holochain.
    const { pipelines } = simulate({
      changedFiles: ['Jenkinsfile', 'elohim/holochain/dna/Jenkinsfile'],
    });
    assert.ok(pipelines.includes('elohim'),
      'root Jenkinsfile triggers the elohim app pipeline');
    assert.ok(pipelines.includes('elohim-holochain'),
      'dna Jenkinsfile triggers the holochain pipeline');
    // unrelated pipelines should not be directly triggered (cascades may add some)
    assert.ok(!pipelines.includes('elohim-steward'), 'steward not triggered');
  });

  it('orchestrator config changes trigger nothing', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile', 'CLAUDE.md'],
    });
    assert.equal(pipelines.length, 0);
  });

  it('steward changes do NOT trigger (manualOnly)', () => {
    const { pipelines } = simulate({
      changedFiles: ['steward/src-tauri/src/lib.rs'],
    });
    assert.ok(!pipelines.includes('elohim-steward'));
  });

  it('change to elohim/holochain/Jenkinsfile triggers elohim-edge only', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/Jenkinsfile'],
    });
    assert.ok(pipelines.includes('elohim-edge'),
      'A pipeline-Jenkinsfile change must trigger that pipeline (Adam-shaped bug regression)');
    // Note: cascade may also include elohim-genesis (triggersGenesis), so don't assert deepEqual.
  });

  it('change to genesis/orchestrator/Jenkinsfile triggers nothing (CI-only)', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile'],
    });
    assert.deepEqual(pipelines, [],
      'Orchestrator self-edits should not cascade');
  });

  it('change to genesis/orchestrator/manifests/ triggers elohim-edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/manifests/elohim-edge/foo.yaml'],
    });
    assert.ok(pipelines.includes('elohim-edge'),
      'K8s manifests are real deploy source for the edge pipeline');
  });

  it('unrelated files trigger nothing', () => {
    const { pipelines } = simulate({
      changedFiles: ['docs/README.md', '.gitignore'],
    });
    assert.equal(pipelines.length, 0);
  });
});

// ── Cascade / Dependency Propagation ────────────────────────────

describe('cascade propagation', () => {
  it('DNA (holochain) cascades to edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'), 'should include holochain');
    assert.ok(pipelines.includes('elohim-edge'), 'should cascade to edge');
  });

  it('DNA cascades transitively: holochain → edge → genesis', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
  });

  it('sophia cascades to app (elohim)', () => {
    const { pipelines } = simulate({
      changedFiles: ['sophia/packages/sophia-core/src/index.ts'],
    });
    assert.ok(pipelines.includes('elohim-sophia'), 'should include sophia');
    assert.ok(pipelines.includes('elohim'), 'should cascade to app');
  });

  it('steward is never auto-triggered even when dependency builds', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(!pipelines.includes('elohim-steward'), 'steward should not cascade (manualOnly)');
  });

  it('edge change cascades to genesis but NOT holochain (upstream)', () => {
    const { pipelines } = simulate({
      changedFiles: ['doorway/doorway-service/src/routes/auth_routes.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis should cascade from edge');
    assert.ok(!pipelines.includes('elohim-holochain'), 'holochain should NOT cascade (its upstream)');
  });

  it('app + edge both trigger → genesis included once', () => {
    const { pipelines } = simulate({
      changedFiles: [
        'doorway/doorway-service/src/main.rs',
        'app/elohim-app/src/app/lamad/services/content.service.ts',
      ],
    });
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim'));
    assert.ok(pipelines.includes('elohim-genesis'));
    // No duplicates
    assert.equal(pipelines.filter(p => p === 'elohim-genesis').length, 1);
  });
});

// ── Commit Message Tags ─────────────────────────────────────────

describe('commit message tags', () => {
  it('[build:edge] forces elohim-edge', () => {
    const tags = parseCommitTags('fix: something [build:edge]');
    assert.deepEqual(tags, ['elohim-edge']);
  });

  it('[build:dna] forces elohim-holochain', () => {
    const tags = parseCommitTags('feat: new zome [build:dna]');
    assert.deepEqual(tags, ['elohim-holochain']);
  });

  it('[build:all] forces all non-manual pipelines', () => {
    const tags = parseCommitTags('chore: rebuild [build:all]');
    assert.ok(tags.includes('elohim-holochain'));
    assert.ok(tags.includes('elohim-edge'));
    assert.ok(tags.includes('elohim'));
    assert.ok(tags.includes('elohim-genesis'));
    assert.ok(tags.includes('elohim-sophia'));
    assert.ok(!tags.includes('elohim-steward'), 'steward is manualOnly');
  });

  it('[build:edge,genesis] forces multiple', () => {
    const tags = parseCommitTags('fix: deploy [build:edge,genesis]');
    assert.ok(tags.includes('elohim-edge'));
    assert.ok(tags.includes('elohim-genesis'));
    assert.equal(tags.length, 2);
  });

  it('case insensitive', () => {
    const tags = parseCommitTags('[Build:Edge]');
    assert.deepEqual(tags, ['elohim-edge']);
  });

  it('multiple [build:] tags in one message', () => {
    const tags = parseCommitTags('fix: stuff [build:dna] and also [build:app]');
    assert.ok(tags.includes('elohim-holochain'));
    assert.ok(tags.includes('elohim'));
  });

  it('[build:edge] + cascade propagation', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile'],
      commitMsg: 'fix(ci): cascade change [build:edge]',
    });
    assert.ok(pipelines.includes('elohim-edge'), 'edge forced by tag');
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis cascades from edge');
  });

  it('[build:dna] triggers full chain: dna → edge → genesis', () => {
    const { pipelines } = simulate({
      changedFiles: [],
      commitMsg: 'chore: force rebuild [build:dna]',
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
  });

  it('[skip ci] suppresses everything', () => {
    const { pipelines, skipped } = simulate({
      changedFiles: ['app/elohim-app/src/app/lamad/services/content.service.ts'],
      commitMsg: 'docs: update readme [skip ci]',
    });
    assert.ok(skipped);
    assert.equal(pipelines.length, 0);
  });

  it('[ci skip] also works', () => {
    const { skipped } = simulate({ commitMsg: '[ci skip]' });
    assert.ok(skipped);
  });

  it('[no ci] also works', () => {
    const { skipped } = simulate({ commitMsg: 'test [no ci]' });
    assert.ok(skipped);
  });

  it('unknown tag is silently ignored', () => {
    const tags = parseCommitTags('[build:banana]');
    assert.equal(tags.length, 0);
  });
});

// ── Ordering ────────────────────────────────────────────────────

describe('dependency ordering', () => {
  it('holochain before edge before genesis', () => {
    const ordered = orderByDependencies(['elohim-genesis', 'elohim-edge', 'elohim-holochain']);
    assert.ok(
      ordered.indexOf('elohim-holochain') < ordered.indexOf('elohim-edge'),
      `holochain (${ordered.indexOf('elohim-holochain')}) should come before edge (${ordered.indexOf('elohim-edge')})`
    );
    assert.ok(
      ordered.indexOf('elohim-edge') < ordered.indexOf('elohim-genesis'),
      `edge (${ordered.indexOf('elohim-edge')}) should come before genesis (${ordered.indexOf('elohim-genesis')})`
    );
  });

  it('sophia before app', () => {
    const ordered = orderByDependencies(['elohim', 'elohim-sophia']);
    assert.equal(ordered[0], 'elohim-sophia');
    assert.equal(ordered[1], 'elohim');
  });

  it('full graph orders correctly', () => {
    const all = ['elohim-genesis', 'elohim', 'elohim-edge', 'elohim-holochain', 'elohim-sophia'];
    const ordered = orderByDependencies(all);
    // holochain and sophia have no deps — they come first (order between them is stable)
    // edge depends on holochain
    // app depends on sophia
    // genesis depends on edge and app
    assert.ok(ordered.indexOf('elohim-holochain') < ordered.indexOf('elohim-edge'));
    assert.ok(ordered.indexOf('elohim-sophia') < ordered.indexOf('elohim'));
    assert.ok(ordered.indexOf('elohim-edge') < ordered.indexOf('elohim-genesis'));
    assert.ok(ordered.indexOf('elohim') < ordered.indexOf('elohim-genesis'));
  });
});

// ── Real-World Scenarios ────────────────────────────────────────

describe('real-world scenarios', () => {
  it('imagodei zome change deploys new hApp (the bug we fixed)', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/imagodei/coordinator/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'), 'DNA builds');
    assert.ok(pipelines.includes('elohim-edge'), 'edge redeploys (installs new hApp)');
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis seeds and tests');
  });

  it('doorway hotfix rebuilds edge and tests', () => {
    const { pipelines } = simulate({
      changedFiles: ['doorway/doorway-service/src/routes/auth_routes.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
    assert.ok(!pipelines.includes('elohim-holochain'), 'no DNA rebuild needed');
    assert.ok(!pipelines.includes('elohim'), 'no app rebuild needed');
  });

  it('pure orchestrator fix with [build:edge] tag forces deploy', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile'],
      commitMsg: 'fix(ci): cascade change [build:edge]',
    });
    // orchestrator file is CI-only, so changeset yields nothing
    // but [build:edge] forces edge, which cascades to genesis
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
    assert.equal(pipelines.filter(p => p === 'elohim-holochain').length, 0);
  });

  it('cross-cutting change: storage + app + sophia', () => {
    const { pipelines } = simulate({
      changedFiles: [
        'elohim/elohim-storage/src/views.rs',
        'app/elohim-app/src/app/lamad/services/content.service.ts',
        'sophia/packages/sophia-core/src/index.ts',
      ],
    });
    assert.ok(pipelines.includes('elohim-edge'), 'storage triggers edge');
    assert.ok(pipelines.includes('elohim-sophia'), 'sophia triggers sophia');
    assert.ok(pipelines.includes('elohim'), 'app triggers app (+ sophia cascade)');
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis cascades');
  });

  it('empty changeset with no tags does nothing', () => {
    const { pipelines } = simulate({ changedFiles: [] });
    assert.equal(pipelines.length, 0);
  });

  it('shared cache-core change triggers both holochain and edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/elohim-cache-core/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'), 'cache-core in holochain patterns');
    assert.ok(pipelines.includes('elohim-edge'), 'cache-core in edge patterns');
  });
});

// ══════════════════════════════════════════════════════════════════
// Drift detection — assert this mirror matches live Jenkinsfile
// ══════════════════════════════════════════════════════════════════

const __dirname = dirname(fileURLToPath(import.meta.url));
const JENKINSFILE_PATH = resolve(__dirname, 'Jenkinsfile');

/**
 * Extract a Groovy list literal like ['.claude', '.github/'] from source.
 * Returns the array of string contents, or null if not found.
 */
function extractGroovyStringList(source, varName) {
  // Match: def varName = [ ... ]   (multi-line, with possible trailing comments)
  const re = new RegExp(`def\\s+${varName}\\s*=\\s*\\[([^\\]]*)\\]`, 'g');
  const matches = Array.from(source.matchAll(re));
  if (matches.length === 0) return null;
  if (matches.length > 1) {
    throw new Error(
      `extractGroovyStringList: found ${matches.length} matches for 'def ${varName}' — expected exactly 1. ` +
      `Drift detector cannot disambiguate.`
    );
  }
  // Guard: detect '//' inside string literals BEFORE comment stripping ate it.
  // This catches the URL-corruption failure mode described in I-2.
  const stringRe = /['"]([^'"]*)['"]/g;
  let sm;
  while ((sm = stringRe.exec(matches[0][1])) !== null) {
    if (sm[1].includes('//')) {
      throw new Error(
        `extractGroovyStringList: string entry '${sm[1]}' in 'def ${varName}' contains '//', ` +
        `which the comment stripper would corrupt. Use a quote-aware parser instead.`
      );
    }
  }
  // Strip line comments (// ...) before extracting strings — Groovy comments
  // can contain apostrophes (e.g., "// orchestrator's pipeline") that would
  // otherwise confuse the string extractor below.
  const cleaned = matches[0][1].replace(/\/\/[^\n]*/g, '');
  // FUTURE RISK: this stripper would corrupt a string entry containing '//'
  // (e.g., a URL like 'https://example.com/hook'). If you add such an entry,
  // either escape it or replace this stripper with a quote-aware tokenizer.
  // I-2 guard above catches the most common case.
  const items = [];
  const itemRe = /['"]([^'"]+)['"]/g;
  let im;
  while ((im = itemRe.exec(cleaned)) !== null) items.push(im[1]);
  return items;
}

/**
 * Extract pipeline names from the @Field def PIPELINES = [ ... ] block.
 * Walks the block respecting bracket depth AND string literals (so glob
 * patterns like 'glob[ab]*' inside entries don't confuse the walker).
 */
function extractPipelineNames(source) {
  const start = source.indexOf('@Field def PIPELINES');
  if (start < 0) return null;
  // Strip line comments (// ...) before walking — Groovy comments routinely
  // contain unbalanced apostrophes ("don't", "doesn't") that would trap the
  // string-aware walker below in an unterminated string state.
  const stripped = source.replace(/\/\/[^\n]*/g, '');
  const strippedStart = stripped.indexOf('@Field def PIPELINES');
  if (strippedStart < 0) return null;
  const open = stripped.indexOf('[', strippedStart);
  let depth = 0;
  let end = -1;
  let inString = false;
  let stringChar = null;
  for (let i = open; i < stripped.length; i++) {
    const c = stripped[i];
    if (inString) {
      if (c === stringChar && stripped[i - 1] !== '\\') {
        inString = false;
        stringChar = null;
      }
      continue;
    }
    if (c === "'" || c === '"') {
      inString = true;
      stringChar = c;
      continue;
    }
    if (c === '[') depth++;
    else if (c === ']') {
      depth--;
      if (depth === 0) { end = i; break; }
    }
  }
  if (end < 0) return null;
  const block = stripped.slice(open + 1, end);
  // Top-level keys look like:    'elohim-holochain': [
  const names = [];
  const re = /^\s*'([a-z][a-z0-9-]*)'\s*:\s*\[/gm;
  let m;
  while ((m = re.exec(block)) !== null) names.push(m[1]);
  return names;
}

describe('drift detection: mirror vs live Jenkinsfile', () => {
  const source = readFileSync(JENKINSFILE_PATH, 'utf8');

  it('CI_ONLY_PATTERNS in mirror matches ciOnlyPatterns in Jenkinsfile', () => {
    const live = extractGroovyStringList(source, 'ciOnlyPatterns');
    assert.ok(live && live.length > 0, 'failed to parse non-empty ciOnlyPatterns from Jenkinsfile');
    assert.deepEqual([...CI_ONLY_PATTERNS].sort(), [...live].sort(),
      `Mirror CI_ONLY_PATTERNS drifted from Jenkinsfile ciOnlyPatterns.\n` +
      `Mirror: ${JSON.stringify(CI_ONLY_PATTERNS)}\n` +
      `Live:   ${JSON.stringify(live)}`);
  });

  it('CI_ONLY_FILES in mirror matches ciOnlyFiles in Jenkinsfile', () => {
    const live = extractGroovyStringList(source, 'ciOnlyFiles');
    assert.ok(live && live.length > 0, 'failed to parse non-empty ciOnlyFiles from Jenkinsfile');
    assert.deepEqual([...CI_ONLY_FILES].sort(), [...live].sort(),
      `Mirror CI_ONLY_FILES drifted from Jenkinsfile ciOnlyFiles.\n` +
      `Mirror: ${JSON.stringify(CI_ONLY_FILES)}\n` +
      `Live:   ${JSON.stringify(live)}`);
  });

  it('PIPELINES keys in mirror match @Field def PIPELINES in Jenkinsfile', () => {
    const live = extractPipelineNames(source);
    assert.ok(live && live.length > 0, 'failed to parse non-empty PIPELINES block from Jenkinsfile');
    const mirror = Object.keys(PIPELINES);
    assert.deepEqual([...mirror].sort(), [...live].sort(),
      `Mirror PIPELINES keys drifted from Jenkinsfile @Field def PIPELINES.\n` +
      `Mirror: ${JSON.stringify(mirror)}\n` +
      `Live:   ${JSON.stringify(live)}`);
  });

  it('PIPELINES changePatterns in mirror match each pipeline block in Jenkinsfile', () => {
    // Catches the exact drift class that bit 57f15bd5 (added genesis/orchestrator/data/
    // to elohim-edge changePatterns in Jenkinsfile, forgot to mirror in .mjs).
    // For each pipeline name, regex its changePatterns array out of the Groovy block
    // and compare to the JS mirror's array (sorted, since order is not semantic).
    const re = /'([a-z][a-z0-9-]*)'\s*:\s*\[[^\]]*?changePatterns\s*:\s*\[([^\]]*)\]/gs;
    const live = {};
    let m;
    while ((m = re.exec(source)) !== null) {
      const items = [...m[2].matchAll(/'([^']+)'/g)].map((x) => x[1]);
      live[m[1]] = items;
    }
    const drifted = [];
    for (const [name, mirror] of Object.entries(PIPELINES)) {
      const livePatterns = live[name];
      if (!livePatterns) continue; // pipelines without changePatterns blocks are fine
      const a = [...mirror.changePatterns].sort();
      const b = [...livePatterns].sort();
      if (JSON.stringify(a) !== JSON.stringify(b)) {
        drifted.push({ name, mirror: a, live: b });
      }
    }
    assert.deepEqual(drifted, [],
      `Mirror PIPELINES changePatterns drifted from Jenkinsfile @Field def PIPELINES blocks.\n` +
      drifted.map((d) =>
        `  ${d.name}:\n    mirror: ${JSON.stringify(d.mirror)}\n    live:   ${JSON.stringify(d.live)}`
      ).join('\n'));
  });
});

// ══════════════════════════════════════════════════════════════════
// Cross-validation: PIPELINES changePatterns vs manifest source globs
// ══════════════════════════════════════════════════════════════════
// For each PIPELINES entry that has a build-manifest.json and is NOT in
// the known divergences list, verify that every changePattern is covered
// by at least one manifest source glob. This catches the exact divergence
// class that causes CI hard failures (e.g., VERSION missing from sources).

import picomatch from 'picomatch';

// Pipelines whose legacy PIPELINES changePatterns aren't fully covered by
// manifest source globs. These are skipped in the cross-validation test.
// Graph-only pipelines have no PIPELINES entry; manifest-gap pipelines have
// legacy patterns that no manifest glob matches yet.
const KNOWN_DIVERGENCES = [
  // Graph-only: manifest exists, no PIPELINES entry
  'elohim-compute',
  'elohim-doorway-app',
  'elohim-orchestrator',
  // Manifest-gap: legacy patterns not yet covered by manifest globs
  'elohim-edge',
  'elohim-genesis',
];

describe('PIPELINES changePatterns covered by manifest source globs', () => {
  const repoRoot = resolve(__dirname, '../..');
  const manifests = loadManifests(repoRoot);
  const manifestByPipeline = new Map(
    manifests.map(m => [m.content.pipeline, m])
  );

  for (const [pipelineName, config] of Object.entries(PIPELINES)) {
    if (KNOWN_DIVERGENCES.includes(pipelineName)) continue;
    // manualOnly pipelines never set shouldRun/shouldBuild on either side,
    // so they can never cause a CI divergence. Skip them.
    if (config.manualOnly) continue;

    const manifest = manifestByPipeline.get(pipelineName);
    if (!manifest) continue;

    // Collect all source globs from all steps in this manifest
    const allSourceGlobs = [];
    for (const step of Object.values(manifest.content.steps)) {
      allSourceGlobs.push(...(step.inputs.sources || []));
    }

    for (const pattern of config.changePatterns) {
      it(`${pipelineName}: pattern '${pattern}' is covered by manifest`, () => {
        // Generate a synthetic file that the legacy algorithm would match.
        // Directory patterns (ending in '/') get a synthetic nested file.
        // File patterns (like 'VERSION') are used as-is.
        const syntheticFile = pattern.endsWith('/')
          ? `${pattern}src/synthetic-test-file.rs`
          : pattern;

        // Check if any manifest source glob matches the synthetic file
        const covered = allSourceGlobs.some(glob => {
          const matcher = picomatch(glob);
          return matcher(syntheticFile);
        });

        assert.ok(covered,
          `PIPELINES['${pipelineName}'] has changePattern '${pattern}' but no manifest source glob ` +
          `matches synthetic file '${syntheticFile}'.\n` +
          `Manifest globs: ${JSON.stringify(allSourceGlobs)}\n` +
          `Fix: add a matching glob to a step in ${manifest.path}`);
      });
    }
  }
});
