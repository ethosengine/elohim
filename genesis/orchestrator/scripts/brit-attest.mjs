#!/usr/bin/env node
// brit-attest — the device writes Jenkins verdicts when its harvest reads an archived graph.
// ci:<pipeline> records the pipeline verdict using --output-json (addressable brit content),
// keyed by the commit tree. ci:<pipeline>:<step> records a produced image using its raw digest
// CID and the exact sha256:-prefixed closure key. A closure hit produces no new step note.
// hardware identifies service:jenkins as performer and the downstream URL as provenance;
// writer identifies this device. brit's agentId remains this device's signing key.
// Notes stay on this device: no remote fetch/push, runner key, or roster entry.
//
// A writer reads before it writes. `build get` is brit's verified read of the note already on
// the commit, and it only NAMES what the put replaces: an existing note with the same inputsHash
// and verdict is preserved, including its builtAt timestamp (`present`); a verified note for
// another tree or verdict is named and replaced under the same lock (`rewritten`); a note brit
// reports `unverifiable:` is absent to this reader, named once with its ref and reason, and
// replaced; a note this device could not read is named and its write is deferred — nothing is
// written over what was not read. `build put` can refuse: `REFUSED concurrent update` (another
// writer moved this commit's note under us, brit CAS) and `refusing to write <ref>: <reason>`
// (the record would not pass the verified read). Both are expected outcomes of a shared notes
// ref: warned and counted under `refused`, never a failure. A concurrent update is also deferred
// so the next harvest reads the other writer's note; identical bytes would refuse again, so a
// would-not-verify refusal is not. The read-side helpers here are shared with gate-evidence.mjs.
// Every failure is advisory; --json always emits one report and exits 0 for harvest retries.

import { createHash } from 'node:crypto';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, openSync, closeSync, mkdirSync } from 'node:fs';
import { arch, hostname, platform } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { loadPipelineRegistry } from '../pipeline-registry.mjs';

const ATTESTED = new Set(['SUCCESS', 'UNSTABLE', 'FAILURE', 'ABORTED']);
const SUCCEEDED = new Set(['SUCCESS', 'UNSTABLE']);
export const NOTES_PREFIX = 'refs/notes/brit/build/';
const B32 = 'abcdefghijklmnopqrstuvwxyz234567';

