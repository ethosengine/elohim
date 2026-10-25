/**
 * T8 drives ONLY this workspace's existing device and daemon. No SSH, git transfer, daemon
 * launch, enrollment, key read, private fixture or remote command. Coordination is berth's
 * ordinary steward channel. Receipts are write-only here, including on failure.
 */
import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { isAbsolute, join, resolve } from 'node:path';

import {
  runDirectory,
  sessionFor,
  T8_CHECK,
  type T8Half,
  type T8Role,
  type T8Command,
} from '../../scripts/lib/t8-receipt.js';

import { resolvedEpr } from './epr-binary.js';

const ROOT = resolve('../..');
const RELAY = 'https://relay.alpha.elohim.host';
export const T8_TAG = '@concern:devices-share-one-berth and @requires:shem';

export function timeoutSeconds(): number {
  const seconds = Number(process.env.A2O_T8_TIMEOUT_S ?? '600');
  assert.ok(Number.isFinite(seconds) && seconds > 0, 'A2O_T8_TIMEOUT_S must be positive seconds');
  return seconds;
}

interface PeerStatus {
  handle: string;
  device: string;
  nodeId: string | null;
  ownSeq: number | null;
  devices: {
    device: string;
    nodeId: string;
    seq: number | null;
    root: string | null;
    ageS: number | null;
  }[];
}
interface HeadTable {
  own: { device: string; seq: number; root: string; folded: Record<string, number> } | null;
  devices: Record<
    string,
    {
      seq: number;
      root: string;
      folded: Record<string, number>;
      folded_at: number;
      events_folded: number;
    }
  >;
  own_log: { seq: number; root: string; events: number; last: string | null }[];
  store: { log: string } | null;
}
interface LeaseView {
  standing: string;
  device: string;
  serving: boolean | null;
  others: Record<string, { seq: number | null; moorings: Partial<Record<string, number>> }>;
  [resource: string]: unknown;
}
interface Verdict {
  verdict: string;
  device?: string;
  session?: string;
  since?: number;
}
interface FlowRow {
  cid: string;
  record: { kind: string; classifiedAs?: string[] };
}

export class WorkspaceHalf {
  readonly receipt: T8Half;
  readonly timeoutMs = timeoutSeconds() * 1000;
  private readonly file: string;
  private readonly ttl = String(Math.ceil(timeoutSeconds() * 4 + 120));
  private readonly headsPath: string;
  private readonly epr: string;
  private readonly freshnessS: number;
  private peerDid = '';
  private peerNode = '';
  private released = false;
  private attemptedClaim = false;
  private moored = false;
  private refusalMoored = false;
  private statusMoored = false;

  constructor(role: T8Role) {
    const runId = process.env.A2O_T8_RUN_ID;
    const peerDevice = process.env.A2O_T8_PEER_DEVICE;
    assert.ok(runId, 'set A2O_T8_RUN_ID to the same fresh run id on both workspaces');
    assert.ok(peerDevice, 'set A2O_T8_PEER_DEVICE to the other device did:key or node id');
    assert.ok(resolvedEpr, 'epr is required');
    this.epr = resolvedEpr;
    const xdg = process.env.XDG_CONFIG_HOME;
    this.headsPath = join(
      xdg && isAbsolute(xdg) ? xdg : join(homedir(), '.config'),
      'epr/peer/heads.json'
    );
    const policy = JSON.parse(
      readFileSync(join(ROOT, 'genesis/agentic/pool-policy.json'), 'utf8')
    ) as { berth: { channel: { freshness_s: number } } };
    this.freshnessS = policy.berth.channel.freshness_s;
    const directory = join(
      ROOT,
      'genesis/a2o/reports/recovery/devices-share-one-berth',
      runDirectory(runId)
    );
    this.file = join(directory, `${role}.json`);
    assert.ok(!existsSync(this.file), 'this half already has a receipt; use a fresh run id');
    mkdirSync(directory, { recursive: true });
    this.receipt = {
      role,
      runId,
      peerDevice,
      gitCommit: '',
      device: { did: '', nodeId: '' },
      session: sessionFor(runId, role),
      resource: `lane:t8-receipt-${runId}`,
      startedAt: new Date().toISOString(),
      env: {
        handle: 'matthew',
        relay: RELAY,
        transport: 'iroh QUIC',
        encryptionEvidence:
          'eprfs-peer iroh endpoint contract; relay URL declared, no packet capture',
        coordinationChannels: ['peer link'],
      },
      declared: { checks: [T8_CHECK] },
      checks: [],
      commands: [],
      folds: [],
    };
    this.save();
  }

