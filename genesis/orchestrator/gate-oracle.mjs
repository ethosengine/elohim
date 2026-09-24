// The rakia oracle for local gate selection — `rakia affected` propagates
// through step `depends`; this module asks it, keeps ONE hop (the local gate's
// depth), and treats every failure as "fall back to path-only", never as a red.
//
// Reason strings match graph-walker's: `source: <path>` for a direct change,
// `upstream: <qualified step>` for a one-hop dependent.

import { accessSync, constants, statSync } from 'fs';
import { spawnSync } from 'child_process';
import { delimiter, isAbsolute, join } from 'path';

/** A stalled `rakia affected` degrades to path-only selection instead of blocking the push. */
export const RAKIA_TIMEOUT_MS = 60_000;

function isExecutable(candidate) {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** RAKIA_BIN if executable, else `rakia` on PATH, else null. */
export function resolveRakiaBin(env = process.env) {
  const declared = (env.RAKIA_BIN || '').trim();
  if (declared) {
    if (isExecutable(isAbsolute(declared) ? declared : join(process.cwd(), declared))) return declared;
    return null;
  }
  for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, 'rakia');
    if (isExecutable(candidate)) return candidate;
  }
  return null;
}

/** Pure: direct steps plus their immediate dependents, with graph-walker-shaped reasons. */
export function depthOne(affected) {
  const direct = new Set(
    affected.filter(step => step.affected_by.some(r => r.kind === 'changedFile')).map(step => step.qualified_name),
  );
  const kept = new Map();
  for (const step of affected) {
    const reasons = [];
    for (const reason of step.affected_by) {
      if (reason.kind === 'changedFile' && reason.path) reasons.push(`source: ${reason.path}`);
      else if (reason.kind === 'upstreamNode' && reason.upstream && direct.has(reason.upstream)) reasons.push(`upstream: ${reason.upstream}`);
    }
    if (reasons.length > 0) kept.set(step.qualified_name, [...new Set(reasons)]);
  }
  return kept;
}

/**
 * Ask rakia which steps a change set affects. Returns a Map of qualified step →
 * reasons at depth one, or null when the oracle is unavailable (no binary,
 * non-zero exit, unparsable output) so the caller can fall back.
 */
export function rakiaAffected(root, changedFiles, { rakiaBin, spawn = spawnSync } = {}) {
  if (!rakiaBin) return null;
  const files = changedFiles.filter(Boolean);
  if (files.length === 0) return new Map();
  // A hung planner must never block selection for every push: bounded, then fall back.
  const result = spawn(rakiaBin, ['affected', '--repo', root, '--files', files.join(',')], { encoding: 'utf8', timeout: RAKIA_TIMEOUT_MS });
  if (!result || result.status !== 0) return null;
  let parsed;
  try {
    parsed = JSON.parse(result.stdout);
  } catch {
    return null;
  }
  if (!parsed || !Array.isArray(parsed.affected)) return null;
  return depthOne(parsed.affected);
}
