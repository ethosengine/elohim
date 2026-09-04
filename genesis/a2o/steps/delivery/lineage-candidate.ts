/**
 * Mints the node-registry v1→v2 `happ-lineage` release manifest fixture
 * (Holochain Evolution Epic, Task 10 Part 1 — the TypeScript half; the mesh
 * reset route and `hc-mesh.sh` reset are Task 10 Part 2, another session's).
 *
 * Unlike `coordinator-candidate.ts` (which BUILDS a fresh coordinator wasm
 * by appending a marker section) this file mints no bytes at all: the v2
 * `.happ` is a real packed bundle — `elohim/holochain/dna/node-registry/
 * node-registry-v2.happ` when it exists, or assembled here from the repo's
 * `node-registry-v2.dna` (the `lineage-witness` feature build — see that
 * DNA's `justfile` `build-witness` recipe) plus a one-role `happ.yaml`. The
 * "candidate" work is entirely the RELEASE MANIFEST that names the v2 DNA
 * as a `happ-lineage` crossing (spec 2026-09-03-holochain-evolution-epic-
 * design §4): `--migrate-from node_registry=<v1>` + `--lineage <v1>` +
 * `--path-commitment <cid>`.
 *
 * The packager (`scripts/epr-release-package.ts`) is shelled out to via
 * `spawnSync('pnpm', ['exec', 'tsx', ...])`, exactly the rail
 * `runtime-upgrade-propagation.steps.ts`'s `runDriver` uses and this file's
 * sibling `coordinator-candidate.ts` documents as the reason no `just` /
 * cargo build is needed here. It is NOT imported as a module: importing it
 * would run its top-level `try { … }` CLI body as an import side effect
 * (verified by reading its tail — there is no `import.meta.url` guard), so
 * every consumer in this repo either shells it out or reimplements its
 * exported pure helpers locally (see `lineage-commitments.ts`'s module doc
 * for the latter).
 */

import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

/** genesis/a2o/steps/delivery -> repo root (4 levels up) — mirrors coordinator-candidate.ts. */
const REPO_ROOT = fileURLToPath(new URL('../../../../', import.meta.url));
/** genesis/a2o/steps/delivery -> genesis/a2o (2 levels up) — the packager's own cwd. */
const A2O_ROOT = fileURLToPath(new URL('../../', import.meta.url));

const PACKAGER_SCRIPT = 'scripts/epr-release-package.ts';

/**
 * The 0.7 `hc` CLI — the ONLY correct binary for hashing the v2 (0.7-era)
 * DNA. `hc` resolves via PATH to `/opt/holochain/bin/hc` (0.6) on this
 * container, so PATH resolution is exactly what must NOT be used here
 * (epic-design assessment: 0.7 changed Action preimages, so a 0.6
 * `hc dna hash` on a 0.7-built DNA is not merely stale — it can silently
 * hash the wrong preimage shape). This constant is pinned instead;
 * `ELOHIM_HC07_BIN` overrides it for a container where the 0.7 install
 * lives elsewhere.
 */
const HC07_DEFAULT_PATH = '/projects/.claude-config/tools/hc-0.7/hc';
const HC_BIN = process.env['ELOHIM_HC07_BIN'] ?? HC07_DEFAULT_PATH;

const NODE_REGISTRY_DIR = path.join(REPO_ROOT, 'elohim/holochain/dna/node-registry');
const NODE_REGISTRY_V2_DNA = path.join(NODE_REGISTRY_DIR, 'node-registry-v2.dna');
const NODE_REGISTRY_V1_DNA = path.join(NODE_REGISTRY_DIR, 'node-registry-v1.dna');

/** Default packed-bundle path; overridable by `LINEAGE_V2_HAPP` (task brief). */
export const DEFAULT_V2_HAPP_PATH = path.join(NODE_REGISTRY_DIR, 'node-registry-v2.happ');

export const NODE_REGISTRY_ROLE = 'node_registry' as const;

export interface LineageDiscipline {
  soakSecs: number;
  attestationThreshold: number;
  canary: string;
}

export interface LineageCandidateOptions {
  /**
   * Path to the packed v2 `.happ`. Optional despite the task brief's literal
   * signature (`v2HappPath: string`) — `resolveV2HappPath` fills it in from
   * `LINEAGE_V2_HAPP` or `DEFAULT_V2_HAPP_PATH` when omitted, which is the
   * "resolves node-registry-v2.happ (default path …, overridable by
   * LINEAGE_V2_HAPP)" behaviour the same brief paragraph asks for. A caller
   * that already has the path keeps passing it explicitly; nothing narrows.
   */
  v2HappPath?: string;
  role: typeof NODE_REGISTRY_ROLE;
  v1DnaHash: string;
  v2DnaHash: string;
  pathCommitmentCid?: string;
  channelId: string;
  storageBaseUrl: string;
  out: string;
  discipline: LineageDiscipline;
  /**
   * Test/offline escape hatch: passes `--no-put` to the packager so the mint
   * never touches a live storage peer's blob route. Not part of Task 11's
   * live fixture contract (the live run PUTs for real); this unit-test spec
   * sets it because no mesh may be written to from this dispatch.
   */
  noPut?: boolean;
}

