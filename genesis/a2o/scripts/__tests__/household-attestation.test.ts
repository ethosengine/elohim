/* eslint-disable @typescript-eslint/no-floating-promises -- node:test owns returned promises. */
/* eslint-disable sonarjs/no-duplicate-string -- fixture paths are intentionally repeated. */
import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { createPrivateKey, sign } from 'node:crypto';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import {
  ATTESTATION_REF_PREFIX,
  admitAttestation,
  attestationsFromReport,
  canonicalSutParts,
  checkNameFor,
  cidShortFingerprint,
  cidToString,
  encodeValidationNode,
  publishHouseholdEvidence,
  readHouseholdAttestations,
  resolveMooring,
  sutArtifactCidBytes,
  sutOf,
  verifyValidationNode,
  workspaceAgentId,
} from '../lib/household-attestation.js';
import { hashSutParts } from '../lib/sut.js';

import type { Runner, ValidationNodeJson } from '../lib/household-attestation.js';

/** A real `brit-build-ref validate put` output (throwaway key) — the cross-implementation vector. */
const VECTOR = JSON.parse(
  readFileSync(
    fileURLToPath(new URL('./fixtures/brit-validation-attestation.json', import.meta.url)),
    'utf8'
  )
) as { seedHex: string; ref: string; storedHex: string; node: ValidationNodeJson };
const CONCERN = 'push-delivers-within-budget';

const PKCS8_ED25519 = Buffer.from('302e020100300506032b657004220420', 'hex');
function resign(node: ValidationNodeJson, seedHex = VECTOR.seedHex): ValidationNodeJson {
  const key = createPrivateKey({
    key: Buffer.concat([PKCS8_ED25519, Buffer.from(seedHex, 'hex')]),
    format: 'der',
    type: 'pkcs8',
  });
  const unsigned = { ...node, signature: '' };
  return { ...node, signature: sign(null, encodeValidationNode(unsigned), key).toString('hex') };
}

function git(cwd: string, args: string[], input?: string): string {
  // eslint-disable-next-line sonarjs/no-os-command-from-path -- git is a standard system tool
  return execFileSync('git', args, { cwd, encoding: 'utf8', input });
}

function scratchRepo(prefix: string): string {
  const dir = mkdtempSync(join(tmpdir(), prefix));
  git(dir, ['init', '-q']);
  return dir;
}

function householdReport(parts: Record<string, string>) {
  return {
    runId: 'run-1',
    env: { lane: 'household', processControl: true, sut: hashSutParts(parts), sutParts: parts },
    summary: {
      byConcern: {
        [CONCERN]: {
          failed: 0,
          scenarios: [
            { name: 'a', status: 'passed', surface: 'features/x.feature', durationMs: 3 },
            { name: 'b', status: 'passed', surface: 'features/x.feature', durationMs: 4 },
          ],
        },
        'doorway-failover': {
          failed: 1,
          scenarios: [{ name: 'c', status: 'failed', surface: 'features/y.feature' }],
        },
        mixed: {
          failed: 0,
          scenarios: [
            { name: 'd', status: 'passed', surface: 'features/z.feature', durationMs: 1 },
            { name: 'e', status: 'skipped', surface: 'features/z.feature', durationMs: 0 },
          ],
        },
      },
    },
  };
}

describe('the attestation is keyed by the same source identity the report carries', () => {
  it('canonical text and sut agree with hashSutParts for every shape', () => {
    const shapes: Record<string, string>[] = [
      {},
      { a2o: 'tree:1' },
      { storage: 'tree:2+dirty:3', a2o: 'tree:1', fixtures: 'file:sha256:4', appStaging: 'tree:5' },
    ];
    for (const parts of shapes) {
      assert.equal(sutOf(parts), hashSutParts(parts));
      assert.ok(canonicalSutParts(parts).split('\n').length >= 1);
    }
  });

  it('the artifact CID is a raw sha2-256 CIDv1 whose short form IS the sut', () => {
    const parts = { a2o: 'tree:1', storage: 'tree:2' };
    const bytes = sutArtifactCidBytes(parts);
    assert.equal(cidShortFingerprint(bytes), hashSutParts(parts));
    assert.match(cidToString(bytes), /^bafkrei[a-z2-7]{52}$/);
    assert.equal(cidShortFingerprint([0x01, 0x71, ...bytes.slice(2)]), null);
  });

  it("brit's own payload carries exactly the CID bytes this module computes", () => {
    const summary = JSON.parse(VECTOR.node.resultSummary);
    assert.deepEqual(VECTOR.node.artifactCid, [...sutArtifactCidBytes(summary.sutParts)]);
    assert.equal(VECTOR.node.checkName, checkNameFor(CONCERN, summary.sut));
  });
});

