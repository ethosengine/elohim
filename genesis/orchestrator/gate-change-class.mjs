import { spawnSync } from 'node:child_process';
import { SIBLING_REUSE_GOVERNANCE_PATHS } from './gate-evidence.mjs';

export const CHANGE_CLASSES = Object.freeze(['0', 'low', 'channel', 'governance', 'high']);
export const CLASS_EFFECTS = Object.freeze({
  0: Object.freeze({
    reexec: false,
    siblingVeto: false,
    chainBreak: false,
    reuseOff: [],
  }),
  low: Object.freeze({
    reexec: false,
    siblingVeto: false,
    chainBreak: false,
    reuseOff: [],
  }),
  channel: Object.freeze({
    reexec: false,
    siblingVeto: true,
    chainBreak: false,
    reuseOff: [],
  }),
  governance: Object.freeze({
    reexec: true,
    siblingVeto: true,
    chainBreak: true,
    reuseOff: [],
  }),
  high: Object.freeze({
    reexec: false,
    siblingVeto: false,
    chainBreak: true,
    reuseOff: [],
  }),
});

export const GOVERNANCE_PATHS = Object.freeze([
  '.husky/**',
  '.ci-ignore',
  '.epr-meta/manifest.md',
  '.claude/scripts/ci-ignore-projector.py',
  'genesis/orchestrator/gate-*.mjs',
  'genesis/orchestrator/gate-runner.mjs',
  'genesis/orchestrator/run-local-gate.sh',
  'genesis/orchestrator/rust-style.sh',
  'genesis/orchestrator/rust-style/**',
  'genesis/orchestrator/scripts/brit-attest.mjs',
  'genesis/orchestrator/scripts/brit-helper.sh',
  'genesis/orchestrator/scripts/t2-receipt.sh',
  'genesis/agentic/bin/pool-lib.sh',
  'genesis/agentic/bin/cargo-pool',
  'genesis/agentic/bin/berth',
  'genesis/orchestrator/commit-tag-parser.mjs',
  '.claude/epr-meta/measures.yaml',
  'elohim/eprfs/epr-cli/src/peer.rs',
  'genesis/orchestrator/ci-ignore.mjs',
  'genesis/orchestrator/manifest-utils.mjs',
  'genesis/orchestrator/pipeline-registry.mjs',
  'genesis/orchestrator/graph-walker.mjs',
  'genesis/orchestrator/cargo-affected.mjs',
]);

function matches(path, rule) {
  if (rule.endsWith('/**')) return path.startsWith(rule.slice(0, -2));
  if (rule.includes('*')) {
    const [before, after] = rule.split('*');
    return path.startsWith(before) && path.endsWith(after);
  }
  return path === rule;
}

export function classifyPath(path) {
  if (path === 'genesis/agentic/pool-policy.json') return { path, deferred: 'pool-policy' };
  if (
    GOVERNANCE_PATHS.some(
      rule =>
        matches(path, rule) &&
        !(rule === 'genesis/orchestrator/gate-*.mjs' && path.endsWith('.test.mjs')),
    )
  ) {
    return { path, class: 'governance', why: 'governance path' };
  }
  if (SIBLING_REUSE_GOVERNANCE_PATHS.some(rule => matches(path, rule))) {
    return { path, class: 'channel', why: 'sibling reuse fence' };
  }
  if (
    path.startsWith('elohim/holochain/dna/') &&
    (path.split('/').some(segment => segment.includes('integrity')) ||
      /^elohim\/holochain\/dna\/[^/]+\/dna\.yaml$/.test(path))
  )
    return { path, class: 'high', why: 'DNA hash path' };
  return { path, class: '0', why: 'ordinary path' };
}

function clean(value) {
  if (Array.isArray(value)) return value.map(clean);
  if (value && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value)
        .filter(([key]) => !key.startsWith('_'))
        .map(([k, v]) => [k, clean(v)]),
    );
  }
  return value;
}

function changedLeaves(a, b, prefix = '') {
  if (JSON.stringify(a) === JSON.stringify(b)) return [];
  if (
    a &&
    b &&
    typeof a === 'object' &&
    typeof b === 'object' &&
    !Array.isArray(a) &&
    !Array.isArray(b)
  ) {
    return [...new Set([...Object.keys(a), ...Object.keys(b)])].flatMap(key =>
      changedLeaves(a[key], b[key], prefix ? `${prefix}.${key}` : key),
    );
  }
  return [prefix];
}

