// Gate evidence by input closure — F1 of the brit evidence-by-identity plan.
//
// A gate verdict is derived data. It is a function of declared inputs and repository reads,
// the recipe that ran, and the environment the recipe ran under — never of the
// branch, the push range, or the checkout that happened to carry them. This module keys a
// project's gate run on exactly that closure and asks brit whether a signed attestation
// already exists for it (`brit-build-ref build find --inputs-hash`), then records the run's
// own verdict after it finishes (`build put`), so the next push whose closure is unchanged
// — a rebase, a docs-only commit, the integrator's fast-forward — can read the evidence
// instead of paying the recipe again (elohim-storage: 1875 s on average, measured).
//
// Reuse is opt-in and accepts only this workspace's newest green attestation. Evidence
// reads are fail-open; a missing brit, fingerprint, or required git object reads as
// `unavailable (<reason>)`, and the gate runs as it always has.

import { nativeGateKey, nativeGateState, usesNativeClosure } from './gate-source-state.mjs';
import { spawnSync } from 'child_process';
import { createHash, createPrivateKey, createPublicKey } from 'crypto';
import { accessSync, constants, readFileSync } from 'fs';
import { delimiter, isAbsolute, join, relative, resolve } from 'path';
import { globCovers, readSet } from './gate-input-audit.mjs';

/** The attestation step name a gate PROJECT writes under: one closure per project, not per manifest step. */
export function attestationStep(project) {
  return `gate:${project.name}`;
}

