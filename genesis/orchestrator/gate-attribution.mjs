// Ephemeral (C): rebuilt from the device's edit journal; attribution decides nothing.
// Gates judge a commit minutes to hours after its edits, unlike the LSP's 30 s window.
// Use run-open time minus GATE_ATTRIBUTION_WINDOW_MS (default 6 h) as windowStartMs.

import { readFileSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';

const configuredWindow = process.env.GATE_ATTRIBUTION_WINDOW_MS;
export const GATE_ATTRIBUTION_WINDOW_MS =
  /^\d+$/.test(configuredWindow ?? '') && Number.isSafeInteger(Number(configuredWindow))
    ? Number(configuredWindow)
    : 6 * 60 * 60 * 1000;

const EDIT_TOOLS = new Set(['Edit', 'Write', 'MultiEdit', 'NotebookEdit']);
const UTC_MILLIS = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;

function isEdit(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
  const atMs = typeof value.at === 'string' ? Date.parse(value.at) : NaN;
  return value.v === 1
    && UTC_MILLIS.test(value.at)
    && Number.isFinite(atMs)
    && new Date(atMs).toISOString() === value.at
    && typeof value.session === 'string' && value.session.length > 0
    && Number.isSafeInteger(value.ownerPid) && value.ownerPid > 0
    && typeof value.ownerStartTime === 'string'
    && typeof value.path === 'string' && isAbsolute(value.path)
    && EDIT_TOOLS.has(value.tool);
}

/**
 * Read C2 rows from <main checkout>/.eprfs/status/lsp/edits.jsonl.
 * maxLines bounds the physical tail inspected, including unknown or torn rows;
 * a missing/unreadable journal supplies no evidence. No journal is created.
 */
export function readEdits(path, { maxLines = 10_000 } = {}) {
  if (!Number.isSafeInteger(maxLines) || maxLines < 0) {
    throw new RangeError('maxLines must be a non-negative safe integer');
  }
  if (maxLines === 0) return [];
  let text;
  try {
    text = readFileSync(path, 'utf8');
  } catch {
    return [];
  }
  const lines = text.split(/\r?\n/);
  if (lines.at(-1) === '') lines.pop();
  const edits = [];
  for (const line of lines.slice(-maxLines)) {
    try {
      const edit = JSON.parse(line);
      if (isEdit(edit)) edits.push(edit);
    } catch {
      // One torn/unknown line must not hide the rest of the journal.
    }
  }
  return edits;
}

/**
 * Match a failure payload (or its enclosing event) to exact C2 file/time evidence.
 * The caller supplies windowStartMs = runOpenedAtMs - GATE_ATTRIBUTION_WINDOW_MS,
 * and the failure's atMs. Anchoring to run-open keeps a long gate's window fixed.
 * Relative loci need an explicit absolute root; no basename, symlink, PID or
 * test-name inference is made. anon-<pid> remains its own session identifier.
 */
export function attribute(failure, edits, { root, session, windowStartMs, atMs } = {}) {
  const none = { label: 'unattributed', sessions: [] };
  const file = failure?.failure?.locus?.file ?? failure?.locus?.file;
  if (typeof file !== 'string' || file.length === 0
    || !Number.isFinite(windowStartMs) || !Number.isFinite(atMs)
    || windowStartMs > atMs || !Array.isArray(edits)) return none;
  if (!isAbsolute(file) && (typeof root !== 'string' || !isAbsolute(root))) return none;
  const absoluteFile = isAbsolute(file) ? resolve(file) : resolve(root, file);
  const candidates = new Set();
  for (const edit of edits) {
    if (!isEdit(edit) || edit.path !== absoluteFile) continue;
    const editedAtMs = Date.parse(edit.at);
    if (editedAtMs >= windowStartMs && editedAtMs <= atMs) candidates.add(edit.session);
  }
  const sessions = [...candidates].sort();
  if (sessions.length === 0) return none;
  if (sessions.length > 1) return { label: 'ambiguous', sessions };
  return { label: sessions[0] === session ? 'mine' : 'other', sessions };
}
