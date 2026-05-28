#!/usr/bin/env node
/**
 * Registry vs Cluster Drift Detector
 *
 * For each pipeline in the build-manifest.json registry, probe its
 * Jenkins job and report:
 *   - registry-only: declared in manifests but no Jenkins job exists
 *   - cluster-only: Jenkins job exists but not in manifest registry (orphan)
 *
 * Exit codes:
 *   0 — no drift
 *   1 — drift detected
 *   2 — JENKINS_URL not set or fetch error
 *
 * Run: JENKINS_URL=... node registry-cluster-drift.mjs [--branch dev]
 *
 * Anonymous-read against the orchestrator's OIDC-fronted Jenkins; do not
 * send auth headers (would trigger OIDC redirect).
 */

import process from 'node:process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { loadPipelineRegistry } from '../pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../../..');
const registry = loadPipelineRegistry(ROOT);

const JENKINS_URL = process.env.JENKINS_URL;
if (!JENKINS_URL) { console.error('FATAL: JENKINS_URL must be set'); process.exit(2); }

const BRANCH = (() => {
  const i = process.argv.indexOf('--branch');
  return i >= 0 ? process.argv[i + 1] : 'dev';
})();

async function jobExists(name) {
  const res = await fetch(`${JENKINS_URL}/job/${name}/job/${BRANCH}/api/json?tree=name`);
  return res.status === 200;
}

async function listOrchestratorChildJobs() {
  // The orchestrator's downstream jobs are top-level Jenkins jobs prefixed `elohim-`.
  const res = await fetch(`${JENKINS_URL}/api/json?tree=jobs[name]`);
  if (!res.ok) throw new Error(`${res.status} on /api/json`);
  const data = await res.json();
  return (data.jobs || []).map(j => j.name).filter(n => n.startsWith('elohim-'));
}

async function main() {
  const registryNames = new Set([...registry.keys()]);
  const clusterNames = new Set(await listOrchestratorChildJobs());

  const registryOnly = [];
  for (const name of registryNames) {
    if (!(await jobExists(name))) registryOnly.push(name);
  }
  const clusterOnly = [...clusterNames].filter(n => !registryNames.has(n) && n !== 'elohim-orchestrator');

  console.log('# Registry vs Cluster Drift');
  console.log('');
  console.log(`Registry pipelines:      ${registryNames.size}`);
  console.log(`Cluster (elohim-*) jobs: ${clusterNames.size}`);
  console.log('');

  if (registryOnly.length === 0 && clusterOnly.length === 0) {
    console.log('✓ No drift — registry and cluster agree.');
    process.exit(0);
  }
  if (registryOnly.length > 0) {
    console.log('Registry-only (no Jenkins job on this branch):');
    for (const n of registryOnly) console.log(`  - ${n}`);
    console.log('');
  }
  if (clusterOnly.length > 0) {
    console.log('Cluster-only (Jenkins job exists but not in registry):');
    for (const n of clusterOnly) console.log(`  - ${n}`);
  }
  process.exit(1);
}

main().catch(e => { console.error('ERROR:', e.message); process.exit(2); });
