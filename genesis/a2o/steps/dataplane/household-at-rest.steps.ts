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
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

import { DataTable, Given, When, Then } from '@cucumber/cucumber';

import { getRaw } from '../../src/framework/dataplane/surfaces.js';
import { householdMeshDir } from '../../src/framework/fixtures/household-mesh.js';
import {
  captureHouseholdResources,
  counterRatePerMinute,
  requiredPrometheusSeriesSum,
  resourceWitness,
  type ResourceSnapshot,
  type ResourceWitness,
} from '../../src/framework/fixtures/process-resources.js';
import { E2EWorld } from '../../src/framework/world.js';

interface PeerReading {
  monotonicMs: number;
  admitted: number | null;
  refused: number | null;
  /** Dispatched conductor calls keyed `zome::fn (class)` — who is asking, not just how much. */
  callers: Map<string, number>;
  issues: string[];
}

interface RestReading {
  peers: Map<string, PeerReading>;
  resources: ResourceSnapshot;
}

interface RestState {
  peers: Map<string, string>;
  before?: RestReading;
  after?: RestReading;
  resourceWitness?: ResourceWitness;
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

/** Every sample of `elohim_conductor_calls_total`, keyed by its zome, function and class labels. */
function callersOf(exposition: string): Map<string, number> {
  const callers = new Map<string, number>();
  for (const line of exposition.split('\n')) {
    if (!line.startsWith('elohim_conductor_calls_total{')) continue;
    const label = (name: string): string => new RegExp(`${name}="([^"]*)"`).exec(line)?.[1] ?? '?';
    const value = Number(line.trim().split(/\s+/)[1]);
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
  try {
    // The framework's bounded GET, not global fetch: the harness owns HTTP.
    const { status, text } = await getRaw(`${url}/metrics`, { timeoutMs: 10_000 });
    const monotonicMs = performance.now();
    if (status !== 200) {
      return {
        monotonicMs,
        admitted: null,
        refused: null,
        callers: new Map(),
        issues: [`/metrics HTTP ${status}`],
      };
    }
    const admitted = requiredPrometheusSeriesSum(text, 'elohim_conductor_admission_hold_ms_count');
    const refused = requiredPrometheusSeriesSum(text, 'elohim_conductor_admission_shed_total');
    return {
      monotonicMs,
      admitted: admitted.value,
      refused: refused.value,
      callers: callersOf(text),
      issues: [admitted.issue, refused.issue].filter((issue): issue is string => Boolean(issue)),
    };
  } catch (error) {
    return {
      monotonicMs: performance.now(),
      admitted: null,
      refused: null,
      callers: new Map(),
      issues: [`/metrics read failed: ${String(error)}`],
    };
  }
}

function expectedConductorConfigs(s: RestState): Record<string, string> {
  const root = householdMeshDir();
  return Object.fromEntries(
    [...s.peers.keys()].map(name => [
      name,
      resolve(root, 'conductors', name, 'conductor-config.yaml'),
    ])
  );
}

async function takeReading(s: RestState): Promise<RestReading> {
  const resources = captureHouseholdResources(expectedConductorConfigs(s));
  const peers = new Map<string, PeerReading>();
  for (const [name, url] of s.peers) peers.set(name, await readPeer(url));
  return { peers, resources };
}

function peerEvidence(reading: PeerReading): object {
  return {
    admitted: reading.admitted,
    refused: reading.refused,
    monotonicMs: reading.monotonicMs,
    callers: Object.fromEntries(reading.callers),
    issues: reading.issues,
  };
}

function unavailable(value: number | null | undefined): value is null | undefined {
  return value === null || value === undefined;
}

function storageEvidence(before: RestReading, after: RestReading): object {
  return Object.fromEntries(
    [...after.peers].map(([name, reading]) => {
      const prior = before.peers.get(name);
      const admittedDelta =
        unavailable(prior?.admitted) || unavailable(reading.admitted)
          ? null
          : reading.admitted - prior.admitted;
      const refusedDelta =
        unavailable(prior?.refused) || unavailable(reading.refused)
          ? null
          : reading.refused - prior.refused;
      return [
        name,
        {
          before: prior ? peerEvidence(prior) : null,
          after: peerEvidence(reading),
          deltas: { admitted: admittedDelta, refused: refusedDelta },
          topCallers: topCallers(prior, reading),
        },
      ];
    })
  );
}

function storageViolations(
  before: RestReading,
  after: RestReading,
  callsBudget: number,
  refusedBudget: number
): string[] {
  const violations: string[] = [];
  for (const [name, reading] of after.peers) {
    const prior = before.peers.get(name);
    violations.push(
      ...[...(prior?.issues ?? []), ...reading.issues].map(
        issue => `${name}: metrics apparatus: ${issue}`
      )
    );
    if (unavailable(prior?.admitted) || unavailable(reading.admitted)) {
      violations.push(`${name}: admitted-call counter was not measurable at both endpoints`);
    } else if (reading.admitted < prior.admitted) {
      violations.push(`${name}: admitted-call counter rolled backwards`);
    } else {
      const callsRate = counterRatePerMinute(
        reading.admitted - prior.admitted,
        prior.monotonicMs,
        reading.monotonicMs
      );
      if (callsRate.issue) violations.push(`${name}: metrics apparatus: ${callsRate.issue}`);
      else if (callsRate.value !== null && callsRate.value > callsBudget) {
        violations.push(
          `${name}: ${callsRate.value.toFixed(1)} calls/min > ${callsBudget} — ${topCallers(
            prior,
            reading
          )}`
        );
      }
    }
    if (unavailable(prior?.refused) || unavailable(reading.refused)) {
      violations.push(`${name}: refused-permit counter was not measurable at both endpoints`);
    } else if (reading.refused < prior.refused) {
      violations.push(`${name}: refused-permit counter rolled backwards`);
    } else if (reading.refused - prior.refused > refusedBudget) {
      violations.push(
        `${name}: ${reading.refused - prior.refused} refused conductor permits > ${refusedBudget}`
      );
    }
  }
  return violations;
}

function readings(world: E2EWorld): { before: RestReading; after: RestReading; s: RestState } {
  const s = state(world);
  assert.ok(s.before && s.after, 'the quiet window has not been observed yet');
  return { before: s.before, after: s.after, s };
}

Given("the household's three storage peer/conductor pairs", function (this: E2EWorld) {
  const declared = process.env.PEER_STORAGE_URLS;
  if (!declared) return 'pending';
  const s = state(this);
  for (const pair of declared.split(',')) {
    const [name, host] = pair.split('=');
    if (!name || !host) continue;
    s.peers.set(name.trim(), host.startsWith('http') ? host.trim() : `http://${host.trim()}`);
  }
  const expected = ['james', 'jessica', 'matthew'];
  const observed = [...s.peers.keys()].sort((a, b) => a.localeCompare(b));
  assert.deepEqual(
    observed,
    expected,
    `the selected household must name exactly its three peer/conductor pairs; ` +
      `PEER_STORAGE_URLS named ${observed.join(', ') || 'none'}`
  );
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
  'no person or external client authors, reads or syncs for {int} seconds',
  { timeout: 1_200_000 },
  async function (this: E2EWorld, seconds: number) {
    const s = state(this);
    s.before = await takeReading(s);
    await new Promise(resolve => setTimeout(resolve, seconds * 1000));
    s.after = await takeReading(s);
    s.resourceWitness = resourceWitness(s.before.resources, s.after.resources);
    // Foreign diagnostic evidence belongs in Cucumber's own run output. Attach
    // it before any budget assertion so a red verdict cannot erase the measure.
    this.attach(
      JSON.stringify(
        {
          kind: 'household-idle-resource-observation/v1',
          processes: s.resourceWitness,
          storagePeers: storageEvidence(s.before, s.after),
        },
        null,
        2
      ),
      'application/json'
    );
  }
);

Then(
  'the quiet household stayed within its resource budgets:',
  function (this: E2EWorld, table: DataTable) {
    const { before, after, s } = readings(this);
    const budgets = table.rowsHash();
    const callsBudget = Number(budgets['average conductor calls per peer per minute']);
    const refusedBudget = Number(budgets['refused conductor permits per peer']);
    const cpuBudget = Number(budgets['average household CPU seconds per minute']);
    assert.ok(
      [callsBudget, refusedBudget, cpuBudget].every(Number.isFinite),
      `resource budget table is incomplete: ${JSON.stringify(budgets)}`
    );

    const violations = storageViolations(before, after, callsBudget, refusedBudget);

    const witness = s.resourceWitness;
    assert.ok(witness, 'the quiet window has no attached process-resource witness');
    violations.push(...witness.issues.map(issue => `resource apparatus: ${issue}`));
    const expectedPeers = [...s.peers.keys()].sort((a, b) => a.localeCompare(b));
    const measuredPeers = Object.keys(witness.deltas).sort((a, b) => a.localeCompare(b));
    if (witness.issues.length === 0 && measuredPeers.join(',') !== expectedPeers.join(',')) {
      violations.push(
        `resource apparatus: expected conductors ${expectedPeers.join(',')}, measured ${measuredPeers.join(',')}`
      );
    }
    if (witness.elapsedMs > 0) {
      const cpuSeconds = Object.values(witness.deltas).reduce(
        (total, reading) => total + reading.cpuSeconds,
        0
      );
      const cpuRate = cpuSeconds / (witness.elapsedMs / 60_000);
      if (cpuRate > cpuBudget) {
        violations.push(
          `household conductors: ${cpuRate.toFixed(1)} CPU s/min > ${cpuBudget}; 60 is one core pinned`
        );
      }
    }

    assert.deepEqual(
      violations,
      [],
      `quiet-household resource budget failed:\n- ${violations.join('\n- ')}`
    );
  }
);
