// The placement status surface (interfaces spec §H.2): what an app author can ask, and the
// guarantee that the unmeasured state never reads as met.
import { describe, expect, it } from 'vitest';

import {
  isDevicePlane,
  needId,
  placementFor,
  placementRoutes,
  type DevicePlacement,
  type NeedRef,
  type NeedStatus,
  type PendingStatus,
  type PlacementRoutes,
} from '../src/placement';
import { StorageError } from '../src/types';

const CLOSURE: NeedRef = { field: 'closure' };
const LOSS: NeedRef = { field: 'loss', contentType: 'journal-entry' };
const INDEXED: NeedRef = { field: 'indexed', contentType: 'note' };

const notCarried = {
  why: { reason: 'no-eligible' },
  comeBack: { when: 'on-event', event: 'posture-changed' },
  meanwhile: { local: 'use-held', what: 'the copy on this device' },
  escalation: null,
};

function device(declared: NeedRef[], closure: NeedStatus = { need: CLOSURE, state: 'present' }): DevicePlacement {
  return {
    declared: async () => declared,
    status: async (need) => (needId(need) === 'closure' ? closure : { need, state: 'unreachable', reason: 'refused' }),
  };
}

function routes(answer: (path: string) => unknown): PlacementRoutes & { paths: string[] } {
  const paths: string[] = [];
  return {
    paths,
    getJson: (async (path: string) => {
      paths.push(path);
      const a = answer(path);
      if (a instanceof Error) throw a;
      return a;
    }) as PlacementRoutes['getJson'],
  };
}

describe('placement.status()', () => {
  it('answers device needs from the device and network needs from the route, by plane', async () => {
    const client = routes(() => ({
      needs: [
        { need: LOSS, state: 'absent', unanswered: notCarried },
        // A remote never reports a residency need (R6): a device-plane entry is dropped.
        { need: CLOSURE, state: 'absent', unanswered: notCarried },
      ],
    }));
    const p = placementFor('journal', client, device([CLOSURE, LOSS]));
    const status = await p.status();
    expect(status).toEqual([
      { need: CLOSURE, state: 'present' },
      { need: LOSS, state: 'absent', unanswered: notCarried },
    ]);
    expect(client.paths).toEqual(['/api/v1/apps/journal/placement']);
  });

  it('never reads an unmeasured need as met: route down, unanswered, or unreadable', async () => {
    const down = placementFor('journal', routes(() => new StorageError('HTTP 404', 404)), device([CLOSURE, LOSS]));
    expect(await down.status()).toEqual([
      { need: CLOSURE, state: 'present' },
      { need: LOSS, state: 'unreachable', reason: 'transport_error' },
    ]);

    const refused = placementFor('journal', routes(() => new StorageError('HTTP 403', 403)), device([LOSS]));
    expect(await refused.status()).toEqual([{ need: LOSS, state: 'unreachable', reason: 'refused' }]);

    const silent = placementFor('journal', routes(() => ({ needs: [] })), device([LOSS, INDEXED]));
    expect(await silent.status()).toEqual([
      { need: LOSS, state: 'unreachable', reason: 'not_yet_delivered' },
      { need: INDEXED, state: 'unreachable', reason: 'not_yet_delivered' },
    ]);

    const garbled = placementFor(
      'journal',
      routes(() => ({ needs: [{ need: LOSS, state: 'met' }, { need: INDEXED, state: 'absent' }] })),
      device([LOSS, INDEXED]),
    );
    for (const s of await garbled.status()) expect(s.state).toBe('unreachable');
  });

  it('asks no route when the app declares only device needs', async () => {
    const client = routes(() => ({ needs: [] }));
    await placementFor('journal', client, device([CLOSURE])).status();
    expect(client.paths).toEqual([]);
    expect(isDevicePlane(CLOSURE)).toBe(true);
    expect(isDevicePlane(LOSS)).toBe(false);
  });
});

