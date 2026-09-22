import { strict as assert } from 'node:assert';
import { resolve } from 'node:path';
import { describe, it } from 'node:test';

import {
  captureHouseholdResources,
  counterRatePerMinute,
  requiredPrometheusSeriesSum,
  resourceWitness,
  type ProcReader,
  type ResourceSnapshot,
} from '../process-resources.js';

function stat(cpuUser: number, cpuSystem: number, startTicks: number): string {
  const fields = Array.from({ length: 20 }, () => '0');
  fields[0] = 'S';
  fields[11] = String(cpuUser);
  fields[12] = String(cpuSystem);
  fields[19] = String(startTicks);
  return `123 (holochain worker) ${fields.join(' ')}`;
}

const MATTHEW_CONFIG = '/mesh/matthew/conductor-config.yaml';

function fakeReader(
  processes: { pid: number; config: string; startTicks?: number; exe?: string }[]
): ProcReader {
  const files = new Map<string, string>();
  const links = new Map<string, string>();
  files.set('/proc/sys/kernel/random/boot_id', 'boot-a\n');
  for (const process of processes) {
    const root = `/proc/${process.pid}`;
    const exe = process.exe ?? '/opt/holochain';
    files.set(`${root}/cmdline`, `${exe}\0--config-path\0${process.config}\0`);
    files.set(`${root}/stat`, stat(20, 5, process.startTicks ?? 100));
    files.set(`${root}/status`, 'Name:\tholochain\nVmRSS:\t2048 kB\n');
    files.set(
      `${root}/io`,
      'rchar: 100\nwchar: 200\nread_bytes: 300\nwrite_bytes: 400\ncancelled_write_bytes: 0\n'
    );
    links.set(`${root}/exe`, exe);
    links.set(`${root}/cwd`, '/');
  }
  return {
    listPids: () => processes.map(process => process.pid),
    readText: path => {
      const value = files.get(path);
      if (value === undefined) throw new Error(`missing fake file ${path}`);
      return value;
    },
    readLink: path => {
      const value = links.get(path);
      if (value === undefined) throw new Error(`missing fake link ${path}`);
      return value;
    },
  };
}

function snapshot(overrides: Partial<ResourceSnapshot> = {}): ResourceSnapshot {
  return {
    atUnixMs: 1_000,
    monotonicMs: 500,
    bootId: 'boot-a',
    clockTicksPerSecond: 100,
    issues: [],
    samples: {
      matthew: {
        peer: 'matthew',
        pid: 42,
        startTicks: 500,
        executable: '/opt/holochain',
        configPath: MATTHEW_CONFIG,
        cpuTicks: 100,
        rssKiB: 2_048,
        io: { rchar: 10, wchar: 20, readBytes: 30, writeBytes: 40, cancelledWriteBytes: 0 },
      },
    },
    ...overrides,
  };
}

