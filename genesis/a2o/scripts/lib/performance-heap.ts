/** Bounded adapter for trusted local jemalloc heap dumps and the existing jeprof tool. */

import { spawn } from 'node:child_process';
import {
  chmodSync,
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readdirSync,
  readSync,
  rmSync,
  statSync,
  writeSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { isAbsolute, join } from 'node:path';

export const MAX_HEAP_DUMP_BYTES = 32 * 1024 * 1024;
export const MAX_JEPROF_OUTPUT_BYTES = 8 * 1024 * 1024;
export const JEPROF_TIMEOUT_MS = 30_000;
export const MAX_JEPROF_TEXT_BYTES = 8 * 1024 * 1024;
export const MAX_HEAP_SYMBOL_ROWS = 100;
export const TOOL_CLEANUP_TIMEOUT_MS = 1000;
const HEAP_CANARY_WORKER_PURPOSE = 'heap-canary-worker';

export interface ToolCleanupEvidence {
  watchdogReaped: true;
  groupWorkStopped: true;
  unreapedZombies: number;
}

export interface HeapSymbolRow {
  symbol: string;
  selfBytes: number;
  cumulativeBytes: number;
  selfPercent: number | null;
  cumulativePercent: number | null;
}

export interface HeapDiffSummary {
  kind: 'jemalloc-retained-heap/v1';
  totalRetainedBytes: number;
  accountedSelfBytes: number;
  unknownCoverage: { selfBytes: number; absoluteSelfBytes: number; rowCount: number };
  omittedRows: number;
  rows: HeapSymbolRow[];
  coverageEligible: false;
  coverageReason: string;
}

export interface ToolInvocation {
  /** Closed caller purpose keeps parser and isolated-worker lifetime ceilings distinct. */
  purpose?: 'jeprof' | 'heap-canary-worker';
  file: string;
  args: string[];
  cwd: string;
  env: NodeJS.ProcessEnv;
  timeoutMs: number;
  maxOutputBytes: number;
  signal?: AbortSignal;
}

/** A caller must preserve private inputs when child cleanup cannot be established. */
export class ToolReapUnresolvedError extends Error {
  constructor() {
    super('bounded tool child reap unresolved; private scratch retained');
    this.name = 'ToolReapUnresolvedError';
  }
}

/** Failed parsing with affirmative, separately typed process-stop evidence. */
export class ToolStoppedError extends Error {
  constructor(
    message: string,
    readonly cleanup: ToolCleanupEvidence
  ) {
    super(message);
    this.name = 'ToolStoppedError';
  }
}

export interface ToolResult {
  exitCode: number | null;
  signal: NodeJS.Signals | null;
  stdout: string;
  stderr: string;
  /** Present on the real runner; zombies cannot execute but are not claimed reaped. */
  cleanup?: ToolCleanupEvidence;
}

export type ToolRunner = (invocation: ToolInvocation) => Promise<ToolResult>;

export interface HeapFs {
  stat(path: string): { isFile(): boolean; size: number };
  mkdtemp(prefix: string): string;
  snapshot(source: string, destination: string, maxBytes: number, label: string): void;
  remove(path: string): void;
}

export interface HeapDiffOptions {
  beforeDump: string;
  afterDump: string;
  binaryPath: string;
  jeprofPath: string;
  signal?: AbortSignal;
  runner?: ToolRunner;
  fs?: HeapFs;
}

const localFs: HeapFs = {
  stat: path => statSync(path),
  mkdtemp: prefix => mkdtempSync(prefix),
  snapshot: snapshotBoundedFile,
  remove: path => rmSync(path, { recursive: true, force: true }),
};

function boundedInteger(raw: string, label: string): number {
  if (!/^-?\d+$/.test(raw)) throw new Error(`invalid ${label}: ${raw}`);
  const value = Number(raw);
  if (!Number.isSafeInteger(value)) throw new Error(`unsafe ${label}: ${raw}`);
  return value;
}

function boundedPercent(raw: string, label: string): number | null {
  if (/^(?:nan|[+-]inf)$/.test(raw)) return null;
  if (!raw.endsWith('%')) throw new Error(`invalid ${label}: ${raw}`);
  const numeric = raw.slice(0, -1);
  const unsigned = numeric.startsWith('-') ? numeric.slice(1) : numeric;
  if (
    !unsigned ||
    [...unsigned].some(character => character !== '.' && !/\d/.test(character)) ||
    unsigned.split('.').length > 2 ||
    !/\d/.test(unsigned)
  )
    throw new Error(`invalid ${label}: ${raw}`);
  const value = Number(numeric);
  if (!Number.isFinite(value)) throw new Error(`non-finite ${label}: ${raw}`);
  return value;
}

function publicSymbol(raw: string): string | null {
  const withoutControls = [...raw]
    .map(character => {
      const code = character.codePointAt(0) ?? 0;
      return code < 32 || code === 127 ? ' ' : character;
    })
    .join('')
    .trim();
  if (!withoutControls || /^(?:0x)?[0-9a-f]+$/i.test(withoutControls) || withoutControls === '??')
    return null;
  // jeprof appends its source/map field after the symbol. Never project local
  // absolute paths, file:line tokens, or map annotations into public evidence.
  const tokens = withoutControls.split(/\s+/);
  const firstPrivate = tokens.findIndex(token => {
    const sourceParts = token.split(':');
    let sourceCoordinates = 0;
    while (sourceParts.length > 1 && /^\d+$/.test(sourceParts.at(-1) ?? '')) {
      sourceParts.pop();
      sourceCoordinates += 1;
    }
    const sourcePath = sourceParts.join(':');
    const extension = sourcePath.slice(sourcePath.lastIndexOf('.') + 1).toLowerCase();
    return (
      token.startsWith('/') ||
      (/^[A-Za-z]:/.test(token) && ['\\', '/'].includes(token[2] ?? '')) ||
      (sourceCoordinates > 0 && ['rs', 'c', 'cc', 'cpp', 'h', 'hpp'].includes(extension)) ||
      (token.startsWith('[') && /[/\\]/.test(token))
    );
  });
  const symbol = (firstPrivate < 0 ? tokens : tokens.slice(0, firstPrivate)).join(' ').trim();
  if (!symbol || /(?:^|\s)(?:0x)?[0-9a-f]+$/i.test(symbol)) return null;
  return symbol.slice(0, 512);
}

/** Parse `jeprof --text --functions --inuse_space --show_bytes --cum` output. */
export function parseJeprofText(text: string): HeapDiffSummary {
  if (Buffer.byteLength(text) > MAX_JEPROF_TEXT_BYTES)
    throw new Error(`jeprof text exceeds ${MAX_JEPROF_TEXT_BYTES} bytes`);
  const lines = text.split(/\r?\n/);
  const totalLine = lines.find(line => /^Total:\s+-?\d+\s+B\s*$/.test(line.trim()));
  if (!totalLine) throw new Error('jeprof output has no signed byte total');
  const totalRetainedBytes = boundedInteger(
    /^Total:\s+(-?\d+)\s+B\s*$/.exec(totalLine.trim())![1],
    'total retained bytes'
  );
  const rows: HeapSymbolRow[] = [];
  let unknownSelfBytes = 0;
  let unknownAbsoluteSelfBytes = 0;
  let unknownRows = 0;
  let omittedRows = 0;
  for (const line of lines) {
    const fields = line.trim().split(/\s+/);
    if (fields.length < 6 || !/^-?\d+$/.test(fields[0])) continue;
    const selfBytes = boundedInteger(fields[0], 'self bytes');
    const selfPercent = boundedPercent(fields[1], 'self percent');
    boundedPercent(fields[2], 'running percent');
    const cumulativeBytes = boundedInteger(fields[3], 'cumulative bytes');
    const cumulativePercent = boundedPercent(fields[4], 'cumulative percent');
    const symbol = publicSymbol(fields.slice(5).join(' '));
    if (!symbol) {
      unknownSelfBytes = safeSum(unknownSelfBytes, selfBytes, 'unknown self bytes');
      unknownAbsoluteSelfBytes = safeSum(
        unknownAbsoluteSelfBytes,
        Math.abs(selfBytes),
        'unknown absolute self bytes'
      );
      unknownRows += 1;
      continue;
    }
    if (rows.length >= MAX_HEAP_SYMBOL_ROWS) {
      unknownSelfBytes = safeSum(unknownSelfBytes, selfBytes, 'omitted self bytes');
      unknownAbsoluteSelfBytes = safeSum(
        unknownAbsoluteSelfBytes,
        Math.abs(selfBytes),
        'omitted absolute self bytes'
      );
      omittedRows += 1;
      continue;
    }
    rows.push({
      symbol,
      selfBytes,
      cumulativeBytes,
      selfPercent,
      cumulativePercent,
    });
  }
  const accountedSelfBytes = rows.reduce(
    (sum, row) => safeSum(sum, row.selfBytes, 'accounted self bytes'),
    0
  );
  // Flat/self bytes partition the total; cumulative bytes overlap call paths
  // and therefore are deliberately never summed as a total.
  const unparsedSelfBytes = safeSum(
    safeSum(totalRetainedBytes, -accountedSelfBytes, 'unparsed self bytes'),
    -unknownSelfBytes,
    'unparsed self bytes'
  );
  return {
    kind: 'jemalloc-retained-heap/v1',
    totalRetainedBytes,
    accountedSelfBytes,
    unknownCoverage: {
      selfBytes: safeSum(unknownSelfBytes, unparsedSelfBytes, 'unknown self bytes'),
      absoluteSelfBytes: safeSum(
        unknownAbsoluteSelfBytes,
        Math.abs(unparsedSelfBytes),
        'unknown absolute self bytes'
      ),
      rowCount: unknownRows + omittedRows + (unparsedSelfBytes === 0 ? 0 : 1),
    },
    omittedRows,
    rows,
    coverageEligible: false,
    coverageReason:
      'caller must provide a matched native process/start-ticks/binary envelope; paths alone are claims',
  };
}

function safeSum(left: number, right: number, label: string): number {
  const value = left + right;
  if (!Number.isSafeInteger(value)) throw new Error(`unsafe ${label}`);
  return value;
}

/** Descriptor-verified immutable copy; jeprof never reads the growing original. */
export function snapshotBoundedFile(
  source: string,
  destination: string,
  maxBytes: number,
  label: string
): void {
  let input = -1;
  let output = -1;
  try {
    const inspected = lstatSync(source, { bigint: true });
    if (!inspected.isFile() || inspected.size <= 0n || inspected.size > BigInt(maxBytes))
      throw new Error(`${label} must be a bounded regular file`);
    input = openSync(source, constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW);
    const opened = fstatSync(input, { bigint: true });
    if (
      !opened.isFile() ||
      opened.dev !== inspected.dev ||
      opened.ino !== inspected.ino ||
      opened.size !== inspected.size ||
      opened.mtimeNs !== inspected.mtimeNs ||
      opened.ctimeNs !== inspected.ctimeNs
    )
      throw new Error(`${label} identity changed before snapshot`);
    const size = Number(opened.size);
    // Reused for pinned canary executables too: memory must not scale with file size.
    const bytes = Buffer.alloc(Math.min(size, 64 * 1024));
    output = openSync(
      destination,
      constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL,
      0o600
    );
    let offset = 0;
    while (offset < size) {
      const count = readSync(input, bytes, 0, Math.min(bytes.length, size - offset), null);
      if (!count) break;
      let written = 0;
      while (written < count) {
        const progress = writeSync(output, bytes, written, count - written, null);
        if (progress <= 0) throw new Error(`${label} snapshot made no write progress`);
        written += progress;
      }
      offset += count;
    }
    const completed = fstatSync(input, { bigint: true });
    if (
      offset !== size ||
      !completed.isFile() ||
      completed.dev !== opened.dev ||
      completed.ino !== opened.ino ||
      completed.size !== opened.size ||
      completed.mtimeNs !== opened.mtimeNs ||
      completed.ctimeNs !== opened.ctimeNs
    )
      throw new Error(`${label} identity changed during snapshot`);
    closeSync(output);
    output = -1;
    chmodSync(destination, 0o400);
  } catch (error) {
    throw error instanceof Error && error.message.startsWith(label)
      ? error
      : new Error(`${label} snapshot failed`);
  } finally {
    if (input >= 0) closeSync(input);
    if (output >= 0) closeSync(output);
  }
}

function assertBoundedRegularFile(
  fs: HeapFs,
  path: string,
  label: string,
  maxBytes?: number
): void {
  if (!isAbsolute(path)) throw new Error(`${label} must be an absolute path`);
  let stat: { isFile(): boolean; size: number };
  try {
    stat = fs.stat(path);
  } catch {
    throw new Error(`${label} is unavailable`);
  }
  if (!stat.isFile()) throw new Error(`${label} must be a regular file`);
  if (maxBytes !== undefined && stat.size > maxBytes)
    throw new Error(`${label} exceeds ${maxBytes} bytes`);
}

/**
 * Run jeprof over an explicit trusted-local dump pair.
 *
 * Heap dumps embed process maps; symbolization may read mapped binaries and
 * source files. Callers must never pass downloaded/untrusted profiles here.
 * This adapter fetches nothing and intentionally cannot certify process or
 * binary identity from path claims.
 */
export async function summarizeHeapDiff(options: HeapDiffOptions): Promise<HeapDiffSummary> {
  const fs = options.fs ?? localFs;
  assertBoundedRegularFile(fs, options.binaryPath, 'binary');
  assertBoundedRegularFile(fs, options.jeprofPath, 'jeprof');
  const scratch = fs.mkdtemp(join(tmpdir(), 'elohim-jeprof-'));
  let preserveScratch = false;
  try {
    const beforeSnapshot = join(scratch, 'before.heap');
    const afterSnapshot = join(scratch, 'after.heap');
    fs.snapshot(options.beforeDump, beforeSnapshot, MAX_HEAP_DUMP_BYTES, 'before dump');
    fs.snapshot(options.afterDump, afterSnapshot, MAX_HEAP_DUMP_BYTES, 'after dump');
    const result = await (options.runner ?? runBoundedTool)({
      file: '/usr/bin/perl',
      args: [
        options.jeprofPath,
        '--text',
        '--functions',
        '--inuse_space',
        '--show_bytes',
        '--cum',
        `--base=${beforeSnapshot}`,
        options.binaryPath,
        afterSnapshot,
      ],
      cwd: scratch,
      env: { ...process.env, JEPROF_TMPDIR: scratch },
      timeoutMs: JEPROF_TIMEOUT_MS,
      maxOutputBytes: MAX_JEPROF_OUTPUT_BYTES,
      signal: options.signal,
    });
    if (result.exitCode !== 0) {
      const failure = result.signal ?? `exit ${result.exitCode}`;
      throw new Error(`jeprof failed (${failure})`);
    }
    return parseJeprofText(result.stdout);
  } catch (error) {
    preserveScratch = error instanceof ToolReapUnresolvedError;
    throw error;
  } finally {
    if (!preserveScratch) fs.remove(scratch);
  }
}

function processBirth(pid: number | undefined): string | null {
  if (!pid) return null;
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
    const fields = stat
      .slice(stat.lastIndexOf(')') + 2)
      .trim()
      .split(/\s+/);
    return /^\d+$/.test(fields[19]) && fields[0] !== 'Z' && Number(fields[2]) === pid
      ? fields[19]
      : null;
  } catch {
    return null;
  }
}

