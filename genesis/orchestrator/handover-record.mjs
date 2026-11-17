#!/usr/bin/env node
// Hand-over record — step 1 of the `self/<name>` lane (row 9, ruling R1).
//
// A push whose every destination is `refs/heads/self/<name>` carries work from one of the steward's
// devices to another and moves no head anyone adopts. The pre-push hook runs its seconds checks and
// no project gate, then calls this to write down what was handed over: one JSON line per distinct
// pushed head, naming the head, its destinations, the time and the gate verdicts this device
// already holds for exactly that commit (read with brit's verified `build get`, the reader the gate
// itself uses). An empty inventory is allowed and printed as such; a note brit cannot read or
// verify is listed as unreadable, never dropped.
//
// The record is ADVISORY. It is a request to land, never a gate pass: nothing on the landing path
// reads it, and a `dev` landing of the same head computes and demands its own verdicts.
//
// The log is device-local, in the checkout this device's peer daemon serves (gate-evidence's
// `deviceRoot`), beside the gate-live run logs: <device>/.eprfs/status/handover/handovers.jsonl.
// A failed write exits non-zero, and the hook refuses the push.
//
//   printf '%s\n' "<push lines>" | node genesis/orchestrator/handover-record.mjs --root <checkout>
//
// Exit: 0 recorded (or nothing to record: deletions only), 1 the record was not written,
// 2 the input is not a hand-over batch.

import { spawnSync } from 'node:child_process';
import { closeSync, fstatSync, fsyncSync, ftruncateSync, mkdirSync, openSync, readFileSync, writeSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isHandoverBranch } from './gate-cost.mjs';
import { deviceRoot, resolveBin } from './gate-evidence.mjs';
import { readBuildNote } from './scripts/brit-attest.mjs';

export const HANDOVER_SCHEMA = 'handover-record@1';
const OID = /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/;
const ZERO = /^0+$/;
const B32 = 'abcdefghijklmnopqrstuvwxyz234567';

/** The log this device keeps, under the device checkout (not a linked worktree's own store). */
export function handoverLog(root, spawn = spawnSync) {
  return join(deviceRoot(root, spawn), '.eprfs', 'status', 'handover', 'handovers.jsonl');
}

/**
 * The push lines as git hands them to pre-push, or null unless the batch is a hand-over: non-empty,
 * every line four well-formed fields, every destination `refs/heads/self/<non-empty>`. The hook
 * classifies first; this is the same rule, checked again where the record is written.
 */
export function handoverLines(text) {
  const lines = String(text).split('\n');
  if (lines.at(-1) === '') lines.pop();
  if (lines.length === 0) return null;
  const parsed = [];
  for (const line of lines) {
    const fields = line.split(' ');
    if (fields.length !== 4) return null;
    const [localRef, localSha, remoteRef, remoteSha] = fields;
    if (!localRef || !OID.test(localSha) || !OID.test(remoteSha)) return null;
    if (!remoteRef.startsWith('refs/heads/') || !isHandoverBranch(remoteRef.slice('refs/heads/'.length))) return null;
    parsed.push({ localRef, localSha, remoteRef, remoteSha });
  }
  return parsed;
}

/** Distinct pushed heads, each with every destination it was pushed to. Deletions carry no head. */
export function pushedHeads(lines) {
  const heads = new Map();
  for (const line of lines) {
    if (ZERO.test(line.localSha)) continue;
    if (!heads.has(line.localSha)) heads.set(line.localSha, []);
    heads.get(line.localSha).push(line.remoteRef);
  }
  return [...heads].map(([head, destinations]) => ({ head, destinations }));
}

