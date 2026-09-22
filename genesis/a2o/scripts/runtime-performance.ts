#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { armDiagnostic, parseDiagnosticControlArgs } from './lib/performance-control.js';
import {
  emptyEvidenceOptions,
  loadExternalEvidence,
  type EvidenceOptions,
} from './lib/performance-evidence.js';
import { summarizeHeapDiff } from './lib/performance-heap.js';
import {
  compareVerdict,
  coverageVerdict,
  COVERAGE_CATEGORIES,
  normalizeCapture,
  renderMarkdown,
  reportVerdict,
  trendVerdict,
  validateVerdict,
  type Verdict,
  type CoverageCategory,
} from './lib/performance-report.js';
import { closeAdminBounded, connectAdminBounded } from './resource-profile.js';

const HEAP_CANARY_COMMAND = 'heap-canary';

const USAGE = `Usage:
  runtime-performance report --input <capture> [--perf /absolute/perf] [evidence options] [--heap-before PATH --heap-after PATH --heap-binary PATH --jeprof PATH] [--json]
  runtime-performance check --input <capture> --require-coverage <all|category,...> [evidence options] [--json]
  runtime-performance compare --baseline <capture> --candidate <capture> --max-regression-percent <N> [--json]
  runtime-performance trend --input <capture> [--input <capture> ...] [--json]
  runtime-performance capture <resource-profile arguments...>
  runtime-performance arm --admin-url ws://127.0.0.1:PORT --family sqlTiming|workflow --nonce UNIQUE --seconds 1..900
  runtime-performance heap-canary --root NEW_PRIVATE_DIR --nonce UNIQUE --lifetime-ms N --observation-ms N --dump-timeout-ms N --holochain PATH --holochain-sha256 HASH --happ PATH --happ-sha256 HASH --jeprof PATH --jeprof-sha256 HASH`;

interface Parsed {
  command: string;
  values: Map<string, string[]>;
  json: boolean;
  forwarded: string[];
}

export function parseArgs(argv: string[]): Parsed {
  const command = argv[0];
  if (
    !command ||
    !['report', 'check', 'compare', 'trend', 'capture', 'arm', HEAP_CANARY_COMMAND].includes(
      command
    )
  )
    throw new Error(USAGE);
  if (command === 'capture' || command === 'arm' || command === HEAP_CANARY_COMMAND)
    return { command, values: new Map(), json: false, forwarded: argv.slice(1) };
  const values = new Map<string, string[]>();
  let json = false;
  for (let index = 1; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === '--json') {
      json = true;
      continue;
    }
    if (
      ![
        '--input',
        '--perf',
        '--require-coverage',
        '--baseline',
        '--candidate',
        '--max-regression-percent',
        '--sqlx-log',
        '--workflow-events',
        '--network-before',
        '--network-after',
        '--heap-before',
        '--heap-after',
        '--heap-binary',
        '--jeprof',
      ].includes(flag)
    )
      throw new Error(`Unknown flag: ${flag}\n${USAGE}`);
    const value = argv[++index];
    if (!value || value.startsWith('--')) throw new Error(`${flag} requires a value`);
    values.set(flag, [...(values.get(flag) ?? []), value]);
  }
  return { command, values, json, forwarded: [] };
}

function evidenceOptions(parsed: Parsed): EvidenceOptions {
  const options = emptyEvidenceOptions();
  options.sqlxLogs = parsed.values.get('--sqlx-log') ?? [];
  options.eventLogs = parsed.values.get('--workflow-events') ?? [];
  const before = parsed.values.get('--network-before') ?? [];
  const after = parsed.values.get('--network-after') ?? [];
  if (before.length > 1 || after.length > 1)
    throw new Error('network evidence paths may be supplied at most once');
  options.networkBefore = before[0];
  options.networkAfter = after[0];
  return options;
}

export function heapTuple(
  parsed: Parsed
): { beforeDump: string; afterDump: string; binaryPath: string; jeprofPath: string } | null {
  const flags = ['--heap-before', '--heap-after', '--heap-binary', '--jeprof'] as const;
  const values = flags.map(flag => parsed.values.get(flag) ?? []);
  if (values.every(entries => entries.length === 0)) return null;
  if (values.some(entries => entries.length !== 1))
    throw new Error(
      'heap evidence requires each of --heap-before, --heap-after, --heap-binary, and --jeprof exactly once'
    );
  const [beforeDump, afterDump, binaryPath, jeprofPath] = values.map(entries =>
    resolve(entries[0])
  );
  if (beforeDump === afterDump)
    throw new Error('--heap-before and --heap-after must be different files');
  return { beforeDump, afterDump, binaryPath, jeprofPath };
}

async function normalizeWithEvidence(parsed: Parsed) {
  const run = normalizeCapture(one(parsed, '--input'), parsed.values.get('--perf')?.[0]);
  const options = evidenceOptions(parsed);
  if (options.sqlxLogs.length || options.eventLogs.length || options.networkBefore) {
    if (!run.window) throw new Error('external evidence requires a valid capture wall window');
    run.externalEvidence = loadExternalEvidence(options, run.window);
  }
  const heap = heapTuple(parsed);
  if (heap) {
    const summary = await summarizeHeapDiff(heap);
    run.heapSummary = {
      ...summary,
      coverageEligible: false,
      coverageReason:
        'trusted local dump paths do not prove capture pid/start-ticks/executable identity; supplied binary identity is unverified',
    };
  }
  return run;
}