export function classifyPoolPolicy(baseText, headText) {
  let base;
  let head;
  try {
    base = clean(JSON.parse(baseText));
    head = clean(JSON.parse(headText));
  } catch {
    return [
      {
        path: 'genesis/agentic/pool-policy.json',
        class: 'governance',
        why: 'pool-policy unreadable',
      },
    ];
  }
  const leaves = changedLeaves(base, head);
  if (leaves.length === 0)
    return [
      {
        path: 'genesis/agentic/pool-policy.json',
        class: '0',
        why: 'comment keys only',
      },
    ];
  return leaves.map(key => {
    if (key.startsWith('cargo_env_overrides.')) {
      const project = key.split('.')[1];
      return {
        path: 'genesis/agentic/pool-policy.json',
        class: 'low',
        why: key,
        reuseOff: [project],
      };
    }
    if (
      key === 'gate_reuse' ||
      key.startsWith('gate_reuse.') ||
      key === 'gate_narrow' ||
      key.startsWith('gate_narrow.')
    ) {
      return {
        path: 'genesis/agentic/pool-policy.json',
        class: 'governance',
        why: key,
      };
    }
    return {
      path: 'genesis/agentic/pool-policy.json',
      class: 'channel',
      why: key,
    };
  });
}

export function classifyChange({ paths, poolPolicy } = {}) {
  const rows = [];
  for (const path of paths || []) {
    const row = classifyPath(path);
    if (row.deferred === 'pool-policy')
      rows.push(...classifyPoolPolicy(poolPolicy?.base, poolPolicy?.head));
    else rows.push(row);
  }
  if (rows.length === 0) rows.push({ path: '', class: '0', why: 'no changed paths' });
  const top = rows.reduce(
    (best, row) =>
      CHANGE_CLASSES.indexOf(row.class) > CHANGE_CLASSES.indexOf(best.class) ? row : best,
    rows[0],
  );
  const effects = rows.reduce(
    (out, row) => {
      const effect = CLASS_EFFECTS[row.class];
      out.reexec ||= effect.reexec;
      out.siblingVeto ||= effect.siblingVeto;
      out.chainBreak ||= effect.chainBreak;
      out.reuseOff.push(...(row.reuseOff || effect.reuseOff));
      return out;
    },
    { reexec: false, siblingVeto: false, chainBreak: false, reuseOff: [] },
  );
  effects.reuseOff = [...new Set(effects.reuseOff)];
  return {
    class: top.class,
    ...effects,
    paths: rows,
    why: `${top.path}: ${top.why}`,
  };
}

export function changeClass({ root, head = 'HEAD', spawn = spawnSync }) {
  const git = args => spawn('git', args, { cwd: root, encoding: 'utf8' });
  let base = git(['merge-base', 'origin/dev', head]);
  if (base.status !== 0) base = git(['merge-base', 'dev', head]);
  if (base.status !== 0 || !base.stdout.trim()) {
    return {
      class: 'governance',
      ...CLASS_EFFECTS.governance,
      base: null,
      why: 'integration merge base unavailable',
      cure: 'git fetch origin',
    };
  }
  const baseOid = base.stdout.trim();
  const diff = git(['diff', '--no-renames', '--name-only', baseOid, head]);
  if (diff.status !== 0)
    return {
      class: 'governance',
      ...CLASS_EFFECTS.governance,
      base: baseOid,
      why: 'governance diff unavailable',
    };
  const paths = diff.stdout.split('\n').filter(Boolean);
  let poolPolicy;
  if (paths.includes('genesis/agentic/pool-policy.json')) {
    const before = git(['show', `${baseOid}:genesis/agentic/pool-policy.json`]);
    const after = git(['show', `${head}:genesis/agentic/pool-policy.json`]);
    poolPolicy = {
      base: before.status === 0 ? before.stdout : '',
      head: after.status === 0 ? after.stdout : '',
    };
  }
  return { ...classifyChange({ paths, poolPolicy }), base: baseOid };
}

if (process.argv[1]?.endsWith('gate-change-class.mjs')) {
  const args = process.argv.slice(2);
  const headIndex = args.indexOf('--head');
  const head = headIndex >= 0 ? args[headIndex + 1] : 'HEAD';
  const rootIndex = args.indexOf('--root');
  let root = rootIndex >= 0 ? args[rootIndex + 1] : null;
  if (!root) {
    const discovered = spawnSync('git', ['rev-parse', '--show-toplevel'], {
      cwd: process.cwd(),
      encoding: 'utf8',
    });
    root = discovered.status === 0 ? discovered.stdout.trim() : process.cwd();
  }
  const result = changeClass({ root, head });
  if (args.includes('--json')) process.stdout.write(`${JSON.stringify(result)}\n`);
  else process.stdout.write(`change class: ${result.class} (${result.why})\n`);
}