/** A note's CID byte array (brit's `outputCid`: the verdict node) as a base32 CIDv1 string, else null. */
export function cidString(bytes) {
  if (!Array.isArray(bytes) || bytes.length === 0 || bytes.some(b => !Number.isInteger(b) || b < 0 || b > 255)) return null;
  let bits = 0;
  let value = 0;
  let out = 'b';
  for (const byte of bytes) {
    value = ((value << 8) | byte) & 0xffff;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += B32[(value << (5 - bits)) & 31];
  return out;
}

/**
 * The gate verdicts this device holds for exactly `head`: every `gate:*` step brit lists, read at
 * that commit with the gate's own verified reader. `{ verdicts, unreadable, inventory }`; a listing
 * that fails makes the inventory unavailable and says why.
 */
export function inventory(head, { root, buildRefBin, spawn = spawnSync }) {
  if (!buildRefBin) return { verdicts: [], unreadable: [], inventory: 'unavailable: brit-build-ref not on BRIT_BUILD_REF_BIN or PATH' };
  let listed;
  try {
    listed = spawn(buildRefBin, ['--repo', root, 'build', 'list'], { cwd: root, encoding: 'utf8', timeout: 30_000 });
  } catch (error) {
    return { verdicts: [], unreadable: [], inventory: `unavailable: build list: ${String(error?.message || error).split('\n')[0]}` };
  }
  if (listed.status !== 0) {
    const why = String(listed.stderr || '').trim().split('\n').pop() || `exit ${listed.status}`;
    return { verdicts: [], unreadable: [], inventory: `unavailable: build list: ${why}` };
  }
  const steps = String(listed.stdout || '').split('\n').map(s => s.trim()).filter(s => s.startsWith('gate:')).sort();
  const verdicts = [];
  const unreadable = [];
  for (const step of steps) {
    const project = step.slice('gate:'.length);
    const note = readBuildNote(step, head, { buildRefBin, root, spawn, timeout: 30_000 });
    if (note.state === 'absent') continue;
    if (note.state !== 'verified') {
      unreadable.push({ project, state: note.state, reason: note.reason || '' });
      continue;
    }
    const { node } = note;
    verdicts.push({
      project,
      closure: typeof node.inputsHash === 'string' ? node.inputsHash : null,
      verdict: cidString(node.outputCid),
      signer: typeof node.agentId === 'string' ? node.agentId : null,
      verdictOf: node.success === true ? 'green' : 'red',
    });
  }
  return { verdicts, unreadable, inventory: 'read' };
}

export function summaryLine(record, log) {
  const held = record.verdicts.length
    ? record.verdicts.map(v => `${v.project} ${v.verdictOf}`).join(', ')
    : 'none';
  const parts = [
    `[pre-push] hand-over ${record.head.slice(0, 12)} → ${record.destinations.join(' ')}:`,
    `verdicts this device holds for this head: ${held}`,
  ];
  if (record.inventory !== 'read') parts.push(`(inventory ${record.inventory})`);
  if (record.unreadable.length) {
    parts.push(`; UNREADABLE evidence: ${record.unreadable.map(u => `${u.project} (${u.state}${u.reason ? `: ${u.reason}` : ''})`).join(', ')}`);
  }
  parts.push(`; recorded in ${log} (advisory: a dev landing demands its own gates)`);
  return parts.join(' ').replaceAll(' ;', ';');
}

/**
 * Append the whole batch with one O_APPEND write and fsync. A short or failed write is rolled back
 * to the log's prior length, so the JSONL never keeps a torn line; the caller still refuses.
 */
export function appendBatch(log, text, fs = { closeSync, fstatSync, fsyncSync, ftruncateSync, mkdirSync, openSync, writeSync }) {
  fs.mkdirSync(dirname(log), { recursive: true });
  const bytes = Buffer.from(text, 'utf8');
  const fd = fs.openSync(log, 'a');
  let before = null;
  try {
    before = fs.fstatSync(fd).size;
    const written = fs.writeSync(fd, bytes, 0, bytes.length);
    if (written !== bytes.length) throw new Error(`short write: ${written} of ${bytes.length} bytes`);
    fs.fsyncSync(fd);
  } catch (error) {
    if (before !== null) {
      try { fs.ftruncateSync(fd, before); } catch { /* the refusal below still stands */ }
    }
    throw error;
  } finally {
    fs.closeSync(fd);
  }
}

export function main(argv, {
  stdin = () => readFileSync(0, 'utf8'), env = process.env, spawn = spawnSync,
  now = () => new Date(), out = line => process.stdout.write(`${line}\n`), err = line => process.stderr.write(`${line}\n`),
} = {}) {
  const at = argv.indexOf('--root');
  const root = resolve(at >= 0 && argv[at + 1] ? argv[at + 1] : process.cwd());
  const lines = handoverLines(stdin());
  if (!lines) {
    err('[pre-push] REFUSED: handover-record: the push is not a hand-over batch (every destination must be refs/heads/self/<name>)');
    return 2;
  }
  const heads = pushedHeads(lines);
  if (heads.length === 0) {
    out('[pre-push] hand-over: deletion only — nothing recorded');
    return 0;
  }
  const log = handoverLog(root, spawn);
  const buildRefBin = resolveBin('brit-build-ref', ['BRIT_BUILD_REF_BIN', 'BRIT_BUILD_REF'], env);
  const time = now().toISOString();
  const records = heads.map(({ head, destinations }) => ({
    schema: HANDOVER_SCHEMA,
    advisory: 'a request to land; never a gate pass',
    head,
    destinations,
    at: time,
    worktree: root,
    ...inventory(head, { root, buildRefBin, spawn }),
  }));
  try {
    appendBatch(log, records.map(r => `${JSON.stringify(r)}\n`).join(''));
  } catch (error) {
    err(`[pre-push] handover-record: could not write ${log}: ${String(error?.message || error).split('\n')[0]}`);
    return 1;
  }
  for (const record of records) out(summaryLine(record, log));
  return 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