export interface MintedLineageManifest {
  manifestPath: string;
  releaseCid?: string;
}

interface PackagerRunResult {
  status: number;
  stdout: string;
  stderr: string;
}

/**
 * Resolves the v2 `.happ` path: explicit argument, else `LINEAGE_V2_HAPP`,
 * else the in-repo default. Throws with a remediation line (build it, or
 * point at the scratch copy) when nothing exists at the resolved path.
 */
export function resolveV2HappPath(explicit?: string): string {
  const resolved = explicit ?? process.env['LINEAGE_V2_HAPP'] ?? DEFAULT_V2_HAPP_PATH;
  if (!existsSync(resolved)) {
    throw new Error(
      `lineage-candidate: node-registry-v2.happ not found at ${resolved} — build it ` +
        `(elohim/holochain/dna/node-registry: just build-witness, needs cargo) or set ` +
        `LINEAGE_V2_HAPP to an already-packed bundle`
    );
  }
  return resolved;
}

/**
 * `hc dna hash <path>` via the 0.7 CLI, trimmed. Exported so a caller (or
 * this file's own cross-checks) never has to shell out twice for the same
 * hash.
 */
export function computeDnaHash(dnaPath: string): string {
  try {
    return execFileSync(HC_BIN, ['dna', 'hash', dnaPath], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    }).trim();
  } catch (error) {
    const stderr =
      error && typeof error === 'object' && 'stderr' in error
        ? String((error as { stderr?: unknown }).stderr)
        : '';
    throw new Error(`hc dna hash ${dnaPath} failed: ${stderr || String(error)}`);
  }
}

/**
 * Unpacks a `.happ` and returns the path to the `node_registry` role's `.dna`
 * inside it. Every `.happ` this file mints or consumes (the workdir
 * `happ.yaml` convention, and the scratch `lineage-happ/happ.yaml` fixture)
 * names that role's dna file `node-registry-v2.dna` — verified 2026-09-04
 * against both the in-repo `node-registry/justfile` output and the scratch
 * fixture's own `happ.yaml`.
 */
