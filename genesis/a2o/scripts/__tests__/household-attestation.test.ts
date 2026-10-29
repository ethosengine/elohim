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
  peerChainVerdict,
  publishHouseholdEvidence,
  putAttestation,
  readHouseholdAttestations,
  resolveMooring,
  resolveWorkspaceIdentity,
  sutArtifactCidBytes,
  sutOf,
  verifyValidationNode,
  workspaceAgentId,
} from '../lib/household-attestation.js';
import { hashSutParts } from '../lib/sut.js';

import type {
  PeerStage,
  PeerTaskStatus,
  Runner,
  ValidationNodeJson,
} from '../lib/household-attestation.js';

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
      assert.deepEqual(
        resolveMooring({
          BERTH_DIR: dir,
          ELOHIM_SESSION_ID: 'sess-1',
          ELOHIM_SESSION_OWNER_PID: String(process.pid),
        }),
        {
          session: 'sess-1',
          principal: 'root',
          recipient: { model: 'opus' },
        }
      );
      assert.equal(
        resolveMooring({
          BERTH_DIR: dir,
          ELOHIM_SESSION_ID: 'nobody',
          ELOHIM_SESSION_OWNER_PID: String(process.pid),
        }),
        null
      );
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
      const puts = calls.filter(c => c.cmd === brit && c.args[3] === 'put');
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
      assert.equal(found[0].verifiedBy, 'local');
      assert.deepEqual(readHouseholdAttestations(repo, 'absent'), []);
    } finally {
      rmSync(repo, { recursive: true, force: true });
    }
  });
});

describe('putAttestation: the one brit put the household writer and the peer collector share', () => {
  it('runs the exact argv the household writer always ran', () => {
    const calls: { cmd: string; args: string[]; cwd: string }[] = [];
    const run: Runner = (cmd, args, cwd) => {
      calls.push({ cmd, args, cwd });
      return { status: 0, stdout: '', stderr: '' };
    };
    const [a] = attestationsFromReport(householdReport({ a2o: 'tree:1' }), 'r.json', null).filter(
      x => x.summary.concern === CONCERN
    );
    const r = putAttestation('/bin/brit-build-ref', '/ws', a, run);
    assert.equal(r.status, 0);
    putAttestation('/bin/brit-build-ref', '/ws', a, run, '/ws/worktree');
    const sut = hashSutParts({ a2o: 'tree:1' });
    const argv = [
      '--repo',
      '/ws',
      'validate',
      'put',
      '--step',
      'a2o-household',
      '--check',
      `${CONCERN}/${sut}`,
      '--artifact',
      cidToString(sutArtifactCidBytes({ a2o: 'tree:1' })),
      '--result',
      'pass',
      '--summary',
      JSON.stringify(a.summary),
      '--validator-version',
      'a2o/build-sprint-report@1',
    ];
    assert.deepEqual(calls, [
      { cmd: '/bin/brit-build-ref', args: argv, cwd: '/ws' },
      { cmd: '/bin/brit-build-ref', args: argv, cwd: '/ws/worktree' },
    ]);
  });
});