function isExecutable(path) {
  try {
    accessSync(path, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/**
 * The first executable among the named env keys, else `<name>` on PATH, else null.
 * `BRIT_BUILD_REF` is accepted beside `BRIT_BUILD_REF_BIN` because a2o's household
 * attestation already resolves the same binary by that name.
 */
export function resolveBin(name, envKeys, env = process.env) {
  for (const key of envKeys) {
    const declared = (env[key] || '').trim();
    if (!declared) continue;
    const path = isAbsolute(declared) ? declared : join(process.cwd(), declared);
    return isExecutable(path) ? path : null;
  }
  for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, name);
    if (isExecutable(candidate)) return candidate;
  }
  return null;
}

/**
 * `brit build fingerprint <manifest> --step <s> --commit <c>` per declared step of the gate
 * project. A project that declares only gate `inputs` (no manifest steps) has no fingerprint
 * yet — that is reported, not guessed.
 */
export function stepFingerprints(project, commit, { britBin, root, spawn = spawnSync }) {
  if (!project.steps || project.steps.length === 0) {
    return { error: 'gate-only inputs, no step fingerprint' };
  }
  const manifest = resolve(root, project.manifestPath);
  const out = [];
  for (const step of project.steps) {
    const r = spawn(britBin, ['build', 'fingerprint', manifest, '--step', step, '--commit', commit], {
      cwd: root,
      encoding: 'utf8',
      timeout: 60_000,
    });
    if (r.status !== 0) {
      return { error: `fingerprint of ${step} failed: ${(r.stderr || '').trim().split('\n').pop() || `exit ${r.status}`}` };
    }
    let parsed;
    try {
      parsed = JSON.parse(r.stdout);
    } catch {
      return { error: `fingerprint of ${step}: not JSON` };
    }
    const row = (parsed.fingerprints || []).find(f => f.step === step);
    if (!row || !row.fingerprint) return { error: `fingerprint of ${step}: absent from output` };
    out.push({ step, fingerprint: row.fingerprint });
  }
  return { fingerprints: out };
}

/** The manifest file's blob at the commit: a recipe or input-glob edit moves the key. */
export function manifestBlob(root, commit, manifestPath, spawn = spawnSync) {
  const rel = relative(root, resolve(root, manifestPath)).split('\\').join('/');
  return gitRead(root, ['rev-parse', `${commit}:${rel}`], spawn)?.trim() || null;
}

function gitRead(root, args, spawn) {
  try {
    const r = spawn('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
    return r.status === 0 ? r.stdout : null;
  } catch {
    return null;
  }
}

function recipeAt(project, root, commit, spawn) {
  // run-local-gate.sh selects the project justfile for `just`, the root for `root-just`.
  // Evidence falls back to the root when the project's justfile is absent at the commit.
  const path = project.run.kind === 'root-just' ? 'justfile' : join(project.dir, 'justfile');
  const blob = manifestBlob(root, commit, path, spawn);
  if (blob || path === 'justfile') return { path, blob };
  return { path: 'justfile', blob: manifestBlob(root, commit, 'justfile', spawn) };
}

function stepSources(project, root, commit, spawn) {
  try {
    const path = relative(root, resolve(root, project.manifestPath)).split('\\').join('/');
    const manifest = JSON.parse(gitRead(root, ['show', `${commit}:${path}`], spawn));
    const sources = project.steps.flatMap(step => manifest.steps[step].inputs.sources);
    return sources.every(source => typeof source === 'string') ? sources : null;
  } catch {
    return null;
  }
}

// These scans exceed two seconds on the larger Rust gates. Retain only process-local
// results, partitioned by repository, project and commit (and tree for moving refs).
const readSets = new Map();

function readSetAt(project, root, commit, sources, tree, spawn) {
  if (!project.run.cargo || !sources) return null;
  const key = JSON.stringify([resolve(root), project.name, commit, tree]);
  if (readSets.has(key)) return readSets.get(key);
  try {
    const names = gitRead(root, ['ls-tree', '-r', '--name-only', '-z', commit], spawn);
    if (names === null) return null;
    const files = new Set(names.split('\0').filter(Boolean));
    if (![...files].some(file => file.endsWith('.rs') && sources.some(source => globCovers(source, file)))) return null;
    const contents = new Map();
    const reads = readSet({
      root, project, sources, listFiles: () => [...files],
      readFile: path => {
        const rel = relative(root, path).split('\\').join('/');
        if (!files.has(rel)) throw Object.assign(new Error(`absent at ${commit}: ${rel}`), { code: 'ENOENT' });
        if (!contents.has(rel)) {
          const text = gitRead(root, ['show', `${commit}:${rel}`], spawn);
          if (text === null) throw new Error(`unreadable at ${commit}: ${rel}`);
          contents.set(rel, text);
        }
        return contents.get(rel);
      },
    });
    readSets.set(key, reads);
    return reads;
  } catch {
    return null;
  }
}

function gitlinksAt(root, commit, sources, spawn) {
  if (!sources) return null;
  const tree = gitRead(root, ['ls-tree', '-r', commit], spawn);
  if (tree === null) return null;
  return tree.split('\n').flatMap(line => {
    const match = /^160000 commit (\S+)\t(.+)$/.exec(line);
    if (!match) return [];
    const [, sha, path] = match;
    // Deliberately simple: a/b/** matches the a/b/ prefix; all other globs are exact paths.
    return sources.some(source => source.endsWith('/**') ? path.startsWith(source.slice(0, -2)) : path === source)
      ? [{ path, sha }] : [];
  }).sort((a, b) => a.path.localeCompare(b.path));
}

function rustcHash(project, root, env, cargoEnv, spawn) {
  try {
    const r = spawn('rustc', ['-vV'], {
      cwd: resolve(root, project.dir), encoding: 'utf8', env: { ...env, ...cargoEnv },
    });
    return r.status === 0 ? createHash('sha256').update(r.stdout).digest('hex') : null;
  } catch {
    return null;
  }
}

/** brit-build-ref's signing seed is shared by every worktree of this repository. */
export function selfAgentId(root, spawn = spawnSync) {
  try {
    const common = gitRead(root, ['rev-parse', '--git-common-dir'], spawn)?.trim();
    if (!common) return null;
    const seed = readFileSync(resolve(root, common, 'brit/agent-key'));
    if (seed.length !== 32) return null;
    const key = Buffer.concat([Buffer.from('302e020100300506032b657004220420', 'hex'), seed]);
    const priv = createPrivateKey({ key, format: 'der', type: 'pkcs8' });
    const pub = createPublicKey(priv).export({ format: 'jwk' });
    return Buffer.from(pub.x, 'base64url').toString('hex');
  } catch {
    return null;
  }
}

function sortedObject(value) {
  if (Array.isArray(value)) return value.map(sortedObject);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(k => [k, sortedObject(value[k])]));
  }
  return value;
}

