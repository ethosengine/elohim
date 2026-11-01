import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { CHANGE_ID_RE } from './commit-tag-parser.mjs';
import { CHANGE_CLASSES } from './gate-change-class.mjs';

export const SKIPPED_WHY_CODES = Object.freeze([
  'dormant',
  'governance-class',
  'cap',
  'bytes not on this device',
  'withdrawn',
  'landed',
]);

const OID_RE = /^[0-9a-f]{40}([0-9a-f]{24})?$/;

export function validRow(row, me = {}) {
  return Boolean(
    row &&
    /^bafy[a-z2-7]+$/.test(row.intent) &&
    CHANGE_ID_RE.test(row.change) &&
    OID_RE.test(row.tree) &&
    OID_RE.test(row.base) &&
    ((row.device === null && row.own === true && me.device === null) ||
      (typeof row.device === 'string' && row.device.startsWith('did:key:'))) &&
    Number.isFinite(row.raisedAt) &&
    Number.isInteger(row.raisedAt),
  );
}

export function precedesIntent(a, b, maxSkewS) {
  if (Math.abs(a.raisedAt - b.raisedAt) > maxSkewS * 1000) return a.raisedAt < b.raisedAt;
  const deviceA = a.device || '';
  const deviceB = b.device || '';
  if (deviceA !== deviceB) return deviceA < deviceB;
  return a.intent < b.intent;
}

function baseResult(dev, additions = {}) {
  return {
    base: { ...dev },
    chain: [],
    chainLen: 0,
    skipped: [],
    degraded: false,
    fromDev: true,
    predictedBase: dev.tree,
    stopped: { at: null, why: 'end of queue' },
    landedObserved: [],
    lines: [],
    ...additions,
  };
}

