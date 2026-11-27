#!/usr/bin/env node
// Resolve the open landing intent carried by a pushed dev head. This is a read-only hint for the
// gate carrier: every failure is rendered as a reason and leaves the gates to run here.

import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseChangeId } from './commit-tag-parser.mjs';
import { deviceEprResolution } from './device-epr.mjs';
import { deviceRoot, resolveBin } from './gate-evidence.mjs';

const DEFAULT_TIMEOUT_S = 5;

function oneLine(value, fallback) {
  return String(value ?? '').trim().split(/\r?\n/, 1)[0].trim() || fallback;
}

function timeoutMs(env) {
  const seconds = Number(env.LANDING_INTENT_TIMEOUT_S ?? DEFAULT_TIMEOUT_S);
  return Math.round((Number.isFinite(seconds) && seconds > 0 ? seconds : DEFAULT_TIMEOUT_S) * 1000);
}

function run(spawn, command, args, { root, env, timeout }) {
  let result;
  try {
    result = spawn(command, args, {
      cwd: root,
      encoding: 'utf8',
      env,
      stdio: 'pipe',
      timeout,
    });
  } catch (error) {
    return { error: oneLine(error?.message, `${command} could not run`) };
  }
  if (result?.error) {
    const timedOut = result.error.code === 'ETIMEDOUT';
    return { error: timedOut ? `timed out after ${timeout / 1000} s` : oneLine(result.error.message, `${command} could not run`) };
  }
  if (result?.status !== 0) {
    return { error: oneLine(result?.stderr, `${command} exited ${result?.status ?? result?.signal ?? 'without a status'}`) };
  }
  return { stdout: String(result.stdout ?? '') };
}

function uniqueIntents(rows) {
  return [...new Set(rows)].sort();
}

/**
 * Find the one live intent associated with `head`, preferring an exact open-miss commit match over
 * the commit's Change-Id. The device projection is read once and failures never escape.
 */
export function findLandingIntent({ head, root, env = process.env, spawn = spawnSync }) {
  try {
    const timeout = timeoutMs(env);
    const resolution = deviceEprResolution({ env, warn: () => {} });
    const epr = resolution.source === 'head'
      ? resolution.command
      : resolveBin('epr', ['EPR_BIN'], env);
    if (!epr) return { intent: null, reason: 'no executable device-head or fallback epr' };

    const store = deviceRoot(root, spawn);
    const read = run(spawn, epr, ['peer', 'intents', '--json', '--root', store], {
      root, env, timeout,
    });
    if (read.error) return { intent: null, reason: `intent read failed: ${read.error}` };

    let output;
    try {
      output = JSON.parse(read.stdout);
    } catch {
      return { intent: null, reason: 'intent read returned unparseable JSON' };
    }
    if (!output || typeof output !== 'object' || !Array.isArray(output.intents) || !Array.isArray(output.misses)) {
      return { intent: null, reason: 'intent read returned an unrecognized JSON shape' };
    }
    if (typeof output.error === 'string' && output.error.trim()) {
      return { intent: null, reason: `intent read failed: ${oneLine(output.error, 'unknown error')}` };
    }

    const live = new Map(output.intents
      .filter(row => row && typeof row === 'object' && row.state === 'live' && typeof row.intent === 'string')
      .map(row => [row.intent, row]));
    const byHead = uniqueIntents(output.misses
      .filter(row => row && typeof row === 'object' && row.commit === head && live.has(row.intent))
      .map(row => row.intent));
    if (byHead.length === 1) {
      return { intent: byHead[0], reason: `matched pushed head ${String(head).slice(0, 12)}` };
    }
    if (byHead.length > 1) {
      return { intent: null, reason: `${byHead.length} open intents match pushed head ${String(head).slice(0, 12)}` };
    }

    const message = run(spawn, 'git', ['show', '-s', '--format=%B', head], { root, env, timeout });
    if (message.error) return { intent: null, reason: `commit message unavailable: ${message.error}` };
    const change = parseChangeId(message.stdout);
    if (!change.id) return { intent: null, reason: `no head-matched intent and ${change.why}` };

    const byChange = uniqueIntents([...live.values()]
      .filter(row => row.change === change.id)
      .map(row => row.intent));
    if (byChange.length === 1) {
      return { intent: byChange[0], reason: `matched Change-Id ${change.id}` };
    }
    if (byChange.length > 1) {
      return { intent: null, reason: `${byChange.length} open intents match Change-Id ${change.id}` };
    }
    return { intent: null, reason: `no open intent matches pushed head or Change-Id ${change.id}` };
  } catch (error) {
    return { intent: null, reason: `intent lookup failed: ${oneLine(error?.message, 'unknown error')}` };
  }
}

function cli(argv) {
  let head = null;
  let root = null;
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === '--head') head = argv[++index] ?? null;
    else if (argv[index] === '--root') root = argv[++index] ?? null;
  }
  if (!head || !root) return { intent: null, reason: 'usage requires --head <sha> --root <checkout>' };
  return findLandingIntent({ head, root: resolve(root) });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const found = cli(process.argv.slice(2));
  if (found.intent) process.stdout.write(`${found.intent}\n`);
  process.stderr.write(`[pre-push] landing intent: ${oneLine(found.reason, 'not found')}\n`);
}
