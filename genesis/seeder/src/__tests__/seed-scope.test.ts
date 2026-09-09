import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const calls = vi.hoisted(() => ({
  content: vi.fn(async (items: unknown[]) => ({ inserted: items.length, skipped: 0, errors: [] })),
  bindings: vi.fn(), hosting: vi.fn(), atom: vi.fn(async () => ({ skipped: true })),
  projections: vi.fn(), stakes: vi.fn(), emit: vi.fn(),
}));

vi.mock('../doorway-client.js', () => ({
  readSeederCredentials: () => null,
  default: class {
    setBearerToken() {}
    async checkStatus() { return null; }
    async checkHealth() { return { healthy: true }; }
    async getSchemaInfo() { return { supportedVersions: [1], currentVersion: 1 }; }
    bulkCreateContent = calls.content;
  },
}));
vi.mock('../seed-operator-bindings.js', () => ({
  seedOperatorBindings: calls.bindings, seedHostingAgreements: calls.hosting,
  defaultM5Bindings: () => [], defaultHostingAgreements: () => [],
  createOperatorBindingClient: () => ({}),
}));
vi.mock('../seed-projections.js', () => ({
  seedProjections: calls.projections, defaultProjectionSeeds: () => [],
  createProjectionClient: () => ({}),
}));
vi.mock('../seed-epr-atom.js', () => ({ seedLandingEprAtom: calls.atom }));
vi.mock('../corpus-trust.js', () => ({
  loadCorpusDeclaration: () => ({ sourcePath: '/tmp/fixture/corpus.json', declaration: { id: 'fixture', trustBootstrap: false } }),
  buildStakesDeclaration: () => ({ operatorConfig: { ELOHIM_NETWORK_STAKES: 'bootstrap' } }),
  emitStakesDeclaration: calls.emit,
  parsePeerStorageUrls: () => ['http://fixture.invalid'],
  seedNetworkStakes: calls.stakes,
  stakesDeclarationLogLine: () => 'fixture stakes',
  CorpusDeclarationError: class extends Error {},
}));

describe('targeted content seeding does not run deployment writers', () => {
  let dataDir: string;
  let argv: string[];
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    calls.stakes.mockResolvedValue({ seeded: ['fixture'], failed: [] });
    argv = process.argv;
    dataDir = mkdtempSync(join(tmpdir(), 'seed-scope-'));
    mkdirSync(join(dataDir, 'content'));
    for (const id of ['selected', 'unselected']) {
      writeFileSync(join(dataDir, 'content', `${id}.json`), JSON.stringify({
        id, title: id, content: 'Actual fixture body', contentType: 'concept',
        contentFormat: 'markdown', reach: 'commons',
      }));
    }
    vi.stubEnv('DATA_DIR', dataDir);
    vi.stubEnv('DOORWAY_URL', 'http://fixture.invalid');
    vi.stubEnv('STORAGE_URL', 'http://fixture.invalid');
    vi.stubEnv('SKIP_BLOB_SYNC', 'true');
    vi.stubEnv('SKIP_VERIFICATION', 'true');
    vi.stubEnv('SEED_CONTENT_ONLY', 'false');
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{"peers":1}')));
    vi.spyOn(console, 'log').mockImplementation(() => {});
  });
  afterEach(() => {
    process.argv = argv;
    vi.unstubAllEnvs();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    rmSync(dataDir, { recursive: true, force: true });
  });

  it('writes the named content but calls no ancillary writer or cache warmup', async () => {
    process.argv = ['node', 'test', '--ids=selected', '--content-only'];
    await (await import('../seed.js')).seed();
    expect(calls.content).toHaveBeenCalledOnce();
    expect(calls.content.mock.calls[0][0]).toEqual([expect.objectContaining({ id: 'selected' })]);
    for (const writer of [calls.bindings, calls.hosting, calls.atom, calls.projections, calls.stakes, calls.emit]) {
      expect(writer).not.toHaveBeenCalled();
    }
    expect(fetch).not.toHaveBeenCalled();
  });

  it('retains the deployment completion chain without an explicit id scope', async () => {
    process.argv = ['node', 'test', '--content-only'];
    await (await import('../seed.js')).seed();
    expect(calls.content.mock.calls[0][0]).toHaveLength(2);
    for (const writer of [calls.bindings, calls.hosting, calls.atom, calls.projections, calls.stakes, calls.emit]) {
      expect(writer).toHaveBeenCalledOnce();
    }
    expect(fetch).toHaveBeenCalledWith('http://fixture.invalid/admin/cache/warm', expect.anything());
  });
});