describe('a peer-executed stage is admitted by its verified DHT chain, never by a key join', () => {
  const id = VECTOR.node.validatorId;
  const base = JSON.parse(VECTOR.node.resultSummary);
  const PEER: PeerStage = {
    rung: 'H',
    provider: 'uhCAk-jessica',
    requester: 'uhCAk-matthew',
    grantActionHash: 'uhCkk-grant',
    grantCid: 'bafy-grant',
    scope: 'measure-stage',
    requestActionHash: 'uhCkk-request',
    completionActionHash: 'uhCkk-completion',
    receiptCid: 'bafy-receipt',
    taskCid: 'bafy-task',
    featureSha256: 'f'.repeat(64),
    reportSha256: 'e'.repeat(64),
  };
  /**
   * Every field PeerTaskStatus declares is populated (the type forces it), so a fixture can no
   * longer mirror an omission in the verdict: the leaf loop below mutates each one.
   */
  type Observed = NonNullable<PeerTaskStatus['observed']>;
  type CompleteStatus = Required<Omit<PeerTaskStatus, 'observed'>> & {
    envelope: { project: string; dna: { sha256: string } };
    completion: { actionHash: string; receiptCid: string };
    observed: Required<Omit<Observed, 'refused'>>;
  };
  function verified(): CompleteStatus {
    return {
      taskCid: PEER.taskCid,
      requestActionHash: PEER.requestActionHash,
      refusal: null,
      state: 'completed',
      requester: PEER.requester,
      provider: PEER.provider,
      grantActionHash: PEER.grantActionHash,
      envelope: { project: 'a2o-stage:peer-executed-stage', dna: { sha256: PEER.featureSha256 } },
      completion: { actionHash: PEER.completionActionHash, receiptCid: PEER.receiptCid },
      observed: {
        verified: true,
        grantCid: PEER.grantCid,
        scope: 'measure-stage',
        grantProvider: PEER.provider,
        grantRecipient: PEER.requester,
        fulfilledEventId: `compute-fulfilled:${PEER.requestActionHash}`,
      },
    };
  }
  const peerNode = (
    peer: PeerStage = PEER,
    seedHex?: string,
    over: Partial<ValidationNodeJson> = {}
  ) =>
    resign({ ...VECTOR.node, ...over, resultSummary: JSON.stringify({ ...base, peer }) }, seedHex);
  const lookup = (status: PeerTaskStatus | undefined) => (hash: string) =>
    hash === PEER.requestActionHash ? status : undefined;
  const refusal = (node: ValidationNodeJson, status: PeerTaskStatus | undefined) => {
    const verdict = admitAttestation(node, CONCERN, id, lookup(status));
    return verdict.ok ? 'admitted' : verdict.reason;
  };

  it('admits a peer summary whose chain the requester task record verifies; the artifact stays the SUT', () => {
    const verdict = admitAttestation(peerNode(), CONCERN, id, lookup(verified()));
    assert.equal(verdict.ok, true);
    assert.deepEqual(verdict.ok && verdict.summary.peer, PEER);
    assert.equal(cidShortFingerprint(peerNode().artifactCid), base.sut);
    // Keying the attestation by the receipt instead of the tree it verified is refused.
    const byReceipt = peerNode(PEER, undefined, {
      artifactCid: [...sutArtifactCidBytes({ receipt: PEER.receiptCid })],
    });
    assert.equal(refusal(byReceipt, verified()), 'artifact CID is not the sut');
  });

  it('refuses a peer summary signed by a NON-workspace key even when its chain verifies', () => {
    const seed = '22'.repeat(32);
    const common = mkdtempSync(join(tmpdir(), 'brit-peer-'));
    try {
      mkdirSync(join(common, 'brit'));
      writeFileSync(join(common, 'brit', 'agent-key'), Buffer.from(seed, 'hex'));
      const foreignId = workspaceAgentId(common) ?? '';
      const foreign = peerNode(PEER, seed, { validatorId: foreignId });
      assert.equal(verifyValidationNode(foreign), true, 'validly signed by its own key');
      assert.equal(peerChainVerdict(PEER, verified()).ok, true, 'and its chain verifies');
      assert.equal(refusal(foreign, verified()), 'not the workspace key');
    } finally {
      rmSync(common, { recursive: true, force: true });
    }
  });

  it('refuses an unreachable chain; a household summary needs no chain at all', () => {
    const reason = 'peer chain unverifiable (requester storage unreachable)';
    assert.equal(refusal(peerNode(), undefined), reason);
    assert.equal(
      (admitAttestation(peerNode(), CONCERN, id) as { reason: string }).reason,
      reason,
      'no lookup at all'
    );
    assert.equal(admitAttestation(VECTOR.node, CONCERN, id, () => undefined).ok, true);
    assert.equal(admitAttestation(VECTOR.node, CONCERN, id).ok, true);
  });

  it('refuses each broken link with its own reason', () => {
    const cases: [string, PeerTaskStatus, string][] = [
      [
        'receipt',
        {
          ...verified(),
          completion: { actionHash: PEER.completionActionHash, receiptCid: 'bafy-other' },
        },
        'peer receipt CID is not the completion receipt',
      ],
      [
        'scope',
        { ...verified(), observed: { ...verified().observed, scope: 'sweettest-feedback' } },
        'peer grant scope is sweettest-feedback, not measure-stage',
      ],
      [
        'grant provider',
        { ...verified(), observed: { ...verified().observed, grantProvider: 'uhCAk-mallory' } },
        'peer provider is not the grant provider',
      ],
      [
        'unobserved',
        { ...verified(), observed: undefined },
        'peer grant not observed by requester storage',
      ],
      [
        'observation refused',
        {
          ...verified(),
          observed: { verified: false, refused: 'signed grant party or scope mismatch' },
        },
        'peer grant not verified by requester storage: signed grant party or scope mismatch',
      ],
      [
        'not completed',
        { ...verified(), state: 'accepted' },
        'peer task is accepted, not completed',
      ],
      [
        'completion',
        { ...verified(), completion: { actionHash: 'uhCkk-other', receiptCid: PEER.receiptCid } },
        'peer completion is not the one the task record carries',
      ],
      [
        'task provider',
        { ...verified(), provider: 'uhCAk-mallory' },
        'peer provider is not the task provider',
      ],
      [
        'requester',
        { ...verified(), requester: 'uhCAk-mallory' },
        'peer requester is not the task requester',
      ],
      [
        'grant',
        { ...verified(), grantActionHash: 'uhCkk-other' },
        'peer grant is not the task grant',
      ],
      [
        'grant cid',
        { ...verified(), observed: { ...verified().observed, grantCid: 'bafy-other' } },
        'peer grant CID is not the observed grant',
      ],
      [
        'recipient',
        { ...verified(), observed: { ...verified().observed, grantRecipient: 'uhCAk-mallory' } },
        'peer requester is not the grant recipient',
      ],
      [
        'project',
        {
          ...verified(),
          envelope: { project: 'sweettest:x', dna: { sha256: PEER.featureSha256 } },
        },
        'peer task is not an a2o-stage: stage',
      ],
      [
        'feature',
        { ...verified(), envelope: { project: 'a2o-stage:x', dna: { sha256: '0'.repeat(64) } } },
        'peer feature digest is not the task payload',
      ],
    ];
    for (const [label, status, reason] of cases) {
      assert.equal(refusal(peerNode(), status), reason, label);
    }
  });

  it('binds the returned task identity: a verified record for task A never admits a summary claiming B', () => {
    const claimsB = peerNode({ ...PEER, taskCid: 'bafy-task-B' });
    assert.equal(
      refusal(claimsB, { ...verified(), taskCid: 'bafy-task-A' }),
      'peer task CID is not the task record'
    );
    assert.equal(refusal(claimsB, verified()), 'peer task CID is not the task record');
    assert.equal(
      refusal(peerNode(), { ...verified(), requestActionHash: 'uhCkk-other-request' }),
      'peer request is not the task record the requester returned'
    );
  });

  it('every populated field of the complete task record binds the verdict', () => {
    // Leaves the verdict deliberately does not compare, named so a new unbound field is a choice.
    const unbound = new Set(['refusal', 'observed.fulfilledEventId']);
    const leaves: string[][] = [];
    const walk = (value: unknown, path: string[]) => {
      if (value && typeof value === 'object') {
        for (const [k, v] of Object.entries(value)) walk(v, [...path, k]);
      } else leaves.push(path);
    };
    walk(verified(), []);
    assert.ok(leaves.length >= 17, `walked ${leaves.length} leaves`);
    for (const path of leaves) {
      if (unbound.has(path.join('.'))) continue;
      const status = verified() as unknown as Record<string, unknown>;
      let at = status;
      for (const k of path.slice(0, -1)) at = at[k] as Record<string, unknown>;
      const leaf = path.at(-1) as string;
      at[leaf] = typeof at[leaf] === 'boolean' ? !at[leaf] : `other-${String(at[leaf])}`;
      assert.notEqual(
        refusal(peerNode(), status as PeerTaskStatus),
        'admitted',
        `${path.join('.')} does not bind`
      );
    }
    assert.equal(refusal(peerNode(), verified()), 'admitted');
  });

  it('refuses a malformed peer block', () => {
    const partial: Partial<PeerStage> = { ...PEER };
    delete partial.receiptCid;
    assert.equal(refusal(peerNode(partial as PeerStage), verified()), 'peer block is malformed');
    assert.equal(
      refusal(peerNode({ ...PEER, rung: 'Z' } as unknown as PeerStage), verified()),
      'peer block is malformed'
    );
  });
});

