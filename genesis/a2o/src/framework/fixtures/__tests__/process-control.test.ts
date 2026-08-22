/**
 * Tests for process-control.ts — the drills' one place to find/capture/re-exec
 * mesh processes. Run: tsx --test src/framework/fixtures/__tests__/process-control.test.ts
 *
 * The load-bearing case is the procfs capture: `/proc/<pid>/environ` has
 * st_size 0, so a copyFile-style capture is empty; the module must read to EOF.
 */

import { strict as assert } from 'node:assert';
import { copyFileSync, mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, beforeEach, describe, it } from 'node:test';

import {
  captureProcFile,
  discoverStoragePeerPid,
  doorwayArgvMatch,
  pidsMatchingArgv,
  processAlive,
  readNulSeparatedProcFile,
  readProcEnvironment,
  storagePeerArgvMatch,
  writeRestartCapture,
} from '../process-control.js';

const PROC_AVAILABLE = process.platform === 'linux';

void describe('process-control', { skip: !PROC_AVAILABLE && 'procfs-only' }, () => {
  let dir = '';
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'process-control-'));
  });
  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  void it('captureProcFile reads /proc/self/environ to EOF (where copyFile yields 0 bytes)', () => {
    const copied = join(dir, 'copied.environ');
    copyFileSync('/proc/self/environ', copied);
    // The trap this module exists for: procfs st_size is 0, so copyFile is empty.
    assert.equal(statSync(copied).size, 0, 'copyFile on procfs should yield 0 bytes (the trap)');
    const captured = captureProcFile(process.pid, 'environ');
    assert.ok(captured.length > 0, 'a read-to-EOF capture is non-empty');
    assert.ok(captured.toString('utf8').includes('PATH='), 'the capture carries real variables');
  });

  void it('writeRestartCapture persists a non-empty environ and the exe path', () => {
    const capture = writeRestartCapture(dir, 'self', process.pid);
    assert.ok(statSync(capture.environPath).size > 0);
    assert.equal(readFileSync(capture.exePath, 'utf8').trim(), capture.exe);
    assert.ok(capture.exe.length > 0 && !capture.exe.endsWith(' (deleted)'));
    assert.equal(readProcEnvironment(process.pid)['PATH'], process.env['PATH']);
  });

  void it('readNulSeparatedProcFile splits cmdline on NUL and drops empties', () => {
    const argv = readNulSeparatedProcFile('/proc/self/cmdline');
    assert.ok(argv.length >= 1);
    assert.ok(argv.every(part => part.length > 0 && !part.includes('\0')));
  });

  void it('argv matchers are exact on the listen port and never match this test run', () => {
    assert.ok(doorwayArgvMatch('8888').matches(['/x/doorway', '--listen', '127.0.0.1:8888']));
    assert.ok(!doorwayArgvMatch('8888').matches(['/x/doorway', '--listen', '127.0.0.1:18888']));
    assert.ok(!doorwayArgvMatch('8888').matches(['/x/not-doorway', '--listen', '127.0.0.1:8888']));
    assert.ok(storagePeerArgvMatch('8090').matches(['/x/elohim-storage', '--http-port', '8090']));
    assert.ok(!storagePeerArgvMatch('8090').matches(['/x/elohim-storage', '--http-port', '8091']));
    // Every pid listed excludes this process and its parent.
    const all = pidsMatchingArgv({ what: 'anything', matches: () => true });
    assert.ok(!all.includes(process.pid) && !all.includes(process.ppid));
    assert.equal(discoverStoragePeerPid('1'), undefined, 'no storage peer serves port 1');
  });

  void it('processAlive reflects the process table', () => {
    assert.ok(processAlive(process.pid));
    assert.ok(!processAlive(2 ** 22 - 1));
  });
});