void describe('process resource observations', () => {
  void it('reports a missing named conductor instead of manufacturing zero usage', () => {
    const reading = captureHouseholdResources(
      { matthew: MATTHEW_CONFIG },
      { reader: fakeReader([]), now: () => 1, clockTicksPerSecond: 100 }
    );
    assert.deepEqual(reading.samples, {});
    assert.match(
      reading.issues.join('\n'),
      /matthew: expected exactly one owned conductor, observed 0/
    );
  });

  void it('ignores an unrelated holochain process and still reports the named peer missing', () => {
    const reading = captureHouseholdResources(
      { matthew: MATTHEW_CONFIG },
      {
        reader: fakeReader([{ pid: 9, config: '/somewhere/else/conductor-config.yaml' }]),
        now: () => 1,
        clockTicksPerSecond: 100,
      }
    );
    assert.deepEqual(reading.samples, {});
    assert.equal(reading.issues.length, 1);
    assert.match(reading.issues[0], /matthew.*observed 0/);
  });

  void it('rejects PID reuse even when the numeric pid is unchanged', () => {
    const before = snapshot();
    const after = snapshot({
      atUnixMs: 61_000,
      monotonicMs: 60_500,
      samples: {
        matthew: { ...before.samples.matthew, startTicks: 900, cpuTicks: 200 },
      },
    });
    const witness = resourceWitness(before, after);
    assert.deepEqual(witness.deltas, {});
    assert.match(witness.issues.join('\n'), /matthew: conductor identity changed/);
  });

  void it('computes CPU seconds and IO deltas from guarded counters', () => {
    const before = snapshot();
    const after = snapshot({
      atUnixMs: 61_000,
      monotonicMs: 60_500,
      samples: {
        matthew: {
          ...before.samples.matthew,
          cpuTicks: 350,
          rssKiB: 3_072,
          io: { rchar: 110, wchar: 220, readBytes: 330, writeBytes: 440, cancelledWriteBytes: 2 },
        },
      },
    });
    const witness = resourceWitness(before, after);
    assert.deepEqual(witness.issues, []);
    assert.equal(witness.elapsedMs, 60_000);
    assert.equal(witness.deltas.matthew.cpuSeconds, 2.5);
    assert.equal(witness.deltas.matthew.rssKiBBefore, 2_048);
    assert.equal(witness.deltas.matthew.rssKiBAfter, 3_072);
    assert.deepEqual(witness.deltas.matthew.io, {
      rchar: 100,
      wchar: 200,
      readBytes: 300,
      writeBytes: 400,
      cancelledWriteBytes: 2,
    });
  });

  void it('reports counter rollback and omits an invalid delta', () => {
    const before = snapshot();
    const after = snapshot({
      atUnixMs: 61_000,
      monotonicMs: 60_500,
      samples: {
        matthew: { ...before.samples.matthew, cpuTicks: 99 },
      },
    });
    const witness = resourceWitness(before, after);
    assert.deepEqual(witness.deltas, {});
    assert.match(witness.issues.join('\n'), /CPU counter rolled backwards/);
  });

  void it('emits no deltas when CLK_TCK is zero', () => {
    const before = snapshot({ clockTicksPerSecond: 0 });
    const after = snapshot({ atUnixMs: 61_000, monotonicMs: 60_500, clockTicksPerSecond: 0 });
    const witness = resourceWitness(before, after);
    assert.deepEqual(witness.deltas, {});
    assert.match(witness.issues.join('\n'), /CLK_TCK/);
  });

  void it('emits no deltas when the host rebooted', () => {
    const before = snapshot();
    const after = snapshot({ atUnixMs: 61_000, monotonicMs: 60_500, bootId: 'boot-b' });
    const witness = resourceWitness(before, after);
    assert.deepEqual(witness.deltas, {});
    assert.match(witness.issues.join('\n'), /boot identity/);
  });

  void it('emits no deltas for a non-positive monotonic window', () => {
    const before = snapshot();
    const after = snapshot({ atUnixMs: 61_000, monotonicMs: 500 });
    const witness = resourceWitness(before, after);
    assert.deepEqual(witness.deltas, {});
    assert.match(witness.issues.join('\n'), /invalid monotonic elapsed/);
  });

  void it('captures one exact config-path identity', () => {
    const config = resolve(MATTHEW_CONFIG);
    const reading = captureHouseholdResources(
      { matthew: config },
      {
        reader: fakeReader([{ pid: 123, config }]),
        now: () => 50,
        clockTicksPerSecond: 100,
      }
    );
    assert.deepEqual(reading.issues, []);
    assert.equal(reading.samples.matthew.pid, 123);
    assert.equal(reading.samples.matthew.cpuTicks, 25);
    assert.equal(reading.samples.matthew.rssKiB, 2_048);
    assert.equal(typeof reading.kernelMonotonicMs, 'number');
    assert.ok(Number.isFinite(reading.kernelMonotonicMs));
    assert.ok((reading.kernelMonotonicMs ?? -1) >= 0);
  });

  void it('records an available kernel monotonic witness without changing rate timing', () => {
    const config = resolve(MATTHEW_CONFIG);
    const reading = captureHouseholdResources(
      { matthew: config },
      {
        reader: fakeReader([{ pid: 123, config }]),
        now: () => 50,
        monotonicNow: () => 75,
        kernelMonotonicNow: () => 12_345.5,
        clockTicksPerSecond: 100,
      }
    );
    assert.equal(reading.monotonicMs, 75);
    assert.equal(reading.kernelMonotonicMs, 12_345.5);
  });

  void it('brackets discovery with two injected kernel monotonic samples', () => {
    const config = resolve(MATTHEW_CONFIG);
    const clocks = [900, 1_000];
    const reading = captureHouseholdResources(
      { matthew: config },
      {
        reader: fakeReader([{ pid: 123, config }]),
        kernelMonotonicNow: () => clocks.shift() ?? 1_000,
        clockTicksPerSecond: 100,
      }
    );
    assert.equal(reading.kernelMonotonicStartMs, 900);
    assert.equal(reading.kernelMonotonicMs, 1_000);
  });

  void it('omits a malformed kernel monotonic witness instead of manufacturing zero', () => {
    const config = resolve(MATTHEW_CONFIG);
    const reading = captureHouseholdResources(
      { matthew: config },
      {
        reader: fakeReader([{ pid: 123, config }]),
        kernelMonotonicNow: () => Number.NaN,
        clockTicksPerSecond: 100,
      }
    );
    assert.equal(reading.kernelMonotonicMs, undefined);
    assert.match(reading.issues.join('\n'), /invalid kernel monotonic clock NaNms/);
  });

  void it('keeps legacy snapshots valid when the kernel witness is absent', () => {
    const before = snapshot();
    const after = snapshot({ atUnixMs: 61_000, monotonicMs: 60_500 });
    const witness = resourceWitness(before, after);
    assert.equal(before.kernelMonotonicMs, undefined);
    assert.equal(after.kernelMonotonicMs, undefined);
    assert.equal(witness.elapsedMs, 60_000);
    assert.deepEqual(witness.issues, []);
  });

  void it('rejects negative kernel CPU counters instead of accepting them as samples', () => {
    const config = resolve(MATTHEW_CONFIG);
    const base = fakeReader([{ pid: 123, config }]);
    const reader: ProcReader = {
      ...base,
      readText: path => (path === '/proc/123/stat' ? stat(-1, 0, 100) : base.readText(path)),
    };
    const reading = captureHouseholdResources(
      { matthew: config },
      { reader, now: () => 50, monotonicNow: () => 10, clockTicksPerSecond: 100 }
    );
    assert.deepEqual(reading.samples, {});
    assert.match(reading.issues.join('\n'), /malformed stat counters/);
  });

  void it('reads Prometheus sample values rather than optional timestamps', () => {
    const reading = requiredPrometheusSeriesSum(
      'metric_total{peer="a"} 2 1700000000000\nmetric_total{peer="b"} 3 1700000000001\n',
      'metric_total'
    );
    assert.deepEqual(reading, { value: 5 });
  });

  void it('refuses a non-positive per-peer metrics window', () => {
    assert.deepEqual(counterRatePerMinute(10, 500, 500), {
      value: null,
      issue: 'invalid metrics elapsed time 0ms',
    });
    assert.equal(counterRatePerMinute(10, 500, 60_500).value, 10);
  });
});