// ─── the brit-build-ref contract, old pin and new ──────────────────────────────
//
// Stub executables print the EXACT streams the two brit lineages print (read from source at
// brit 7e4a83115e and executed on the pinned 76ba95d8a0): `signer --json`, `validate get`'s
// miss + `unverifiable:` lines, and `validate put`'s refusals. Fixtures live under TMPDIR, never
// in the tree.

const PINNED_SIGNER_STDERR =
  "error: unrecognized subcommand 'signer'\n\nUsage: brit-build-ref [OPTIONS] <COMMAND>\n\nFor more information, try '--help'.\n";

/** The pinned signer stderr as a single-quoted `printf` argument (newlines as `\n`). */
const PINNED_SIGNER_SH = PINNED_SIGNER_STDERR.replaceAll('\n', String.raw`\n`);

/** A stub brit-build-ref: `kind` picks the lineage; `get/<key>.{out,err,code}` script `validate get`. */
function stubBrit(kind: 'new' | 'pinned', agentId: string | null): { bin: string; dir: string } {
  const dir = mkdtempSync(join(tmpdir(), `brit-stub-${kind}-`));
  mkdirSync(join(dir, 'get'));
  const signer =
    kind === 'new'
      ? String.raw`printf '%s\n' '${JSON.stringify({ agentId })}'; exit 0`
      : String.raw`printf '%s' "${PINNED_SIGNER_SH}" >&2; exit 2`;
  const script = String.raw`#!/bin/sh
DIR='${dir}'
printf '%s\n' "$*" >> "$DIR/calls.log"
if [ "$1" = "--repo" ]; then shift 2; fi
case "$1 $2" in
  "signer --json"|"signer ") ${signer} ;;
  "validate get")
    step=''; check=''
    while [ $# -gt 0 ]; do
      [ "$1" = "--step" ] && step="$2"
      [ "$1" = "--check" ] && check="$2"
      shift
    done
    key=$(printf '%s' "$check" | tr '/:' '__')
    if [ -f "$DIR/get/$key.out" ]; then cat "$DIR/get/$key.out"; fi
    if [ -f "$DIR/get/$key.err" ]; then cat "$DIR/get/$key.err" >&2
    elif [ ! -f "$DIR/get/$key.out" ]; then printf 'no validate ref found for step=%s check=%s\\n' "$step" "$check" >&2; fi
    if [ -f "$DIR/get/$key.code" ]; then exit "$(cat "$DIR/get/$key.code")"; fi
    exit 0 ;;
  "validate put")
    if [ -f "$DIR/put.out" ]; then cat "$DIR/put.out"; else printf 'bafyreistub\\n'; fi
    if [ -f "$DIR/put.err" ]; then cat "$DIR/put.err" >&2; fi
    if [ -f "$DIR/put.code" ]; then exit "$(cat "$DIR/put.code")"; fi
    exit 0 ;;
  *) printf "error: unrecognized subcommand '%s'\\n" "$1" >&2; exit 2 ;;
esac
`;
  const bin = join(dir, 'brit-build-ref');
  writeFileSync(bin, script);
  chmodSync(bin, 0o755);
  return { bin, dir };
}