export function pickLandingBase({
  dev,
  queue,
  me,
  settlement,
  heads = new Map(),
  now = Date.now(),
  git,
  cap = 4,
}) {
  const lines = [];
  const line = text => lines.push(`[landing] ${text}`);
  if (!queue || queue.error) {
    const why = queue?.why || queue?.error || 'invalid queue';
    line(`queue unreadable (${why}) — gating on the dev base`);
    return baseResult(dev, {
      degraded: true,
      lines,
      stopped: { at: null, why: 'queue unreadable' },
    });
  }
  if (me.change === null) {
    line(
      'no Change-Id on the pushed tip — no chain position; gating on the dev base (cure: git commit --amend)',
    );
    return baseResult(dev, {
      degraded: queue.siblingsRead === false,
      lines,
      stopped: { at: null, why: 'no Change-Id' },
    });
  }
  const degraded = queue.siblingsRead === false;
  if (degraded) line(`sibling queue degraded (${queue.siblingsWhy || 'siblings unreadable'})`);
  const skipped = [];
  const landedObserved = [];
  const seen = new Set();
  const advisory = new Map((queue.advisory || []).map(row => [row.intent, row]));
  const rows = [];
  const malformedRows = [];
  for (const row of queue.intents || []) {
    if (seen.has(row.intent)) continue;
    seen.add(row.intent);
    if (!validRow(row, me)) {
      line(`malformed intent row ${String(row?.intent || '<unknown>')}`);
      malformedRows.push(row);
      continue;
    }
    if (row.state !== 'live') {
      if (row.state === 'withdrawn' || row.state === 'landed')
        skipped.push({ intent: row.intent, why: row.state });
      continue;
    }
    if (row.change === me.change) continue;
    const landing = git.landingCommit(row.change);
    if ((landing && git.isAncestor(landing.commit, dev.commit)) || row.tree === dev.tree) {
      landedObserved.push(row.intent);
      continue;
    }
    const advice = advisory.get(row.intent);
    if (advice) {
      const kind =
        advice.kind ||
        String(advice.why || '').match(/\b(withdraw|landed|dismiss)\b/)?.[1] ||
        advice.why;
      const why = `advisory ${kind} by ${advice.via}`;
      skipped.push({ intent: row.intent, why });
      line(`skipping ${row.intent.slice(0, 8)}: ${why}`);
      continue;
    }
    let dormant = false;
    if (row.device === me.device) {
      const live = git.ownLive(row);
      dormant = live === false || (live === null && now - row.raisedAt > 3_600_000);
    } else {
      const age = heads.get(row.device);
      dormant = age === undefined || age === null || age > settlement.freshness_s;
    }
    if (dormant) {
      skipped.push({ intent: row.intent, why: 'dormant' });
      line(`skipping ${row.intent.slice(0, 8)}: dormant`);
      continue;
    }
    rows.push(row);
  }

  let predicted = dev.tree;
  let prevCommit = dev.commit;
  const chain = [];
  let last = { ...dev };
  let stopped = { at: null, why: 'end of queue' };
  while (chain.length < cap) {
    const candidates = rows.filter(row => row.base === predicted && !chain.includes(row.intent));
    if (candidates.length === 0) {
      const malformed = malformedRows.find(row => row?.base === predicted);
      if (malformed) stopped = { at: malformed.intent || null, why: 'malformed intent row' };
      break;
    }
    const compareCodeUnits = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
    const ordered = candidates.sort(
      (a, b) =>
        compareCodeUnits(a.device || '', b.device || '') || compareCodeUnits(a.intent, b.intent),
    );
    const winner = ordered.reduce((best, row) =>
      precedesIntent(row, best, settlement.max_skew_s) ? row : best,
    );
    for (const loser of ordered.filter(row => row !== winner)) {
      line(
        `fork on ${predicted.slice(0, 8)}: ${winner.intent.slice(0, 8)} precedes ${loser.intent.slice(0, 8)}`,
      );
    }
    const classIndex = CHANGE_CLASSES.indexOf(winner.class);
    if (classIndex < 0 || classIndex >= CHANGE_CLASSES.indexOf('governance')) {
      skipped.push({ intent: winner.intent, why: 'governance-class' });
      stopped = {
        at: winner.intent,
        why: `never chained behind class ${winner.class}`,
      };
      line(stopped.why);
      break;
    }
    if (!git.hasObject(winner.tree)) {
      skipped.push({ intent: winner.intent, why: 'bytes not on this device' });
      stopped = { at: winner.intent, why: 'sibling bytes absent (A7b)' };
      line(stopped.why);
      break;
    }
    const landing = git.landingCommit(winner.change);
    if (!landing || landing.tree !== winner.tree) {
      stopped = {
        at: winner.intent,
        why: `no refs/landing/${winner.change} naming that tree`,
      };
      line(stopped.why);
      break;
    }
    if (!git.isAncestor(prevCommit, landing.commit)) {
      stopped = {
        at: winner.intent,
        why: `refs/landing/${winner.change} does not descend from ${prevCommit.slice(0, 8)}`,
      };
      line(stopped.why);
      break;
    }
    chain.push(winner.intent);
    predicted = winner.tree;
    prevCommit = landing.commit;
    last = landing;
  }
  if (
    chain.length === cap &&
    rows.some(row => row.base === predicted && !chain.includes(row.intent))
  ) {
    const next = rows.find(row => row.base === predicted && !chain.includes(row.intent));
    skipped.push({ intent: next.intent, why: 'cap' });
    stopped = { at: next.intent, why: `cap ${cap} reached` };
    line(stopped.why);
  }
  return {
    base: last,
    chain,
    chainLen: chain.length,
    skipped,
    degraded,
    fromDev: chain.length === 0,
    predictedBase: last.tree,
    stopped,
    landedObserved,
    lines,
  };
}

export function phaseLVerdict({ dev, predicted }) {
  if (dev.commit === predicted.commit) return 'ff';
  if (dev.tree === predicted.tree) return 'same-tree';
  return 'moved';
}

