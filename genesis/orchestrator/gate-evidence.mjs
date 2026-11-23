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
// Reuse is opt-in, and gate-runner.mjs's `decideReuse` is its one decision for the Task path and
// the direct path. The sibling channel is asked first: an admitted sibling's red for the same
// closure runs the gate before this workspace's newest green is considered; a sibling green stands
// in only when this workspace has none, under D5's red, dirt, selector and governance fences. Just
// before a local green is accepted its rows are read again (a newer local red refuses it) and its
// attested output must parse as a CID. Evidence reads are fail-open toward running: a missing
// brit, fingerprint or required git object runs the gate. A failed sibling-channel read is an
// empty candidate list, so a local green stays eligible, as before E11.
//
// Every read here is brit's VERIFIED read (`build find`, `build get`, `signer`): stdout holds
// only what brit verified; a stored note that fails verification is `unverifiable: <commit>
// <reason>` on stderr at exit 0. Such a note is absent to this reader and is named once on the
// warn stream with its ref and reason, never counted as a hit and never dropped silently. The
// writer reads the commit's note before `build put`, and brit's two refusals (`REFUSED
// concurrent update`, `refusing to write`) are expected outcomes, never a gate failure.

import { nativeGateKey, nativeGateState, usesNativeClosure } from './gate-source-state.mjs';
import { spawnSync } from 'child_process';
import { createHash, createPrivateKey, createPublicKey } from 'crypto';
import { accessSync, constants, readFileSync } from 'fs';
import { basename, delimiter, dirname, isAbsolute, join, relative, resolve } from 'path';
import { SUITE_READ_LISTS, UNAUDITED_RECIPE_PROJECTS, globCovers, isReadPattern, readSet, suiteReadSet } from './gate-input-audit.mjs';
import { closureDigest, dirtyInView, gitEnv, packageClosure, suiteInventory } from './suite-inventory.mjs';
import { buildNotesRef, expectedRefusal, heldNotes, noteEpoch, readBuildNote, unverifiableNotes } from './scripts/brit-attest.mjs';
import { LEGACY_ALL, refusesInputsFlag } from './gate-inputs.mjs';

/** The attestation step name a gate PROJECT writes under: one closure per project, not per manifest step. */
export function attestationStep(project) {
  return `gate:${project.name}`;
}

/** The notes ref a gate project's attestations live under (brit's segment encoding). */
export function attestationRef(project) {
  return buildNotesRef(attestationStep(project));
}

const BUILD_REF_ENV = ['BRIT_BUILD_REF_BIN', 'BRIT_BUILD_REF'];
const stderrWarn = line => process.stderr.write(line);

/** A withheld gate has no reusable verdict writer; its policy row is the user-facing reason. */
export function gateReuseWithheldReason(project, root) {
  try {
    const policy = JSON.parse(readFileSync(resolve(root, 'genesis/agentic/pool-policy.json'), 'utf8'));
    const reason = policy?.gate_reuse?.withheld?.[project.name];
    return typeof reason === 'string' && reason.trim() ? reason : null;
  } catch {
    return null;
  }
}

/** No key means no verdict can be written; the line names why, from the policy when it says. */
export function noKeyLine(error, project, root) {
  const withheld = gateReuseWithheldReason(project, root);
  return `unavailable (${error})${withheld ? `; not recorded — ${withheld}` : ''}`;
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
 * project, followed by the input class's `britArgs` (gate-inputs.mjs; LEGACY_ALL passes none, so
 * the call is today's byte for byte). A project that declares only gate `inputs` (no manifest
 * steps) has no fingerprint yet — that is reported, not guessed. A failure for want of the class's
 * flag says so (`refusedFlag`).
 */
