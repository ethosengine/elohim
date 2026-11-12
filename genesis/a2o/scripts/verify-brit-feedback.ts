/** Exercise the installed pair in a disposable private store; never promote a candidate. */
import { spawnSync } from 'node:child_process';
import { constants, accessSync, mkdtempSync, realpathSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { resolveBinary } from './lib/household-attestation.js';

import type { CheckWitness } from './lib/performance-report.js';

const CHECK = 'private-feedback-loop';
const LAUNCHER = 'bin-launch';
const REPO = fileURLToPath(new URL('../../../', import.meta.url));
const OVERRIDES = ['EPR_BIN', 'BRIT_BIN', 'EPR_PIN_CID', 'BRIT_PIN_CID'];

interface Options {
  epr?: string;
  brit?: string;
  resolver?: string;
  policyRoot?: string;
  env?: NodeJS.ProcessEnv;
}
interface Selection {
  path: string;
  source: 'current' | 'explicit-candidate';
  cid: string | null;
}
interface Resolver {
  path: string;
  source: 'explicit-resolver' | 'ambient-bootstrap' | 'path-executable' | 'native-launcher';
}
class ProbeFailure extends Error {
  constructor(
    message: string,
    readonly outcome: 'failed' | 'skipped' = 'failed'
  ) {
    super(message);
  }
}

function executable(path: string | null): string {
  if (!path)
    throw new ProbeFailure(
      'Required executable is unavailable; coverage is unmeasured.',
      'skipped'
    );
  try {
    accessSync(path, constants.X_OK);
    return realpathSync(path);
  } catch {
    throw new ProbeFailure(
      'Required executable is unavailable; coverage is unmeasured.',
      'skipped'
    );
  }
}

function invoke(
  binary: string,
  args: string[],
  root: string,
  env: NodeJS.ProcessEnv,
  timeout = 10000
) {
  const result = spawnSync(binary, args, {
    cwd: root,
    env,
    encoding: 'utf8',
    timeout,
    maxBuffer: 1024 * 1024,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  if (result.error)
    throw new ProbeFailure('Executable could not complete within the probe boundary.', 'skipped');
  return result;
}

function decode(
  binary: string,
  args: string[],
  root: string,
  env: NodeJS.ProcessEnv,
  resolvingCurrent = false
): Record<string, unknown> {
  const result = invoke(binary, args, root, env);
  if (resolvingCurrent && result.status !== 0 && /^REFUSED head-absent:/m.test(result.stderr)) {
    throw new ProbeFailure(
      'Installed current is unavailable; establish its native release before measuring feedback.',
      'skipped'
    );
  }
  if (
    resolvingCurrent &&
    basename(binary) === LAUNCHER &&
    result.status === 127 &&
    /^REFUSED launcher-name-invalid:/m.test(result.stderr)
  ) {
    throw new ProbeFailure(
      'Installed launcher has no read-only resolution command; supply --resolver with a direct EPR executable.',
      'skipped'
    );
  }
  if (result.status !== 0)
    throw new ProbeFailure(
      'The selected executable refused the required native feedback interface.'
    );
  try {
    const value: unknown = JSON.parse(result.stdout);
    if (!value || typeof value !== 'object' || Array.isArray(value))
      throw new Error('object required');
    return value as Record<string, unknown>;
  } catch {
    throw new ProbeFailure('The selected executable returned an invalid native response.');
  }
}

function selectPair(
  options: Options,
  root: string,
  env: NodeJS.ProcessEnv,
  resolver?: Resolver
): Record<'epr' | 'brit', Selection> {
  if (options.epr || options.brit) {
    if (!options.epr || !options.brit)
      throw new ProbeFailure('Candidate selection requires both --epr and --brit.');
    return {
      epr: { path: executable(options.epr), source: 'explicit-candidate', cid: null },
      brit: { path: executable(options.brit), source: 'explicit-candidate', cid: null },
    };
  }
  if (!resolver) throw new ProbeFailure('A direct EPR resolver is required.', 'skipped');
  // Invoke the launcher's own inspection mode, never its session-pinning epr alias.
  const current = (tool: string): Selection => {
    const value = decode(
      resolver.path,
      [
        ...(basename(resolver.path) === LAUNCHER ? [] : ['bin']),
        'resolve',
        '--tool',
        tool,
        '--current',
        '--json',
      ],
      root,
      env,
      true
    );
    if (
      value.source !== 'current' ||
      typeof value.path !== 'string' ||
      typeof value.cid !== 'string'
    ) {
      throw new ProbeFailure('Native resolution did not identify an installed current release.');
    }
    return { path: executable(value.path), source: 'current', cid: value.cid };
  };
  return { epr: current('epr'), brit: current('brit') };
}

function selectResolver(options: Options, env: NodeJS.ProcessEnv): Resolver {
  const path = executable(
    options.resolver ?? env.EPR_BIN ?? resolveBinary('epr', undefined, env.PATH)
  );
  if (basename(path) === LAUNCHER) return { path, source: 'native-launcher' };
  return {
    path,
    source: options.resolver
      ? 'explicit-resolver'
      : env.EPR_BIN
        ? 'ambient-bootstrap'
        : 'path-executable',
  };
}

function qualify(
  pair: Record<'epr' | 'brit', Selection>,
  policyRoot: string,
  root: string,
  env: NodeJS.ProcessEnv
): CheckWitness {
  const result = invoke(
    pair.epr.path,
    ['feedback', 'qualify', '--brit', pair.brit.path, '--root', policyRoot, '--json'],
    root,
    env,
    180000
  );
  let value: unknown;
  try {
    value = JSON.parse(result.stdout);
  } catch {
    throw new ProbeFailure(
      'The selected executable refused the required native feedback interface.'
    );
  }
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new ProbeFailure('Native qualification returned an invalid witness.');
  const witness = value as CheckWitness;
  const observed = witness.observed as
    | { mode?: string; pairQualified?: boolean; fixtureOnly?: boolean }
    | undefined;
  if (
    witness.checkId !== CHECK ||
    !['passed', 'failed', 'skipped'].includes(witness.outcome) ||
    typeof witness.summary !== 'string' ||
    observed?.mode !== 'pair' ||
    observed.fixtureOnly !== true ||
    (witness.outcome === 'passed' && (result.status !== 0 || observed.pairQualified !== true)) ||
    (witness.outcome !== 'passed' && (result.status === 0 || observed.pairQualified !== false))
  ) {
    throw new ProbeFailure('Native qualification returned an inconsistent witness.');
  }
  return witness;
}

export async function verifyBritFeedback(options: Options = {}): Promise<CheckWitness> {
  return await Promise.resolve(verifySelected(options));
}

function verifySelected(options: Options): CheckWitness {
  const supplied = options.env ?? process.env;
  const ignoredOverrides = OVERRIDES.filter(key => Boolean(supplied[key]));
  const env: NodeJS.ProcessEnv = { ...supplied, EPR_SESSION_LAZY: '1' };
  for (const key of OVERRIDES) delete env[key];
  const root = mkdtempSync(join(tmpdir(), 'brit-feedback-probe-'));
  let pair: Record<'epr' | 'brit', Selection> | undefined;
  let resolver: Resolver | undefined;
  try {
    if (!options.epr && !options.brit) resolver = selectResolver(options, supplied);
    pair = selectPair(options, root, env, resolver);
    const witness = qualify(pair, options.policyRoot ?? REPO, root, env);
    return {
      ...witness,
      observed: {
        ...(witness.observed as Record<string, unknown>),
        pair,
        resolver,
        ignoredOverrides,
      },
    };
  } catch (error) {
    return {
      checkId: CHECK,
      outcome: error instanceof ProbeFailure ? error.outcome : 'failed',
      summary:
        error instanceof ProbeFailure
          ? error.message
          : 'Feedback contract fixture did not satisfy its assertion.',
      observed: { pair, resolver, ignoredOverrides, fixtureOnly: true },
    };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

async function main(args: string[]) {
  const options: Options = {};
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--help') {
      console.log(
        'verify-brit-feedback.ts [--epr PATH --brit PATH] [--resolver PATH] [--policy-root DIR]\nDefault: native installed current pair through read-only launcher resolution or direct EPR. Ambient EPR_BIN is bootstrap-only. An older launcher requires --resolver. Explicit pair: candidate fixture only. Exit 0 passed, 1 failed, 2 unmeasured.'
      );
      return;
    }
    if (!['--epr', '--brit', '--resolver', '--policy-root'].includes(arg) || !args[i + 1])
      throw new Error('Expected --epr PATH --brit PATH, --resolver PATH or --policy-root DIR.');
    const value = resolve(args[++i]);
    if (arg === '--epr') options.epr = value;
    else if (arg === '--brit') options.brit = value;
    else if (arg === '--resolver') options.resolver = value;
    else options.policyRoot = value;
  }
  const witness = await verifyBritFeedback(options);
  console.log(JSON.stringify(witness));
  process.exitCode = witness.outcome === 'passed' ? 0 : witness.outcome === 'failed' ? 1 : 2;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  void main(process.argv.slice(2));