  private save(): void {
    writeFileSync(this.file, `${JSON.stringify(this.receipt, null, 2)}\n`, { mode: 0o600 });
  }

  private async command(
    phase: string,
    argv: string[],
    extraEnv: Record<string, string> = {}
  ): Promise<{ entry: T8Command; stdout: string }> {
    const startedAt = new Date().toISOString();
    const output = await new Promise<{ code: number | null; stdout: string; stderr: string }>(
      done => {
        execFile(
          argv[0],
          argv.slice(1),
          {
            cwd: ROOT,
            env: { ...process.env, EPR_BIN: this.epr, CLAUDE_PROJECT_DIR: ROOT, ...extraEnv },
            timeout: 30_000,
            maxBuffer: 8 * 1024 * 1024,
            encoding: 'utf8',
          },
          (error, stdout, stderr) => {
            let code: number | null = 0;
            if (error) code = typeof error.code === 'number' ? error.code : null;
            done({ code, stdout, stderr: stderr.length > 0 ? stderr : (error?.message ?? '') });
          }
        );
      }
    );
    const lines = `${output.stderr}\n${output.stdout}`
      .split('\n')
      .map(s => s.trim())
      .filter(Boolean);
    const entry: T8Command = {
      phase,
      argv,
      exitCode: output.code,
      startedAt,
      finishedAt: new Date().toISOString(),
      reasonLine: lines.find(s => s.startsWith(`${this.receipt.resource}:`)) ?? lines[0] ?? '',
    };
    this.receipt.commands.push(entry);
    this.save();
    return { entry, stdout: output.stdout };
  }

  private async peer<T>(verb: string): Promise<T> {
    const { entry, stdout } = await this.command(`peer-${verb}`, [
      this.epr,
      'peer',
      verb,
      '--handle',
      'matthew',
      '--root',
      ROOT,
      '--json',
    ]);
    assert.equal(entry.exitCode, 0, entry.reasonLine);
    const result = JSON.parse(stdout) as T;
    entry.result = result;
    this.save();
    return result;
  }

  private async berth(
    phase: string,
    args: string[],
    session = this.receipt.session
  ): Promise<T8Command> {
    // On an envelope-capable device a session acts only for itself: this process declares the
    // session it moors and owns that declaration, so native resolution answers with it.
    const { entry } = await this.command(
      phase,
      ['python3', join(ROOT, 'genesis/agentic/bin/berth'), ...args, '--session', session],
      { ELOHIM_SESSION_ID: session, ELOHIM_SESSION_OWNER_PID: String(process.pid) }
    );
    return entry;
  }

  private async waitFor(what: string, check: () => Promise<boolean>): Promise<void> {
    const deadline = Date.now() + this.timeoutMs;
    do {
      if (await check()) return;
      await new Promise(done => setTimeout(done, 1000));
    } while (Date.now() < deadline);
    assert.fail(`timed out after ${this.timeoutMs / 1000}s: ${what}`);
  }

  private heads(): HeadTable {
    return JSON.parse(readFileSync(this.headsPath, 'utf8')) as HeadTable;
  }

  private snapshot(phase: string): boolean {
    const table = this.heads();
    const peer = table.devices[this.peerDid];
    if (
      table.own?.device !== this.receipt.device.did ||
      !peer ||
      Date.now() - peer.folded_at > this.freshnessS * 1000
    )
      return false;
    this.receipt.folds.push({
      phase,
      observedAt: new Date().toISOString(),
      own: { seq: table.own.seq, root: table.own.root, ackVector: table.own.folded },
      peer: {
        via: 'peer link',
        device: this.peerDid,
        nodeId: this.peerNode,
        seq: peer.seq,
        root: peer.root,
        foldedAt: peer.folded_at,
        eventsFolded: peer.events_folded,
        ackVector: peer.folded,
      },
    });
    this.save();
    return true;
  }

