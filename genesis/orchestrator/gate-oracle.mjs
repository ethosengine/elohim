// The graph oracle for local gate selection — `brit build affected` (rakia's
// graph, served by the brit CLI) propagates a change set through step `depends`;
// this module asks it, keeps ONE hop (the local gate's depth), and treats every
// failure as "fall back to path-only", never as a red. A failure carries one
// short reason so the caller can say why the oracle was unavailable.
//
// Reason strings match graph-walker's: `source: <path>` for a direct change,
// `upstream: <qualified step>` for a one-hop dependent.

import { accessSync, constants, statSync } from 'fs';
import { spawnSync } from 'child_process';
import { delimiter, isAbsolute, join } from 'path';

/** A stalled `brit build affected` degrades to path-only selection instead of blocking the push. */
export const ORACLE_TIMEOUT_MS = 60_000;

function isExecutable(candidate) {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/** BRIT_BIN if executable, else the first executable `brit` on PATH, else null. */
export function resolveBritBin(env = process.env) {
  const declared = (env.BRIT_BIN || '').trim();
  if (declared) {
    if (isExecutable(isAbsolute(declared) ? declared : join(process.cwd(), declared))) return declared;
    return null;
  }
  for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, 'brit');
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
 * Ask `brit build affected` which steps a change set affects. Returns a Map of
 * qualified step → reasons at depth one, or null when the oracle is unavailable
 * (no binary, timeout, non-zero exit, unparsable output) so the caller can fall
 * back; in that case `onUnavailable(reason)` receives one short reason string.
 */
export function britAffected(root, changedFiles, { britBin, spawn = spawnSync, onUnavailable } = {}) {
  const unavailable = reason => {
    onUnavailable?.(reason);
    return null;
  };
  if (!britBin) return unavailable('no brit executable (BRIT_BIN or PATH)');
  const files = changedFiles.filter(Boolean);
  if (files.length === 0) return new Map();
  // A hung planner must never block selection for every push: bounded, then fall back.
  const result = spawn(britBin, ['build', 'affected', '--repo', root, '--files', files.join(',')], { encoding: 'utf8', timeout: ORACLE_TIMEOUT_MS });
  if (result?.error?.code === 'ETIMEDOUT') return unavailable(`brit build affected timed out after ${ORACLE_TIMEOUT_MS / 1000} s`);
  // Not a timeout: a spawn that never ran, or a child another process ended (the RAM guard sheds with SIGTERM).
  if (result?.error) return unavailable(`brit build affected could not run: ${result.error.code || result.error.message}`);
  if (result && result.status === null && result.signal) return unavailable(`brit build affected was ended by ${result.signal}`);
  if (!result || result.status !== 0) {
    const line = String(result?.stderr || '').split('\n').map(l => l.trim()).find(Boolean) || '';
    return unavailable(`brit build affected exited ${result?.status ?? 'without a status'}${line ? `: ${line.slice(0, 160)}` : ''}`);
  }
  let parsed;
  try {
    parsed = JSON.parse(result.stdout);
  } catch {
    return unavailable('brit build affected printed no affected list');
  }
  if (!parsed || !Array.isArray(parsed.affected)) return unavailable('brit build affected printed no affected list');
  return depthOne(parsed.affected);
}