/**
 * The closure material: the canonical JSON (sorted keys, no whitespace) of everything the
 * verdict is a function of. A gate run as an ark Task names these exact bytes as its input
 * (`ark task spec --material`), where they become an eprfs tree; CID math stays in Rust.
 */
export function closureMaterial({ project, fingerprints, manifestBlob: blob, cargoEnv, nativeKey, recipeFile = null, runner = null, dirTree = null, cargoConfig = null, rustc = null, gitlinks = null, packages = [], reads = null }) {
  return JSON.stringify(sortedObject({
    v: 3,
    ...(nativeKey ? { native: nativeKey } : {}),
    project: project.name,
    run: project.run,
    manifest: blob,
    steps: fingerprints,
    env: cargoEnv || {},
    recipeFile,
    runner,
    dirTree,
    cargoConfig,
    rustc,
    gitlinks,
    packages,
    reads,
  }));
}

/**
 * The closure key: sha256 over the closure material. It is an index key into brit's
 * `inputs_hash` field, not a content address of its own.
 */
export function closureKey(closure) {
  return createHash('sha256').update(closureMaterial(closure)).digest('hex');
}

/**
 * Paths under the project's dir, manifest, declared inputs, reads and recipe that are modified,
 * staged, or untracked. The closure key names the COMMIT's bytes; the recipe runs
 * against the WORKING TREE. When these differ, a verdict must not be recorded against the
 * commit, and a hit for the commit says nothing about the bytes about to be gated.
 */
export function dirtyPaths(project, { root, spawn = spawnSync, commit = 'HEAD', sources = stepSources(project, root, commit, spawn), recipePath = recipeAt(project, root, commit, spawn).path, reads = { scope: 'tree' } }) {
  if (!sources) return null;
  const paths = [project.dir, project.manifestPath, ...sources.map(source => source.endsWith('/**') ? source.slice(0, -2) : source), recipePath,
    ...(reads.scope === 'paths' ? reads.oids.map(row => row.path) : ['.'])];
  const out = gitRead(root, ['status', '--porcelain', '--untracked-files=normal', '--', ...new Set(paths)], spawn);
  if (out === null) return null;
  return out.split('\n').map(line => line.slice(3).trim()).filter(Boolean);
}

/** `brit-build-ref build find` rows for this project's closure, or `{ error }`. */
export function findEvidence(project, key, { buildRefBin, root, spawn = spawnSync }) {
  // `--repo` is a top-level option of brit-build-ref: it precedes the subcommand.
  const r = spawn(
    buildRefBin,
    ['--repo', root, 'build', 'find', '--step', attestationStep(project), '--inputs-hash', key],
    { cwd: root, encoding: 'utf8', timeout: 60_000 }
  );
  if (r.status !== 0) return { error: `build find failed: ${(r.stderr || '').trim().split('\n').pop() || `exit ${r.status}`}` };
  try {
    const rows = JSON.parse(r.stdout);
    return Array.isArray(rows) ? { rows } : { error: 'build find: not an array' };
  } catch {
    return { error: 'build find: not JSON' };
  }
}

export function ageOf(iso, now = Date.now()) {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return 'age unknown';
  const s = Math.max(0, Math.round((now - then) / 1000));
  if (s < 90) return `${s}s ago`;
  if (s < 5400) return `${Math.round(s / 60)}m ago`;
  if (s < 172_800) return `${Math.round(s / 3600)}h ago`;
  return `${Math.round(s / 86_400)}d ago`;
}

