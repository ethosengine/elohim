import assert from 'node:assert/strict';
import {
  chmodSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';

import { repository } from './devices-share-one-berth.fixture.js';

export const ENTRIES = [
  'genesis/orchestrator/gate-runner.mjs',
  'genesis/orchestrator/gate-evidence.mjs',
  'genesis/orchestrator/gate-landing-base.mjs',
  'genesis/orchestrator/gate-change-class.mjs',
  'genesis/orchestrator/gate-change-id.mjs',
  'genesis/orchestrator/gate-landing-markers.mjs',
  'genesis/orchestrator/ci-ignore.mjs',
  'genesis/orchestrator/gate-live.mjs',
  'genesis/orchestrator/gate-tap.mjs',
  'genesis/orchestrator/run-local-gate.sh',
  'genesis/orchestrator/scripts/t2-receipt.sh',
] as const;

function repoPath(from: string, specifier: string): string {
  const absolute = resolve(repository, dirname(from), specifier);
  const path = relative(repository, absolute).split('\\').join('/');
  assert.ok(
    !path.startsWith('../'),
    `orchestrator import leaves the repository: ${from} -> ${specifier}`
  );
  return path;
}

function relativeImports(path: string, source: string): string[] {
  const imports = new Set<string>();
  let offset = 0;
  for (const line of source.split('\n')) {
    const trimmed = line.trimStart();
    const keyword = trimmed.startsWith('import ') ? 'import' : 'export';
    if (!trimmed.startsWith(`${keyword} `)) {
      offset += line.length + 1;
      continue;
    }
    const start = offset + line.indexOf(keyword) + keyword.length;
    const end = source.indexOf(';', start);
    const statement = source.slice(start, end < 0 ? source.indexOf('\n', start) : end);
    const specifier = /(?:from\s+)?['"](\.{1,2}\/[^'"]+)['"]/.exec(statement);
    if (specifier) imports.add(repoPath(path, specifier[1]));
    offset += line.length + 1;
  }
  for (const match of source.matchAll(/\bimport\(\s*['"](\.{1,2}\/[^'"]+)['"]\s*\)/g)) {
    const line = source.slice(source.lastIndexOf('\n', match.index) + 1, match.index);
    if (/^\s*\*/.test(line) || line.includes('/**')) continue;
    imports.add(repoPath(path, match[1]));
  }
  return [...imports];
}

function shellTargets(path: string, source: string): string[] {
  if (!path.endsWith('run-local-gate.sh')) return [];
  const targets = new Set<string>();
  const command = /(?:source|bash|node)\s+["']?\$repo_root\/([^"'\s;|]+)/;
  for (const line of source.split('\n')) {
    const match = command.exec(line);
    if (match) targets.add(match[1]);
  }
  return [...targets];
}

/** Follow every repository-relative module and executable read needed by the seeded gate. */
export function orchestratorClosure(entries: readonly string[]): string[] {
  const seen = new Set<string>();
  const queue = [...entries];
  while (queue.length > 0) {
    const path = queue.shift() as string;
    if (seen.has(path)) continue;
    const absolute = join(repository, path);
    assert.ok(existsSync(absolute), `unseeded orchestrator target ${path} (S2 must land it first)`);
    seen.add(path);
    const source = readFileSync(absolute, 'utf8');
    for (const target of [...relativeImports(path, source), ...shellTargets(path, source)]) {
      assert.ok(
        existsSync(join(repository, target)),
        `unseeded target ${target} reached from ${path}`
      );
      if (!seen.has(target)) queue.push(target);
    }
  }
  return [...seen].sort((a, b) => {
    if (a < b) return -1;
    if (a > b) return 1;
    return 0;
  });
}

function copy(seed: string, path: string): void {
  mkdirSync(dirname(join(seed, path)), { recursive: true });
  copyFileSync(join(repository, path), join(seed, path));
}

function writeJson(path: string, value: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

export function seedGateClosure(
  seed: string,
  options: {
    reuse: string[];
    channel?: Partial<{ max_skew_s: number; freshness_s: number }>;
  }
): void {
  for (const path of orchestratorClosure(ENTRIES)) copy(seed, path);
  for (const path of ['.husky/pre-push', '.husky/pre-push.bash', '.husky/commit-msg']) {
    if (existsSync(join(repository, path))) copy(seed, path);
  }

  const real = JSON.parse(
    readFileSync(join(repository, 'genesis/agentic/pool-policy.json'), 'utf8')
  ) as {
    gate_reuse: Record<string, unknown>;
    berth: { channel: Record<string, unknown> };
    [key: string]: unknown;
  };
  writeJson(join(seed, 'genesis/agentic/pool-policy.json'), {
    ...real,
    gate_reuse: { ...real.gate_reuse, projects: options.reuse },
    berth: {
      ...real.berth,
      channel: { ...real.berth.channel, ...options.channel },
    },
  });

  writeJson(join(seed, 'app/build-manifest.json'), {
    manifestVersion: '1.0',
    pipeline: 'fixture',
    description: 'landing queue fixture',
    steps: {
      'gate-app': {
        description: 'fixture app gate',
        inputs: { sources: ['app/**'], buildProcess: [] },
        outputs: { artifacts: [], verify: null },
        depends: [],
        executor: { stage: 'S', function: null },
      },
    },
    gate: {
      projects: {
        app: {
          dir: 'app',
          steps: ['gate-app'],
          run: { kind: 'just', recipe: 'gate' },
        },
      },
    },
  });
  mkdirSync(join(seed, 'app'), { recursive: true });
  writeFileSync(
    join(seed, 'app/justfile'),
    `gate:\n    printf '%s %s %s\\n' "$(date +%s%N)" "$(git rev-parse HEAD:app)" "$PWD" >> "$(git rev-parse --path-format=absolute --git-common-dir)/gate-calls.log"\n    test ! -e left.txt -o ! -e right.txt\n`
  );
  writeFileSync(join(seed, 'app/README.md'), '# fixture app\n');
  writeFileSync(
    join(seed, 'genesis/orchestrator/rust-style.sh'),
    '#!/usr/bin/env bash\necho "[fixture] rust-style: nothing to measure"\nexit 0\n'
  );
  chmodSync(join(seed, 'genesis/orchestrator/rust-style.sh'), 0o755);
}

const BRIT_STUB = String.raw`#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
const args = process.argv.slice(2);
if (args[0] !== 'build' || args[1] !== 'fingerprint') process.exit(2);
const manifest = args[2];
const value = key => args[args.indexOf(key) + 1];
const step = value('--step');
const commit = value('--commit');
const root = manifest.replace(/\/app\/build-manifest\.json$/, '');
const git = spawnSync('git', ['rev-parse', commit + ':app'], { cwd: root, encoding: 'utf8' });
if (git.status !== 0) { process.stderr.write(git.stderr); process.exit(git.status ?? 1); }
process.stdout.write(JSON.stringify({ fingerprints: [{ step, fingerprint: git.stdout.trim() }] }));
`;

const BUILD_REF_STUB = String.raw`#!/usr/bin/env node
import { appendFileSync, existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
const args = process.argv.slice(2);
const repoAt = args.indexOf('--repo');
const repo = repoAt >= 0 ? args[repoAt + 1] : process.cwd();
if (repoAt >= 0) args.splice(repoAt, 2);
const git = spawnSync('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], { cwd: repo, encoding: 'utf8' });
if (git.status !== 0) process.exit(git.status ?? 1);
const log = join(git.stdout.trim(), 'brit-notes.jsonl');
const agentId = process.env.BRIT_STUB_AGENT || 'a'.repeat(64);
const value = key => args[args.indexOf(key) + 1];
const rows = () => existsSync(log) ? readFileSync(log, 'utf8').split('\n').filter(Boolean).map(JSON.parse) : [];
if (args[0] === 'signer') {
  process.stdout.write(JSON.stringify({ agentId }));
} else if (args[0] === 'build' && args[1] === 'put') {
  const row = { step: value('--step'), commit: value('--commit'), inputsHash: value('--inputs-hash'), success: value('--success') === 'true', agentId, builtAt: new Date().toISOString() };
  appendFileSync(log, JSON.stringify(row) + '\n');
  process.stdout.write(JSON.stringify({ agentId }));
} else if (args[0] === 'build' && args[1] === 'get') {
  const row = rows().filter(row => row.step === value('--step') && row.commit === value('--commit')).pop();
  if (row) process.stdout.write(JSON.stringify({ inputsHash: row.inputsHash, success: row.success, agentId: row.agentId, builtAt: row.builtAt }));
} else if (args[0] === 'build' && args[1] === 'find') {
  const found = rows().filter(row => row.step === value('--step') && row.inputsHash === value('--inputs-hash')).map(row => ({ commit: row.commit, attestation: { success: row.success, agentId: row.agentId, builtAt: row.builtAt } }));
  process.stdout.write(JSON.stringify(found));
} else process.exit(2);
`;

const RAKIA_STUB = `#!/usr/bin/env node
const args = process.argv.slice(2);
if (args[0] !== 'affected') process.exit(2);
const value = key => args[args.indexOf(key) + 1];
const files = (value('--files') || '').split(',').filter(Boolean);
const affected = files.some(path => path.startsWith('app/')) ? [{ qualified_name: 'fixture:gate-app', affected_by: files.filter(path => path.startsWith('app/')).map(path => ({ kind: 'changedFile', path })) }] : [];
process.stdout.write(JSON.stringify({ affected }));
`;

/** Write deterministic command-line stand-ins in exactly the shapes the gate parsers consume. */
export function stubTools(base: string): {
  BRIT_BIN: string;
  BRIT_BUILD_REF_BIN: string;
  RAKIA_BIN: string;
} {
  const bin = join(base, 'bin');
  mkdirSync(bin, { recursive: true });
  const tools = {
    BRIT_BIN: join(bin, 'brit'),
    BRIT_BUILD_REF_BIN: join(bin, 'brit-build-ref'),
    RAKIA_BIN: join(bin, 'rakia'),
  };
  for (const [path, source] of [
    [tools.BRIT_BIN, BRIT_STUB],
    [tools.BRIT_BUILD_REF_BIN, BUILD_REF_STUB],
    [tools.RAKIA_BIN, RAKIA_STUB],
  ] as const) {
    writeFileSync(path, source);
    chmodSync(path, 0o755);
  }
  return tools;
}
