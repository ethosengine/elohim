// Gate evidence by input closure — F1 of the brit evidence-by-identity plan.
//
// A gate verdict is derived data. It is a function of the bytes the step declares as its
// inputs, the recipe that ran, and the environment the recipe ran under — never of the
// branch, the push range, or the checkout that happened to carry them. This module keys a
// project's gate run on exactly that closure and asks brit whether a signed attestation
// already exists for it (`brit-build-ref build find --inputs-hash`), then records the run's
// own verdict after it finishes (`build put`), so the next push whose closure is unchanged
// — a rebase, a docs-only commit, the integrator's fast-forward — can read the evidence
// instead of paying the recipe again (elohim-storage: 1875 s on average, measured).
//
// REPORT-ONLY in this slice: nothing here decides whether a gate runs. Every entry point is
// fail-open; a missing brit, a step without a fingerprint, or a refused git read all read as
// `unavailable (<reason>)`, and the gate runs as it always has.

import { spawnSync } from 'child_process';
import { createHash } from 'crypto';
import { accessSync, constants } from 'fs';
import { delimiter, isAbsolute, join, relative, resolve } from 'path';

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
  const r = spawn('git', ['rev-parse', `${commit}:${rel}`], { cwd: root, encoding: 'utf8' });
  if (r.status !== 0) return null;
  return r.stdout.trim() || null;
}

function sortedObject(value) {
  if (Array.isArray(value)) return value.map(sortedObject);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(k => [k, sortedObject(value[k])]));
  }
  return value;
}

/**
 * The closure key: sha256 over the canonical JSON of everything the verdict is a function
 * of. It is an index key into brit's `inputs_hash` field, not a content address of its own.
 */
export function closureKey({ project, fingerprints, manifestBlob: blob, cargoEnv }) {
  const material = sortedObject({
    v: 1,
    project: project.name,
    run: project.run,
    manifest: blob,
    steps: fingerprints,
    env: cargoEnv || {},
  });
  return createHash('sha256').update(JSON.stringify(material)).digest('hex');
}

/** `brit-build-ref build find` rows for this project's closure, or `{ error }`. */
export function findEvidence(project, key, { buildRefBin, root, spawn = spawnSync }) {
  const r = spawn(
    buildRefBin,
    ['build', 'find', '--step', attestationStep(project), '--inputs-hash', key, '--repo', root],
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

function ageOf(iso, now) {
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return 'age unknown';
  const s = Math.max(0, Math.round((now - then) / 1000));
  if (s < 90) return `${s}s ago`;
  if (s < 5400) return `${Math.round(s / 60)}m ago`;
  if (s < 172_800) return `${Math.round(s / 3600)}h ago`;
  return `${Math.round(s / 86_400)}d ago`;
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
  const fp = stepFingerprints(project, commit, { britBin, root, spawn });
  if (fp.error) return { line: `unavailable (${fp.error})` };
  const blob = manifestBlob(root, commit, project.manifestPath, spawn);
  if (!blob) return { line: `unavailable (manifest ${project.manifestPath} absent at ${String(commit).slice(0, 9)})` };
  const key = closureKey({ project, fingerprints: fp.fingerprints, manifestBlob: blob, cargoEnv });
  const found = findEvidence(project, key, { buildRefBin, root, spawn });
  if (found.error) return { key, line: `unavailable (${found.error})` };
  return { key, rows: found.rows, line: describeEvidence(found.rows, now), buildRefBin };
}

/**
 * Record this run's verdict against the closure: `build put` with the manifest file's own
 * CID, a stored verdict node as the output, and `--success` as observed (red is stored too —
 * a known-red closure is information). Returns the spawn result; never throws.
 */
export function recordEvidence(project, key, { success, seconds, commit, buildRefBin, root, jobs, spawn = spawnSync }) {
  const verdict = { exit: success ? 0 : 1, seconds: Math.round(seconds), ...(jobs ? { jobs } : {}) };
  const args = [
    'build', 'put',
    '--step', attestationStep(project),
    '--manifest-path', resolve(root, project.manifestPath),
    '--output-json', JSON.stringify(verdict),
    '--inputs-hash', key,
    '--success', success ? 'true' : 'false',
    '--duration-ms', String(Math.round(seconds * 1000)),
    '--commit', commit,
    '--repo', root,
  ];
  try {
    return spawn(buildRefBin, args, { cwd: root, encoding: 'utf8', timeout: 60_000 });
  } catch (error) {
    return { status: 1, stdout: '', stderr: String(error) };
  }
}
