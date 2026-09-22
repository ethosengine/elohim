import { strict as assert } from 'node:assert';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import {
  JEPROF_TIMEOUT_MS,
  MAX_HEAP_SYMBOL_ROWS,
  MAX_JEPROF_OUTPUT_BYTES,
  MAX_JEPROF_TEXT_BYTES,
  parseJeprofText,
  runBoundedTool,
  snapshotBoundedFile,
  summarizeHeapDiff,
  ToolReapUnresolvedError,
  ToolStoppedError,
  type HeapFs,
  type ToolInvocation,
} from '../lib/performance-heap.js';

const SYNTHETIC_TEXT = `Total: -20 B
     -30 150.0% 150.0%      -10  50.0% crate::retire /private/src/lib.rs:7
      10 -50.0% 100.0%       40 -200.0% crate::grow [unknown]
       0   0.0% 100.0%        5 -25.0% 0xdeadbeef
`;
const BEFORE_DUMP = '/dump/early.heap';
const AFTER_DUMP = '/dump/late.heap';
const HOLOCHAIN_BINARY = '/bin/holochain';
const JEPROF_BINARY = '/tools/jeprof';
const SYNTHETIC_DUMP = 'synthetic dump';

void describe('synthetic jeprof heap attribution', () => {
  void it('preserves signed self/cumulative values without summing cumulative rows', () => {
    const parsed = parseJeprofText(SYNTHETIC_TEXT);
    assert.equal(parsed.totalRetainedBytes, -20);
    assert.equal(parsed.accountedSelfBytes, -20);
    assert.deepEqual(parsed.rows, [
      {
        symbol: 'crate::retire',
        selfBytes: -30,
        cumulativeBytes: -10,
        selfPercent: 150,
        cumulativePercent: 50,
      },
      {
        symbol: 'crate::grow [unknown]',
        selfBytes: 10,
        cumulativeBytes: 40,
        selfPercent: -50,
        cumulativePercent: -200,
      },
    ]);
    assert.deepEqual(parsed.unknownCoverage, {
      selfBytes: 0,
      absoluteSelfBytes: 0,
      rowCount: 1,
    });
    assert.equal(parsed.omittedRows, 0);
    assert.equal(parsed.coverageEligible, false);
  });

  void it('does not expose private paths or raw addresses in public symbol rows', () => {
    const serialized = JSON.stringify(parseJeprofText(SYNTHETIC_TEXT));
    assert.doesNotMatch(serialized, /private\/src|lib\.rs|deadbeef/);
  });

  void it('preserves spaced template symbols while removing Unix, Windows, and map paths', () => {
    const parsed = parseJeprofText(String.raw`Total: -6 B
-1 nan +inf -2 -inf operator new<unsigned long> /private/src/alloc.cc:7
-2 -1.5% 33.0% -3 50.0% ns::Type<std::pair<int, long> >::run C:\private\src\run.cpp:8
-3 50.0% 100.0% -4 66.0% ns::forward_path C:/private/src/run.rs:9
0 0% 100% 0 0% ns::mapped [private/maps/lib.so]
`);
    assert.deepEqual(
      parsed.rows.map(row => row.symbol),
      [
        'operator new<unsigned long>',
        'ns::Type<std::pair<int, long> >::run',
        'ns::forward_path',
        'ns::mapped',
      ]
    );
    assert.equal(parsed.rows[0].selfPercent, null);
    assert.equal(parsed.rows[0].cumulativePercent, null);
    assert.doesNotMatch(JSON.stringify(parsed), /private|alloc\.cc|run\.cpp|run\.rs|lib\.so/);
  });

  void it('cuts spaced map annotations and line-column source suffixes without leaking paths', () => {
    const parsed = parseJeprofText(String.raw`Total: 3 B
1 1% 1% 1 1% crate::relative source secret.rs:12:7
1 1% 1% 1 1% crate::windows_map [C:\Program Files\secret.dll]
1 1% 1% 1 1% crate::unix_map [/private/maps with spaces/libsecret.so]
`);
    assert.deepEqual(
      parsed.rows.map(row => row.symbol),
      ['crate::relative source', 'crate::windows_map', 'crate::unix_map']
    );
    assert.doesNotMatch(JSON.stringify(parsed), /secret|Program|private|maps|\.rs|\.dll|\.so/);
  });

  void it('rejects unsafe row numeric tokens rather than silently dropping them', () => {
    assert.throws(
      () => parseJeprofText('Total: 0 B\n999999999999999999999 1% 1% 0 1% symbol'),
      /unsafe self bytes/
    );
    assert.throws(
      () => parseJeprofText('Total: 0 B\n1 1.2.3% 1% 0 1% symbol'),
      /invalid self percent/
    );
  });

  void it('rejects malformed output instead of inventing a zero', () => {
    assert.throws(() => parseJeprofText('no profile here'), /no signed byte total/);
    assert.throws(
      () => parseJeprofText(`Total: 0 B\n${'x'.repeat(MAX_JEPROF_TEXT_BYTES)}`),
      /jeprof text exceeds/
    );
  });

  void it('caps public rows and retains signed plus absolute omitted coverage', () => {
    const rows = Array.from(
      { length: MAX_HEAP_SYMBOL_ROWS + 2 },
      (_, index) => `${index % 2 ? -1 : 1} 1.0% 1.0% 1 1.0% symbol_${index}`
    );
    const parsed = parseJeprofText(`Total: 0 B\n${rows.join('\n')}`);
    assert.equal(parsed.rows.length, MAX_HEAP_SYMBOL_ROWS);
    assert.equal(parsed.omittedRows, 2);
    assert.equal(parsed.unknownCoverage.selfBytes, 0);
    assert.equal(parsed.unknownCoverage.absoluteSelfBytes, 2);
  });

  void it('builds the exact bounded trusted-local invocation and always removes scratch', async () => {
    const sizes = new Map([
      [BEFORE_DUMP, 12],
      [AFTER_DUMP, 13],
      [HOLOCHAIN_BINARY, 14],
      [JEPROF_BINARY, 15],
    ]);
    let removed = '';
    const snapshots: [string, string][] = [];
    const fs: HeapFs = {
      stat: path => ({ isFile: () => sizes.has(path), size: sizes.get(path) ?? 0 }),
      mkdtemp: prefix => `${prefix}synthetic`,
      snapshot: (source, destination) => snapshots.push([source, destination]),
      remove: path => {
        removed = path;
      },
    };
    let invocation: ToolInvocation | undefined;
    const controller = new AbortController();
    const result = await summarizeHeapDiff({
      beforeDump: BEFORE_DUMP,
      afterDump: AFTER_DUMP,
      binaryPath: HOLOCHAIN_BINARY,
      jeprofPath: JEPROF_BINARY,
      signal: controller.signal,
      fs,
      runner: async value => {
        await Promise.resolve();
        invocation = value;
        return { exitCode: 0, signal: null, stdout: SYNTHETIC_TEXT, stderr: '' };
      },
    });
    assert.equal(result.coverageEligible, false);
    assert.equal(invocation?.file, '/usr/bin/perl');
    assert.deepEqual(invocation?.args, [
      JEPROF_BINARY,
      '--text',
      '--functions',
      '--inuse_space',
      '--show_bytes',
      '--cum',
      `--base=${invocation?.cwd}/before.heap`,
      HOLOCHAIN_BINARY,
      `${invocation?.cwd}/after.heap`,
    ]);
    assert.equal(invocation?.timeoutMs, JEPROF_TIMEOUT_MS);
    assert.equal(invocation?.signal, controller.signal);
    assert.equal(invocation?.maxOutputBytes, MAX_JEPROF_OUTPUT_BYTES);
    assert.equal(invocation?.env.JEPROF_TMPDIR, invocation?.cwd);
    assert.deepEqual(snapshots, [
      [BEFORE_DUMP, `${invocation?.cwd}/before.heap`],
      [AFTER_DUMP, `${invocation?.cwd}/after.heap`],
    ]);
    assert.equal(removed, invocation?.cwd);
  });

  void it('refuses non-regular and oversized dumps before invoking the tool', async () => {
    let calls = 0;
    const fs: HeapFs = {
      stat: () => ({ isFile: () => true, size: 1 }),
      mkdtemp: () => '/tmp/not-created',
      snapshot: (_source, _destination, _maxBytes, label) => {
        throw new Error(`${label} must be a bounded regular file`);
      },
      remove: () => undefined,
    };
    const base = {
      beforeDump: BEFORE_DUMP,
      afterDump: AFTER_DUMP,
      binaryPath: HOLOCHAIN_BINARY,
      jeprofPath: JEPROF_BINARY,
      fs,
      runner: async () => {
        await Promise.resolve();
        calls += 1;
        return { exitCode: 0, signal: null, stdout: SYNTHETIC_TEXT, stderr: '' };
      },
    };
    await assert.rejects(summarizeHeapDiff(base), /before dump must be a bounded regular file/);
    assert.equal(calls, 0);
  });

  void it('removes private scratch when the bounded tool fails', async () => {
    let removed = '';
    const fs: HeapFs = {
      stat: () => ({ isFile: () => true, size: 1 }),
      mkdtemp: () => '/tmp/synthetic-private-scratch',
      snapshot: () => undefined,
      remove: path => {
        removed = path;
      },
    };
    await assert.rejects(
      summarizeHeapDiff({
        beforeDump: BEFORE_DUMP,
        afterDump: AFTER_DUMP,
        binaryPath: HOLOCHAIN_BINARY,
        jeprofPath: JEPROF_BINARY,
        fs,
        runner: async () => {
          await Promise.resolve();
          return {
            exitCode: 2,
            signal: null,
            stdout: '',
            stderr: '/private/source/path',
          };
        },
      }),
      /^Error: jeprof failed \(exit 2\)$/
    );
    assert.equal(removed, '/tmp/synthetic-private-scratch');
  });

  void it('snapshot helper refuses symlinks, non-files, and oversized inputs', () => {
    const dir = mkdtempSync(join(tmpdir(), 'heap-snapshot-synthetic-'));
    try {
      const regular = join(dir, 'regular.heap');
      writeFileSync(regular, 'synthetic');
      const link = join(dir, 'link.heap');
      symlinkSync(regular, link);
      assert.throws(
        () => snapshotBoundedFile(link, join(dir, 'link-copy'), 32, SYNTHETIC_DUMP),
        /bounded regular file/
      );
      assert.throws(
        () => snapshotBoundedFile(regular, join(dir, 'large-copy'), 1, SYNTHETIC_DUMP),
        /bounded regular file/
      );
      const directory = join(dir, 'directory.heap');
      mkdirSync(directory);
      assert.throws(
        () => snapshotBoundedFile(directory, join(dir, 'directory-copy'), 32, SYNTHETIC_DUMP),
        /bounded regular file/
      );
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  void it('snapshots multi-chunk artifacts exactly without replacing an existing destination', () => {
    const directory = mkdtempSync(join(tmpdir(), 'heap-copy-chunks-'));
    try {
      const input = join(directory, 'input');
      const output = join(directory, 'output');
      const bytes = Buffer.alloc(3 * 64 * 1024 + 17, 0x5a);
      writeFileSync(input, bytes);
      snapshotBoundedFile(input, output, bytes.length, SYNTHETIC_DUMP);
      assert.deepEqual(readFileSync(output), bytes);
      assert.throws(() => snapshotBoundedFile(input, output, bytes.length, SYNTHETIC_DUMP));
      assert.deepEqual(readFileSync(output), bytes);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });

  void it('uses a hard process-group deadline and acknowledges termination before rejecting', async () => {
    const started = Date.now();
    await assert.rejects(
      runBoundedTool({
        file: process.execPath,
        args: [
          '-e',
          `require('node:child_process').spawn(process.execPath,['-e',"process.on('SIGTERM',()=>{});setInterval(()=>{},1000)"],{stdio:'ignore'});process.on('SIGTERM',()=>{});setInterval(()=>{},1000)`,
        ],
        cwd: '/tmp',
        env: process.env,
        timeoutMs: 20,
        maxOutputBytes: 1024,
      }),
      /jeprof exceeded 20ms/
    );
    assert.ok(Date.now() - started < 2000, 'cleanup must be bounded');
  });

  void it('acknowledges cancellation and refuses already-aborted work', async () => {
    const controller = new AbortController();
    const task = runBoundedTool({
      file: process.execPath,
      args: ['-e', 'setInterval(()=>{},1000)'],
      cwd: '/tmp',
      env: process.env,
      timeoutMs: 2000,
      maxOutputBytes: 1024,
      signal: controller.signal,
    });
    setTimeout(() => controller.abort(), 40);
    await assert.rejects(task, (error: unknown) => {
      assert.ok(error instanceof ToolStoppedError);
      assert.match(error.message, /jeprof cancelled after child reap/);
      assert.equal(error.cleanup.watchdogReaped, true);
      assert.equal(error.cleanup.groupWorkStopped, true);
      return true;
    });
    await assert.rejects(
      runBoundedTool({
        file: process.execPath,
        args: [],
        cwd: '/tmp',
        env: process.env,
        timeoutMs: 2000,
        maxOutputBytes: 1024,
        signal: controller.signal,
      }),
      /cancelled/
    );
  });

  void it('keeps the isolated-worker and parser lifetime ceilings separate', async () => {
    const invocation: ToolInvocation = {
      file: process.execPath,
      args: ['-e', 'process.exit(0)'],
      cwd: '/tmp',
      env: process.env,
      timeoutMs: 902_000,
      maxOutputBytes: 1024,
    };
    await assert.rejects(runBoundedTool(invocation), /outside bounds/);
    await assert.rejects(
      runBoundedTool({ ...invocation, purpose: 'heap-canary-worker', timeoutMs: 902_001 }),
      /outside bounds/
    );
    const result = await runBoundedTool({ ...invocation, purpose: 'heap-canary-worker' });
    assert.equal(result.exitCode, 0);
    assert.equal(result.cleanup?.groupWorkStopped, true);
  });

  void it('the OS deadline kills the watchdog while the Node event loop is blocked', async () => {
    const children = () =>
      readFileSync(`/proc/${process.pid}/task/${process.pid}/children`, 'utf8')
        .trim()
        .split(/\s+/)
        .filter(Boolean);
    const before = new Set(children());
    const task = runBoundedTool({
      file: process.execPath,
      args: ['-e', 'setInterval(()=>{},1000)'],
      cwd: '/tmp',
      env: process.env,
      timeoutMs: 100,
      maxOutputBytes: 1024,
    });
    const spawned = children().filter(pid => !before.has(pid));
    assert.equal(spawned.length, 1);
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
    // No JS timer or child-exit callback has run since spawn. Only the OS
    // watchdog can have stopped this exact still-unreaped direct child.
    const stat = readFileSync(`/proc/${spawned[0]}/stat`, 'utf8');
    assert.equal(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[0], 'Z');
    await assert.rejects(task, ToolStoppedError);
  });

  void it('does not equate a closed watchdog with stopped parser helpers', async () => {
    const started = performance.now();
    const result = await runBoundedTool({
      file: process.execPath,
      args: [
        '-e',
        `require('node:child_process').spawn(process.execPath,['-e','setTimeout(()=>{},150)'],{stdio:'ignore'}).unref()`,
      ],
      cwd: '/tmp',
      env: process.env,
      timeoutMs: 2000,
      maxOutputBytes: 1024,
    });
    assert.ok(performance.now() - started >= 150);
    assert.equal(result.cleanup?.watchdogReaped, true);
    assert.equal(result.cleanup?.groupWorkStopped, true);
    assert.equal(typeof result.cleanup?.unreapedZombies, 'number');
  });

  void it('retains parser inputs if the runner cannot establish reap', async () => {
    let removed = false;
    await assert.rejects(
      summarizeHeapDiff({
        beforeDump: BEFORE_DUMP,
        afterDump: AFTER_DUMP,
        binaryPath: HOLOCHAIN_BINARY,
        jeprofPath: JEPROF_BINARY,
        fs: {
          stat: () => ({ isFile: () => true, size: 1 }),
          mkdtemp: () => '/private/scratch',
          snapshot: () => undefined,
          remove: () => {
            removed = true;
          },
        },
        // eslint-disable-next-line @typescript-eslint/require-await -- deterministic async failure double.
        runner: async () => {
          throw new ToolReapUnresolvedError();
        },
      }),
      ToolReapUnresolvedError
    );
    assert.equal(removed, false);
  });
});