function base32(bytes) {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += B32[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += B32[(value << (5 - bits)) & 31];
  return out;
}

/** Raw CIDv1 (codec 0x55, sha2-256), base32: byte-identical to brit's BritCid::compute_raw. */
export function rawCid(bytes) {
  const digest = createHash('sha256').update(bytes).digest();
  return 'b' + base32(Buffer.concat([Buffer.from([0x01, 0x55, 0x12, 0x20]), digest]));
}

/** Wrap an OCI sha256 digest as raw CIDv1; the digest already hashes the manifest bytes. */
export function cidOfDigest(digest) {
  if (!/^sha256:[a-f0-9]{64}$/.test(digest)) throw new Error(`invalid sha256 digest: ${digest}`);
  return 'b' + base32(Buffer.concat([Buffer.from([0x01, 0x55, 0x12, 0x20]), Buffer.from(digest.slice(7), 'hex')]));
}

const REF_ESCAPES = { ':': '%3A', '@': '%40', '?': '%3F', '*': '%2A', '[': '%5B', '\\': '%5C', '^': '%5E', '~': '%7E', ' ': '%20' };

/** brit's ref segment encoding (brit-epr refs.rs `encode_segment`): how a step names its notes ref. */
export function encodeRefSegment(segment) {
  let out = '';
  for (const c of String(segment)) {
    const code = c.codePointAt(0);
    if (Object.hasOwn(REF_ESCAPES, c)) out += REF_ESCAPES[c];
    else if (code < 0x20 || code === 0x7f) out += `%${code.toString(16).toUpperCase().padStart(2, '0')}`;
    else out += c;
  }
  return out.replaceAll('..', '%2E%2E');
}

/** The notes ref a step's build attestations live under, one note per commit. */
export function buildNotesRef(step) {
  return `${NOTES_PREFIX}${encodeRefSegment(step)}`;
}

/** The `unverifiable: <commit> <reason>` lines of a brit verified read's stderr. */
export function unverifiableNotes(stderr) {
  return String(stderr || '').split('\n').flatMap(line => {
    const m = /^unverifiable: (\S+) (.*)$/.exec(line);
    return m ? [{ commit: m[1], reason: m[2] }] : [];
  });
}

/**
 * A `build put` stderr that is one of brit's expected refusals, or null for any other failure.
 * `kind` names the class; `line` is brit's own line without the anyhow `Error: ` prefix.
 */
export function expectedRefusal(stderr) {
  for (const raw of String(stderr || '').split('\n')) {
    const line = raw.replace(/^Error: /, '').trim();
    if (line.startsWith('REFUSED concurrent update:')) return { kind: 'concurrent update', line };
    if (line.startsWith('refusing to write ')) return { kind: 'would not verify', line };
  }
  return null;
}

const lastLine = text => String(text || '').trim().split('\n').filter(Boolean).pop() || '';
const MISS = 'no build ref found for step=';

/**
 * brit's verified read of the build note on `commit` for `step` (`build get`):
 *   { state: 'verified', node }                    stdout held the signed node (verified by brit);
 *   { state: 'absent' }                            no note, miss line only;
 *   { state: 'unverifiable', commit, reason }      a note is stored but failed verification;
 *   { state: 'unreadable', reason, code? }         git could not read the ref, or the output was
 *                                                  not brit's (exit 1, non-JSON stdout, no spawn;
 *                                                  `code` is the spawn error's, ENOENT = no binary).
 * Exit 0 with no node is absent-or-unverifiable; stderr tells which. Never throws.
 *
 * Lineage note: the pinned 76ba95d8a0 prints a hit's `hardwareProfile` as a JSON OBJECT (observed
 * live); the verified-reads line (7e4a83115e) prints the signed STRING. `manifestCid`/`outputCid`
 * are byte arrays on both. Callers read only `inputsHash` and `success` from `node` — the two
 * fields spelled the same on both sides of the pin — never `hardwareProfile` or a CID.
 */
export function readBuildNote(step, commit, { buildRefBin, root, spawn = spawnSync, env, timeout = 60_000 }) {
  let r;
  try {
    r = spawn(buildRefBin, ['--repo', root, 'build', 'get', '--step', step, '--commit', commit], {
      cwd: root, encoding: 'utf8', timeout, stdio: ['ignore', 'pipe', 'pipe'], ...(env ? { env } : {}),
    });
  } catch (error) {
    return { state: 'unreadable', reason: String(error?.message || error).split('\n')[0], ...(error?.code ? { code: error.code } : {}) };
  }
  if (r.status !== 0) {
    return { state: 'unreadable', reason: lastLine(r.stderr) || r.error?.message || `exit ${r.status}`, ...(r.error?.code ? { code: r.error.code } : {}) };
  }
  const text = String(r.stdout || '').trim();
  if (text && !text.startsWith(MISS)) {
    try {
      const node = JSON.parse(text);
      return node && typeof node === 'object' ? { state: 'verified', node } : { state: 'unreadable', reason: 'build get: not an object' };
    } catch {
      return { state: 'unreadable', reason: 'build get: not JSON' };
    }
  }
  const [note] = unverifiableNotes(r.stderr);
  return note ? { state: 'unverifiable', ...note } : { state: 'absent' };
}

const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const buildNumber = value => /^(?:[1-9][0-9]*)$/.test(String(value)) && Number.isSafeInteger(Number(value));
export function validateGraph(graph) {
  if (!object(graph) || typeof graph.commitSha !== 'string' || !/^[a-f0-9]{40}$/i.test(graph.commitSha)) throw new Error('invalid commitSha: full 40-hex SHA required');
  if (!buildNumber(graph.buildNumber)) throw new Error('invalid buildNumber: numeric build identity required');
  if (!object(graph.results)) throw new Error('invalid results: map required');
  if (graph.executionOrder !== undefined && (!Array.isArray(graph.executionOrder) || graph.executionOrder.some(p => typeof p !== 'string' || !p))) throw new Error('invalid executionOrder');
  for (const [pipeline, r] of Object.entries(graph.results)) {
    if (!object(r) || !new Set([...ATTESTED, 'DISPATCHED', 'NOT_DISPATCHED', 'NOT_BUILT', 'ERROR', 'SKIPPED']).has(r.result)) throw new Error(`invalid results.${pipeline}: result record required`);
    if (r.buildNumber != null && !buildNumber(r.buildNumber)) throw new Error(`invalid results.${pipeline}.buildNumber`);
    if (r.duration != null && (typeof r.duration !== 'number' || !Number.isFinite(r.duration) || r.duration < 0)) throw new Error(`invalid results.${pipeline}.duration`);
    if (r.url != null && typeof r.url !== 'string') throw new Error(`invalid results.${pipeline}.url`);
    for (const field of ['closure', 'outputDigest', 'closureHit']) {
      if (r[field] != null && (!object(r[field]) || Object.values(r[field]).some(v => typeof v !== (field === 'closureHit' ? 'boolean' : 'string')))) throw new Error(`invalid results.${pipeline}.${field}`);
    }
  }
}

/** Pure plan: repository reads and the device identity are supplied by the caller. */
export function planAttestations(graph, ctx) {
  validateGraph(graph);
  const planned = [];
  const skipped = [];
  // Jenkins env.BUILD_NUMBER is a decimal string in archived graphs.
  const orchestratorBuild = Number(graph.buildNumber) || null;
  const results = graph?.results ?? {};
  const order = graph?.executionOrder?.length ? graph.executionOrder : Object.keys(results);
  for (const pipeline of order) {
    const r = results[pipeline];
    const result = r?.result ?? 'NOT_DISPATCHED';
    if (!ATTESTED.has(result)) {
      skipped.push({ pipeline, why: result === 'DISPATCHED' ? 'fire-and-forget: no verdict yet' : `result ${result}` });
      continue;
    }
    const manifest = ctx.manifestBytes(pipeline);
    if (!manifest) {
      skipped.push({ pipeline, why: 'no build-manifest.json for this pipeline' });
      continue;
    }
    const durationMs = Math.max(0, Math.round(Number(r.duration) || 0));
    const record = {
      pipeline, commit: ctx.commit, result, buildNumber: r.buildNumber ?? null,
      url: r.url ?? null, durationMs, orchestratorBuild,
    };
    for (const field of ['closure', 'outputDigest', 'closureHit']) {
      if (r[field] != null) record[field] = r[field];
    }
    const hardware = {
      runner: 'jenkins', performer: 'service:jenkins', provenance: r.url ?? null,
      orchestratorBuild, writer: ctx.machine,
    };
    const plan = (step, kind, output, inputs, success, extra = {}) => planned.push({
      pipeline, step, kind,
      args: ['build', 'put', '--step', step, '--manifest', rawCid(manifest), ...output,
        '--inputs-hash', inputs, '--success', String(success), '--duration-ms', String(durationMs),
        '--hardware', JSON.stringify({ ...hardware, inputsHashKind: kind, ...extra }), '--commit', ctx.commit],
    });
    plan(`ci:${pipeline}`, 'tree', ['--output-json', JSON.stringify(record)], ctx.tree, SUCCEEDED.has(result));
    for (const [step, closure] of Object.entries(r.closure ?? {})) {
      if (r.closureHit?.[step] === true) {
        skipped.push({ pipeline, step, why: 'closure hit: nothing produced' });
        continue;
      }
      const digest = r.outputDigest?.[step];
      if (!/^sha256:[a-f0-9]{64}$/.test(digest)) {
        skipped.push({ pipeline, step, why: 'no digest archived' });
        continue;
      }
      if (!/^sha256:[a-f0-9]{64}$/.test(closure)) {
        skipped.push({ pipeline, step, why: 'invalid closure key' });
        continue;
      }
      const image = { 'build-storage': 'elohim-storage', 'build-doorway': 'elohim-doorway' }[step];
      plan(`ci:${pipeline}:${step}`, 'closure-v3', ['--output', cidOfDigest(digest)], closure, true,
        image ? { artifact: `harbor.ethosengine.com/ethosengine/${image}@${digest}` } : {});
    }
  }
  return { planned, skipped };
}

let deadline = Infinity;
function timeout() {
  const left = deadline - performance.now();
  if (left <= 0) throw new Error('attestation budget exhausted');
  return Math.max(1, Math.min(5000, Math.floor(left)));
}

function git(repo, args, opts = {}) {
  return execFileSync('git', args, { cwd: repo, stdio: ['ignore', 'pipe', 'pipe'], timeout: timeout(), ...opts });
}

function warn(msg) {
  console.error(`[brit-attest] WARN: ${msg}`);
}

function manifestReader(repo, commit) {
  let registry = new Map();
  try {
    registry = loadPipelineRegistry(repo);
  } catch (e) {
    warn(`pipeline registry unreadable (${e.message}); no manifest CIDs`);
  }
  return pipeline => {
    const rel = registry.get(pipeline)?.manifestPath?.replace(/^\.\//, '');
    if (!rel) return null;
    try {
      return git(repo, ['show', `${commit}:${rel}`]);
    } catch {
      // A manifest inside a submodule is not in this repo's tree: read the checkout.
      try {
        return readFileSync(join(repo, rel));
      } catch {
        return null;
      }
    }
  };
}

function machine(env) {
  return { node: env.HOSTNAME ?? hostname(), os: platform(), arch: arch() };
}

const firstLine = text => String(text || '').trim().split('\n')[0];

/** Always 0; deferred work is explicitly reported so the harvest does not memoise it. */
export async function main(argv, { env = process.env } = {}) {
  deadline = performance.now() + (Number(env.ATTEST_BUDGET_MS) > 0 ? Number(env.ATTEST_BUDGET_MS) : 60_000);
  const graphPath = argv.find(arg => arg !== '--json');
  const bin = env.BRIT_BUILD_REF_BIN;
  const repo = env.REPO_ROOT || process.cwd();
  const report = { commit: null, orchestratorBuild: null, written: [], present: [], refused: [], skipped: [], deferred: [] };
  const defer = why => { report.deferred.push({ why }); warn(why); };
  let lock;
  let planned = [];
  let short = '';
  try {
    const graph = JSON.parse(readFileSync(graphPath, 'utf8'));
    validateGraph(graph);
    report.commit = graph.commitSha;
    report.orchestratorBuild = Number(graph.buildNumber) || null;
    if (!bin) { defer('brit-build-ref absent'); return 0; }
    let commit;
    let tree;
    try {
      commit = git(repo, ['rev-parse', `${graph.commitSha}^{commit}`]).toString().trim();
      tree = git(repo, ['rev-parse', `${commit}^{tree}`]).toString().trim();
    } catch {
      defer(`commit ${graph.commitSha} not in this clone`);
      return 0;
    }
    report.commit = commit;
    short = commit.slice(0, 12);
    const plan = planAttestations(graph, {
      commit, tree, manifestBytes: manifestReader(repo, commit), machine: machine(env),
    });
    planned = plan.planned;
    report.skipped = plan.skipped;
    for (const s of plan.skipped) console.error(`[brit-attest] skip ${s.pipeline}: ${s.why}`);
    // flock locks the inherited open file description; this parent owns it until close.
    // The common directory serializes direct invocations across every linked worktree.
    const common = resolve(repo, git(repo, ['rev-parse', '--git-common-dir']).toString().trim());
    mkdirSync(join(common, 'brit'), { recursive: true });
    lock = openSync(join(common, 'brit', 'attest.lock'), 'a');
    const acquired = spawnSync('flock', ['-n', '-E', '75', '3'], { stdio: ['ignore', 'pipe', 'pipe', lock], timeout: timeout() });
    if (acquired.status === 75) { defer('deferred: attest lock busy'); return 0; }
    if (acquired.error || acquired.status !== 0) throw new Error('attest lock unavailable');
    for (const p of planned) {
      const entry = { pipeline: p.pipeline, step: p.step, kind: p.kind };
      const ref = buildNotesRef(p.step);
      const inputs = p.args[p.args.indexOf('--inputs-hash') + 1];
      const success = p.args[p.args.indexOf('--success') + 1] === 'true';
      const verdict = success ? 'success' : 'failure';
      try {
        // The selected path consumes this override. Passing it to bin-launch again
        // asks the launcher to execute itself and is correctly refused as launcher-alias.
        const childEnv = { ...env };
        delete childEnv.BRIT_BUILD_REF_BIN;
        const options = { cwd: repo, env: childEnv, stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8' };
        // Read before write: what does the commit's note already say, verified by brit?
        const existing = readBuildNote(p.step, commit, { buildRefBin: bin, root: repo, env: childEnv, timeout: timeout() });
        if (existing.state === 'unreadable') {
          if (existing.code === 'ENOENT') { defer('brit-build-ref absent'); break; }
          defer(`${ref} on ${short} could not be read (${existing.reason}); ${p.step} not written`);
          continue;
        }
        if (existing.state === 'unverifiable') {
          warn(`${ref} on ${existing.commit} is unverifiable (${existing.reason}); replacing it`);
        } else if (existing.state === 'verified') {
          if (existing.node.inputsHash === inputs && existing.node.success === success) {
            report.present.push(entry);
            console.error(`[brit-attest] present ${p.step}`);
            continue;
          }
          console.error(`[brit-attest] ${p.step} replacing ${ref} on ${short} (it attests ${existing.node.success ? 'success' : 'failure'} for ${p.kind} ${String(existing.node.inputsHash).slice(0, 12)})`);
        }
        const r = spawnSync(bin, ['--repo', repo, ...p.args], { ...options, timeout: timeout() });
        if (r.error?.code === 'ENOENT') { defer('brit-build-ref absent'); break; }
        if (r.error) throw r.error;
        if (r.status === 0) {
          const cid = String(r.stdout).trim();
          const rewritten = existing.state !== 'absent';
          report.written.push({ ...entry, cid, ...(rewritten ? { rewritten: true } : {}) });
          console.error(`[brit-attest] ${rewritten ? 'rewritten' : 'written'} ${p.step} ${verdict} → ${cid}`);
          continue;
        }
        const refusal = expectedRefusal(r.stderr);
        if (refusal) {
          report.refused.push({ ...entry, refusal: refusal.kind, why: refusal.line });
          warn(`build put ${p.step} refused (${refusal.kind}, expected; not a failure): ${refusal.line}`);
          if (refusal.kind === 'concurrent update') defer(`deferred: ${p.step} moved by another writer; the next harvest reads it`);
          continue;
        }
        defer(`build put ${p.step} failed: ${firstLine(r.stderr) || `exit ${r.status}`}`);
      } catch (e) {
        defer(`build get/put ${p.step} failed: ${firstLine(e.stderr?.toString() || e.message)}`);
      }
    }
    const tally = [report.present.length ? `${report.present.length} present` : '', report.refused.length ? `${report.refused.length} refused` : '']
      .filter(Boolean).map(t => `, ${t}`).join('');
    console.error(`[brit-attest] ${report.written.length}/${planned.length} notes attested on ${short}${tally}`);
  } catch (e) {
    defer(`${graphPath ?? 'graph'} unreadable or attestation failed: ${e.message.split('\n')[0]}`);
  } finally {
    if (lock !== undefined) closeSync(lock);
    if (argv.includes('--json')) console.log(JSON.stringify(report));
  }
  return 0;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  main(process.argv.slice(2)).finally(() => process.exit(0));
}
