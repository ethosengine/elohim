/**
 * The derivation key (Law II) and the denominator (Law I), guidestar
 * 2026-08-22 §3.
 *
 * The load-bearing property under test is the one the guidestar says the whole
 * design turns on: `sut` MUST move when the operator edits a source file the
 * running binaries were built from. Everything is driven through an injected
 * probe, so these assertions hold with no git tree, no mesh and no build.
 */

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import { parseDeclaredConcerns, readDeclaredConcerns } from '../lib/declared-concerns.js';
import { computeRunEnv, laneForProfile, resolveGitCommit } from '../lib/run-env.js';
import { computeSut, hashSutParts, sha256Short } from '../lib/sut.js';

import type { RunEnvProbe } from '../lib/run-env.js';

const STORAGE = 'elohim/elohim-storage';
const DOORWAY = 'doorway/doorway-service';
const CONDUCTOR = 'elohim/holochain-conductor';
const DNA = 'elohim/holochain/dna';
// Not under /tmp: nothing here touches a real filesystem (the probe is fake),
// and a literal public temp path invites a lint rail that has nothing to guard.
const FIXTURE_PATH = '/fake/mesh/household-fixture.json';
const UNKNOWN_DNA_HASH = 'sut.dnaHash';
const UNKNOWN_FIXTURES = 'sut.fixtures';
const STORAGE_SRC = 'elohim/elohim-storage/src/p2p/behaviour.rs';
const CONTENT_SYNC = 'content-sync';

interface FakeWorld {
  env?: Record<string, string | undefined>;
  head?: string;
  trees?: Record<string, string>;
  porcelain?: Record<string, string>;
  diffs?: Record<string, string>;
  untracked?: Record<string, string>;
  files?: Record<string, string>;
}

function makeProbe(world: FakeWorld): RunEnvProbe {
  const trees: Record<string, string> = {
    [STORAGE]: 'tree-storage',
    [DOORWAY]: 'tree-doorway',
    [CONDUCTOR]: 'tree-conductor',
    [DNA]: 'tree-dna',
    ...world.trees,
  };
  return {
    env: world.env ?? {},
    git(args: string[], stdin?: string): string | null {
      const [verb] = args;
      if (verb === 'rev-parse' && args[1] === 'HEAD') return world.head ?? 'head-sha';
      if (verb === 'rev-parse') {
        const path = args[1].slice('HEAD:'.length);
        return trees[path] ?? null;
      }
      const path = args.at(-1) ?? '';
      if (verb === 'status') return world.porcelain?.[path] ?? '';
      if (verb === 'diff') return world.diffs?.[path] ?? '';
      if (verb === 'ls-files') return world.untracked?.[path] ?? '';
      if (verb === 'hash-object')
        return (stdin ?? '')
          .split('\n')
          .filter(Boolean)
          .map(line => sha256Short(world.files?.[line] ?? line))
          .join('\n');
      return null;
    },
    hashFile(path: string): string | null {
      const content = world.files?.[path];
      return content === undefined ? null : sha256Short(content);
    },
    readText(path: string): string | null {
      return world.files?.[path] ?? null;
    },
  };
}