/** Read-only confirmation after close; never signal a potentially reused PGID. */
async function stoppedGroup(pgid: number, deadline: bigint): Promise<number> {
  do {
    if (process.hrtime.bigint() >= deadline) throw new ToolReapUnresolvedError();
    let active = false;
    let zombies = 0;
    const entries = readdirSync('/proc');
    if (entries.length > 100_000) throw new ToolReapUnresolvedError();
    for (const entry of entries) {
      if (process.hrtime.bigint() >= deadline) throw new ToolReapUnresolvedError();
      if (!/^\d+$/.test(entry)) continue;
      try {
        const stat = readFileSync(`/proc/${entry}/stat`, 'utf8');
        const fields = stat
          .slice(stat.lastIndexOf(')') + 2)
          .trim()
          .split(/\s+/);
        if (Number(fields[2]) !== pgid) continue;
        // A dead thread-group leader alone is not proof its other threads stopped.
        if (fields[0] === 'Z' && readdirSync(`/proc/${entry}/task`).length === 1) zombies += 1;
        else active = true;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== 'ENOENT') throw new ToolReapUnresolvedError();
      }
    }
    if (!active) return zombies;
    await new Promise(resolve => setTimeout(resolve, 25));
  } while (process.hrtime.bigint() < deadline);
  throw new ToolReapUnresolvedError();
}

