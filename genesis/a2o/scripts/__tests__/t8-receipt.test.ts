import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  closeSync,
  openSync,
  mkdtempSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { describe, it } from 'node:test';
import { fileURLToPath } from 'node:url';

import {
  joinHalves,
  runDirectory,
  sessionFor,
  T8_CHECK,
  type T8Command,
  type T8Half,
  type T8Role,
} from '../lib/t8-receipt.js';

const RUN = 'run / with spaces ..';
const AT = '2026-10-24T12:00:00.000Z';
const STATUS_PHASE = 'berth-status-peer';

function pair(): [T8Half, T8Half] {
  const make = (role: T8Role): T8Half => {
    const other = role === 'holder' ? 'contender' : 'holder';
    const did = `did:key:${role}`;
    const peer = `did:key:${other}`;
    const session = sessionFor(RUN, role);
    const resource = `lane:t8-receipt-${RUN}`;
    const cmd = (phase: string, verb: string, exitCode: number): T8Command => ({
      phase,
      argv: ['python3', '/repo/genesis/agentic/bin/berth', verb, resource, '--session', session],
      exitCode,
      reasonLine: '',
      startedAt: AT,
      finishedAt: AT,
    });
    const commands =
      role === 'holder'
        ? [cmd('holder-claim', 'claim', 0)]
        : [
            {
              ...cmd('contender-claim', 'claim', 3),
              reasonLine: `${resource}: REFUSED — held by did:key:holder/${sessionFor(RUN, 'holder')} since ${AT}`,
            },
            cmd('contender-post-release-claim', 'claim', 0),
          ];
    commands.push(cmd('release', 'release', 0));
    commands.push({
      ...cmd(STATUS_PHASE, 'status', 0),
      argv: ['python3', '/repo/genesis/agentic/bin/berth', 'status'],
      result: `${peer}\n      moored ${sessionFor(RUN, other)} since ${AT}\n      ${resource}  held by ${peer}/${sessionFor(RUN, other)} since ${AT}`,
    });
    return {
      role,
      runId: RUN,
      gitCommit: 'a'.repeat(40),
      device: { did, nodeId: `${role}-node` },
      peerDevice: peer,
      session,
      resource,
      startedAt: AT,
      finishedAt: AT,
      env: {
        handle: 'matthew',
        relay: 'https://relay.alpha.elohim.host',
        transport: 'iroh QUIC',
        encryptionEvidence:
          'eprfs-peer iroh endpoint contract; relay URL declared, no packet capture',
        coordinationChannels: ['peer link'],
      },
      declared: { checks: [T8_CHECK] },
      checks: [{ checkId: T8_CHECK, outcome: 'passed', summary: 'fixture' }],
      commands,
      folds: ['release-crossed', 'peer-status-visible'].map(phase => ({
        phase,
        observedAt: AT,
        own: { seq: 8, root: 'own-root', ackVector: { [peer]: 5 } },
        peer: {
          via: 'peer link',
          device: peer,
          nodeId: `${other}-node`,
          seq: 6,
          root: 'peer-root',
          ackVector: { [did]: 8 },
          foldedAt: Date.parse(AT),
          eventsFolded: 4,
        },
      })),
      release: { seq: 7, root: 'release-root', eventCid: 'release-cid' },
    };
  };
  return [make('holder'), make('contender')];
}