export function stepFingerprints(project, commit, { britBin, root, spawn = spawnSync, inputs = LEGACY_ALL }) {
  if (!project.steps || project.steps.length === 0) {
    return { error: 'gate-only inputs, no step fingerprint' };
  }
  const manifest = resolve(root, project.manifestPath);
  const out = [];
  for (const step of project.steps) {
    const r = spawn(britBin, ['build', 'fingerprint', manifest, '--step', step, '--commit', commit, ...inputs.britArgs], {
      cwd: root,
      encoding: 'utf8',
      timeout: 60_000,
    });
    if (r.status !== 0) {
      return {
        error: `fingerprint of ${step} failed: ${(r.stderr || '').trim().split('\n').pop() || `exit ${r.status}`}`,
        refusedFlag: inputs.britArgs.length > 0 && refusesInputsFlag(r),
      };
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

/**
 * A habit atom (`<dir>/…/.epr-meta/<id>.habit.md`) is a habit's evidence ledger. No gate recipe
 * reads one, and a DELTA line in one is the deliverable every finished lane writes, so keying a
 * project's tree on it made a docs-only DELTA re-run that crate's whole gate (elohim-storage,
 * 25–86 min; backlog gate-key-dirtree-rekeys-on-habit-atom-edit). A gate that does read habit
 * atoms names them in its audited reads, or keys on the whole commit tree; both stay in the
 * material independently of this.
 */
export const isHabitAtom = path => /(^|\/)\.epr-meta\/[^/]+\.habit\.md$/.test(path);

/**
 * The project directory's tree at `commit`, minus its habit atoms. With no atom to drop it is the
 * plain tree OID, so keys for such directories are unchanged; with atoms it is a digest of the
 * remaining `<mode> <type> <oid>\t<path>` entries. Null when the directory is absent; an unreadable
 * listing keeps the plain tree OID (conservative: the atoms stay in the key).
 */
export function projectTree(root, commit, dir, spawn = spawnSync) {
  const oid = manifestBlob(root, commit, dir, spawn);
  if (!oid) return null;
  const rel = relative(root, resolve(root, dir)).split('\\').join('/');
  const listing = gitRead(root, ['ls-tree', '-r', '-z', commit, '--', rel], spawn);
  if (listing === null) return oid;
  const entries = listing.split('\0').filter(Boolean);
  const kept = entries.filter(entry => !isHabitAtom(entry.slice(entry.indexOf('\t') + 1)));
  if (kept.length === entries.length) return oid;
  return `tree-minus-habit-atoms:${createHash('sha256').update(kept.sort().join('\n')).digest('hex')}`;
}

/**
 * One digest for a `**\/<name>` read: sha256 over the sorted `<blob oid> <path>` lines of every
 * tracked file at `commit` whose basename is `<name>`. Null when the listing cannot be read.
 */
export function patternDigest(root, commit, pattern, spawn = spawnSync) {
  const listing = gitRead(root, ['ls-tree', '-r', '-z', commit], spawn);
  if (listing === null) return null;
  const name = pattern.slice(3);
  const lines = listing.split('\0').filter(Boolean).flatMap(entry => {
    const tab = entry.indexOf('\t');
    const [, type, oid] = entry.slice(0, tab).split(' ');
    const path = entry.slice(tab + 1);
    return type === 'blob' && (path === name || path.endsWith(`/${name}`)) ? [`${oid} ${path}`] : [];
  }).sort();
  return createHash('sha256').update(lines.join('\n')).digest('hex');
}

function gitRead(root, args, spawn) {
  try {
    const r = spawn('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
    return r.status === 0 ? r.stdout : null;
  } catch {
    return null;
  }
}

/** The full sha `rev` names (`HEAD`, a short sha, a ref), or `rev` itself when git cannot say. */
function resolvedCommit(root, rev, spawn) {
  const sha = gitRead(root, ['rev-parse', '--verify', '--quiet', `${rev}^{commit}`], spawn)?.trim();
  return sha && /^[0-9a-f]{40,64}$/.test(sha) ? sha : rev;
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
  // A suite that declares its reads, and runs where only those bytes exist, keys on its list at the commit.
  const list = SUITE_READ_LISTS[project.name];
  if (list) {
    const text = gitRead(root, ['show', `${commit}:${list}`], spawn);
    return text === null ? null : suiteReadSet(text, list);
  }
  if (!project.run.cargo || !sources || UNAUDITED_RECIPE_PROJECTS.includes(project.name)) return null;
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

// The pinned brit (76ba95d8a0) has no `signer` verb: clap answers with this usage error, exit 2.
const SIGNER_UNKNOWN = /^error: unrecognized subcommand 'signer'/;

/**
 * The signer-preference rule, owned HERE for every orchestrator reader (delivery-series.mjs
 * delegates; genesis/a2o/scripts/lib/household-attestation.ts `resolveWorkspaceIdentity` is the
 * a2o-side port of the same rule, kept separate only because that module runs under tsx and
 * node type-stripping with node builtins alone — its test pins the same streams).
 *
 * `brit-build-ref signer --json` is the authority (it loads the key exactly as `build put` would
 * and never mints one): its `{"agentId": …}` is the answer, null included, and a brit that
 * cannot load the key (exit 1) or prints no JSON answers null — never second-guessed by reading
 * the file. The seed read under `<git-common-dir>/brit/agent-key` stands in only when no
 * brit-build-ref resolves or the resolved one predates the verb (the pin before 7e4a83115e,
 * clap usage error exit 2), so the answer is the same on either side of the pin move.
 *
 * @returns {{agentId: string|null, via: 'signer'|'agent-key'}} `via` names who answered.
 */
export function signerIdentity(root, spawn = spawnSync, { env = process.env, buildRefBin = resolveBin('brit-build-ref', BUILD_REF_ENV, env) } = {}) {
  if (buildRefBin) {
    let r = null;
    try {
      r = spawn(buildRefBin, ['--repo', root, 'signer', '--json'], { cwd: root, encoding: 'utf8', timeout: 15_000, env });
    } catch {
      r = null;
    }
    if (r && r.status === 0) {
      try {
        const id = JSON.parse(r.stdout).agentId;
        return { agentId: typeof id === 'string' && /^[0-9a-f]{64}$/.test(id) ? id : null, via: 'signer' };
      } catch {
        return { agentId: null, via: 'signer' };
      }
    }
    const predatesVerb = r?.status === 2 && SIGNER_UNKNOWN.test(String(r.stderr || '').trim());
    if (r && !predatesVerb) return { agentId: null, via: 'signer' };
  }
  return { agentId: seedAgentId(root, spawn), via: 'agent-key' };
}

/** The hex ed25519 public key brit-build-ref signs this repository's attestations with (see `signerIdentity`). */
export function selfAgentId(root, spawn = spawnSync, options = {}) {
  return signerIdentity(root, spawn, options).agentId;
}

/** The public key derived from the seed at `<git-common-dir>/brit/agent-key`, or null. Pre-`signer` brit only. */
function seedAgentId(root, spawn) {
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

// Shared compiler runners are execution inputs, never CI dispatch sources. pool-lib.sh is sourced
// by run-local-gate.sh for every cargo gate and resolves the slot it builds in (row 28).
export const COMPILER_GATE_INPUTS = [
  'genesis/orchestrator/cargo-lint.mjs',
  'genesis/orchestrator/gate-steps.sh',
  'genesis/agentic/bin/pool-lib.sh',
];

/**
 * The closure material: the canonical JSON (sorted keys, no whitespace) of everything the
 * verdict is a function of. A gate run as an ark Task names these exact bytes as its input
 * (`ark task spec --material`), where they become an eprfs tree; CID math stays in Rust.
 */
export function closureMaterial({ project, fingerprints, manifestBlob: blob, cargoEnv, nativeKey, compilerTools = null, recipeFile = null, runner = null, dirTree = null, cargoConfig = null, rustc = null, gitlinks = null, packages = [], reads = null, suite = null }) {
  return JSON.stringify(sortedObject({
    v: 3,
    ...(nativeKey ? { native: nativeKey } : {}),
    project: project.name,
    run: project.run,
    manifest: blob,
    steps: fingerprints,
    env: cargoEnv || {},
    recipeFile,
    compilerTools,
    runner,
    dirTree,
    cargoConfig,
    rustc,
    gitlinks,
    packages,
    reads,
    // Only a suite project carries this, so no other gate's key changes.
    ...(suite ? { suite } : {}),
  }));
}

/**
 * The closure key: sha256 over the closure material. It is an index key into brit's
 * `inputs_hash` field, not a content address of its own.
 */
export function closureKey(closure) {
  return createHash('sha256').update(closureMaterial(closure)).digest('hex');
}

// Machine-written observations, never recipe inputs. Keep this promise beside the
// dirty-tree decision; gate-input-audit.test.mjs checks every registered gate.
// Cursors are the harvesters' progress, not configuration (other JSON stays input).
export const GATE_EXHAUST = Object.freeze([
  '.claude/data/*.jsonl',
  '.claude/data/ci-cursor.json',
  '.claude/data/governance-cursor.json',
  '.claude/data/runtime-cursor.json',
]);

/**
 * Paths under the project's dir, manifest, declared inputs, reads and recipe that are modified,
 * staged, or untracked. The closure key names the COMMIT's bytes; the recipe runs
 * against the WORKING TREE. When these differ, a verdict must not be recorded against the
 * commit, and a hit for the commit says nothing about the bytes about to be gated.
 */
export function dirtyPaths(project, { root, spawn = spawnSync, commit = 'HEAD', sources = stepSources(project, root, commit, spawn), recipePath = recipeAt(project, root, commit, spawn).path, reads = { scope: 'tree' } }) {
  if (!sources) return null;
  // A basename read (`**\/.epr-meta`) is a glob pathspec, so a NEW untracked match anywhere is dirt too.
  const pathspec = path => (isReadPattern(path) ? `:(glob)${path}` : path);
  const paths = [...COMPILER_GATE_INPUTS, 'genesis/orchestrator/run-local-gate.sh', project.dir, project.manifestPath, ...sources.map(source => source.endsWith('/**') ? source.slice(0, -2) : source), recipePath,
    ...(reads.scope === 'paths' ? reads.oids.map(row => pathspec(row.path)) : ['.'])];
  const out = gitRead(root, ['status', '--porcelain', '--untracked-files=normal', '--', ...new Set(paths),
    ...GATE_EXHAUST.map(path => `:(top,glob,exclude)${path}`)], spawn);
  if (out === null) return null;
  return out.split('\n').map(line => line.slice(3).trim()).filter(Boolean);
}

/**
 * `brit-build-ref build find` for this project's closure: `{ rows, unverifiable }` or `{ error }`.
 * `rows` are the attestations brit verified (stdout). Each note brit could not verify under this
 * step's ref — whatever closure it claims — is listed in `unverifiable` and named once on `warn`
 * with the ref, commit and brit's reason; it is never a row. A non-zero exit is a failed read.
 */
export function findEvidence(project, key, { buildRefBin, root, spawn = spawnSync, warn = stderrWarn }) {
  // `--repo` is a top-level option of brit-build-ref: it precedes the subcommand.
  const r = spawn(
    buildRefBin,
    ['--repo', root, 'build', 'find', '--step', attestationStep(project), '--inputs-hash', key],
    { cwd: root, encoding: 'utf8', timeout: 60_000 }
  );
  if (r.status !== 0) return { error: `build find failed: ${(r.stderr || '').trim().split('\n').pop() || `exit ${r.status}`}` };
  let rows;
  try {
    rows = JSON.parse(r.stdout);
  } catch {
    return { error: 'build find: not JSON' };
  }
  if (!Array.isArray(rows)) return { error: 'build find: not an array' };
  const unverifiable = unverifiableNotes(r.stderr);
  for (const note of unverifiable) {
    warn(`[gate] ${project.name} evidence: unverifiable note on ${attestationRef(project)} at ${note.commit}: ${note.reason}; it vetoes reuse until replaced or removed\n`);
  }
  // Epoch decides the row's shape: absent/1 frozen, 2 envelope (read as is: `sig`/`key`/`scheme`
  // ride on the row), anything else held. A held note (brit's `held:` line, or a row whose epoch
  // this reader does not implement) is named once and is never a row and never reuse evidence.
  const held = heldNotes(r.stderr);
  const admitted = [];
  for (const row of rows) {
    const read = noteEpoch(row?.attestation);
    if (read.held) held.push({ commit: row?.commit ?? '', reason: read.held });
    else admitted.push(row);
  }
  for (const note of held) {
    warn(`[gate] ${project.name} evidence: held note on ${attestationRef(project)} at ${note.commit}: ${note.reason}; this reader does not admit it\n`);
  }
  const heldAt = new Set(held.map(note => note.commit));
  return { rows: admitted.filter(row => !heldAt.has(row?.commit)), unverifiable, held };
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

// D5: "the space never rides what it upgrades". A sibling verdict cannot judge candidate
// bytes that change the gate, its crossing substrate, admission, or measure vocabulary.
export const SIBLING_REUSE_GOVERNANCE_PATHS = Object.freeze([
  ...COMPILER_GATE_INPUTS,
  'genesis/orchestrator/gate-evidence.mjs',
  'genesis/orchestrator/gate-runner.mjs',
  'genesis/orchestrator/gate-*.mjs',
  'genesis/orchestrator/suite-run.mjs',
  'genesis/orchestrator/suite-reads.json',
  'genesis/orchestrator/suite-inventory.mjs',
  'genesis/orchestrator/suite-skips.mjs',
  'genesis/agentic/pool-policy.json',
  'elohim/eprfs/eprfs-peer/**',
  'elohim/eprfs/epr-cli/src/peer.rs',
  '.claude/epr-meta/measures.yaml',
]);

function isSiblingGovernancePath(path) {
  return SIBLING_REUSE_GOVERNANCE_PATHS.some(rule => {
    if (rule.endsWith('/**')) return path.startsWith(rule.slice(0, -2));
    if (rule.includes('*')) {
      const [before, after] = rule.split('*');
      return path.startsWith(before) && path.endsWith(after);
    }
    return path === rule;
  });
}

/** Whether HEAD changes sibling-reuse governance from its integration merge base. Fail closed. */
export function siblingGovernanceChange({ root, spawn = spawnSync }) {
  let base = gitRead(root, ['merge-base', 'HEAD', 'origin/dev'], spawn)?.trim();
  if (!base) base = gitRead(root, ['merge-base', 'HEAD', 'dev'], spawn)?.trim();
  if (!base) return { changed: true, why: 'integration merge base unavailable' };
  const names = gitRead(root, ['diff', '--name-only', base, 'HEAD'], spawn);
  if (names === null) return { changed: true, why: 'governance diff unavailable' };
  const paths = names.split('\n').filter(Boolean).filter(isSiblingGovernancePath);
  return paths.length > 0
    ? { changed: true, why: `governance-class change: ${paths[0]}`, paths }
    : { changed: false, paths: [] };
}

/**
 * The checkout whose flow store this device's peer daemon crosses: the main worktree of the git
 * common dir. A gate runs in whichever linked worktree pushed, but that worktree's own
 * `.eprfs/status/flows.jsonl` is never served or folded into, so a verdict written there never
 * reaches a sibling and a sibling's verdict is never read there (G1, landing 37). A submodule
 * (common dir under `.git/modules/`), a bare repository or an unreadable answer keeps `root`.
 */
export function deviceRoot(root, spawn = spawnSync) {
  const common = gitRead(root, ['rev-parse', '--path-format=absolute', '--git-common-dir'], spawn)?.trim();
  if (!common || !isAbsolute(common) || basename(common) !== '.git') return root;
  return dirname(common);
}

/** Admitted sibling verdict rows from the native device-channel read. Any failure is a miss. */
export function siblingVerdicts(project, key, { root, env = process.env, spawn = spawnSync }) {
  const device = deviceRoot(root, spawn);
  try {
    const r = spawn(env.EPR_BIN || 'epr', [
      'peer', 'verdicts', '--project', project.name, '--closure', key, '--json', '--root', device,
    ], { cwd: device, encoding: 'utf8', timeout: 5_000 });
    if (r.status !== 0) return { candidates: [], error: 'epr peer verdicts failed' };
    const rows = JSON.parse(r.stdout);
    if (!Array.isArray(rows)) return { candidates: [], error: 'epr peer verdicts: not an array' };
    return { candidates: rows.map(row => ({ source: 'sibling', ...row })) };
  } catch {
    return { candidates: [], error: 'epr peer verdicts unavailable' };
  }
}

/**
 * Whether a row's attested output id parses as a CID (version 1: varint codec, varint multihash
 * code, varint length and exactly that many digest bytes; or a 34-byte version 0 sha2-256), the
 * parse ark performed before accepting a local derivation (elohim/ark/cli/src/brit_index.rs
 * record_of).
 */
export function attestedOutputParses(row) {
  const bytes = row?.attestation?.outputCid;
  if (!Array.isArray(bytes) || !bytes.every(b => Number.isInteger(b) && b >= 0 && b <= 255)) return false;
  if (bytes.length === 34 && bytes[0] === 0x12 && bytes[1] === 0x20) return true;
  let at = 0;
  const varint = () => {
    let value = 0;
    for (let shift = 0; shift < 63 && at < bytes.length; shift += 7) {
      const byte = bytes[at++];
      value += (byte & 0x7f) * 2 ** shift;
      if (byte < 0x80) return value;
    }
    return null;
  };
  if (varint() !== 1) return false;
  if (varint() === null || varint() === null) return false;
  const length = varint();
  return length !== null && length > 0 && bytes.length - at === length;
}

/** A green older than a red is not a reusable verdict, even if the report shows it. */
export function reusableHit(evidence, {
  selfId, enabled, mode, selectorCheck, candidates = [], admitCandidate = () => false, governanceWhy = null,
}) {
  if (mode !== 'reuse') return { reuse: false, why: 'report-only' };
  if (!enabled) return { reuse: false, why: 'project not enabled for reuse' };
  if (selfId == null) return { reuse: false, why: 'no workspace key' };
  // D5 (steward-space spec :850): a red of any provenance or verification state vetoes. brit
  // cannot say what closure or verdict a note it will not vouch for carried, so one unverifiable
  // note under this step is read as a possible red: the gate runs, and the line names the cure.
  const vetoed = (evidence.unverifiable || [])[0];
  if (vetoed) {
    return { reuse: false, why: `unverifiable note under this step at ${String(vetoed.commit || '').slice(0, 9)}: ${vetoed.reason} (a red of any verification state vetoes; replace or remove the note)` };
  }
  const siblings = candidates.filter(candidate =>
    candidate?.source === 'sibling'
    && candidate.project === evidence.project
    && candidate.closure === evidence.key
    && admitCandidate(candidate));
  // A red from either device for this closure runs the gate, before this workspace's green is
  // accepted. Cross-device clocks are display-only, so verdicts are never time-ordered.
  if (siblings.some(candidate => candidate.success !== true)) {
    return { reuse: false, why: 'sibling verdict is red' };
  }
  const row = [...(evidence.rows || [])].sort(
    (a, b) => Date.parse(b.attestation?.builtAt || 0) - Date.parse(a.attestation?.builtAt || 0)
  )[0];
  if (row?.attestation?.success === true && row.attestation.agentId === selfId) {
    if (evidence.dirty !== false) return { reuse: false, why: evidence.dirty === true ? 'declared inputs dirty' : 'working tree state unknown' };
    const selected = selectorCheck(row.commit);
    if (!selected.ok) return { reuse: false, why: selected.message };
    return { reuse: true, row };
  }

  if (siblings.length === 0) {
    if (governanceWhy) return { reuse: false, why: governanceWhy };
    if (!row) return { reuse: false, why: 'no attestation for this closure yet' };
    if (row.attestation?.success !== true) return { reuse: false, why: 'newest attestation is red' };
    return { reuse: false, why: `signer ${String(row.attestation.agentId || 'unsigned').slice(0, 12)} is not this workspace` };
  }
  if ((evidence.rows || []).some(local => local.attestation?.success === false)) {
    return { reuse: false, why: 'local attestation is red' };
  }
  if (evidence.dirty !== false) return { reuse: false, why: evidence.dirty === true ? 'declared inputs dirty' : 'working tree state unknown' };
  const candidate = siblings[0];
  const selected = selectorCheck(candidate.commit);
  if (!selected.ok) return { reuse: false, why: selected.message };
  return { reuse: true, row: candidate, independence: 'sibling' };
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
export function gateEvidence(project, { env = process.env, root, commit, cargoEnv, spawn = spawnSync, now = Date.now(), warn = stderrWarn, inputs = null } = {}) {
  if (project.run?.kind === 'attested') return { line: 'unavailable (attested project, no local recipe)' };
  const britBin = resolveBin('brit', ['BRIT_BIN'], env);
  const buildRefBin = resolveBin('brit-build-ref', BUILD_REF_ENV, env);
  if (!britBin || !buildRefBin) {
    return { line: `unavailable (${!britBin ? 'brit' : 'brit-build-ref'} not on BRIT_BIN/BRIT_BUILD_REF_BIN or PATH)` };
  }
  const native = usesNativeClosure(project)
    ? nativeGateState(project, { root, env, run: spawn, toolchain: cargoEnv?.RUSTUP_TOOLCHAIN, commit }) : undefined;
  if (native?.miss) return { line: `unavailable (${native.miss})`, dirty: true };
  const nativeKey = native?.key;
  // The run's input class (gate-inputs.mjs policy `cls`; LEGACY_ALL until `on` exists).
  const fp = stepFingerprints(project, commit, { britBin, root, spawn, inputs: inputs?.cls ?? LEGACY_ALL });
  if (fp.error) return { line: noKeyLine(fp.error, project, root) };
  const blob = manifestBlob(root, commit, project.manifestPath, spawn);
  if (!blob) return { line: `unavailable (manifest ${project.manifestPath} absent at ${String(commit).slice(0, 9)})` };
  const recipe = recipeAt(project, root, commit, spawn);
  const runner = manifestBlob(root, commit, 'genesis/orchestrator/run-local-gate.sh', spawn);
  const compilerTools = COMPILER_GATE_INPUTS.map(path => ({ path, oid: manifestBlob(root, commit, path, spawn) }));
  const dirTree = project.dir === '.'
    ? gitRead(root, ['rev-parse', `${commit}^{tree}`], spawn)?.trim() || null
    : projectTree(root, commit, project.dir, spawn);
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
  // A basename read keys on every tracked match at the commit (path + blob), so adding, removing
  // or editing any one of them moves the key; an unreadable listing falls back to tree scope.
  const readOid = path => (isReadPattern(path) ? patternDigest(root, commit, path, spawn) : manifestBlob(root, commit, path, spawn));
  const named = read && read.unresolved.length === 0
    ? read.paths.map(path => ({ path, oid: readOid(path) })) : null;
  const reads = named && !named.some(row => isReadPattern(row.path) && row.oid === null)
    ? { scope: 'paths', oids: named }
    : { scope: 'tree', oid: tree };
  const scope = reads.scope === 'paths' ? `paths (${read.paths.length} reads)`
    : `tree (${read ? 'unresolved' : 'unknown'} reads)`;
  // The step fingerprint hashes every step source (genesis/orchestrator/**, elohim/eprfs/**, ...),
  // habit atoms included. A suite whose named half runs in a view of its declared reads cannot read
  // those bytes, so in paths scope its key leaves the fingerprint out; dirTree, reads, the manifest,
  // the recipe and the runner name everything the view holds. Tree scope keeps it.
  // The named half never falls back to the tree: an unreadable listing makes this run neither
  // recordable nor reusable, with its cause.
  if (SUITE_READ_LISTS[project.name] && reads.scope !== 'paths') {
    const cause = read ? 'a declared **/<name> walk could not be listed' : `${SUITE_READ_LISTS[project.name]} is unreadable or not a suite list`;
    return { line: `unavailable (the named listing at ${String(commit).slice(0, 9)}: ${cause}; nothing is reused or recorded)` };
  }
  const suiteNamed = Boolean(SUITE_READ_LISTS[project.name]);
  // The suite's named half keys on its resolved inventory (every path, mode, type and object id the
  // view holds, and each pin with its entries), its package closure's file digest and node's
  // version: the same value suite-run.mjs builds the view from and reports in its result.
  let suite = null;
  if (suiteNamed) {
    try {
      const list = SUITE_READ_LISTS[project.name];
      const listText = gitRead(root, ['show', `${commit}:${list}`], spawn);
      const inventory = suiteInventory({ root, commit: resolvedCommit(root, commit, spawn), listText, env: gitEnv() });
      const closure = packageClosure(root, JSON.parse(listText).modules || {});
      suite = { inventory: inventory.digest, packages: closureDigest(closure, root), node: process.version,
        commit: inventory.commit, matcher: inventory.matcher };
    } catch (error) {
      return { line: `unavailable (${String(error?.message || error).split('\n')[0]})` };
    }
  }
  const material = closureMaterial({
    project, fingerprints: suiteNamed ? [] : fp.fingerprints, manifestBlob: blob, cargoEnv, nativeKey,
    recipeFile: recipe.blob, compilerTools, runner, dirTree,
    cargoConfig: manifestBlob(root, commit, '.cargo/config.toml', spawn),
    rustc: rustcHash(project, root, env, cargoEnv, spawn),
    gitlinks: gitlinksAt(root, commit, sources, spawn),
    packages: (env.GATE_CARGO_PACKAGES || '').trim().split(/\s+/).filter(Boolean).sort(),
    reads,
    suite: suite && { inventory: suite.inventory, packages: suite.packages, node: suite.node },
  });
  const key = createHash('sha256').update(material).digest('hex');
  const found = findEvidence(project, key, { buildRefBin, root, spawn, warn });
  if (found.error) return { key, material, line: `unavailable (${found.error}); closure scope ${scope}` };
  // The suite's recording decision asks the builder's own status call, with its flags.
  let dirty;
  try {
    dirty = suite ? dirtyInView(suite.matcher, { cwd: root }) : dirtyPaths(project, { root, spawn, sources, recipePath: recipe.path, reads });
  } catch { dirty = null; }
  let line = `${describeEvidence(found.rows, now)}; closure scope ${scope}`;
  const ignored = found.unverifiable.length;
  if (ignored > 0) line += `; ${ignored} unverifiable note${ignored === 1 ? '' : 's'} (reuse vetoed)`;
  if (dirty === null) line += '; working tree state unknown, nothing will be recorded';
  else if (dirty.length > 0) line += `; working tree dirty under ${project.dir} (${dirty.length} path${dirty.length === 1 ? '' : 's'}): the evidence names the commit, not these bytes, and nothing will be recorded`;
  const checkoutMismatch = env.GATE_EVIDENCE_RECORD === '0';
  if (checkoutMismatch) line += '; pushed commit is not checked out, nothing will be recorded';
  return {
    project: project.name,
    key,
    material,
    // What gate-inputs' shadow compares with: the brit that keyed this and the step fingerprints.
    britBin,
    fingerprints: suiteNamed ? [] : fp.fingerprints,
    rows: found.rows,
    unverifiable: found.unverifiable,
    line,
    buildRefBin,
    nativeKey,
    ...(suite ? { suite: { inventory: suite.inventory, packages: suite.packages, node: suite.node, commit: suite.commit } } : {}),
    dirty: checkoutMismatch ? true : dirty === null ? null : dirty.length > 0,
  };
}

/**
 * Record this run's verdict against the closure: `build put` with the manifest file's own
 * CID, a stored verdict node as the output, and `--success` as observed (red is stored too —
 * a known-red closure is information). The commit's note is read first (brit's verified
 * `build get`): an unverifiable note, a note for another closure, or a failed read is named on
 * `warn` before the write replaces it; the same closure and verdict is refreshed silently.
 * brit's expected refusals come back as `status` 1 with `refused` naming the class and a
 * one-line `stderr` that says so; any other failure passes through. Returns the spawn result
 * shape; never throws.
 */
export function recordEvidence(project, key, { success, seconds, cost, commit, buildRefBin, root, jobs, dirty, nativeKey, env = process.env, cargoEnv, spawn = spawnSync, warn = stderrWarn }) {
  if (dirty !== false) {
    return { status: 1, stdout: '', stderr: dirty === null ? 'working tree state unknown' : `working tree dirty under ${project.dir}` };
  }
  if (usesNativeClosure(project)) {
    const after = nativeGateKey(project, { root, env, run: spawn, toolchain: cargoEnv?.RUSTUP_TOOLCHAIN, commit });
    if (!nativeKey || !after || JSON.stringify(nativeKey) !== JSON.stringify(after)) {
      return { status: 1, stdout: '', stderr: 'source closure dirty, changed, or unavailable after gate' };
    }
  }
  const ref = attestationRef(project);
  const existing = readBuildNote(attestationStep(project), commit, { buildRefBin, root, spawn });
  if (success === false && existing.state === 'verified'
      && existing.node.success === true && existing.node.inputsHash === key
      && typeof existing.node.agentId === 'string'
      && existing.node.agentId === selfAgentId(root, spawn, { env, buildRefBin })) {
    warn(`[gate] ${project.name} evidence: kept green for closure ${key.slice(0, 12)}; this run was red for the same closure (a flake: the red stays in this run log and cycle row, and the stored green stands)\n`);
    // No separate observation is emitted: a measure for it is not declared. The runner still
    // records this run's red cycle and its live verdict, which is where the flake is counted.
    return { status: 0, stdout: '', stderr: '', keptGreen: true };
  }
  // The warn names the commit brit read, not the caller's rev (`HEAD` under `just gate`): the
  // unverifiable line carries brit's resolved sha; the other two resolve it here.
  const sha = existing.state === 'unverifiable' ? existing.commit : resolvedCommit(root, commit, spawn);
  if (existing.state === 'unverifiable') {
    warn(`[gate] ${project.name} evidence: unverifiable note on ${ref} at ${sha}: ${existing.reason}; replacing it\n`);
  } else if (existing.state === 'held') {
    warn(`[gate] ${project.name} evidence: ${ref} at ${sha} holds a note this reader does not implement (${existing.reason}); writing anyway\n`);
  } else if (existing.state === 'unreadable') {
    // The gate's rule is last run wins, named: the read above only names what the put replaces,
    // and a verdict that cost a run is not withheld because the name could not be read (brit's own
    // put still refuses a concurrent update). brit-attest.mjs defers the same state, because a
    // harvest retries for free; the two rules are deliberate and named in both places.
    warn(`[gate] ${project.name} evidence: ${ref} at ${sha} could not be read (${existing.reason}); writing anyway\n`);
  } else if (existing.state === 'verified' && (existing.node.inputsHash !== key || existing.node.success !== success)) {
    const held = `${existing.node.success === true ? 'green' : 'red'}, signer ${String(existing.node.agentId || 'unsigned').slice(0, 12)}`;
    warn(`[gate] ${project.name} evidence: ${ref} at ${sha} holds closure ${String(existing.node.inputsHash).slice(0, 12)} (${held}); replacing it with closure ${key.slice(0, 12)}\n`);
  }
  const verdict = { exit: success ? 0 : 1, seconds: Math.round(seconds), ...(jobs ? { jobs } : {}), ...(cost ? { cost } : {}) };
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
  let r;
  try {
    r = spawn(buildRefBin, args, { cwd: root, encoding: 'utf8', timeout: 60_000 });
  } catch (error) {
    return { status: 1, stdout: '', stderr: String(error) };
  }
  if (r.status === 0) return r;
  const refusal = expectedRefusal(r.stderr);
  return refusal
    ? { ...r, status: 1, refused: refusal.kind, stderr: `expected refusal (${refusal.kind}), not a failure: ${refusal.line}` }
    : r;
}

function buildPutIdentity(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return {};
  try {
    const value = JSON.parse(text);
    return {
      ...(value.attestationId || value.id || value.cid ? { attestationId: value.attestationId || value.id || value.cid } : {}),
      ...(value.agentId || value.attestation?.agentId ? { agentId: value.agentId || value.attestation.agentId } : {}),
    };
  } catch {
    return { attestationId: text.split('\n').pop() };
  }
}

/** Append one completed gate verdict to this device's crossing flow. Best effort, never a gate. */
export function emitGateVerdict(project, key, {
  success, seconds, commit, put, root, env = process.env, spawn = spawnSync,
  warn = line => process.stderr.write(line),
}) {
  const epr = env.EPR_BIN || 'epr';
  const failed = reason => {
    warn(`[gate] ${project.name} verdict crossing: not emitted (${reason})\n`);
    return false;
  };
  // The verdict is written where the daemon crosses from, never to the pushing worktree's store.
  const device = deviceRoot(root, spawn);
  let standing;
  try {
    const leases = spawn(epr, ['peer', 'leases', '--json', '--root', device], {
      cwd: device, encoding: 'utf8', timeout: 5_000,
    });
    if (leases.status !== 0) return failed('device standing unavailable');
    standing = JSON.parse(leases.stdout);
  } catch {
    return failed('device standing unavailable');
  }
  if (standing?.standing !== 'device') return true;
  // A sibling checks the commit against its own tree, so `HEAD` must cross as the sha it named here.
  const sha = gitRead(root, ['rev-parse', '--verify', `${commit}^{commit}`], spawn)?.trim();
  if (!sha) return failed(`commit ${commit} does not resolve`);
  const identity = buildPutIdentity(put?.stdout);
  const fields = {
    project: project.name,
    closure: key,
    commit: sha,
    success: success ? 'true' : 'false',
    seconds: String(Math.round(seconds)),
    ...identity,
  };
  const args = [
    'flow', 'note', '--kind', 'observation', '--measure', 'gate-verdict@1', '--subject', '.',
    '--value', '1', '--reason', 'verdict',
  ];
  for (const [name, value] of Object.entries(fields)) args.push('--env', `${name}=${value}`);
  args.push('--root', device);
  try {
    const emitted = spawn(epr, args, { cwd: device, encoding: 'utf8', timeout: 5_000 });
    return emitted.status === 0 || failed((emitted.stderr || '').trim().split('\n').pop() || `exit ${emitted.status}`);
  } catch (error) {
    return failed(String(error?.message || error).split('\n')[0]);
  }
}