/**
 * GNU timeout owns the independent deadline and process group. Promise settlement
 * acknowledges direct-child reap, except ToolReapUnresolvedError. Identity checks
 * narrow the unavoidable procfs-check/signal race; never signal after observed exit.
 */
export const runBoundedTool: ToolRunner = async invocation =>
  new Promise((resolveResult, reject) => {
    const purpose = invocation.purpose ?? 'jeprof';
    const toolLabel = purpose === HEAP_CANARY_WORKER_PURPOSE ? 'heap canary worker' : 'jeprof';
    const maxTimeoutMs = purpose === HEAP_CANARY_WORKER_PURPOSE ? 902_000 : JEPROF_TIMEOUT_MS;
    if (
      !['jeprof', HEAP_CANARY_WORKER_PURPOSE].includes(purpose) ||
      process.platform !== 'linux' ||
      invocation.signal?.aborted ||
      !Number.isSafeInteger(invocation.timeoutMs) ||
      invocation.timeoutMs < 1 ||
      invocation.timeoutMs > maxTimeoutMs ||
      !Number.isSafeInteger(invocation.maxOutputBytes) ||
      invocation.maxOutputBytes < 1 ||
      invocation.maxOutputBytes > MAX_JEPROF_OUTPUT_BYTES
    ) {
      reject(new Error('bounded tool invocation unavailable, cancelled, or outside bounds'));
      return;
    }
    const child = spawn(
      '/usr/bin/timeout',
      ['--signal=KILL', `${invocation.timeoutMs / 1000}s`, invocation.file, ...invocation.args],
      {
        cwd: invocation.cwd,
        env: invocation.env,
        detached: true,
        stdio: ['ignore', 'pipe', 'pipe'],
      }
    );
    let stdout: Buffer<ArrayBufferLike> = Buffer.alloc(0);
    let stderr: Buffer<ArrayBufferLike> = Buffer.alloc(0);
    let outputBytes = 0;
    let settled = false;
    let failure: Error | null = null;
    const birth = processBirth(child.pid);
    let exitObserved = false;
    let reapTimer: NodeJS.Timeout | undefined;
    let cleanupDeadline: bigint | undefined;
    const cleanup = (): void => {
      clearTimeout(timeout);
      if (reapTimer) clearTimeout(reapTimer);
      invocation.signal?.removeEventListener('abort', abort);
    };
    const failBound = (error: Error): void => {
      if (settled || failure) return;
      failure = error;
      cleanupDeadline = process.hrtime.bigint() + BigInt(TOOL_CLEANUP_TIMEOUT_MS) * 1_000_000n;
      clearTimeout(timeout);
      if (!exitObserved && birth && birth === processBirth(child.pid)) {
        try {
          process.kill(-child.pid!, 'SIGKILL');
        } catch {
          /* OS watchdog remains armed. */
        }
      }
      reapTimer = setTimeout(() => {
        if (settled) return;
        settled = true;
        cleanup();
        child.stdout.destroy();
        child.stderr.destroy();
        child.unref();
        reject(new ToolReapUnresolvedError());
      }, TOOL_CLEANUP_TIMEOUT_MS);
    };
    const abort = (): void => failBound(new Error(`${toolLabel} cancelled after child reap`));
    const timeout = setTimeout(
      () => failBound(new Error(`${toolLabel} exceeded ${invocation.timeoutMs}ms`)),
      invocation.timeoutMs
    );
    invocation.signal?.addEventListener('abort', abort, { once: true });
    const append = (
      current: Buffer<ArrayBufferLike>,
      chunk: Buffer<ArrayBufferLike>
    ): Buffer<ArrayBufferLike> => {
      if (failure) return current;
      outputBytes += chunk.length;
      if (outputBytes > invocation.maxOutputBytes) {
        failBound(new Error(`${toolLabel} output exceeds ${invocation.maxOutputBytes} bytes`));
        return current;
      }
      return Buffer.concat([current, chunk]);
    };
    child.stdout.on('data', (chunk: Buffer<ArrayBufferLike>) => {
      stdout = append(stdout, chunk);
    });
    child.stderr.on('data', (chunk: Buffer<ArrayBufferLike>) => {
      stderr = append(stderr, chunk);
    });
    child.once('error', () => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(new Error(`${toolLabel} watchdog spawn failed`));
    });
    child.once('exit', () => {
      exitObserved = true;
    });
    child.once('close', (exitCode, signal) => {
      if (settled) return;
      settled = true;
      cleanup();
      const deadline =
        cleanupDeadline ?? process.hrtime.bigint() + BigInt(TOOL_CLEANUP_TIMEOUT_MS) * 1_000_000n;
      void stoppedGroup(child.pid!, deadline).then(
        unreapedZombies => {
          const stopEvidence: ToolCleanupEvidence = {
            watchdogReaped: true,
            groupWorkStopped: true,
            unreapedZombies,
          };
          if (failure || signal === 'SIGKILL' || exitCode === 124 || exitCode === 137) {
            reject(
              new ToolStoppedError(
                failure?.message ?? `${toolLabel} exceeded ${invocation.timeoutMs}ms or was killed`,
                stopEvidence
              )
            );
            return;
          }
          resolveResult({
            exitCode,
            signal,
            stdout: stdout.toString('utf8'),
            stderr: stderr.toString('utf8'),
            cleanup: stopEvidence,
          });
        },
        () => reject(new ToolReapUnresolvedError())
      );
    });
    if (invocation.signal?.aborted) abort();
  });