/** A green older than a red is not a reusable verdict, even if the report shows it. */
export function reusableHit(evidence, { selfId, enabled, mode, selectorCheck }) {
  if (mode !== 'reuse') return { reuse: false, why: 'report-only' };
  if (!enabled) return { reuse: false, why: 'project not enabled for reuse' };
  if (selfId == null) return { reuse: false, why: 'no workspace key' };
  const row = [...(evidence.rows || [])].sort(
    (a, b) => Date.parse(b.attestation?.builtAt || 0) - Date.parse(a.attestation?.builtAt || 0)
  )[0];
  if (!row) return { reuse: false, why: 'no attestation for this closure yet' };
  if (row.attestation?.success !== true) return { reuse: false, why: 'newest attestation is red' };
  if (row.attestation.agentId !== selfId) return { reuse: false, why: `signer ${String(row.attestation.agentId || 'unsigned').slice(0, 12)} is not this workspace` };
  if (evidence.dirty !== false) return { reuse: false, why: evidence.dirty === true ? 'declared inputs dirty' : 'working tree state unknown' };
  const selected = selectorCheck(row.commit);
  if (!selected.ok) return { reuse: false, why: selected.message };
  return { reuse: true, row };
}

/**
 * One line a person can read: the newest green attestation first, else the newest red,
 * each with who signed it and how old it is. A miss is a miss.
 */
export function describeEvidence(rows, now = Date.now()) {
  if (!rows || rows.length === 0) return 'miss';
  const byNewest = [...rows].sort(
    (a, b) => Date.parse(b.attestation?.builtAt || 0) - Date.parse(a.attestation?.builtAt || 0)
  );
  const green = byNewest.find(r => r.attestation?.success === true);
  const pick = green || byNewest[0];
  const a = pick.attestation || {};
  const signer = String(a.agentId || 'unsigned').slice(0, 12);
  const verdict = a.success === true ? 'green' : 'red';
  const more = rows.length > 1 ? `, ${rows.length} attestations` : '';
  return `hit ${verdict} (signer ${signer}, ${ageOf(a.builtAt, now)}, commit ${String(pick.commit || '').slice(0, 9)}${more})`;
}

/**
 * Resolve the closure and look for evidence. Returns `{ key, rows, line }` on a resolved
 * closure (rows may be empty) or `{ line }` alone when the closure is unavailable.
 */