function unpackHappNodeRegistryDna(happPath: string): string {
  // `hc app unpack --output <dir>` REFUSES when `<dir>` already exists — even
  // empty ("The target directory '…' already exists"). `mkdtempSync` always
  // creates its directory, so the unpack target has to be a NOT-YET-existing
  // child of one (verified 2026-09-04: passing the mkdtemp dir itself fails).
  const parent = mkdtempSync(path.join(tmpdir(), 'lineage-candidate-unpack-'));
  const scratchDir = path.join(parent, 'unpack');
  execFileSync(HC_BIN, ['app', 'unpack', '--output', scratchDir, happPath], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const dnaFile = path.join(scratchDir, 'node-registry-v2.dna');
  if (!existsSync(dnaFile)) {
    throw new Error(`unpacked ${happPath} but found no node-registry-v2.dna at ${dnaFile}`);
  }
  return dnaFile;
}

/**
 * Cross-checks the caller-declared `v1DnaHash`/`v2DnaHash` against `hc dna
 * hash` computed from real bytes — "computes both DNA hashes with the 0.7
 * hc" (task brief). v2 is computed from the EXACT bytes the mint is about to
 * name in the manifest (`v2HappPath`, unpacked), so a mismatch there is a
 * refusal: the manifest would otherwise assert a DNA hash the bundle it
 * ships does not have. v1 is computed from the repo's `node-registry-v1.dna`
 * fixture when present, but only WARNED on mismatch — that file is a
 * point-in-time snapshot, not necessarily byte-identical to whatever a live
 * peer has actually installed (the caller's `v1DnaHash`, typically read from
 * a live `/version` passport, is the ground truth for that).
 */
function crossCheckDnaHashes(v2HappPath: string, v1DnaHash: string, v2DnaHash: string): void {
  const v2Dna = unpackHappNodeRegistryDna(v2HappPath);
  const computedV2 = computeDnaHash(v2Dna);
  if (computedV2 !== v2DnaHash) {
    throw new Error(
      `lineage-candidate: v2DnaHash mismatch — declared ${v2DnaHash}, ` +
        `hc dna hash of ${v2HappPath}'s node_registry role computed ${computedV2}`
    );
  }
  if (existsSync(NODE_REGISTRY_V1_DNA)) {
    const computedV1 = computeDnaHash(NODE_REGISTRY_V1_DNA);
    if (computedV1 !== v1DnaHash) {
      console.error(
        `[lineage-candidate] warning: declared v1DnaHash ${v1DnaHash} does not match ` +
          `hc dna hash of the repo fixture ${NODE_REGISTRY_V1_DNA} (${computedV1}) — the ` +
          `declared hash is trusted (it should come from a live peer's /version passport), ` +
          `this fixture file may simply be a different snapshot`
      );
    }
  }
}

function runPackager(args: string[], timeoutMs = 60_000): PackagerRunResult {
  // Same posture as runtime-upgrade-propagation.steps.ts's `runDriver`: this
  // deliberately shells out to `pnpm exec tsx` for the packager driver — the
  // composition this dispatch requires (see module doc).
  // eslint-disable-next-line sonarjs/no-os-command-from-path
  const result = spawnSync('pnpm', ['exec', 'tsx', PACKAGER_SCRIPT, ...args], {
    cwd: A2O_ROOT,
    encoding: 'utf8',
    timeout: timeoutMs,
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) {
    throw new Error(`failed to spawn "pnpm exec tsx ${PACKAGER_SCRIPT}": ${result.error.message}`);
  }
  return { status: result.status ?? 1, stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
}

function appliesToLiteral(role: string, dnaHash: string): string {
  // No live conductor has this DNA installed yet (that is the whole point of
  // a candidate), so there is no `/version` passport to read
  // `coordinatorWasmHashes` from — the packager's own module doc explains
  // why reproducing Holochain's blake2b wasm hashing in TypeScript instead
  // would be a second, undetectably-driftable implementation of a
  // consensus-critical hash. The schema requires the array but not that it
  // be non-empty (`elohim/rakia/schemas/v1/release-manifest.schema.json`
  // `$defs.roleBinding`), so an empty array here is the honest "not read
  // from a live install" declaration, not a placeholder.
  return JSON.stringify({ roles: { [role]: { dnaHash, coordinatorWasmHashes: [] } } });
}

function assertPackaged(result: PackagerRunResult, out: string): void {
  if (result.status !== 0) {
    throw new Error(
      `epr-release-package.ts failed (exit ${result.status}):\n--- stdout ---\n` +
        `${result.stdout.trim()}\n--- stderr ---\n${result.stderr.trim()}`
    );
  }
  if (!existsSync(out)) {
    throw new Error(`epr-release-package.ts exited 0 but wrote no manifest to ${out}`);
  }
}

/**
 * Mints the happ-lineage candidate release: v2, declared as migrating FROM
 * v1 on the `node_registry` role, notarized by `pathCommitmentCid` once
 * Station 2 has produced one (omit it and the packager itself refuses —
 * "a lineage crossing is adoptable only when a notarized migrates-lineage
 * commitment names it").
 */
export async function mintLineageCandidate(
  opts: LineageCandidateOptions
): Promise<MintedLineageManifest> {
  // No real await below — everything the packager needs is synchronous
  // (spawnSync, fs). This one keeps the function honestly `async` (its
  // return type is `Promise<…>` and every caller in this fixture awaits it)
  // without lint fighting itself: `require-await` (error) demands an await
  // in an `async` function, `promise-function-async` (warn) demands `async`
  // on anything returning a Promise — same posture as
  // `federation-failover.steps.ts`'s and `self-healing-flow-control.steps.ts`'s
  // `await Promise.resolve();`.
  await Promise.resolve();
  const v2HappPath = resolveV2HappPath(opts.v2HappPath);
  crossCheckDnaHashes(v2HappPath, opts.v1DnaHash, opts.v2DnaHash);

  mkdirSync(path.dirname(opts.out), { recursive: true });
  const args = [
    '--artifact',
    v2HappPath,
    '--artifact-class',
    'happ-lineage',
    '--channel-id',
    opts.channelId,
    '--applies-to',
    appliesToLiteral(opts.role, opts.v2DnaHash),
    '--migrate-from',
    `${opts.role}=${opts.v1DnaHash}`,
    '--lineage',
    opts.v1DnaHash,
    '--soak-secs',
    String(opts.discipline.soakSecs),
    '--attestation-threshold',
    String(opts.discipline.attestationThreshold),
    '--canary',
    opts.discipline.canary,
    '--peer',
    opts.storageBaseUrl,
    '--notes',
    'a2o lineage fixture: node_registry v1->v2 happ-lineage candidate',
    '--out',
    opts.out,
  ];
  if (opts.pathCommitmentCid) args.push('--path-commitment', opts.pathCommitmentCid);
  if (opts.noPut) args.push('--no-put');

  const result = runPackager(args);
  assertPackaged(result, opts.out);
  return { manifestPath: opts.out };
}

/**
 * Station 1's negative control: mints a plain `happ-bundle` release for v2
 * on the SAME role, naming no `migrateFrom` / `lineage` / adoption-discipline
 * `path` at all — "a second release that installs v2 without naming what it
 * migrates from" (the feature file's own words). Deliberately NOT
 * `artifact-class happ-lineage` with an omitted `--migrate-from`: that class
 * still demands `--path-commitment` from the packager regardless of whether
 * a migrate-from is named, which would mint a manifest carrying lineage
 * machinery (`adoptionDiscipline.path`) around a release that is supposed to
 * carry none. A plain `happ-bundle` release is the honest shape of "v2,
 * unclaimed" — the verifier refusing it with "lineage mismatch" tests the
 * SAME thing (a DNA-hash crossing with nothing naming what it crosses from)
 * without this fixture pre-deciding which artifactClass string the refusal
 * has to fire on.
 */
export async function lineageReleaseWithoutParent(
  opts: Omit<LineageCandidateOptions, 'v1DnaHash' | 'pathCommitmentCid'>
): Promise<MintedLineageManifest> {
  // See `mintLineageCandidate`'s matching comment.
  await Promise.resolve();
  const v2HappPath = resolveV2HappPath(opts.v2HappPath);
  const computedV2 = computeDnaHash(unpackHappNodeRegistryDna(v2HappPath));
  if (computedV2 !== opts.v2DnaHash) {
    throw new Error(
      `lineage-candidate: v2DnaHash mismatch — declared ${opts.v2DnaHash}, ` +
        `hc dna hash of ${v2HappPath}'s node_registry role computed ${computedV2}`
    );
  }

  mkdirSync(path.dirname(opts.out), { recursive: true });
  const args = [
    '--artifact',
    v2HappPath,
    '--artifact-class',
    'happ-bundle',
    '--channel-id',
    opts.channelId,
    '--applies-to',
    appliesToLiteral(opts.role, opts.v2DnaHash),
    '--soak-secs',
    String(opts.discipline.soakSecs),
    '--attestation-threshold',
    String(opts.discipline.attestationThreshold),
    '--canary',
    opts.discipline.canary,
    '--peer',
    opts.storageBaseUrl,
    '--notes',
    'a2o lineage fixture: Station 1 negative control — v2 with no migrate-from',
    '--out',
    opts.out,
  ];
  if (opts.noPut) args.push('--no-put');

  const result = runPackager(args);
  assertPackaged(result, opts.out);
  return { manifestPath: opts.out };
}

/** Writes a fresh happ.yaml + copies the v2 dna into `dir`, then packs it — the
 * fallback path when no `.happ` exists yet but `node-registry-v2.dna` does
 * (task brief step 3's "if no .happ exists ... pack a one-role hApp"). Not
 * used by `mintLineageCandidate` directly (that function expects a
 * pre-resolved `.happ`); exported for a caller (or this file's own spec) that
 * needs to produce one from the raw `.dna`.
 */
export function packOneRoleHapp(dnaPath: string, outDir: string): string {
  if (!existsSync(dnaPath)) {
    throw new Error(`packOneRoleHapp: no dna at ${dnaPath}`);
  }
  mkdirSync(outDir, { recursive: true });
  const dnaDest = path.join(outDir, 'node-registry-v2.dna');
  if (path.resolve(dnaPath) !== path.resolve(dnaDest)) {
    copyFileSync(dnaPath, dnaDest);
  }
  const happYaml = [
    'manifest_version: "0"',
    'name: node_registry_lineage_fixture',
    'roles:',
    '  - name: node_registry',
    '    provisioning:',
    '      strategy: create',
    '      deferred: false',
    '    dna:',
    '      path: "node-registry-v2.dna"',
    '      modifiers:',
    '        network_seed: "elohim_node_registry_alpha"',
    '        properties:',
    '          progenitor_pubkey: ~',
    '      installed_hash: ~',
    '      clone_limit: 0',
    '',
  ].join('\n');
  writeFileSync(path.join(outDir, 'happ.yaml'), happYaml);
  const out = path.join(outDir, 'node-registry-v2.happ');
  execFileSync(HC_BIN, ['app', 'pack', outDir, '-o', out], { encoding: 'utf8' });
  return out;
}

export { NODE_REGISTRY_V2_DNA };