void describe('sut — the system under test in the derivation key', () => {
  void it('is stable across two runs of an unchanged tree', () => {
    const a = computeSut(makeProbe({}));
    const b = computeSut(makeProbe({}));
    assert.equal(a.sut, b.sut);
    assert.match(a.sut, /^sha256:[0-9a-f]{16}$/);
  });

  void it('MOVES when a source file in a hashed tree is dirty', () => {
    const clean = computeSut(makeProbe({}));
    const dirty = computeSut(
      makeProbe({
        porcelain: { [STORAGE]: ` M ${STORAGE_SRC}\n` },
        diffs: { [STORAGE]: '@@ -1 +1 @@\n-let a = 1;\n+let a = 2;\n' },
      })
    );
    assert.notEqual(clean.sut, dirty.sut, 'an uncommitted edit must move the key');
    assert.match(dirty.sutParts['storage'], /^tree:tree-storage\+dirty:[0-9a-f]{16}$/);
    assert.equal(clean.sutParts['storage'], 'tree:tree-storage');
  });

  void it('MOVES AGAIN when the same dirty file is edited further', () => {
    // The porcelain line names WHICH files are dirty, never what they now
    // contain. A key built from it alone would freeze after the first edit.
    const porcelain = { [STORAGE]: ` M ${STORAGE_SRC}\n` };
    const first = computeSut(makeProbe({ porcelain, diffs: { [STORAGE]: '+let a = 2;\n' } }));
    const second = computeSut(makeProbe({ porcelain, diffs: { [STORAGE]: '+let a = 3;\n' } }));
    assert.notEqual(first.sut, second.sut);
  });

  void it('MOVES when an untracked source file appears in a hashed tree', () => {
    const clean = computeSut(makeProbe({}));
    const added = computeSut(
      makeProbe({
        porcelain: { [STORAGE]: '?? elohim/elohim-storage/src/new_mod.rs\n' },
        untracked: { [STORAGE]: 'elohim/elohim-storage/src/new_mod.rs\n' },
        files: { 'elohim/elohim-storage/src/new_mod.rs': 'pub fn x() {}' },
      })
    );
    assert.notEqual(clean.sut, added.sut);
  });

  void it('populates unknown — never omits it — for parts it cannot hash', () => {
    const identity = computeSut(makeProbe({}));
    // No E2E_DNA_HASH and no fixture manifest in this world.
    assert.deepEqual(identity.unknown, [UNKNOWN_DNA_HASH, UNKNOWN_FIXTURES]);
    assert.equal(identity.sutParts['dnaHash'], undefined);
    assert.equal(identity.sutParts['fixtures'], undefined);
    // and the named-but-absent parts are LEFT OUT of the hash, not hashed as ''
    assert.equal(identity.sut, hashSutParts(identity.sutParts));
  });

  void it('names every component in unknown when nothing at all is obtainable', () => {
    const blind = computeSut(makeProbe({ trees: {} as Record<string, string> }));
    const probe = makeProbe({});
    // Force every tree lookup to fail.
    probe.git = () => null;
    const nothing = computeSut(probe);
    assert.deepEqual(nothing.sutParts, {});
    assert.deepEqual(nothing.unknown, [
      'sut.storage',
      'sut.doorway',
      'sut.conductor',
      'sut.dna',
      UNKNOWN_DNA_HASH,
      UNKNOWN_FIXTURES,
    ]);
    assert.ok(blind.sut.startsWith('sha256:'));
  });

  void it('prefers a declared artifact identity over the source tree', () => {
    const declared = computeSut(makeProbe({ env: { A2O_SUT_STORAGE: 'sha256:deadbeefdeadbeef' } }));
    assert.equal(declared.sutParts['storage'], 'declared:sha256:deadbeefdeadbeef');
    assert.notEqual(declared.sut, computeSut(makeProbe({})).sut);
  });

  void it('hashes the fixture manifest by its bytes, and moves when they change', () => {
    const one = computeSut(
      makeProbe({
        env: { E2E_HOUSEHOLD_FIXTURE_PATH: FIXTURE_PATH },
        files: { [FIXTURE_PATH]: '{"storagePeers":{"matthew":{"url":"a"}}}' },
      })
    );
    const two = computeSut(
      makeProbe({
        env: { E2E_HOUSEHOLD_FIXTURE_PATH: FIXTURE_PATH },
        files: { [FIXTURE_PATH]: '{"storagePeers":{"matthew":{"url":"b"}}}' },
      })
    );
    assert.match(one.sutParts['fixtures'], /^file:sha256:[0-9a-f]{16}$/);
    assert.notEqual(one.sut, two.sut);
    assert.ok(!one.unknown.includes(UNKNOWN_FIXTURES));
  });

  void it('declares a named-but-unreadable artifact unknown rather than falling back', () => {
    const identity = computeSut(
      makeProbe({ env: { E2E_HOUSEHOLD_FIXTURE_PATH: '/fake/gone.json' } })
    );
    assert.ok(identity.unknown.includes(UNKNOWN_FIXTURES));
  });

  void it('hashes parts independently of key insertion order', () => {
    assert.equal(hashSutParts({ a: '1', b: '2' }), hashSutParts({ b: '2', a: '1' }));
    assert.notEqual(hashSutParts({ a: '1' }), hashSutParts({ a: '2' }));
  });
});