describe('the reader verifies the signature brit made', () => {
  it('re-encodes the node to the exact DAG-CBOR bytes brit stored', () => {
    assert.equal(encodeValidationNode(VECTOR.node).toString('hex'), VECTOR.storedHex);
  });

  it('verifies the real signature and refuses any edited field', () => {
    assert.equal(verifyValidationNode(VECTOR.node), true);
    assert.equal(verifyValidationNode({ ...VECTOR.node, result: 'fail' }), false);
    assert.equal(
      verifyValidationNode({ ...VECTOR.node, resultSummary: VECTOR.node.resultSummary + ' ' }),
      false
    );
    assert.equal(verifyValidationNode({ ...VECTOR.node, signature: 'zz' }), false);
    const extra = { ...VECTOR.node, protocolReach: 'trusted' } as unknown as ValidationNodeJson;
    assert.equal(verifyValidationNode(extra), false);
  });

  it('derives the workspace id from the 32-byte seed brit keeps in the common dir', () => {
    const common = mkdtempSync(join(tmpdir(), 'brit-common-'));
    try {
      assert.equal(workspaceAgentId(common), null);
      mkdirSync(join(common, 'brit'));
      writeFileSync(join(common, 'brit', 'agent-key'), Buffer.from(VECTOR.seedHex, 'hex'));
      assert.equal(workspaceAgentId(common), VECTOR.node.validatorId);
    } finally {
      rmSync(common, { recursive: true, force: true });
    }
  });
});

describe('admission', () => {
  const id = VECTOR.node.validatorId;
  it('admits the real attestation for its concern under the workspace key', () => {
    const verdict = admitAttestation(VECTOR.node, CONCERN, id);
    assert.equal(verdict.ok, true);
    assert.equal(verdict.ok && verdict.summary.reach, 'trusted');
  });

  it('refuses another key, no key, and another concern', () => {
    assert.equal(admitAttestation(VECTOR.node, CONCERN, 'ab'.repeat(32)).ok, false);
    assert.equal(admitAttestation(VECTOR.node, CONCERN, null).ok, false);
    assert.equal(admitAttestation(VECTOR.node, 'doorway-failover', id).ok, false);
  });

  it('refuses a signed non-pass, a sut that does not re-derive, and a CID that is not the sut', () => {
    const summary = JSON.parse(VECTOR.node.resultSummary);
    const cases: [string, ValidationNodeJson][] = [
      ['fail', resign({ ...VECTOR.node, result: 'fail' })],
      [
        'sut',
        resign({
          ...VECTOR.node,
          resultSummary: JSON.stringify({
            ...summary,
            sutParts: { ...summary.sutParts, a2o: 'x' },
          }),
        }),
      ],
      [
        'cid',
        resign({ ...VECTOR.node, artifactCid: [...sutArtifactCidBytes({ other: 'tree:0' })] }),
      ],
      ['check', resign({ ...VECTOR.node, checkName: `${CONCERN}/sha256:0000000000000000` })],
      [
        'reach',
        resign({ ...VECTOR.node, resultSummary: JSON.stringify({ ...summary, reach: 'public' }) }),
      ],
    ];
    for (const [label, node] of cases) {
      assert.equal(verifyValidationNode(node), true, `${label} is validly signed`);
      assert.equal(admitAttestation(node, CONCERN, id).ok, false, label);
    }
  });
});

