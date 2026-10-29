#!/usr/bin/env node
// brit-attest — the device writes Jenkins verdicts when its harvest reads an archived graph.
// ci:<pipeline> records the pipeline verdict using --output-json (addressable brit content),
// keyed by the commit tree. ci:<pipeline>:<step> records a produced image using its raw digest
// CID and the exact sha256:-prefixed closure key. A closure hit produces no new step note.
// hardware identifies service:jenkins as performer and the downstream URL as provenance;
// writer identifies this device. brit's agentId remains this device's signing key.
// Notes stay on this device: no remote fetch/push, runner key, or roster entry.
// An existing note with the same inputsHash is preserved, including its builtAt timestamp.
// Explicit rewrite policy: a different inputsHash replaces the note under the same lock.
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

/** Always 0; deferred work is explicitly reported so the harvest does not memoise it. */
export async function main(argv, { env = process.env } = {}) {
  deadline = performance.now() + (Number(env.ATTEST_BUDGET_MS) > 0 ? Number(env.ATTEST_BUDGET_MS) : 60_000);
  const graphPath = argv.find(arg => arg !== '--json');
  const bin = env.BRIT_BUILD_REF_BIN;
  const repo = env.REPO_ROOT || process.cwd();
  const report = { commit: null, orchestratorBuild: null, written: [], present: [], skipped: [], deferred: [] };
  const defer = why => { report.deferred.push({ why }); warn(why); };
  let lock;
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
    const { planned, skipped } = planAttestations(graph, {
      commit, tree, manifestBytes: manifestReader(repo, commit), machine: machine(env),
    });
    report.skipped = skipped;
    for (const s of skipped) console.error(`[brit-attest] skip ${s.pipeline}: ${s.why}`);
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
      try {
        // The selected path consumes this override. Passing it to bin-launch again
        // asks the launcher to execute itself and is correctly refused as launcher-alias.
        const childEnv = { ...env };
        delete childEnv.BRIT_BUILD_REF_BIN;
        const options = { cwd: repo, env: childEnv, stdio: ['ignore', 'pipe', 'pipe'], timeout: timeout() };
        const existing = spawnSync(bin, ['--repo', repo, 'build', 'get', '--step', p.step, '--commit', commit], { ...options, encoding: 'utf8' });
        if (existing.error) throw existing.error;
        if (existing.status !== 0) throw new Error(existing.stderr.trim() || `build get exited ${existing.status}`);
        let node;
        const absent = `${existing.stdout}\n${existing.stderr}`.trim().startsWith('no build ref found for step=');
        if (!absent) node = JSON.parse(existing.stdout);
        const inputs = p.args[p.args.indexOf('--inputs-hash') + 1];
        if (node?.inputsHash === inputs) {
          report.present.push(entry);
          console.error(`[brit-attest] present ${p.step}`);
          continue;
        }
        const cid = execFileSync(bin, ['--repo', repo, ...p.args], { ...options, timeout: timeout() }).toString().trim();
        report.written.push({ ...entry, cid, ...(node ? { rewritten: true } : {}) });
        console.error(`[brit-attest] ${node ? 'rewritten' : 'written'} ${p.step} → ${cid}`);
      } catch (e) {
        if (e.code === 'ENOENT') { defer('brit-build-ref absent'); break; }
        defer(`build get/put ${p.step} failed: ${(e.stderr?.toString() || e.message).trim().split('\n')[0]}`);
      }
    }
    console.error(`[brit-attest] ${report.written.length}/${planned.length} notes attested on ${commit.slice(0, 12)}`);
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
