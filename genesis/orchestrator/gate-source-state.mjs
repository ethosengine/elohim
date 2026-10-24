// Native source closure is shared by receipts and eprfs gate evidence. JS never walks Cargo.
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { rawCid } from './scripts/brit-attest.mjs';

export function usesNativeClosure(project) {
  return project.dir === 'elohim/eprfs' || project.run?.cargo?.workspace === 'elohim/eprfs';
}

/** Fail closed for old evaluators, dirty closures, or a commit other than the checkout. */
export function nativeGateKey(project, { root, env = process.env, run = spawnSync, toolchain, commit = env.GATE_COMMIT || 'HEAD' }) {
  if (!toolchain) return null;
  const call = (bin, args) => {
    const result = run(bin, args, { cwd: root, env, encoding: 'utf8', timeout: 90_000, maxBuffer: 16 * 1024 * 1024 });
    if (result.error || result.status !== 0) throw new Error('source closure unavailable');
    return result.stdout;
  };
  try {
    /** @type {import('../../elohim/sdk/storage-client-ts/src/generated/source-state.js').SourceState} */
    const state = JSON.parse(call(env.EPR_BIN || 'epr', ['bin', 'source-state', '--tool', 'epr', '--repo', root, '--json']));
    if (state.clean !== true || state.status !== '' || !/^bafkrei[a-z2-7]{52}$/.test(state.closureCid)
      || !Array.isArray(state.closure) || !state.closure.length) return null;
    if (state.head !== call('git', ['rev-parse', `${commit}^{commit}`]).trim()) return null;
    const manifestPath = project.manifestPath || `${project.dir}/build-manifest.json`;
    const bytes = call('git', ['show', `${commit}:${manifestPath}`]);
    if (bytes !== readFileSync(resolve(root, manifestPath), 'utf8')) return null;
    return { closureCid: state.closureCid, toolchain, manifest: rawCid(Buffer.from(bytes)) };
  } catch { return null; }
}