export function dismissTargets({
  chain,
  project,
  redAtPredicted,
  greenAtDev,
  keyPredicted,
  keyDev,
  heads,
  now,
}) {
  if (!(redAtPredicted && greenAtDev)) return { targets: [], capsule: null };
  return {
    targets: chain.slice(),
    capsule: {
      key_predicted: String(keyPredicted),
      key_dev: String(keyDev),
      project: String(project),
      chain: chain.join(','),
      heads: JSON.stringify(Object.fromEntries(heads)),
      now: String(now),
    },
  };
}

export function gitOracle(root, spawn = spawnSync) {
  const run = args => spawn('git', args, { cwd: root, encoding: 'utf8' });
  return {
    hasObject(oid) {
      return run(['cat-file', '-e', oid]).status === 0;
    },
    landingCommit(change) {
      const commit = run(['rev-parse', '--verify', '-q', `refs/landing/${change}^{commit}`]);
      const tree = run(['rev-parse', '--verify', '-q', `refs/landing/${change}^{tree}`]);
      return commit.status === 0 && tree.status === 0
        ? { commit: commit.stdout.trim(), tree: tree.stdout.trim() }
        : null;
    },
    isAncestor(a, b) {
      const result = run(['merge-base', '--is-ancestor', a, b]);
      if (result.status !== 0 && result.status !== 1)
        process.stderr.write(`[landing] git ancestry failed: ${(result.stderr || '').trim()}\n`);
      return result.status === 0;
    },
    ownLive() {
      return null;
    },
  };
}

function option(args, name, fallback = null) {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : fallback;
}

if (process.argv[1]?.endsWith('gate-landing-base.mjs') && process.argv[2] === 'pick') {
  const args = process.argv.slice(3);
  const root = process.cwd();
  const run = gitOracle(root);
  const devCommit = option(args, '--dev');
  const devTreeResult = spawnSync('git', ['rev-parse', `${devCommit}^{tree}`], {
    cwd: root,
    encoding: 'utf8',
  });
  if (devTreeResult.status !== 0) {
    const detail = (devTreeResult.stderr || '').trim().split('\n')[0];
    process.stderr.write(`[landing] --dev ${devCommit} does not resolve: ${detail}\n`);
    process.exit(2);
  }
  const headsArg = option(args, '--heads');
  let headsJson = { devices: [] };
  if (headsArg)
    headsJson = JSON.parse(headsArg.startsWith('{') ? headsArg : readFileSync(headsArg, 'utf8'));
  const heads = new Map((headsJson.devices || []).map(row => [row.device, row.ageS ?? null]));
  const policyPath = option(
    args,
    '--policy',
    resolve(process.env.GATE_RECIPE_ROOT || root, 'genesis/agentic/pool-policy.json'),
  );
  let settlement = { max_skew_s: 30, freshness_s: 90 };
  try {
    settlement = {
      ...settlement,
      ...JSON.parse(readFileSync(policyPath, 'utf8')).berth.channel,
    };
  } catch {
    process.stderr.write('[landing] policy unreadable — using max_skew_s=30 freshness_s=90\n');
  }
  let queue;
  const queueError = option(args, '--queue-error');
  try {
    queue = queueError ? { error: true, why: queueError } : JSON.parse(readFileSync(0, 'utf8'));
    if (
      !queue ||
      typeof queue !== 'object' ||
      Array.isArray(queue) ||
      (!queue.error && !Array.isArray(queue.intents))
    ) {
      queue = { error: true, why: 'queue JSON has an invalid top-level shape' };
    }
  } catch (error) {
    queue = { error: true, why: `queue JSON unreadable: ${error.message}` };
  }
  const result = pickLandingBase({
    dev: { commit: devCommit, tree: devTreeResult.stdout.trim() },
    queue,
    me: {
      change: option(args, '--change') === '-' ? null : option(args, '--change'),
      device: option(args, '--device') === '-' ? null : option(args, '--device'),
    },
    settlement,
    heads,
    now: Number(option(args, '--now', Date.now())),
    git: run,
  });
  for (const line of result.lines) process.stderr.write(`${line}\n`);
  process.stdout.write(`${JSON.stringify(result)}\n`);
}