void describe('T8 receipt joiner', () => {
  void it('joins two roles with reciprocal device and node identities, independent commits and proven releases', () => {
    const [holder, contender] = pair();
    contender.peerDevice = holder.device.nodeId;
    contender.gitCommit = 'b'.repeat(40);
    const receipt = joinHalves(holder, contender);
    assert.equal(receipt.checks[0].outcome, 'passed');
    assert.equal(receipt.halves.contender.commands[0].exitCode, 3);
  });

  const invalid: [string, (a: T8Half, b: T8Half) => void][] = [
    [
      'holder claim failed',
      a => {
        a.commands[0].exitCode = 3;
      },
    ],
    [
      'contender claim succeeded',
      (_a, b) => {
        b.commands[0].exitCode = 0;
      },
    ],
    [
      'claim refused for silence',
      (_a, b) => {
        b.commands[0].reasonLine = 'unconfirmed: never seen';
      },
    ],
    [
      'wrong holder session',
      (_a, b) => {
        b.commands[0].reasonLine = b.commands[0].reasonLine.replace(
          '-holder since',
          '-stranger since'
        );
      },
    ],
    [
      'post-release claim failed',
      (_a, b) => {
        b.commands[1].exitCode = 3;
      },
    ],
    [
      'missing post-release claim',
      (_a, b) => {
        b.commands.splice(1, 1);
      },
    ],
    [
      'wrong resource',
      a => {
        a.commands[0].argv[3] = 'dev-integration';
      },
    ],
    [
      'offline override',
      a => {
        a.commands[0].argv.push('--offline-ok');
      },
    ],
    [
      'different runs',
      (_a, b) => {
        b.runId = 'another run';
      },
    ],
    [
      'same device',
      (a, b) => {
        b.device.did = a.device.did;
      },
    ],
    [
      'same node',
      (a, b) => {
        b.device.nodeId = a.device.nodeId;
      },
    ],
    [
      'wrong peer selector',
      a => {
        a.peerDevice = 'did:key:stranger';
      },
    ],
    [
      'no holder fold',
      a => {
        a.folds = [];
      },
    ],
    [
      'no contender fold',
      (_a, b) => {
        b.folds = [];
      },
    ],
    [
      'empty root',
      a => {
        a.folds.forEach(f => {
          f.peer.root = '';
        });
      },
    ],
    [
      'wrong folded device',
      a => {
        a.folds.forEach(f => {
          f.peer.device = 'did:key:stranger';
        });
      },
    ],
    [
      'missing acknowledgement',
      a => {
        a.folds.forEach(f => {
          f.peer.ackVector = {};
        });
      },
    ],
    [
      'old release acknowledgement',
      a => {
        a.release!.seq = 9;
      },
    ],
    [
      'no release head',
      a => {
        delete a.release;
      },
    ],
    [
      'missing berth status',
      a => {
        a.commands = a.commands.filter(c => c.phase !== STATUS_PHASE);
      },
    ],
    [
      'status without peer mooring',
      a => {
        const c = a.commands.find(c => c.phase === STATUS_PHASE)!;
        c.result = String(c.result).replace('moored ', 'absent ');
      },
    ],
    [
      'status without peer lease',
      (_a, b) => {
        const c = b.commands.find(c => c.phase === STATUS_PHASE)!;
        c.result = String(c.result).replace('held by ', 'released by ');
      },
    ],
    [
      'status without a corresponding fold',
      a => {
        a.folds = a.folds.filter(f => f.phase !== 'peer-status-visible');
      },
    ],
    [
      'different declared relay',
      a => {
        a.env.relay = 'https://another-relay.invalid';
      },
    ],
    [
      'missing QUIC transport',
      a => {
        a.env.transport = '';
      },
    ],
    [
      'missing encryption evidence basis',
      a => {
        a.env.encryptionEvidence = '';
      },
    ],
    [
      'missing coordination channel list',
      a => {
        a.env.coordinationChannels = [];
      },
    ],
    [
      'coordination through an additional channel',
      a => {
        a.env.coordinationChannels.push('operator');
      },
    ],
    [
      'one folded head received outside the peer link',
      a => {
        a.folds[0].peer.via = 'git';
      },
    ],
    [
      'failed half',
      a => {
        a.checks[0].outcome = 'failed';
      },
    ],
    [
      'unfinished half',
      a => {
        delete a.finishedAt;
      },
    ],
    [
      'duplicate command',
      a => {
        a.commands.push(a.commands[0]);
      },
    ],
  ];
  for (const [name, mutate] of invalid) {
    void it(`refuses ${name}`, () => {
      const halves = pair();
      mutate(...halves);
      assert.throws(() => joinHalves(...halves));
    });
  }

  void it('encodes arbitrary run names without path traversal or session collisions', () => {
    assert.ok(!runDirectory(RUN).includes('/'));
    assert.equal(runDirectory('..'), '%2e%2e');
    assert.notEqual(runDirectory('a/b'), runDirectory('a%2Fb'));
    assert.notEqual(sessionFor('a/b', 'holder'), sessionFor('a_b', 'holder'));
  });

  void it('CLI refuses a missing half, then writes receipt.json and prints PASS only with both halves', () => {
    const directory = mkdtempSync(join(tmpdir(), 't8-join-test-'));
    try {
      const path = join(directory, 'reports/recovery/devices-share-one-berth', runDirectory(RUN));
      mkdirSync(path, { recursive: true });
      const [holder, contender] = pair();
      writeFileSync(join(path, 'holder.json'), JSON.stringify(holder));
      const script = join(dirname(fileURLToPath(import.meta.url)), '../t8-receipt.ts');
      // This is a CLI, not another node:test worker. Inheriting NODE_TEST_CONTEXT makes
      // console output use the test runner's private IPC protocol instead of stdout.
      const childEnv = { ...process.env };
      delete childEnv.NODE_TEST_CONTEXT;
      const run = () => {
        const stdoutPath = join(directory, 'stdout');
        const stderrPath = join(directory, 'stderr');
        const stdout = openSync(stdoutPath, 'w');
        const stderr = openSync(stderrPath, 'w');
        try {
          // Files also work in sandboxes that forbid child-process capture pipes.
          const result = spawnSync(
            process.execPath,
            [...process.execArgv.filter(arg => !arg.startsWith('--test')), script, RUN],
            {
              cwd: directory,
              env: childEnv,
              stdio: ['ignore', stdout, stderr],
            }
          );
          assert.ifError(result.error);
          return {
            status: result.status,
            stdout: readFileSync(stdoutPath, 'utf8'),
            stderr: readFileSync(stderrPath, 'utf8'),
          };
        } finally {
          closeSync(stdout);
          closeSync(stderr);
        }
      };
      const missing = run();
      assert.equal(missing.status, 1, missing.stderr);
      assert.doesNotMatch(missing.stdout, /PASS/);
      writeFileSync(join(path, 'contender.json'), JSON.stringify(contender));
      const complete = run();
      assert.equal(complete.status, 0, complete.stderr);
      assert.match(complete.stdout, /^PASS /);
      const receipt = JSON.parse(readFileSync(join(path, 'receipt.json'), 'utf8')) as {
        runId: string;
      };
      assert.equal(receipt.runId, RUN);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
});