function stubGet(dir: string, check: string, r: { out?: string; err?: string; code?: number }) {
  const key = check.replaceAll(/[/:]/g, '_');
  if (r.out !== undefined) writeFileSync(join(dir, 'get', `${key}.out`), r.out);
  if (r.err !== undefined) writeFileSync(join(dir, 'get', `${key}.err`), r.err);
  if (r.code !== undefined) writeFileSync(join(dir, 'get', `${key}.code`), String(r.code));
}

function stubPut(dir: string, r: { out?: string; err?: string; code?: number }) {
  if (r.out !== undefined) writeFileSync(join(dir, 'put.out'), r.out);
  if (r.err !== undefined) writeFileSync(join(dir, 'put.err'), r.err);
  if (r.code !== undefined) writeFileSync(join(dir, 'put.code'), String(r.code));
}

function calls(dir: string): string[] {
  try {
    return readFileSync(join(dir, 'calls.log'), 'utf8').trim().split('\n');
  } catch {
    return [];
  }
}

function keyedRepo(prefix: string, seedHex = VECTOR.seedHex): string {
  const repo = scratchRepo(prefix);
  mkdirSync(join(repo, '.git', 'brit'));
  writeFileSync(join(repo, '.git', 'brit', 'agent-key'), Buffer.from(seedHex, 'hex'));
  return repo;
}

