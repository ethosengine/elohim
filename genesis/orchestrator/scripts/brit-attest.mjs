#!/usr/bin/env node
// brit-attest — one brit build attestation per pipeline an orchestrator run executed.
//
// Called as `brit-helper.sh attest <actual-build-graph.json>` at the end of an orchestrator
// run (Lane F3 of the native-delivery sprint). For each pipeline the run awaited, it runs
//   brit-build-ref build put --step <pipeline> --manifest <cid> --output <cid>
//     --inputs-hash <tree> --success <bool> --duration-ms <ms> --hardware <json> --commit <sha>
// which writes a signed BuildAttestationContentNode to refs/notes/brit/build/<pipeline>, a note
// on the built commit. delivery-series.mjs --from attestations reads those notes back, so the
// delivery series no longer needs the Jenkins API.
//
// The fields, and what each one honestly is today:
//   manifest_cid  raw CIDv1 (sha2-256) of the pipeline's build-manifest.json bytes at the commit.
//   inputs_hash   the commit's git tree hash: the whole source tree the build consumed. A
//                 per-pipeline input closure is rakia's job, not this script's.
//   output_cid    raw CIDv1 of the run record {pipeline, commit, result, buildNumber, url}. The
//                 artifact's own CID (image digest, bundle CID) joins when the pipeline publishes
//                 one (Lane N's release manifest); until then the run record is what was produced.
//   agent_id      brit's signing key for this workspace. Z.D names a deploy-service agent per
//                 pipeline (`agent:deploy-service-<operator>-<pipeline>`), but that agent is a
//                 NAME today, with no provisioned key. agent_id must stay the key that signed
//                 (brit verifies the signature against it), so the name rides in
//                 hardware_profile.performer with performer_key "unprovisioned".
//   success       Jenkins SUCCESS or UNSTABLE. FAILURE and ABORTED attest success=false.
//                 DISPATCHED (fire-and-forget) has no verdict yet and is not attested.
//
// BRIT_NOTES_REMOTE=<remote> fetches refs/notes/brit/build/* first and pushes it after, so the
// notes outlive the CI workspace. Unset, the notes stay in the local clone.
//
// Never fails the build: every problem is a WARN on stderr and exit 0.

import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { arch, cpus, platform, totalmem } from 'node:os';
import { join } from 'node:path';
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

/** The Z.D deploy-service agent name for a pipeline. A name only: no key is provisioned. */
export function performerName(pipeline, operator = 'matthew') {
  return `agent:deploy-service-${operator}-${pipeline}`;
}

/**
 * One orchestrator run → the `build put` argument lists, plus what it skipped and why.
 * Pure: every read of the repository is passed in.
 *
 * @param {{commitSha?:string, executionOrder?:string[], results?:object}} graph
 * @param {{commit:string, tree:string, manifestBytes:(pipeline:string)=>Buffer|null,
 *   machine:object, operator?:string}} ctx
 */
export function planAttestations(graph, ctx) {
  const planned = [];
  const skipped = [];
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
    const record = {
      pipeline,
      commit: ctx.commit,
      result,
      buildNumber: r.buildNumber ?? null,
      url: r.url ?? null,
    };
    const hardware = {
      ...ctx.machine,
      performer: performerName(pipeline, ctx.operator),
      performer_key: 'unprovisioned',
    };
    planned.push({
      pipeline,
      args: [
        'build',
        'put',
        '--step',
        pipeline,
        '--manifest',
        rawCid(manifest),
        '--output',
        rawCid(Buffer.from(JSON.stringify(record))),
        '--inputs-hash',
        ctx.tree,
        '--success',
        String(SUCCEEDED.has(result)),
        '--duration-ms',
        String(Math.max(0, Math.round(Number(r.duration) || 0))),
        '--hardware',
        JSON.stringify(hardware),
        '--commit',
        ctx.commit,
      ],
    });
  }
  return { planned, skipped };
}

function git(repo, args, opts = {}) {
  return execFileSync('git', args, { cwd: repo, stdio: ['ignore', 'pipe', 'pipe'], ...opts });
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
  return {
    // BUILD_URL exists only inside a Jenkins build (JENKINS_URL is also set on dev workspaces).
    runner: env.BUILD_URL ? 'jenkins' : 'local',
    node: env.NODE_NAME ?? env.HOSTNAME ?? null,
    os: platform(),
    arch: arch(),
    cpus: cpus().length,
    mem_gb: Math.round(totalmem() / 2 ** 30),
  };
}

/** @returns {Promise<number>} always 0 — attestation is advisory until Stage 2. */
export async function main(argv, { env = process.env } = {}) {
  const [graphPath] = argv;
  const bin = env.BRIT_BUILD_REF_BIN;
  const repo = env.REPO_ROOT || process.cwd();
  if (!graphPath || !bin) {
    warn('usage: BRIT_BUILD_REF_BIN=<bin> brit-attest.mjs <actual-build-graph.json>');
    return 0;
  }
  let graph;
  try {
    graph = JSON.parse(readFileSync(graphPath, 'utf8'));
  } catch (e) {
    warn(`${graphPath} unreadable (${e.message}); nothing attested`);
    return 0;
  }
  let commit;
  let tree;
  try {
    commit = git(repo, ['rev-parse', `${graph.commitSha || env.GIT_COMMIT_HASH || 'HEAD'}^{commit}`]).toString().trim();
    tree = git(repo, ['rev-parse', `${commit}^{tree}`]).toString().trim();
  } catch (e) {
    warn(`commit ${graph.commitSha || 'HEAD'} not in this clone (${e.message.split('\n')[0]}); nothing attested`);
    return 0;
  }
  const { planned, skipped } = planAttestations(graph, {
    commit,
    tree,
    manifestBytes: manifestReader(repo, commit),
    machine: machine(env),
    operator: env.BRIT_DEPLOY_OPERATOR || 'matthew',
  });
  for (const s of skipped) console.error(`[brit-attest] skip ${s.pipeline}: ${s.why}`);
  if (planned.length === 0) return 0;

  const remote = env.BRIT_NOTES_REMOTE;
  const refspec = `${NOTES_PREFIX}*:${NOTES_PREFIX}*`;
  if (remote) {
    try {
      git(repo, ['fetch', '--quiet', remote, `+${refspec}`]);
    } catch (e) {
      warn(`fetch ${remote} ${NOTES_PREFIX}* failed (${e.message.split('\n')[0]}); attesting on the local notes`);
    }
  }
  let written = 0;
  for (const p of planned) {
    try {
      const cid = execFileSync(bin, ['--repo', repo, ...p.args], { stdio: ['ignore', 'pipe', 'pipe'] }).toString().trim();
      console.error(`[brit-attest] ${p.pipeline} ${p.args[p.args.indexOf('--success') + 1] === 'true' ? 'success' : 'failure'} → ${cid}`);
      written += 1;
    } catch (e) {
      warn(`build put ${p.pipeline} failed: ${(e.stderr?.toString() || e.message).trim().split('\n')[0]}`);
    }
  }
  if (remote && written > 0) {
    try {
      git(repo, ['push', '--quiet', remote, refspec]);
      console.error(`[brit-attest] pushed ${NOTES_PREFIX}* to ${remote}`);
    } catch (e) {
      warn(`push ${remote} ${NOTES_PREFIX}* failed (${e.message.split('\n')[0]}); attestations stay in this clone`);
    }
  }
  console.error(`[brit-attest] ${written}/${planned.length} pipelines attested on ${commit.slice(0, 12)}`);
  return 0;
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  main(process.argv.slice(2))
    .catch(e => warn(e.message))
    .finally(() => process.exit(0));
}