  async prepare(): Promise<void> {
    const commit = await this.command('checkout', ['git', 'rev-parse', 'HEAD']);
    assert.equal(commit.entry.exitCode, 0, commit.entry.reasonLine);
    this.receipt.gitCommit = commit.stdout.trim();
    await this.waitFor('the other admitted device has a fresh folded head', async () => {
      const status = await this.peer<PeerStatus>('status');
      assert.equal(status.handle, 'matthew');
      assert.ok(status.nodeId && status.device);
      this.receipt.device = { did: status.device, nodeId: status.nodeId };
      const peer = status.devices.find(d => [d.device, d.nodeId].includes(this.receipt.peerDevice));
      assert.ok(peer, `${this.receipt.peerDevice} is not admitted by matthew's roster here`);
      assert.notEqual(peer.device, status.device);
      this.peerDid = peer.device;
      this.peerNode = peer.nodeId;
      return (
        peer.seq !== null &&
        peer.root !== null &&
        peer.ageS !== null &&
        peer.ageS <= this.freshnessS &&
        this.snapshot('ready')
      );
    });
    const leases = await this.peer<LeaseView>('leases');
    assert.equal(leases.standing, 'device');
    assert.equal(leases.device, this.receipt.device.did);
    assert.ok(
      leases.serving,
      'start this device’s epr peer serve with EPR_PEER_RELAY_URL=https://relay.alpha.elohim.host first'
    );
    assert.ok(
      leases.others[this.peerDid],
      'peer must also be admitted in the daemon channel store'
    );
    this.save();
  }

  private async moor(phase: string, session = this.receipt.session): Promise<void> {
    const result = await this.berth(
      phase,
      ['moor', '--pid', String(process.pid), '--ttl', this.ttl, '--task', this.receipt.resource],
      session
    );
    assert.equal(result.exitCode, 0, result.reasonLine);
  }

  private async claim(phase: string, expected: number): Promise<T8Command> {
    this.attemptedClaim = true;
    const result = await this.berth(phase, ['claim', this.receipt.resource, '--ttl', this.ttl]);
    assert.equal(result.exitCode, expected, result.reasonLine);
    // Exit 0 is berth's proof that every admitted device acknowledged the claim's head.
    return result;
  }

  private async observePeerStatus(session: string): Promise<void> {
    await this.waitFor('peer mooring and held lease in this workspace’s berth status', async () => {
      const view = await this.peer<LeaseView>('leases');
      const lease = view[this.receipt.resource] as Verdict | undefined;
      if (
        lease?.verdict !== 'held_by' ||
        lease.device !== this.peerDid ||
        lease.session !== session ||
        view.others[this.peerDid]?.moorings[session] === undefined
      )
        return false;
      const { entry, stdout } = await this.command('berth-status-peer', [
        'python3',
        join(ROOT, 'genesis/agentic/bin/berth'),
        'status',
      ]);
      assert.equal(entry.exitCode, 0, entry.reasonLine);
      entry.result = stdout;
      assert.ok(stdout.includes(this.peerDid), 'status must name the peer device');
      assert.ok(stdout.includes(`moored ${session} since `), 'status must show its mooring');
      assert.ok(
        stdout.includes(`${this.receipt.resource}  held by ${this.peerDid}/${session} since `),
        'status must show its held lease'
      );
      return this.snapshot('peer-status-visible');
    });
  }

  async exchange(): Promise<void> {
    const holder = sessionFor(this.receipt.runId, 'holder');
    const contender = sessionFor(this.receipt.runId, 'contender');
    const refusal = `${contender}-refused`;
    if (this.receipt.role === 'holder') {
      await this.moor('moor');
      this.moored = true;
      await this.claim('holder-claim', 0);
      try {
        await this.waitFor('contender refusal mooring to cross', async () => {
          const view = await this.peer<LeaseView>('leases');
          return (
            view.others[this.peerDid]?.moorings[refusal] !== undefined &&
            this.snapshot('refusal-crossed')
          );
        });
      } finally {
        // Even a hold timeout releases this lane and checks that the release crossed.
        await this.releaseAndConfirm();
      }
      await this.observePeerStatus(contender);
      await this.moor('status-moor', `${holder}-status-read`);
      this.statusMoored = true;
      // Keep the acknowledgement moored until the contender has seen it and released.
      await this.waitFor('contender release after its status was read', async () => {
        const view = await this.peer<LeaseView>('leases');
        return view[this.receipt.resource] === undefined && this.snapshot('contender-released');
      });
      return;
    } else {
      await this.waitFor('holder lease and mooring to fold', async () => {
        const view = await this.peer<LeaseView>('leases');
        const v = view[this.receipt.resource] as Verdict | undefined;
        return !!(
          v?.verdict === 'held_by' &&
          v.device === this.peerDid &&
          v.session === holder &&
          view.others[this.peerDid]?.moorings[holder] !== undefined &&
          this.snapshot('holder-visible')
        );
      });
      await this.observePeerStatus(holder);
      await this.moor('moor');
      this.moored = true;
      const refused = await this.claim('contender-claim', 3);
      assert.ok(
        refused.reasonLine.startsWith(
          `${this.receipt.resource}: REFUSED — held by ${this.peerDid}/${holder} since `
        ),
        refused.reasonLine
      );
      // The second mooring is published ONLY after exit 3, so A cannot release before B's claim.
      await this.moor('refusal-moor', refusal);
      this.refusalMoored = true;
      await this.waitFor('holder release to fold', async () => {
        const view = await this.peer<LeaseView>('leases');
        return view[this.receipt.resource] === undefined && this.snapshot('holder-released');
      });
      await this.claim('contender-post-release-claim', 0);
      await this.waitFor('holder has read the contender lease in berth status', async () => {
        const view = await this.peer<LeaseView>('leases');
        return view.others[this.peerDid]?.moorings[`${holder}-status-read`] !== undefined;
      });
    }
    await this.releaseAndConfirm();
  }