void describe('env — one measure, many environments', () => {
  void it('reads peers and processControl from the household fixture', () => {
    const env = computeRunEnv(
      makeProbe({
        env: { E2E_HOUSEHOLD_FIXTURE_PATH: FIXTURE_PATH },
        files: {
          [FIXTURE_PATH]: JSON.stringify({
            processControl: true,
            storagePeers: {
              matthew: { url: 'http://localhost:8090' },
              jessica: { url: 'http://localhost:8091' },
              james: { url: 'http://localhost:8092' },
            },
          }),
        },
      }),
      { profile: 'mesh' }
    );
    assert.equal(env.lane, 'household');
    assert.equal(env.peers, 3);
    assert.equal(env.processControl, true);
    assert.ok(!env.unknown.includes('peers'));
    // A lane that CAN kill a peer is not answering for a cluster it does not run.
    assert.ok(!env.unknown.includes('clusterState'));
  });

  void it('declares the fleet unhashables — cluster state, PVC pressure, live DNS', () => {
    const env = computeRunEnv(makeProbe({}), { profile: 'dataplane' });
    assert.equal(env.lane, 'alpha-fleet');
    assert.equal(env.processControl, false);
    for (const name of ['peers', 'networkStage', 'clusterState', 'pvcPressure', 'liveDns']) {
      assert.ok(env.unknown.includes(name), `${name} must be declared unknown, not omitted`);
    }
    assert.equal(env.peers, 0, 'an unobservable peer count is 0 AND named, never guessed');
  });

  void it('honours an explicit --lane over the profile default', () => {
    const env = computeRunEnv(makeProbe({}), { profile: 'dataplane', lane: 'household' });
    assert.equal(env.lane, 'household');
    assert.equal(env.processControl, true);
  });

  void it('accepts a declared network stage and stops calling it unknown', () => {
    const env = computeRunEnv(makeProbe({ env: { E2E_NETWORK_STAGE: 'coordinated' } }), {
      profile: 'mesh',
    });
    assert.equal(env.networkStage, 'coordinated');
    assert.ok(!env.unknown.includes('networkStage'));
  });

  void it('refuses an undeclared network stage value', () => {
    const env = computeRunEnv(makeProbe({ env: { E2E_NETWORK_STAGE: 'productionish' } }), {
      profile: 'mesh',
    });
    assert.equal(env.networkStage, 'unknown');
    assert.ok(env.unknown.includes('networkStage'));
  });

  void it('maps profiles to lanes, and refuses to guess an unlisted one', () => {
    assert.equal(laneForProfile('mesh'), 'household');
    assert.equal(laneForProfile('dataplane'), 'alpha-fleet');
    assert.equal(laneForProfile('something-new'), 'unknown');
  });

  void it('prefers GIT_COMMIT, falls back to git, and omits when neither answers', () => {
    assert.equal(resolveGitCommit(makeProbe({ env: { GIT_COMMIT: 'abc123' } })), 'abc123');
    assert.equal(resolveGitCommit(makeProbe({ head: 'def456' })), 'def456');
    const blind = makeProbe({});
    blind.git = () => null;
    assert.equal(resolveGitCommit(blind), undefined);
  });
});

const SYNTHETIC_HABITS = `habits:
  - id: dataplane-convergence
    status: green
    checks:
      - "a2o @concern:federation-deploy (features/dataplane/federation-deploy.feature)"
      - "a2o @concern:content-sync, @concern:peer-mesh (edge Dataplane Validation stage)"
    evidence: >
      DELTA: @concern:notary-authority went 0/3 on the fleet — prose, not a check.
      A whole-file grep would mint a concern nobody declared.
  - id: blob-durability
    status: red
    checks:
      - "a2o @concern:blob-durability — 8 scenarios"
    refs:
      - "@concern:some-backlog-tag"
`;

void describe('declared — the denominator, read from outside the run', () => {
  void it('reads concerns from checks: blocks only, never from evidence prose', () => {
    assert.deepEqual(parseDeclaredConcerns(SYNTHETIC_HABITS), [
      'blob-durability',
      CONTENT_SYNC,
      'federation-deploy',
      'peer-mesh',
    ]);
  });

  void it('reads the real habits.yaml as a non-empty declared set', () => {
    const path = fileURLToPath(new URL('../../../manifests/habits.yaml', import.meta.url));
    const concerns = parseDeclaredConcerns(readFileSync(path, 'utf8'));
    assert.ok(concerns.length > 0, 'the covenant file must yield a denominator');
    assert.ok(concerns.includes('notary-authority'));
    assert.deepEqual(
      concerns,
      [...concerns].sort((a, b) => a.localeCompare(b))
    );
  });

  void it('intersects with a lane scope when one is declared', () => {
    const set = readDeclaredConcerns(
      'habits.yaml',
      [CONTENT_SYNC, 'not-declared'],
      () => SYNTHETIC_HABITS
    );
    assert.deepEqual(set.concerns, [CONTENT_SYNC]);
    assert.match(set.source, /lane scope/);
  });

  void it('carries an unreadable denominator in source instead of reporting an empty one', () => {
    const set = readDeclaredConcerns('/nowhere/habits.yaml', undefined, () => {
      throw new Error('ENOENT');
    });
    assert.deepEqual(set.concerns, []);
    assert.match(set.source, /UNREADABLE/);
  });
});