describe('placement.watch()', () => {
  async function collect(it: AsyncIterable<PendingStatus<NeedStatus, unknown>>, max = 10) {
    const out: PendingStatus<NeedStatus, unknown>[] = [];
    for await (const s of it) {
      out.push(s);
      if (out.length >= max) break;
    }
    return out;
  }

  it('reports only changes, and stops at answered', async () => {
    const answers = [
      { needs: [] },
      { needs: [] },
      { needs: [{ need: LOSS, state: 'absent', unanswered: notCarried }] },
      { needs: [{ need: LOSS, state: 'absent', unanswered: notCarried }] },
      { needs: [{ need: LOSS, state: 'present' }] },
    ];
    let call = 0;
    const waits: number[] = [];
    // The clock advances with every wait, as a real one would: the retry time moves each poll,
    // and the quiet need must still yield nothing.
    let clock = 1_000_000;
    const p = placementFor('journal', routes(() => answers[Math.min(call++, answers.length - 1)]), device([LOSS]), {
      pollMs: 5000,
      now: () => clock,
      sleep: async (ms) => {
        waits.push(ms);
        clock += ms;
      },
    });
    const seen = await collect(p.watch(LOSS));
    expect(seen.map((s) => s.status)).toEqual(['retrying-at', 'unanswered', 'answered']);
    expect(seen[0]).toEqual({ status: 'retrying-at', epochSecs: 1005 });
    expect(seen[2]).toEqual({ status: 'answered', value: { need: LOSS, state: 'present' } });
    // A quiet need yields nothing: five reads, three reports.
    expect(call).toBe(5);
    expect(waits).toEqual([5000, 5000, 5000, 5000]);
  });

  it('an unreachable need stays quiet across polls on a moving clock, and reports when its reason changes', async () => {
    // Reads: four route failures (403), then two where the worker cannot say what is declared,
    // then two more 403s, then present.
    const plan: ('refused' | 'worker-down' | 'present')[] = [
      'refused', 'refused', 'refused', 'refused', 'worker-down', 'worker-down', 'refused', 'refused', 'present',
    ];
    let read = 0;
    let clock = 2_000_000;
    const step = () => plan[Math.min(read, plan.length - 1)];
    const dev: DevicePlacement = {
      declared: async () => {
        if (step() === 'worker-down') {
          read++;
          throw new Error('worker did not answer');
        }
        return [LOSS];
      },
      status: async (need) => ({ need, state: 'unreachable', reason: 'refused' }),
    };
    const client = routes(() => {
      const s = step();
      read++;
      return s === 'present' ? { needs: [{ need: LOSS, state: 'present' }] } : new StorageError('HTTP 403', 403);
    });
    const p = placementFor('journal', client, dev, {
      pollMs: 30_000,
      now: () => clock,
      sleep: async (ms) => void (clock += ms),
    });
    const seen = await collect(p.watch(LOSS));
    expect(read).toBe(plan.length);
    // Nine reads, four reports: refused, worker-down, refused again, answered.
    expect(seen.map((s) => s.status)).toEqual(['retrying-at', 'retrying-at', 'retrying-at', 'answered']);
    expect(seen[0]).toEqual({ status: 'retrying-at', epochSecs: 2_030 });
    expect(seen[1]).toEqual({ status: 'retrying-at', epochSecs: 2_000 + 4 * 30 + 30 });
  });

  it('waits until the come-back time an answer gave', async () => {
    const at = { ...notCarried, comeBack: { when: 'at', epochSecs: 1_060 } };
    let call = 0;
    const waits: number[] = [];
    const p = placementFor(
      'journal',
      routes(() =>
        call++ === 0 ? { needs: [{ need: LOSS, state: 'absent', unanswered: at }] } : { needs: [{ need: LOSS, state: 'present' }] },
      ),
      device([LOSS]),
      { now: () => 1_000_000, sleep: async (ms) => void waits.push(ms) },
    );
    const seen = await collect(p.watch(LOSS));
    expect(seen.map((s) => s.status)).toEqual(['unanswered', 'answered']);
    expect(waits).toEqual([60_000]);
  });

  it('never spins on a come-back time already past, or one too close', async () => {
    // The route keeps naming the same past time (a stale answer or clock skew), then answers.
    const past = { ...notCarried, comeBack: { when: 'at', epochSecs: 900 } };
    const now = { ...notCarried, comeBack: { when: 'at', epochSecs: 1_000 } };
    const soon = { ...notCarried, comeBack: { when: 'at', epochSecs: 1_000 }, meanwhile: { local: 'run-here' } };
    const answers = [past, past, now, soon];
    let call = 0;
    const waits: number[] = [];
    let clock = 1_000_000;
    const p = placementFor(
      'journal',
      routes(() => {
        const a = answers[call++];
        return a === undefined ? { needs: [{ need: LOSS, state: 'present' }] } : { needs: [{ need: LOSS, state: 'absent', unanswered: a }] };
      }),
      device([LOSS]),
      {
        pollMs: 7_000,
        now: () => clock,
        sleep: async (ms) => {
          waits.push(ms);
          // The last wait lands 200 ms short of the given time.
          if (waits.length === 3) clock = 999_800;
        },
      },
    );
    const seen = await collect(p.watch(LOSS));
    expect(seen.map((s) => s.status)).toEqual(['unanswered', 'unanswered', 'unanswered', 'answered']);
    // past, past, exactly now: the poll interval; 200 ms ahead: the 1 s floor.
    expect(waits).toEqual([7_000, 7_000, 7_000, 1_000]);
  });

  it('reads an absent entry with a malformed come-back time as unreachable, never waits on NaN', async () => {
    const bad = { ...notCarried, comeBack: { when: 'at' } };
    const p = placementFor('journal', routes(() => ({ needs: [{ need: LOSS, state: 'absent', unanswered: bad }] })), device([LOSS]));
    expect(await p.status()).toEqual([{ need: LOSS, state: 'unreachable', reason: 'unverifiable' }]);
    const unknownReason = { ...notCarried, why: { reason: 'made-up' } };
    const q = placementFor('journal', routes(() => ({ needs: [{ need: LOSS, state: 'absent', unanswered: unknownReason }] })), device([LOSS]));
    expect(await q.status()).toEqual([{ need: LOSS, state: 'unreachable', reason: 'unverifiable' }]);
  });

  it('ends withdrawn when the app no longer declares the need', async () => {
    let declared: NeedRef[] = [LOSS];
    const p = placementFor('journal', routes(() => ({ needs: [] })), {
      declared: async () => declared,
      status: async (need) => ({ need, state: 'present' }),
    }, { sleep: async () => void (declared = []) });
    const seen = await collect(p.watch(LOSS));
    expect(seen.map((s) => s.status)).toEqual(['retrying-at', 'withdrawn']);
  });
});