export function gateEvidence(project, { env = process.env, root, commit, cargoEnv, spawn = spawnSync, now = Date.now() } = {}) {
  if (project.run?.kind === 'attested') return { line: 'unavailable (attested project, no local recipe)' };
  const britBin = resolveBin('brit', ['BRIT_BIN'], env);
  const buildRefBin = resolveBin('brit-build-ref', ['BRIT_BUILD_REF_BIN', 'BRIT_BUILD_REF'], env);
  if (!britBin || !buildRefBin) {
    return { line: `unavailable (${!britBin ? 'brit' : 'brit-build-ref'} not on BRIT_BIN/BRIT_BUILD_REF_BIN or PATH)` };
  }
  const native = usesNativeClosure(project)
    ? nativeGateState(project, { root, env, run: spawn, toolchain: cargoEnv?.RUSTUP_TOOLCHAIN, commit }) : undefined;
  if (native?.miss) return { line: `unavailable (${native.miss})`, dirty: true };
  const nativeKey = native?.key;
  const fp = stepFingerprints(project, commit, { britBin, root, spawn });
  if (fp.error) return { line: `unavailable (${fp.error})` };
  const blob = manifestBlob(root, commit, project.manifestPath, spawn);
  if (!blob) return { line: `unavailable (manifest ${project.manifestPath} absent at ${String(commit).slice(0, 9)})` };
  const recipe = recipeAt(project, root, commit, spawn);
  const runner = manifestBlob(root, commit, 'genesis/orchestrator/run-local-gate.sh', spawn);
  const dirTree = project.dir === '.'
    ? gitRead(root, ['rev-parse', `${commit}^{tree}`], spawn)?.trim() || null
    : manifestBlob(root, commit, project.dir, spawn);
  for (const [piece, oid] of Object.entries({ recipeFile: recipe.blob, runner, dirTree })) {
    if (!oid) return { line: `unavailable (${piece} absent at ${String(commit).slice(0, 9)})` };
  }
  const sources = stepSources(project, root, commit, spawn);
  const tree = gitRead(root, ['rev-parse', `${commit}^{tree}`], spawn)?.trim() || null;
  if (!tree) return { line: `unavailable (read tree absent at ${String(commit).slice(0, 9)})` };
  const read = readSetAt(project, root, commit, sources, tree, spawn);
  // Deliberately conservative tier for gates whose reads cannot be named: eprfs
  // walks the repository and JS gates read many manifests. Require identical
  // repository trees and check all checkout dirt, including hook-written ledgers.
  const reads = read && read.unresolved.length === 0
    ? { scope: 'paths', oids: read.paths.map(path => ({ path, oid: manifestBlob(root, commit, path, spawn) })) }
    : { scope: 'tree', oid: tree };
  const scope = reads.scope === 'paths' ? `paths (${read.paths.length} reads)`
    : `tree (${read ? 'unresolved' : 'unknown'} reads)`;
  const material = closureMaterial({
    project, fingerprints: fp.fingerprints, manifestBlob: blob, cargoEnv, nativeKey,
    recipeFile: recipe.blob, runner, dirTree,
    cargoConfig: manifestBlob(root, commit, '.cargo/config.toml', spawn),
    rustc: rustcHash(project, root, env, cargoEnv, spawn),
    gitlinks: gitlinksAt(root, commit, sources, spawn),
    packages: (env.GATE_CARGO_PACKAGES || '').trim().split(/\s+/).filter(Boolean).sort(),
    reads,
  });
  const key = createHash('sha256').update(material).digest('hex');
  const found = findEvidence(project, key, { buildRefBin, root, spawn });
  if (found.error) return { key, material, line: `unavailable (${found.error}); closure scope ${scope}` };
  const dirty = dirtyPaths(project, { root, spawn, sources, recipePath: recipe.path, reads });
  let line = `${describeEvidence(found.rows, now)}; closure scope ${scope}`;
  if (dirty === null) line += '; working tree state unknown, nothing will be recorded';
  else if (dirty.length > 0) line += `; working tree dirty under ${project.dir} (${dirty.length} path${dirty.length === 1 ? '' : 's'}): the evidence names the commit, not these bytes, and nothing will be recorded`;
  const checkoutMismatch = env.GATE_EVIDENCE_RECORD === '0';
  if (checkoutMismatch) line += '; pushed commit is not checked out, nothing will be recorded';
  return {
    key,
    material,
    rows: found.rows,
    line,
    buildRefBin,
    nativeKey,
    dirty: checkoutMismatch ? true : dirty === null ? null : dirty.length > 0,
  };
}

/**
 * Record this run's verdict against the closure: `build put` with the manifest file's own
 * CID, a stored verdict node as the output, and `--success` as observed (red is stored too —
 * a known-red closure is information). Returns the spawn result; never throws.
 */
export function recordEvidence(project, key, { success, seconds, commit, buildRefBin, root, jobs, dirty, nativeKey, env = process.env, cargoEnv, spawn = spawnSync }) {
  if (dirty !== false) {
    return { status: 1, stdout: '', stderr: dirty === null ? 'working tree state unknown' : `working tree dirty under ${project.dir}` };
  }
  if (usesNativeClosure(project)) {
    const after = nativeGateKey(project, { root, env, run: spawn, toolchain: cargoEnv?.RUSTUP_TOOLCHAIN, commit });
    if (!nativeKey || !after || JSON.stringify(nativeKey) !== JSON.stringify(after)) {
      return { status: 1, stdout: '', stderr: 'source closure dirty, changed, or unavailable after gate' };
    }
  }
  const verdict = { exit: success ? 0 : 1, seconds: Math.round(seconds), ...(jobs ? { jobs } : {}) };
  const args = [
    '--repo', root,
    'build', 'put',
    '--step', attestationStep(project),
    '--manifest-path', resolve(root, project.manifestPath),
    '--output-json', JSON.stringify(verdict),
    '--inputs-hash', key,
    '--success', success ? 'true' : 'false',
    '--duration-ms', String(Math.round(seconds * 1000)),
    '--commit', commit,
  ];
  try {
    return spawn(buildRefBin, args, { cwd: root, encoding: 'utf8', timeout: 60_000 });
  } catch (error) {
    return { status: 1, stdout: '', stderr: String(error) };
  }
}
