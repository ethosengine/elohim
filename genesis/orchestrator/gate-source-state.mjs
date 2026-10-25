// Native source closure: key material for eprfs gate evidence (gate-evidence.mjs). JS never walks Cargo.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { rawCid } from './scripts/brit-attest.mjs';

export function usesNativeClosure(project) {
  return project.dir === 'elohim/eprfs' || project.run?.cargo?.workspace === 'elohim/eprfs';
}

/** An evaluator older than the verb is the device's miss, not the closure's. */
function evaluatorMiss(bin, result) {
  if (result.error) return `native evaluator ${bin} did not run (${result.error.code || result.error.message})`;
  const stderr = String(result.stderr || '');
  if (/unknown command `bin`/.test(stderr)) {
    return `native evaluator ${bin} predates \`epr bin source-state\`; set EPR_BIN to a bin-store build from dev`;
  }
  return `native evaluator ${bin} exited ${result.status}: ${stderr.trim().split('\n')[0] || 'no stderr'}`;
}

/** `{ key }` on a clean closure at the commit, else `{ miss }` naming why. Fails closed. */
export function nativeGateState(project, { root, env = process.env, run = spawnSync, toolchain, commit = env.GATE_COMMIT || 'HEAD' }) {
  if (!toolchain) return { miss: 'no toolchain declared' };
  const spawn = (bin, args) =>
    run(bin, args, { cwd: root, env, encoding: 'utf8', timeout: 90_000, maxBuffer: 16 * 1024 * 1024 });
  const git = args => {
    const result = spawn('git', args);
    return result.error || result.status !== 0 ? null : result.stdout;
  };
  const bin = env.EPR_BIN || 'epr';
  const evaluated = spawn(bin, ['bin', 'source-state', '--tool', 'epr', '--repo', root, '--json']);
  if (evaluated.error || evaluated.status !== 0) return { miss: evaluatorMiss(bin, evaluated) };
  let state;
  try {
    /** @type {import('../../elohim/sdk/storage-client-ts/src/generated/source-state.js').SourceState} */
    state = JSON.parse(evaluated.stdout);
  } catch { return { miss: `native evaluator ${bin} printed no source state` }; }
  if (state.clean !== true || state.status !== '' || !/^bafkrei[a-z2-7]{52}$/.test(state.closureCid)
    || !Array.isArray(state.closure) || !state.closure.length) return { miss: 'source closure dirty' };
  if (state.head !== git(['rev-parse', `${commit}^{commit}`])?.trim()) return { miss: 'source closure is not at the gated commit' };
  const manifestPath = project.manifestPath || `${project.dir}/build-manifest.json`;
  const bytes = git(['show', `${commit}:${manifestPath}`]);
  if (bytes === null || bytes !== readFileSync(resolve(root, manifestPath), 'utf8')) {
    return { miss: `${manifestPath} differs from the gated commit` };
  }
  return { key: { closureCid: state.closureCid, toolchain, manifest: rawCid(Buffer.from(bytes)) } };
}

/** The key alone; null on any miss. */
export function nativeGateKey(project, options) {
  return nativeGateState(project, options).key ?? null;
}