const VECTOR_CHECK = VECTOR.node.checkName;
const OTHER_ID = 'ab'.repeat(32);

describe('workspace identity prefers `brit-build-ref signer --json`', () => {
  it('reads the id the new brit reports, with no key file to read at all', () => {
    const repo = scratchRepo('ident-new-');
    const brit = stubBrit('new', OTHER_ID);
    try {
      const id = resolveWorkspaceIdentity(repo, { BRIT_BUILD_REF: brit.bin });
      assert.deepEqual(id, {
        agentId: OTHER_ID,
        source: 'brit signer',
        brit: { kind: 'verified-reads', bin: brit.bin },
      });
      assert.deepEqual(calls(brit.dir), [`--repo ${repo} signer --json`]);
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(brit.dir, { recursive: true, force: true });
    }
  });

  it('BRIT_BUILD_REF_BIN (the orchestrator name) wins over BRIT_BUILD_REF, which wins over PATH', () => {
    const repo = scratchRepo('ident-env-');
    const a = stubBrit('new', OTHER_ID);
    const b = stubBrit('new', VECTOR.node.validatorId);
    try {
      const env = { BRIT_BUILD_REF_BIN: a.bin, BRIT_BUILD_REF: b.bin, PATH: '/nonexistent-bin' };
      assert.equal(resolveWorkspaceIdentity(repo, env).agentId, OTHER_ID);
      assert.equal(
        resolveWorkspaceIdentity(repo, { BRIT_BUILD_REF: b.bin, PATH: '/nonexistent-bin' }).agentId,
        VECTOR.node.validatorId
      );
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(a.dir, { recursive: true, force: true });
      rmSync(b.dir, { recursive: true, force: true });
    }
  });

  it('a new brit that reports no key, or refuses to load one, is never second-guessed by the file', () => {
    const repo = keyedRepo('ident-none-');
    const none = stubBrit('new', null);
    const broken = stubBrit('new', OTHER_ID);
    writeFileSync(
      broken.bin,
      '#!/bin/sh\nprintf "Error: cannot load the repository signer key\\n\\nCaused by:\\n    seed is 31 bytes\\n" >&2\nexit 1\n'
    );
    try {
      const noKey = resolveWorkspaceIdentity(repo, { BRIT_BUILD_REF: none.bin });
      assert.equal(noKey.agentId, null, 'the key file is not consulted');
      assert.equal(noKey.source, 'brit signer');
      const refused = resolveWorkspaceIdentity(repo, { BRIT_BUILD_REF: broken.bin });
      assert.equal(refused.agentId, null);
      assert.equal(refused.source, 'brit signer');
      assert.equal(refused.note, 'Error: cannot load the repository signer key');
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(none.dir, { recursive: true, force: true });
      rmSync(broken.dir, { recursive: true, force: true });
    }
  });

  it('falls back to the key file only for the pinned brit (clap exit 2) or no brit at all', () => {
    const repo = keyedRepo('ident-pinned-');
    const pinned = stubBrit('pinned', null);
    try {
      const viaPinned = resolveWorkspaceIdentity(repo, { BRIT_BUILD_REF: pinned.bin });
      assert.deepEqual(viaPinned, {
        agentId: VECTOR.node.validatorId,
        source: 'agent-key file',
        brit: { kind: 'pinned', bin: pinned.bin },
      });
      const viaNone = resolveWorkspaceIdentity(repo, { PATH: '/nonexistent-bin' });
      assert.deepEqual(viaNone, {
        agentId: VECTOR.node.validatorId,
        source: 'agent-key file',
        brit: { kind: 'absent', bin: null },
      });
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(pinned.dir, { recursive: true, force: true });
    }
  });
});