  private flowEvents(table: HeadTable): FlowRow[] {
    assert.ok(table.store, 'daemon must name its channel store');
    return readFileSync(table.store.log, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map(line => JSON.parse(line) as FlowRow)
      .filter(row => row.record.kind === 'event');
  }

  private async releaseAndConfirm(): Promise<void> {
    const result = await this.berth('release', ['release', this.receipt.resource]);
    assert.equal(result.exitCode, 0, result.reasonLine);
    assert.ok(result.reasonLine.includes(': released by '), 'must release an actual held lease');
    this.released = true;
    await this.waitFor('peer acknowledgement of the head carrying this release', async () => {
      await this.peer<PeerStatus>('status');
      const table = this.heads();
      const events = this.flowEvents(table);
      const releaseIndex = events.findIndex(row => {
        const tags = row.record.classifiedAs ?? [];
        return (
          tags.includes('reason:release') &&
          tags.includes(`env:session=${this.receipt.session}`) &&
          tags.includes(`env:resource=${this.receipt.resource}`) &&
          !tags.some(t => t.startsWith('via:'))
        );
      });
      assert.ok(releaseIndex >= 0, 'release did not enter the local channel store');
      // Same position + CID rule as the native lease view. A heartbeat after release is NOT proof.
      const carried = table.own_log.find(
        h => h.events > releaseIndex && events[h.events - 1]?.cid === h.last
      );
      const acked = table.devices[this.peerDid]?.folded[this.receipt.device.did] ?? -1;
      if (!carried || acked < carried.seq) return false;
      this.receipt.release = {
        seq: carried.seq,
        root: carried.root,
        eventCid: events[releaseIndex].cid,
      };
      return this.snapshot('release-crossed');
    });
  }

  async finish(passed: boolean, summary: string): Promise<void> {
    let completed = passed;
    let outcomeSummary = summary;
    try {
      if (this.attemptedClaim && !this.released) {
        const cleanup = await this.berth('cleanup-release', ['release', this.receipt.resource]);
        assert.equal(cleanup.exitCode, 0, cleanup.reasonLine);
      }
      if (this.refusalMoored) {
        const cleanup = await this.berth(
          'cleanup-refusal',
          ['unmoor'],
          `${this.receipt.session}-refused`
        );
        assert.equal(cleanup.exitCode, 0, cleanup.reasonLine);
      }
      if (this.statusMoored) {
        const cleanup = await this.berth(
          'cleanup-status',
          ['unmoor'],
          `${this.receipt.session}-status-read`
        );
        assert.equal(cleanup.exitCode, 0, cleanup.reasonLine);
      }
      if (this.moored) {
        const cleanup = await this.berth('cleanup-moor', ['unmoor']);
        assert.equal(cleanup.exitCode, 0, cleanup.reasonLine);
      }
    } catch (error) {
      completed = false;
      outcomeSummary += `; cleanup: ${String(error)}`;
      throw error;
    } finally {
      this.receipt.finishedAt = new Date().toISOString();
      this.receipt.checks = [
        { checkId: T8_CHECK, outcome: completed ? 'passed' : 'failed', summary: outcomeSummary },
      ];
      this.save();
    }
  }
}
