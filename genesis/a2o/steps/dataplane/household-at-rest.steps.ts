/**
 * Household at rest — the runaway guard.
 *
 * With nobody authoring, reading or syncing, a household's nodes should be
 * nearly silent. These steps take two snapshots a quiet window apart and read
 * what each storage peer asked of its conductor in between, and what the
 * conductors burned doing it. Every self-generated loop — a sweep that re-asks
 * what it already knows, a retry that never backs off, a poll where an event
 * would do — shows up here, on the household mesh, before the fleet sees it.
 *
 * Origin (2026-09-19): all seven alpha conductors pinned at their CPU limit
 * while serving nobody; a reconnect loop that failed every 5 s for 14 hours; a
 * sweep re-probing the same dead candidates forever. None of it was visible
 * until a person read the logs.
 */
import assert from 'node:assert/strict';
import { execSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';

import { Given, When, Then } from '@cucumber/cucumber';

import { getRaw } from '../../src/framework/dataplane/surfaces.js';
import { E2EWorld } from '../../src/framework/world.js';

interface PeerReading {
  admitted: number;
  shed: number;
  /** Dispatched conductor calls keyed `zome::fn (class)` — who is asking, not just how much. */
  callers: Map<string, number>;
}

interface RestReading {
  at: number;
  peers: Map<string, PeerReading>;
  conductorCpuSeconds: number;
}

interface RestState {
  peers: Map<string, string>;
  before?: RestReading;
  after?: RestReading;
}

const states = new WeakMap<E2EWorld, RestState>();

function state(world: E2EWorld): RestState {
  let s = states.get(world);
  if (!s) {
    s = { peers: new Map() };
    states.set(world, s);
  }
  return s;
}

/** Sum every sample of one Prometheus series family in a text exposition. */
function sumSeries(exposition: string, family: string): number {
  let total = 0;
  for (const line of exposition.split('\n')) {
    if (!line.startsWith(family)) continue;
    const next = line.charAt(family.length);
    if (next !== '{' && next !== ' ') continue;
    const value = Number(line.slice(line.lastIndexOf(' ') + 1));
    if (Number.isFinite(value)) total += value;
  }
  return total;
}

/** Every sample of `elohim_conductor_calls_total`, keyed by its zome, function and class labels. */
function callersOf(exposition: string): Map<string, number> {
  const callers = new Map<string, number>();
  for (const line of exposition.split('\n')) {
    if (!line.startsWith('elohim_conductor_calls_total{')) continue;
    const label = (name: string): string => new RegExp(`${name}="([^"]*)"`).exec(line)?.[1] ?? '?';
    const value = Number(line.slice(line.lastIndexOf(' ') + 1));
    if (Number.isFinite(value))
      callers.set(`${label('zome')}::${label('fn')} (${label('class')})`, value);
  }
  return callers;
}

/** The busiest callers between two readings of one peer, as `name ×count`, busiest first. */
function topCallers(before: PeerReading | undefined, after: PeerReading, limit = 5): string {
  const deltas = [...after.callers]
    .map(([name, value]) => [name, value - (before?.callers.get(name) ?? 0)] as const)
    .filter(([, delta]) => delta > 0)
    .sort((a, b) => b[1] - a[1])
    .slice(0, limit);
  return deltas.length > 0
    ? deltas.map(([name, delta]) => `${name} ×${delta}`).join(', ')
    : 'no per-function series on this peer (binary predates elohim_conductor_calls_total)';
}

async function readPeer(url: string): Promise<PeerReading> {
  // The framework's bounded GET, not global fetch: the harness owns HTTP.
  const { status, text } = await getRaw(`${url}/metrics`, { timeoutMs: 10_000 });
  assert.equal(status, 200, `${url}/metrics answered ${status}`);
  return {
    // Every conductor call passes the admission gate; a released permit is a call that was made.
    admitted: sumSeries(text, 'elohim_conductor_admission_hold_ms_count'),
    shed: sumSeries(text, 'elohim_conductor_admission_shed_total'),
    callers: callersOf(text),
  };
}

/** Total user+system CPU seconds of every running holochain conductor on this host. */
function conductorCpuSeconds(): number {
  const ticksPerSecond = Number(execSync('/usr/bin/getconf CLK_TCK').toString().trim()) || 100;
  let ticks = 0;
  for (const entry of readdirSync('/proc')) {
    if (!/^\d+$/.test(entry)) continue;
    try {
      const cmdline = readFileSync(`/proc/${entry}/cmdline`, 'utf8').split('\0');
      if (!/(^|\/)holochain$/.test(cmdline[0] ?? '')) continue;
      const stat = readFileSync(`/proc/${entry}/stat`, 'utf8');
      // Fields after the parenthesised command name; utime and stime are the 12th and 13th of those.
      const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ');
      ticks += Number(fields[11]) + Number(fields[12]);
    } catch {
      // The process exited between the listing and the read.
    }
  }
  return ticks / ticksPerSecond;
}

async function takeReading(s: RestState): Promise<RestReading> {
  const peers = new Map<string, PeerReading>();
  for (const [name, url] of s.peers) peers.set(name, await readPeer(url));
  return { at: Date.now(), peers, conductorCpuSeconds: conductorCpuSeconds() };
}

function perMinute(delta: number, before: RestReading, after: RestReading): number {
  const minutes = (after.at - before.at) / 60_000;
  return minutes > 0 ? delta / minutes : Number.POSITIVE_INFINITY;
}

function readings(world: E2EWorld): { before: RestReading; after: RestReading; s: RestState } {
  const s = state(world);
  assert.ok(s.before && s.after, 'the quiet window has not been observed yet');
  return { before: s.before, after: s.after, s };
}

Given("the household's storage peers", function (this: E2EWorld) {
  const declared = process.env.PEER_STORAGE_URLS;
  if (!declared) return 'pending';
  const s = state(this);
  for (const pair of declared.split(',')) {
    const [name, host] = pair.split('=');
    if (!name || !host) continue;
    s.peers.set(name.trim(), host.startsWith('http') ? host.trim() : `http://${host.trim()}`);
  }
  assert.ok(s.peers.size > 0, `PEER_STORAGE_URLS named no peers: ${declared}`);
  return undefined;
});

Given('every storage peer reports its content in sync', async function (this: E2EWorld) {
  const behind: string[] = [];
  for (const [name, url] of state(this).peers) {
    const { status, text } = await getRaw(`${url}/p2p/status`, { timeoutMs: 10_000 });
    assert.equal(status, 200, `${url}/p2p/status answered ${status}`);
    const body = JSON.parse(text) as {
      replication?: { caughtUp?: boolean };
      pull?: { caughtUp?: boolean };
      projectionReconcile?: { caughtUp?: boolean };
    };
    const lagging = (['replication', 'pull', 'projectionReconcile'] as const).filter(
      plane => body[plane]?.caughtUp !== true
    );
    if (lagging.length > 0) behind.push(`${name}: ${lagging.join('+')} not caught up`);
  }
  // A household still catching up is doing real work; measuring it would prove nothing about rest.
  assert.deepEqual(behind, [], `the household is not at rest yet — ${behind.join('; ')}`);
});

When(
  'nobody authors, reads or syncs for {int} seconds',
  { timeout: 1_200_000 },
  async function (this: E2EWorld, seconds: number) {
    const s = state(this);
    s.before = await takeReading(s);
    await new Promise(resolve => setTimeout(resolve, seconds * 1000));
    s.after = await takeReading(s);
  }
);

Then(
  'each storage peer asked its conductor for at most {int} calls a minute',
  function (this: E2EWorld, budget: number) {
    const { before, after } = readings(this);
    const over: string[] = [];
    for (const [name, reading] of after.peers) {
      const rate = perMinute(
        reading.admitted - (before.peers.get(name)?.admitted ?? 0),
        before,
        after
      );
      // A count alone sends a person to the logs; name the callers so the runaway names itself.
      if (rate > budget) {
        over.push(
          `${name}: ${rate.toFixed(1)}/min — ${topCallers(before.peers.get(name), reading)}`
        );
      }
    }
    assert.deepEqual(over, [], `at rest, over the ${budget} calls/min budget — ${over.join(', ')}`);
  }
);

Then('no storage peer was refused a conductor permit', function (this: E2EWorld) {
  const { before, after } = readings(this);
  const refused: string[] = [];
  for (const [name, reading] of after.peers) {
    const delta = reading.shed - (before.peers.get(name)?.shed ?? 0);
    if (delta > 0) refused.push(`${name}: ${delta}`);
  }
  assert.deepEqual(
    refused,
    [],
    `conductor permits refused while nothing was happening — ${refused.join(', ')}`
  );
});

Then(
  "the household's conductors together used at most {int} CPU seconds a minute",
  function (this: E2EWorld, budget: number) {
    const { before, after } = readings(this);
    const rate = perMinute(after.conductorCpuSeconds - before.conductorCpuSeconds, before, after);
    assert.ok(
      rate <= budget,
      `at rest the conductors burned ${rate.toFixed(1)} CPU s/min (budget ${budget}); 60 is one core pinned`
    );
  }
);