describe('a record brit reports `unverifiable:` for is absent to the reader, and said so', () => {
  const TAMPERED_REF = `${ATTESTATION_REF_PREFIX}${CONCERN}/sha256%3Abad`;
  const TAMPERED_CHECK = `${CONCERN}/sha256:bad`;

  function refsRepo(): string {
    const repo = scratchRepo('read-brit-');
    const good = git(repo, ['hash-object', '-w', '--stdin'], JSON.stringify(VECTOR.node)).trim();
    const bad = git(
      repo,
      ['hash-object', '-w', '--stdin'],
      JSON.stringify({ ...VECTOR.node, result: 'fail' })
    ).trim();
    git(repo, ['update-ref', VECTOR.ref, good]);
    git(repo, ['update-ref', TAMPERED_REF, bad]);
    return repo;
  }

  it('new brit: `validate get` per listed ref, decoded from the ref; stdout is the hit, `unverifiable:` is a warned absence', () => {
    const repo = refsRepo();
    const brit = stubBrit('new', VECTOR.node.validatorId);
    const reason = `signature does not verify against agentId ${JSON.stringify(VECTOR.node.validatorId)}`;
    stubGet(brit.dir, VECTOR_CHECK, { out: JSON.stringify(VECTOR.node, null, 2) + '\n' });
    stubGet(brit.dir, TAMPERED_CHECK, {
      err: `no validate ref found for step=a2o-household check=${TAMPERED_CHECK}\nunverifiable: ${TAMPERED_REF} ${reason}\n`,
    });
    try {
      const warned: string[] = [];
      const found = readHouseholdAttestations(repo, CONCERN, {
        brit: { kind: 'verified-reads', bin: brit.bin },
        warn: l => warned.push(l),
      });
      assert.deepEqual(
        found.map(f => [f.ref, f.verifiedBy]),
        [[VECTOR.ref, 'brit']]
      );
      assert.deepEqual(found[0].node, VECTOR.node);
      assert.deepEqual(warned, [
        `household attestation unverifiable: ${TAMPERED_REF} ${reason} (brit-build-ref verified read)`,
      ]);
      assert.deepEqual(
        calls(brit.dir).sort(),
        [
          `--repo ${repo} validate get --step a2o-household --check ${TAMPERED_CHECK}`,
          `--repo ${repo} validate get --step a2o-household --check ${VECTOR_CHECK}`,
        ].sort()
      );
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(brit.dir, { recursive: true, force: true });
    }
  });

  it('new brit: a git failure (exit 1) or non-JSON stdout is an absence with a warning, never a hit', () => {
    const repo = refsRepo();
    const brit = stubBrit('new', VECTOR.node.validatorId);
    stubGet(brit.dir, VECTOR_CHECK, {
      err: 'Error: git command failed: git for-each-ref: fatal: not a git repository\n',
      code: 1,
    });
    stubGet(brit.dir, TAMPERED_CHECK, { out: 'not json\n' });
    try {
      const warned: string[] = [];
      const found = readHouseholdAttestations(repo, CONCERN, {
        brit: { kind: 'verified-reads', bin: brit.bin },
        warn: l => warned.push(l),
      });
      assert.deepEqual(found, []);
      assert.equal(warned.length, 2);
      assert.match(warned.join('\n'), /validate get exited 1: Error: git command failed/);
      assert.match(warned.join('\n'), /printed no JSON/);
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(brit.dir, { recursive: true, force: true });
    }
  });

  it('pinned or absent brit: the reader verifies locally with the same reasons, and names its verifier', () => {
    const repo = refsRepo();
    const filed = `${ATTESTATION_REF_PREFIX}${CONCERN}/sha256%3Aother`;
    const good = git(repo, ['hash-object', '-w', '--stdin'], JSON.stringify(VECTOR.node)).trim();
    git(repo, ['update-ref', filed, good]);
    const junk = git(repo, ['hash-object', '-w', '--stdin'], '{not json').trim();
    git(repo, ['update-ref', `${ATTESTATION_REF_PREFIX}${CONCERN}/sha256%3Ajunk`, junk]);
    try {
      for (const kind of ['pinned', 'absent'] as const) {
        const warned: string[] = [];
        const found = readHouseholdAttestations(repo, CONCERN, {
          brit: { kind, bin: kind === 'pinned' ? '/bin/false' : null },
          warn: l => warned.push(l),
        });
        assert.deepEqual(
          found.map(f => [f.ref, f.verifiedBy]),
          [[VECTOR.ref, 'local']]
        );
        const verifier =
          kind === 'pinned'
            ? 'verified locally; the pinned brit-build-ref has no verified read'
            : 'verified locally; brit-build-ref is not on this host';
        const otherCheck = JSON.stringify(`${CONCERN}/sha256:other`);
        const got = [...warned];
        got.sort();
        const want = [
          `household attestation unverifiable: ${TAMPERED_REF} signature does not verify against agentId ${JSON.stringify(VECTOR.node.validatorId)} (${verifier})`,
          `household attestation unverifiable: ${filed} checkName ${JSON.stringify(VECTOR_CHECK)} does not match this ref's ${otherCheck} (${verifier})`,
          `household attestation unverifiable: ${ATTESTATION_REF_PREFIX}${CONCERN}/sha256%3Ajunk does not decode: Expected property name or '}' in JSON at position 1 (line 1 column 2) (${verifier})`,
        ];
        want.sort();
        assert.deepEqual(got, want);
      }
    } finally {
      rmSync(repo, { recursive: true, force: true });
    }
  });
});