describe('placement.why()', () => {
  it('explains network needs through the storage route', async () => {
    const view = { need: 'bafyrei-stand-in', outcome: { outcome: 'held' } };
    const client = routes(() => view);
    const why = await placementFor('journal', client, device([LOSS])).why(LOSS);
    expect(why).toEqual({ plane: 'network', view });
    expect(client.paths).toEqual([placementRoutes('journal').why(LOSS)]);
    expect(client.paths[0]).toBe('/api/v1/apps/journal/placement/why?field=loss&contentType=journal-entry');
  });

  it('explains device needs from the device, carrying no device facts', async () => {
    const client = routes(() => ({}));
    const kept = await placementFor('journal', client, device([CLOSURE])).why(CLOSURE);
    expect(kept).toEqual({ plane: 'device', kept: true, deferred: false, unanswered: null });
    const deferred = await placementFor(
      'journal',
      client,
      device([CLOSURE], { need: CLOSURE, state: 'absent', unanswered: notCarried as never }),
    ).why(CLOSURE);
    expect(deferred).toMatchObject({ plane: 'device', kept: false, deferred: true, unanswered: { why: 'no-eligible' } });
    const unknown = await placementFor(
      'journal',
      client,
      device([CLOSURE], { need: CLOSURE, state: 'unreachable', reason: 'unverifiable' }),
    ).why(CLOSURE);
    expect(unknown).toMatchObject({ plane: 'device', kept: false, unanswered: { why: 'unverifiable' } });
    expect(client.paths).toEqual([]);
  });
});
