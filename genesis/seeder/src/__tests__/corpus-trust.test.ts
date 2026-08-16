import { mkdtempSync, mkdirSync, rmSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { describe, it, expect, afterEach } from 'vitest';

import {
  CORPUS_DECLARATION_FILE,
  CorpusDeclarationError,
  ENVIRONMENT_TIERS,
  NETWORK_STAGES,
  buildStakesDeclaration,
  emitStakesDeclaration,
  loadCorpusDeclaration,
  parseCorpusDeclaration,
  stakesDeclarationLogLine,
  type CorpusDeclaration,
} from '../corpus-trust.js';
import { findCorpusDirs, validateCorpora } from '../validate-corpora.js';

const SRC = '/fixture/corpus.json';

/** Minimal valid declaration; spread-override per test. */
function decl(overrides: Record<string, unknown> = {}): string {
  return JSON.stringify({
    id: 'genesis-lamad',
    title: 'Genesis Lamad Corpus',
    environment: 'preproduction',
    realism: { rung: 0, why: 'corpus body stays rung 0 forever' },
    trustBootstrap: { stage: 'simulacra', grantor: 'human-adam-firstman', scope: 'genesis-lamad' },
    ...overrides,
  });
}

const tempDirs: string[] = [];
function tempCorpus(fileContent?: string): string {
  const dir = mkdtempSync(path.join(tmpdir(), 'corpus-trust-'));
  tempDirs.push(dir);
  if (fileContent !== undefined) {
    writeFileSync(path.join(dir, CORPUS_DECLARATION_FILE), fileContent, 'utf8');
  }
  return dir;
}

afterEach(() => {
  while (tempDirs.length) rmSync(tempDirs.pop()!, { recursive: true, force: true });
});

// =============================================================================
// PARSE — the happy path
// =============================================================================

describe('parseCorpusDeclaration — valid declarations', () => {
  it('parses a full declaration', () => {
    const d = parseCorpusDeclaration(decl(), SRC);
    expect(d).toEqual<CorpusDeclaration>({
      id: 'genesis-lamad',
      title: 'Genesis Lamad Corpus',
      environment: 'preproduction',
      realism: { rung: 0, why: 'corpus body stays rung 0 forever' },
      trustBootstrap: { stage: 'simulacra', grantor: 'human-adam-firstman', scope: 'genesis-lamad' },
    });
  });

  it('defaults trustBootstrap.scope to the corpus id when omitted', () => {
    const d = parseCorpusDeclaration(
      decl({ trustBootstrap: { stage: 'simulacra', grantor: 'human-adam-firstman' } }),
      SRC,
    );
    expect(d.trustBootstrap?.scope).toBe('genesis-lamad');
  });

  it('accepts every NetworkStage variant under a compatible environment', () => {
    for (const stage of NETWORK_STAGES) {
      const environment = stage === 'simulacra' ? 'preproduction' : 'production';
      const d = parseCorpusDeclaration(
        decl({ environment, trustBootstrap: { stage, grantor: 'human-adam-firstman' } }),
        SRC,
      );
      expect(d.trustBootstrap?.stage).toBe(stage);
    }
  });

  it('accepts every declared environment tier', () => {
    for (const environment of ENVIRONMENT_TIERS) {
      const d = parseCorpusDeclaration(decl({ environment, trustBootstrap: undefined }), SRC);
      expect(d.environment).toBe(environment);
    }
  });

  it('ignores unknown extra fields rather than rejecting them', () => {
    const d = parseCorpusDeclaration(decl({ contentType: 'article', notes: 'x' }), SRC);
    expect(d.id).toBe('genesis-lamad');
  });
});

// =============================================================================
// PARSE — fail-closed + hard errors
// =============================================================================

describe('parseCorpusDeclaration — fail-closed on absent grant', () => {
  it('parses cleanly with NO trustBootstrap and mints nothing', () => {
    const d = parseCorpusDeclaration(decl({ trustBootstrap: undefined }), SRC);
    expect(d.trustBootstrap).toBeUndefined();

    const artifact = buildStakesDeclaration(
      { declaration: d, sourcePath: SRC, sha256: 'a'.repeat(64) },
      { seedTarget: 'http://localhost:8888', declaredBy: 'genesis/data/lamad/corpus.json' },
    );
    expect(artifact).toBeNull();
  });

  it('treats an explicit null trustBootstrap the same as absent', () => {
    const d = parseCorpusDeclaration(decl({ trustBootstrap: null }), SRC);
    expect(d.trustBootstrap).toBeUndefined();
  });
});

describe('parseCorpusDeclaration — hard errors name the field', () => {
  const cases: [string, string, string][] = [
    ['missing id', decl({ id: undefined }), 'id'],
    ['empty id', decl({ id: '   ' }), 'id'],
    ['missing title', decl({ title: undefined }), 'title'],
    ['unknown environment', decl({ environment: 'staging' }), 'environment'],
    ['missing environment', decl({ environment: undefined }), 'environment'],
    ['missing realism block', decl({ realism: undefined }), 'realism'],
    ['bad rung', decl({ realism: { rung: 7, why: 'x' } }), 'realism.rung'],
    ['missing why', decl({ realism: { rung: 0 } }), 'realism.why'],
    [
      'unknown stage',
      decl({ trustBootstrap: { stage: 'dev-mode', grantor: 'human-adam-firstman' } }),
      'trustBootstrap.stage',
    ],
    [
      'capitalized stage is not vocabulary',
      decl({ trustBootstrap: { stage: 'Simulacra', grantor: 'human-adam-firstman' } }),
      'trustBootstrap.stage',
    ],
    ['missing grantor', decl({ trustBootstrap: { stage: 'simulacra' } }), 'trustBootstrap.grantor'],
    [
      'empty grantor',
      decl({ trustBootstrap: { stage: 'simulacra', grantor: '' } }),
      'trustBootstrap.grantor',
    ],
    [
      'scope naming a different corpus',
      decl({ trustBootstrap: { stage: 'simulacra', grantor: 'human-adam-firstman', scope: 'genesis-humans' } }),
      'trustBootstrap.scope',
    ],
    [
      'simulacra declared in production',
      decl({
        environment: 'production',
        trustBootstrap: { stage: 'simulacra', grantor: 'human-adam-firstman' },
      }),
      'trustBootstrap.stage',
    ],
    ['trustBootstrap not an object', decl({ trustBootstrap: 'simulacra' }), 'trustBootstrap'],
  ];

  for (const [name, text, field] of cases) {
    it(`rejects ${name} naming '${field}'`, () => {
      let thrown: unknown;
      try {
        parseCorpusDeclaration(text, SRC);
      } catch (e) {
        thrown = e;
      }
      expect(thrown).toBeInstanceOf(CorpusDeclarationError);
      expect((thrown as CorpusDeclarationError).field).toBe(field);
      expect((thrown as CorpusDeclarationError).message).toContain(SRC);
      expect((thrown as CorpusDeclarationError).message).toContain(`'${field}'`);
    });
  }

  it('rejects malformed JSON', () => {
    expect(() => parseCorpusDeclaration('{ not json', SRC)).toThrow(CorpusDeclarationError);
  });

  it('rejects a JSON array root', () => {
    expect(() => parseCorpusDeclaration('[]', SRC)).toThrow(CorpusDeclarationError);
  });

  it('the unknown-stage error lists the NetworkStage vocabulary', () => {
    try {
      parseCorpusDeclaration(decl({ trustBootstrap: { stage: 'dev', grantor: 'g' } }), SRC);
      expect.unreachable('should have thrown');
    } catch (e) {
      for (const stage of NETWORK_STAGES) {
        expect((e as Error).message).toContain(stage);
      }
    }
  });
});

// =============================================================================
// LOAD — absence is not an error
// =============================================================================

describe('loadCorpusDeclaration', () => {
  it('returns null when the corpus declares nothing (fail-closed by absence)', () => {
    expect(loadCorpusDeclaration(tempCorpus())).toBeNull();
  });

  it('returns null for a directory that does not exist', () => {
    expect(loadCorpusDeclaration('/nope/does/not/exist')).toBeNull();
  });

  it('loads and fingerprints an existing declaration', () => {
    const dir = tempCorpus(decl());
    const loaded = loadCorpusDeclaration(dir)!;
    expect(loaded.declaration.id).toBe('genesis-lamad');
    expect(loaded.sourcePath).toBe(path.join(dir, CORPUS_DECLARATION_FILE));
    expect(loaded.sha256).toMatch(/^[0-9a-f]{64}$/);
  });

  it('fingerprint changes when the bytes change', () => {
    const a = loadCorpusDeclaration(tempCorpus(decl()))!;
    const b = loadCorpusDeclaration(tempCorpus(decl({ title: 'Other' })))!;
    expect(a.sha256).not.toBe(b.sha256);
  });

  it('throws (never silently downgrades) when an existing declaration is invalid', () => {
    const dir = tempCorpus(decl({ trustBootstrap: { stage: 'yolo', grantor: 'g' } }));
    expect(() => loadCorpusDeclaration(dir)).toThrow(CorpusDeclarationError);
  });
});

// =============================================================================
// EMIT — the Q6 contract
// =============================================================================

describe('buildStakesDeclaration / emitStakesDeclaration', () => {
  const loaded = () => ({
    declaration: parseCorpusDeclaration(decl(), SRC),
    sourcePath: SRC,
    sha256: 'b'.repeat(64),
  });

  it('builds the artifact on the Q6 contract', () => {
    const artifact = buildStakesDeclaration(loaded(), {
      seedTarget: 'http://localhost:8888',
      declaredBy: 'genesis/data/lamad/corpus.json',
      itemsDeclared: 3432,
      mintedAt: '2026-08-16T12:00:00.000Z',
    })!;

    expect(artifact).toEqual({
      schemaVersion: '1',
      manifestKind: 'network-stakes',
      manifestCid: `stakes:genesis-lamad:sha256-${'b'.repeat(64)}`,
      stakes: {
        stage: 'simulacra',
        scope: 'genesis-lamad',
        grantor: 'human-adam-firstman',
        environment: 'preproduction',
      },
      corpus: { corpusId: 'genesis-lamad', title: 'Genesis Lamad Corpus', realismRung: 0, itemsDeclared: 3432 },
      provenance: {
        declaredBy: 'genesis/data/lamad/corpus.json',
        declarationSha256: 'b'.repeat(64),
        mintedBy: 'genesis-seeder',
        mintedAt: '2026-08-16T12:00:00.000Z',
        seedTarget: 'http://localhost:8888',
      },
      operatorConfig: { ELOHIM_NETWORK_STAKES: 'simulacra' },
    });
  });

  it('defaults itemsDeclared to null when the caller does not know it', () => {
    const artifact = buildStakesDeclaration(loaded(), {
      seedTarget: 'http://localhost:8888',
      declaredBy: 'genesis/data/lamad/corpus.json',
    })!;
    expect(artifact.corpus.itemsDeclared).toBeNull();
    expect(artifact.provenance.mintedAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });

  it('emits the exact log line the plan asks for', () => {
    const artifact = buildStakesDeclaration(loaded(), {
      seedTarget: 'x',
      declaredBy: 'y',
    })!;
    expect(stakesDeclarationLogLine(artifact)).toBe(
      'stakes declaration: simulacra for corpus genesis-lamad by human-adam-firstman',
    );
  });

  it('writes readable JSON with a trailing newline', () => {
    const dir = tempCorpus();
    const out = path.join(dir, 'nested', 'stakes-declaration.json');
    const artifact = buildStakesDeclaration(loaded(), {
      seedTarget: 'http://localhost:8888',
      declaredBy: 'genesis/data/lamad/corpus.json',
      mintedAt: '2026-08-16T12:00:00.000Z',
    })!;
    emitStakesDeclaration(artifact, out);

    expect(existsSync(out)).toBe(true);
    const raw = readFileSync(out, 'utf8');
    expect(raw.endsWith('\n')).toBe(true);
    expect(JSON.parse(raw)).toEqual(artifact);
  });
});

// =============================================================================
// REPO-WIDE VALIDATION + the real declarations on disk
// =============================================================================

describe('validateCorpora', () => {
  it('finds only directories that carry a declaration', () => {
    const root = mkdtempSync(path.join(tmpdir(), 'corpora-root-'));
    tempDirs.push(root);
    mkdirSync(path.join(root, 'declared'));
    mkdirSync(path.join(root, 'undeclared'));
    writeFileSync(path.join(root, 'declared', CORPUS_DECLARATION_FILE), decl(), 'utf8');

    expect(findCorpusDirs(root).map((d) => path.basename(d))).toEqual(['declared']);
  });

  it('reports an invalid declaration without throwing', () => {
    const root = mkdtempSync(path.join(tmpdir(), 'corpora-root-'));
    tempDirs.push(root);
    mkdirSync(path.join(root, 'broken'));
    writeFileSync(
      path.join(root, 'broken', CORPUS_DECLARATION_FILE),
      decl({ trustBootstrap: { stage: 'simulacra' } }),
      'utf8',
    );

    const rows = validateCorpora(root);
    expect(rows).toHaveLength(1);
    expect(rows[0].ok).toBe(false);
    expect(rows[0].error).toContain('trustBootstrap.grantor');
  });

  it('returns an empty list for a root that does not exist', () => {
    expect(findCorpusDirs('/nope/not/here')).toEqual([]);
  });

  it('every declaration committed under genesis/data/ is valid', () => {
    const dataRoot = path.resolve(__dirname, '../../../data');
    const rows = validateCorpora(dataRoot);
    expect(rows.length).toBeGreaterThan(0);
    for (const row of rows) {
      expect(row.error ?? null).toBeNull();
      expect(row.ok).toBe(true);
    }
  });

  it('the lamad corpus — the one the local mesh seeds — declares a simulacra grant', () => {
    const lamadDir = path.resolve(__dirname, '../../../data/lamad');
    const loaded = loadCorpusDeclaration(lamadDir)!;
    expect(loaded.declaration.id).toBe('genesis-lamad');
    expect(loaded.declaration.environment).toBe('preproduction');
    expect(loaded.declaration.trustBootstrap).toEqual({
      stage: 'simulacra',
      grantor: 'human-adam-firstman',
      scope: 'genesis-lamad',
    });
  });
});