describe('the writer gets before it puts, and a brit refusal is an expected, warn-only outcome', () => {
  const PARTS = { a2o: 'tree:1' };
  const SUT = hashSutParts(PARTS);
  const CHECKS = [CONCERN, 'doorway-failover', 'mixed'].map(c => `${c}/${SUT}`).sort();

  function publish(repo: string, brit: string) {
    const lines: string[] = [];
    publishHouseholdEvidence({
      report: householdReport(PARTS),
      reportPath: '/r/sprint-report-household-1.json',
      repoRoot: repo,
      env: { BRIT_BUILD_REF: brit, PATH: '/nonexistent-bin' },
      log: l => lines.push(l),
    });
    return lines;
  }

  it('every put is preceded by a get of the same check; the summary names the signer', () => {
    const repo = keyedRepo('writer-order-');
    const brit = stubBrit('new', VECTOR.node.validatorId);
    try {
      const lines = publish(repo, brit.bin);
      const log = calls(brit.dir);
      assert.equal(log[0], `--repo ${repo} signer --json`);
      const verbs = log.slice(1).map(l => l.split(' ').slice(2, 4).join(' '));
      assert.deepEqual(verbs, [
        'validate get',
        'validate put',
        'validate get',
        'validate put',
        'validate get',
        'validate put',
      ]);
      const checks = log.slice(1).map(l => / --check (\S+)/.exec(l)?.[1]);
      for (let i = 0; i < checks.length; i += 2) assert.equal(checks[i], checks[i + 1]);
      assert.deepEqual([...new Set(checks)].sort(), CHECKS);
      assert.match(
        lines.at(-1) ?? '',
        /3\/3 written under refs\/notes\/brit\/validate\/a2o-household\/<concern>\/sha256:[0-9a-f]{16} \(workspace key [0-9a-f]{12}… via brit signer; no berth mooring\)/
      );
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(brit.dir, { recursive: true, force: true });
    }
  });

  it('a verified record of this same run is kept, not rewritten', () => {
    const repo = keyedRepo('writer-idem-');
    const brit = stubBrit('new', VECTOR.node.validatorId);
    const [mine] = attestationsFromReport(
      householdReport(PARTS),
      'sprint-report-household-1.json',
      null
    ).filter(a => a.summary.concern === CONCERN);
    const stored = resign({
      ...VECTOR.node,
      artifactCid: [...sutArtifactCidBytes(PARTS)],
      checkName: mine.check,
      resultSummary: JSON.stringify(mine.summary),
    });
    stubGet(brit.dir, mine.check, { out: JSON.stringify(stored, null, 2) + '\n' });
    try {
      const lines = publish(repo, brit.bin);
      const puts = calls(brit.dir).filter(l => l.includes(' validate put '));
      assert.equal(puts.length, 2, 'the other two concerns are put');
      assert.ok(!puts.some(l => l.includes(`--check ${mine.check} `)), 'this one is not');
      assert.ok(
        lines.includes(
          `validation attestation already on record for ${mine.check} (run run-1, verified by brit); not rewritten`
        ),
        lines.join('\n')
      );
      assert.match(lines.at(-1) ?? '', /3\/3 written under/);
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(brit.dir, { recursive: true, force: true });
    }
  });

  it('REFUSED concurrent update and refusing-to-write are deferred (warn-only), anything else still FAILED', () => {
    const repo = keyedRepo('writer-refuse-');
    const cases: [string, string, RegExp][] = [
      [
        'concurrent',
        `Error: REFUSED concurrent update: ref=refs/notes/brit/validate/a2o-household/${CONCERN}/${SUT.replace(':', '%3A')} expected=0000000000000000000000000000000000000000 observed=1111111111111111111111111111111111111111 attempts=3\n`,
        /deferred for .*: Error: REFUSED concurrent update: ref=refs\/notes\/brit\/validate\/a2o-household\/.* attempts=3$/m,
      ],
      [
        'would not verify',
        `Error: refusing to write refs/notes/brit/validate/a2o-household/${CONCERN}/${SUT.replace(':', '%3A')}: signature does not verify against agentId "${OTHER_ID}"\n`,
        /deferred for .*: Error: refusing to write refs\/notes\/brit\/validate\/a2o-household\/.*: signature does not verify against agentId/m,
      ],
      [
        'other',
        'Error: git command failed: update-ref refs/notes/brit/validate/x failed 3 times with the ref unmoved: fatal: disk full\n',
        /validation attestation FAILED for .* \(exit 1\): Error: git command failed: update-ref/m,
      ],
    ];
    try {
      for (const [label, stderr, expect] of cases) {
        const brit = stubBrit('new', VECTOR.node.validatorId);
        stubPut(brit.dir, { err: stderr, code: 1 });
        try {
          const lines = publish(repo, brit.bin);
          assert.match(lines.join('\n'), expect, label);
          assert.match(lines.at(-1) ?? '', /0\/3 written under/, label);
          if (label !== 'other') assert.doesNotMatch(lines.join('\n'), /FAILED/, label);
        } finally {
          rmSync(brit.dir, { recursive: true, force: true });
        }
      }
    } finally {
      rmSync(repo, { recursive: true, force: true });
    }
  });

  it('an unverifiable record at the ref is named, then replaced', () => {
    const repo = keyedRepo('writer-unverifiable-');
    const brit = stubBrit('new', VECTOR.node.validatorId);
    const check = `${CONCERN}/${SUT}`;
    const ref = `${ATTESTATION_REF_PREFIX}${CONCERN}/${SUT.replace(':', '%3A')}`;
    stubGet(brit.dir, check, {
      err: `no validate ref found for step=a2o-household check=${check}\nunverifiable: ${ref} does not decode: unknown field \`protocolReach\`\n`,
    });
    try {
      const lines = publish(repo, brit.bin);
      const named = `household attestation unverifiable: ${ref} does not decode: unknown field \`protocolReach\` (brit-build-ref verified read); replacing it`;
      assert.ok(lines.includes(named), lines.join('\n'));
      assert.equal(
        calls(brit.dir).filter(l => l.includes(` validate put `) && l.includes(`--check ${check} `))
          .length,
        1
      );
    } finally {
      rmSync(repo, { recursive: true, force: true });
      rmSync(brit.dir, { recursive: true, force: true });
    }
  });
});
