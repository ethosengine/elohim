/** Evidence from two independently run workspace halves, never a coordination channel. */
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

export type T8Role = 'holder' | 'contender';
export const T8_CHECK = 'devices-share-one-berth';

export function runDirectory(runId: string): string {
  assert.ok(runId.length > 0, 'A2O_T8_RUN_ID must be nonempty');
  // Any run id is allowed; encode separators and dot-only path segments instead of traversing.
  return encodeURIComponent(runId).replace(/[.!'()*]/g, c => `%${c.codePointAt(0)!.toString(16)}`);
}

export function sessionFor(runId: string, role: T8Role): string {
  return `t8-${createHash('sha256').update(runId).digest('hex').slice(0, 24)}-${role}`;
}

export interface T8Command {
  phase: string;
  argv: string[];
  exitCode: number | null;
  reasonLine: string;
  startedAt: string;
  finishedAt: string;
  result?: unknown;
}

export interface T8Head {
  seq: number;
  root: string;
  ackVector: Record<string, number>;
}

export interface T8Fold {
  phase: string;
  observedAt: string;
  own: T8Head;
  peer: T8Head & {
    via: string;
    device: string;
    nodeId: string;
    foldedAt: number;
    eventsFolded: number;
  };
}

export interface T8Half {
  role: T8Role;
  runId: string;
  gitCommit: string;
  device: { did: string; nodeId: string };
  peerDevice: string;
  session: string;
  resource: string;
  startedAt: string;
  finishedAt?: string;
  env: {
    handle: 'matthew';
    relay: string;
    transport: string;
    encryptionEvidence: string;
    coordinationChannels: string[];
  };
  declared: { checks: string[] };
  checks: { checkId: string; outcome: 'passed' | 'failed'; summary: string }[];
  commands: T8Command[];
  folds: T8Fold[];
  release?: { seq: number; root: string; eventCid: string };
}

function command(half: T8Half, phase: string, verb: string, exitCode: number): T8Command {
  const matches = half.commands.filter(c => c.phase === phase);
  assert.equal(matches.length, 1, `${half.role}: exactly one ${phase}`);
  const c = matches[0];
  assert.equal(c.exitCode, exitCode, `${phase}: exit ${exitCode} required`);
  const verbIndex = c.argv.indexOf(verb);
  assert.ok(verbIndex >= 0 && c.argv[verbIndex + 1] === half.resource, `${phase}: lane argv`);
  assert.equal(c.argv[c.argv.indexOf('--session') + 1], half.session, `${phase}: session argv`);
  assert.ok(
    !c.argv.includes('--offline-ok') && !c.argv.includes('--force'),
    `${phase}: no override`
  );
  assert.ok(Number.isFinite(Date.parse(c.startedAt)) && Number.isFinite(Date.parse(c.finishedAt)));
  return c;
}

function validateHalf(half: T8Half, peer: T8Half): void {
  assert.equal(half.env.handle, 'matthew');
  assert.equal(half.env.relay, 'https://relay.alpha.elohim.host');
  assert.equal(half.env.transport, 'iroh QUIC');
  assert.ok(half.env.encryptionEvidence, 'relay encryption evidence must name its basis');
  assert.deepEqual(half.env.coordinationChannels, ['peer link']);
  assert.ok(half.folds.length > 0 && half.folds.every(f => f.peer.via === 'peer link'));
  assert.match(half.device.did, /^did:key:.+/);
  assert.ok(half.device.nodeId && peer.device.nodeId);
  assert.ok(
    [peer.device.did, peer.device.nodeId].includes(half.peerDevice),
    'peer must name the other half'
  );
  assert.equal(half.resource, `lane:t8-receipt-${half.runId}`);
  assert.equal(half.session, sessionFor(half.runId, half.role));
  assert.match(half.gitCommit, /^[a-f0-9]{40,64}$/);
  assert.ok(half.finishedAt, `${half.role}: unfinished half`);
  assert.ok(half.checks.some(c => c.checkId === T8_CHECK && c.outcome === 'passed'));
  assert.ok(!half.checks.some(c => c.outcome === 'failed'));
  const folded = half.folds.filter(
    f => f.peer.device === peer.device.did && f.peer.nodeId === peer.device.nodeId
  );
  assert.ok(
    folded.some(f => f.phase === 'peer-status-visible'),
    'missing status fold'
  );
  assert.ok(
    half.commands.some(
      c =>
        c.phase === 'berth-status-peer' &&
        c.exitCode === 0 &&
        c.argv.at(-1) === 'status' &&
        typeof c.result === 'string' &&
        c.result.includes(peer.device.did) &&
        c.result.includes(`moored ${peer.session} since `) &&
        c.result.includes(`${half.resource}  held by ${peer.device.did}/${peer.session} since `)
    ),
    `${half.role}: missing local berth status listing the peer mooring and lease`
  );
  assert.ok(
    folded.some(
      f =>
        f.peer.seq > 0 &&
        f.peer.root &&
        f.peer.eventsFolded > 0 &&
        f.peer.foldedAt > 0 &&
        f.own.ackVector[peer.device.did] > 0 &&
        f.peer.ackVector[half.device.did] > 0
    ),
    `${half.role}: missing the other device's folded head and acknowledgements`
  );
  command(half, 'release', 'release', 0);
  assert.ok(
    half.release && half.release.seq > 0 && half.release.root && half.release.eventCid,
    `${half.role}: missing release head`
  );
  assert.ok(
    folded.some(
      f => f.phase === 'release-crossed' && f.peer.ackVector[half.device.did] >= half.release!.seq
    ),
    `${half.role}: release not acknowledged`
  );
}

export function joinHalves(holder: T8Half, contender: T8Half) {
  assert.equal(holder.role, 'holder');
  assert.equal(contender.role, 'contender');
  assert.equal(holder.runId, contender.runId, 'run ids differ');
  assert.notEqual(holder.device.did, contender.device.did, 'two distinct devices required');
  assert.notEqual(
    holder.device.nodeId,
    contender.device.nodeId,
    'two distinct peer endpoints required'
  );
  validateHalf(holder, contender);
  validateHalf(contender, holder);
  command(holder, 'holder-claim', 'claim', 0);
  const refused = command(contender, 'contender-claim', 'claim', 3);
  const prefix = `${holder.resource}: REFUSED — held by ${holder.device.did}/${holder.session} since `;
  assert.ok(
    refused.reasonLine.startsWith(prefix) &&
      Number.isFinite(Date.parse(refused.reasonLine.slice(prefix.length))),
    'contender refusal must name the holder device/session and time'
  );
  command(contender, 'contender-post-release-claim', 'claim', 0);
  return {
    runId: holder.runId,
    gitCommit: { holder: holder.gitCommit, contender: contender.gitCommit },
    env: { handle: 'matthew', devices: [holder.device, contender.device] },
    declared: { checks: [T8_CHECK] },
    checks: [
      {
        checkId: T8_CHECK,
        outcome: 'passed',
        summary:
          'Held claim refused across two devices; release crossed and contender claimed effectively.',
      },
    ],
    joinedAt: new Date().toISOString(),
    halves: { holder, contender },
  };
}