function one(parsed: Parsed, flag: string): string {
  const values = parsed.values.get(flag) ?? [];
  if (values.length !== 1) throw new Error(`${flag} must be supplied exactly once`);
  return values[0];
}

export function parseCoverage(raw: string): CoverageCategory[] {
  const values = raw === 'all' ? [...COVERAGE_CATEGORIES] : raw.split(',');
  if (!values.length || values.some(value => !value))
    throw new Error('--require-coverage expects all or a comma-separated category list');
  if (new Set(values).size !== values.length)
    throw new Error('--require-coverage categories must not be duplicated');
  const unknown = values.filter(
    (value): value is string => !COVERAGE_CATEGORIES.includes(value as CoverageCategory)
  );
  if (unknown.length) throw new Error(`Unknown coverage category: ${unknown.join(', ')}`);
  return values as CoverageCategory[];
}

export async function execute(argv: string[]): Promise<{ verdict?: Verdict; code: number }> {
  const parsed = parseArgs(argv);
  if (parsed.command === HEAP_CANARY_COMMAND) {
    const { parseHeapCanaryArgs, runIsolatedHeapCanary } =
      await import('./lib/performance-canary-launcher.js');
    const witness = await runIsolatedHeapCanary(parseHeapCanaryArgs(parsed.forwarded));
    process.stdout.write(`${JSON.stringify(witness)}\n`);
    // Passing proves only the isolated lifecycle, not full attribution coverage.
    return { code: witness.outcome === 'passed' ? 0 : 2 };
  }
  if (parsed.command === 'arm') {
    const { url, family, nonce, seconds } = parseDiagnosticControlArgs(parsed.forwarded);
    const admin = await connectAdminBounded(url);
    try {
      const receipt = await armDiagnostic(
        async (operation, payload, timeout) => admin._requester(operation)(payload, timeout),
        family,
        nonce,
        seconds
      );
      process.stdout.write(`${JSON.stringify(receipt)}\n`);
      return { code: receipt.admitted ? 0 : 2 };
    } finally {
      await closeAdminBounded(admin);
    }
  }
  if (parsed.command === 'capture') {
    const result = spawnSync(
      process.execPath,
      ['--import', 'tsx', resolve('scripts/resource-profile.ts'), ...parsed.forwarded],
      { stdio: 'inherit' }
    );
    return { code: result.status ?? 2 };
  }
  let verdict: Verdict;
  if (parsed.command === 'report') verdict = reportVerdict(await normalizeWithEvidence(parsed));
  else if (parsed.command === 'check') {
    verdict = coverageVerdict(
      await normalizeWithEvidence(parsed),
      parseCoverage(one(parsed, '--require-coverage'))
    );
  } else if (parsed.command === 'compare') {
    if (
      [...parsed.values].some(
        ([flag]) =>
          flag.startsWith('--sqlx-') ||
          flag.startsWith('--workflow-') ||
          flag.startsWith('--network-') ||
          flag.startsWith('--heap-') ||
          flag === '--jeprof'
      )
    )
      throw new Error('external evidence options are supported only by report and check');
    const threshold = Number(one(parsed, '--max-regression-percent'));
    if (!Number.isFinite(threshold) || threshold < 0)
      throw new Error('--max-regression-percent expects a non-negative number');
    verdict = compareVerdict(
      normalizeCapture(one(parsed, '--baseline')),
      normalizeCapture(one(parsed, '--candidate')),
      threshold
    );
  } else {
    if (
      [...parsed.values].some(
        ([flag]) =>
          flag.startsWith('--sqlx-') ||
          flag.startsWith('--workflow-') ||
          flag.startsWith('--network-') ||
          flag.startsWith('--heap-') ||
          flag === '--jeprof'
      )
    )
      throw new Error('external evidence options are supported only by report and check');
    const inputs = parsed.values.get('--input') ?? [];
    if (!inputs.length || inputs.length > 32)
      throw new Error('trend requires 1..32 --input values');
    verdict = trendVerdict(inputs.map(input => normalizeCapture(input)));
  }
  validateVerdict(verdict);
  process.stdout.write(
    parsed.json ? `${JSON.stringify(verdict, null, 2)}\n` : renderMarkdown(verdict)
  );
  return {
    verdict,
    code:
      parsed.command === 'compare' || parsed.command === 'check'
        ? verdict.decision.type === 'refuse'
          ? 1
          : verdict.decision.type === 'refer'
            ? 2
            : 0
        : 0,
  };
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href;
if (isMain) {
  try {
    process.exitCode = (await execute(process.argv.slice(2))).code;
  } catch (error) {
    process.stderr.write(`${String(error)}\n${USAGE}\n`);
    process.exitCode = 2;
  }
}