describe('writer: one attestation per measured concern of a household run', () => {
  const parts = { a2o: 'tree:1', storage: 'tree:2' };

  it('maps each concern to pass | fail | skip (never warn) with a sut-keyed check and artifact', () => {
    const moored = { session: 's', principal: 'root', recipient: { runtime: 'claude-code' } };
    const out = attestationsFromReport(householdReport(parts), 'r.json', moored);
    const byConcern = Object.fromEntries(out.map(a => [a.summary.concern, a]));
    assert.equal(byConcern[CONCERN].result, 'pass');
    assert.equal(byConcern['doorway-failover'].result, 'fail');
    assert.equal(byConcern.mixed.result, 'skip', 'passed + skipped is not proven');
    const sut = hashSutParts(parts);
    assert.equal(byConcern[CONCERN].check, `${CONCERN}/${sut}`);
    assert.equal(byConcern[CONCERN].artifact, cidToString(sutArtifactCidBytes(parts)));
    assert.deepEqual(byConcern[CONCERN].summary.moored, moored);
    assert.equal(byConcern[CONCERN].summary.report, 'r.json');
  });

  it('names no attestation for a fleet run, an uncontrolled run, or a sut that does not re-derive', () => {
    const fleet = householdReport(parts);
    fleet.env.lane = 'alpha-fleet';
    const loose = householdReport(parts);
    loose.env.processControl = false;
    const forged = householdReport(parts);
    forged.env.sut = 'sha256:0000000000000000';
    for (const r of [fleet, loose, forged])
      assert.deepEqual(attestationsFromReport(r, 'r', null), []);
  });

  it('reads the mooring berth keeps for this session', () => {
    const dir = mkdtempSync(join(tmpdir(), 'berth-'));
    try {
      mkdirSync(join(dir, 'moorings'));
      writeFileSync(
        join(dir, 'moorings', 'sess-1.json'),
        JSON.stringify({ session: 'sess-1', principal: 'root', recipient: { model: 'opus' } })
      );
      assert.deepEqual(resolveMooring({ BERTH_DIR: dir, BERTH_SESSION: 'sess-1' }), {
        session: 'sess-1',
        principal: 'root',
        recipient: { model: 'opus' },
      });
      assert.equal(resolveMooring({ BERTH_DIR: dir, BERTH_SESSION: 'nobody' }), null);
      assert.equal(resolveMooring({ BERTH_DIR: dir }), null);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

describe('publishHouseholdEvidence', () => {
  function fakeBin(dir: string, name: string): string {
    const path = join(dir, name);
    writeFileSync(path, '#!/bin/sh\nexit 0\n');
    chmodSync(path, 0o755);
    return path;
  }

  it('fulfills the report and puts one signed attestation per concern with the workspace repo', () => {
    const repo = scratchRepo('publish-');
    const bin = mkdtempSync(join(tmpdir(), 'bin-'));
    try {
      const calls: { cmd: string; args: string[] }[] = [];
      const run: Runner = (cmd, args) => {
        calls.push({ cmd, args });
        return { status: 0, stdout: 'ok\n', stderr: '' };
      };
      const epr = fakeBin(bin, 'epr');
      const brit = fakeBin(bin, 'brit-build-ref');
      const lines = publishHouseholdEvidence({
        report: householdReport({ a2o: 'tree:1' }),
        reportPath: '/r/sprint-report-household-1.json',
        repoRoot: repo,
        env: { PATH: bin },
        run,
        log: () => {},
      });
      assert.deepEqual(calls[0], {
        cmd: epr,
        args: ['flow', 'fulfill', '/r/sprint-report-household-1.json', '--root', repo],
      });
      const puts = calls.filter(c => c.cmd === brit);
      assert.equal(puts.length, 3);
      for (const p of puts) {
        assert.deepEqual(p.args.slice(0, 6), [
          '--repo',
          repo,
          'validate',
          'put',
          '--step',
          'a2o-household',
        ]);
      }
      assert.match(
        lines.at(-1) ?? '',
        /3\/3 written under refs\/notes\/brit\/validate\/a2o-household\//
      );
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(bin, { recursive: true, force: true });
    }
  });

  it('says so, and writes nothing, when brit-build-ref or epr is not built on this host', () => {
    const calls: string[] = [];
    const lines = publishHouseholdEvidence({
      report: householdReport({ a2o: 'tree:1' }),
      reportPath: '/r/x.json',
      repoRoot: '/nonexistent',
      env: { PATH: '/nonexistent-bin' },
      run: cmd => {
        calls.push(cmd);
        return { status: 0, stdout: '', stderr: '' };
      },
      log: () => {},
    });
    assert.deepEqual(calls, []);
    assert.match(lines.join('\n'), /no epr binary/);
    assert.match(lines.join('\n'), /brit-build-ref is not built on this host/);
  });

  it('does nothing for a fleet report or when A2O_POST_REPORT=0', () => {
    const fleet = householdReport({ a2o: 'tree:1' });
    fleet.env.lane = 'alpha-fleet';
    for (const [report, env] of [
      [fleet, { PATH: '/bin' }],
      [householdReport({ a2o: 'tree:1' }), { PATH: '/bin', A2O_POST_REPORT: '0' }],
    ] as const) {
      const lines = publishHouseholdEvidence({
        report,
        reportPath: '/r/x.json',
        repoRoot: '/',
        env,
        run: () => assert.fail('nothing runs'),
        log: () => {},
      });
      assert.deepEqual(lines, []);
    }
  });
});

describe('readHouseholdAttestations', () => {
  it('reads every blob ref under the concern and skips unreadable ones', () => {
    const repo = scratchRepo('read-');
    try {
      const good = git(repo, ['hash-object', '-w', '--stdin'], JSON.stringify(VECTOR.node)).trim();
      const bad = git(repo, ['hash-object', '-w', '--stdin'], '{not json').trim();
      git(repo, ['update-ref', VECTOR.ref, good]);
      git(repo, ['update-ref', `${ATTESTATION_REF_PREFIX}${CONCERN}/sha256%3Abad`, bad]);
      git(repo, ['update-ref', `${ATTESTATION_REF_PREFIX}doorway-failover/sha256%3Aother`, good]);
      const found = readHouseholdAttestations(repo, CONCERN);
      assert.deepEqual(
        found.map(f => f.ref),
        [VECTOR.ref]
      );
      assert.deepEqual(found[0].node, VECTOR.node);
      assert.deepEqual(readHouseholdAttestations(repo, 'absent'), []);
    } finally {
      rmSync(repo, { recursive: true, force: true });
    }
  });
});
