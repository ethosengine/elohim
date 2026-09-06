/**
 * Mints a FRESH, run-scoped coordinator-only candidate `.happ` for
 * `runtime-upgrade-propagation.steps.ts` — the fix for the flaw the
 * 2026-09-02 04:10-04:28Z run measured: a FIXED candidate bundle
 * (`reports/release-ceremony/2026-09-01/elohim-P.happ`) is whatever the
 * household already converged on by the time a later run starts, so
 * Station 3's apply is `already_current` (no observable coordinator
 * effect), and Stations 6/7/8 cascade on `threshold_unmet` with no soak
 * attestation ever recorded.
 *
 * The recipe is the one the 2026-09-01 r2 receipt used to mint "candidate
 * P" (`genesis/a2o/reports/release-ceremony/2026-09-01/transcript.md`,
 * "Station 3+4 (r2, candidate P)"): a coordinator-only rebuild via a custom
 * wasm section (there, `COORD_BUILD_MARKER`; here, appended directly since
 * this file may run when the DNA workspace itself cannot rebuild — see the
 * transcript's "Build-environment finding"). Appending a trailing custom
 * section (wasm section id 0) to an otherwise-valid module is always a
 * valid module — custom sections carry no semantics and may appear
 * anywhere, including after every other section — so this needs no `just`
 * / cargo build at all.
 *
 * ## Base bytes: the WORKDIR bundle, not the cargo target dir
 *
 * The task that authored this fix named
 * `target/wasm32-unknown-unknown/release/{content_store,content_store_integrity}.wasm`
 * as "the installed lamad coordinator wasm." Verified against the live
 * household mesh on 2026-09-02 before wiring this in: those files no
 * longer match what is installed — a concurrent build (this fix landed
 * during a CI shift) rewrote them underneath this task. Byte comparison:
 *
 *   - target dir content_store_integrity.wasm  sha256 d1c4e709…
 *   - workdir elohim.happ's lamad.dna integrity  sha256 6bc3f9f2…
 *   - the 2026-09-01 candidate P's own integrity  sha256 6bc3f9f2…  (MATCH)
 *   - all three peers' live /version dnaHash for lamad: identical, and a
 *     control repack (dna.yaml + the workdir integrity/coordinator wasm)
 *     reproduces the workdir bundle's own `hc dna hash` byte-for-byte.
 *
 * So this file reads integrity+coordinator bytes out of `BASELINE_HAPP`
 * (the already-stable, already-verified workdir bundle used elsewhere in
 * the steps file as Station 6's revert target) instead — stable regardless
 * of what a concurrent build is doing to the cargo target dir, and proven
 * (by the control repack above) to reproduce the exact DNA hash every live
 * peer already reports. Integrity bytes are carried through completely
 * unmodified (DNA hash — and therefore lineage — never moves); only the
 * coordinator wasm gets the marker appended.
 *
 * ## 2026-09-06 fix: single-role candidates only
 *
 * Measured on alpha 2026-09-06: a workspace→fleet coordinator candidate
 * assembled as a full 5-role `.happ` — the OTHER four roles copied
 * unmodified from `elohim/holochain/dna/elohim/workdir/` — carried
 * DIFFERENT bytes for those untouched roles whenever that workdir had been
 * rebuilt for an unrelated reason (a different rustc, `just`'s
 * `-C link-arg=--import-undefined`, or simply a later build in the same
 * tree). Applying that candidate made `sync_coordinators` swap the one
 * role that actually changed (lamad) and REFUSE the other four on DNA
 * lineage (`drifted=5, applied=1`) — a mixed peer no later release could
 * verify, because a candidate for role X described by the fleet's own
 * `appliesTo` should never carry bytes for role Y at all.
 *
 * The cure: `assembleCandidateHappDir` now emits a happ dir holding ONLY
 * the changed role's `.dna` and a `happ.yaml` whose `roles:` list has
 * exactly one entry — copied verbatim from `BASELINE_HAPP`'s own unpacked
 * happ.yaml, never from the mutable workspace `workdir/`. Untouched roles
 * are never read, copied, or referenced. The changed role defaults to
 * `lamad` (`DEFAULT_ROLE`) and is parameterised via `MintCandidateOptions.role`
 * for any of this DNA family's other roles (imagodei, infrastructure,
 * mishpat, node_registry), since each is packed from its own single
 * integrity + single coordinator zome the same way lamad is.
 *
 * A second option, `rollbackToBaseline`, skips the marker entirely — the
 * candidate's coordinator wasm is then byte-identical to the baseline's,
 * which is how a peer that adopted a forward (marker) candidate is
 * returned to the fleet baseline through the same release channel (a
 * revert release, not a live `hc` invocation against a running conductor).
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import * as path from 'node:path';
import { pathToFileURL } from 'node:url';

const HC_BIN = process.env['ELOHIM_HC_BIN'] ?? '/opt/holochain/bin/hc';

const MARKER_SECTION_NAME = 'elohim.coord-build-marker';

/** The role a candidate carries when the caller names none — lamad is the role
 * every existing fixture/story in this repo exercises. */
export const DEFAULT_ROLE = 'lamad';

export interface MintedCandidate {
  /** Absolute path to the freshly-packed, SINGLE-ROLE `.happ` bundle. */
  happPath: string;
  /** sha256 (hex) of the packed bundle's bytes — for the blob PUT. */
  sha256: string;
  /**
   * sha256 (hex) of the coordinator wasm bytes ALONE (post-marker, or the
   * verbatim baseline bytes when `rollbackToBaseline` is set) — the piece
   * whose Holochain `WasmHash` (`uhCok…`) the conductor actually hot-swaps
   * on. This is NOT the same value the conductor reports (Holochain
   * computes `WasmHash` via its own blake2b + holo-hash encoding, not a bare
   * sha256), so a caller that needs ground truth for "is the conductor
   * actually running this candidate" must read it back from a peer's
   * `GET /version` passport AFTER an apply — this field only proves the
   * MINT step itself produced fresh, non-cached bytes for this run.
   */
  coordinatorWasmSha256: string;
  /** The one role this candidate's happ.yaml declares and whose `.dna` it carries. */
  role: string;
  /** True when the coordinator wasm was kept byte-identical to the baseline (no marker). */
  rollbackToBaseline: boolean;
}

export interface MintCandidateOptions {
  /** Scratch-directory suffix only — identity comes from the cache key
   * (marker + role + rollback), never from this label. */
  variantLabel?: string;
  /** The role this candidate carries — the ONLY role copied into the
   * candidate happ. Defaults to `DEFAULT_ROLE` ('lamad'). */
  role?: string;
  /**
   * When true, the role's coordinator wasm is the baseline's own bytes,
   * verbatim — no marker section appended, so the candidate's coordinator
   * wasm hash is IDENTICAL to what the fleet baseline already ships. This
   * is the rollback shape: publishing THIS release on a channel a peer
   * follows returns that peer to the fleet baseline without touching its
   * running conductor directly.
   */
  rollbackToBaseline?: boolean;
}

/**
 * Memoized per-process, keyed by marker payload + role + rollback flag —
 * every scenario in one run shares the candidate for a given key, and a run
 * that needs a SECOND, distinct candidate (Station 9's next fix on the same
 * long-lived channel) mints it by asking for a different marker. Keying the
 * cache on more than a single slot is what makes that possible: a lone
 * `cached` would have handed a second request the first candidate's bytes.
 */
const cache = new Map<string, MintedCandidate>();

function runHc(args: string[], cwd?: string): string {
  try {
    return execFileSync(HC_BIN, args, {
      cwd,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch (error) {
    const stderr =
      error && typeof error === 'object' && 'stderr' in error
        ? String((error as { stderr?: unknown }).stderr)
        : '';
    throw new Error(`hc ${args.join(' ')} failed: ${stderr || String(error)}`);
  }
}

function encodeULEB128(value: number): Buffer {
  const bytes: number[] = [];
  let remaining = value;
  do {
    let byte = remaining & 0x7f;
    remaining >>>= 7;
    if (remaining !== 0) byte |= 0x80;
    bytes.push(byte);
  } while (remaining !== 0);
  return Buffer.from(bytes);
}

/**
 * Appends a wasm custom section (id 0: LEB128 size, LEB128 name-len + name +
 * payload) to the end of an otherwise-valid module — always valid, since
 * custom sections carry no semantics and may appear anywhere in the binary,
 * including trailing every standard section.
 */
function appendCustomSection(wasm: Buffer, name: string, payload: Buffer): Buffer {
  const nameBytes = Buffer.from(name, 'utf8');
  const content = Buffer.concat([encodeULEB128(nameBytes.length), nameBytes, payload]);
  const section = Buffer.concat([Buffer.from([0x00]), encodeULEB128(content.length), content]);
  return Buffer.concat([wasm, section]);
}

// ---------------------------------------------------------------------------
// happ.yaml / dna.yaml parsing — `hc app unpack` / `hc dna unpack` produce a
// fixed, checked shape (0-indent `- name:` list items, one non-role trailer
// key at most). These helpers throw loudly on a shape they don't recognise
// rather than silently emitting a wrong single-role candidate.
// ---------------------------------------------------------------------------

const ROLE_HEADER_RE = /^- name: (\S+)\s*$/;

interface ParsedHappYaml {
  /** Everything up to and including the `roles:` line. */
  header: string[];
  /** Role name -> that role's own 0-indent `- name: …` block lines. */
  roleBlocks: Map<string, string[]>;
  /** Anything after the last role block at 0-indent (e.g. `allow_deferred_memproofs`). */
  trailer: string[];
}

function parseHappYaml(happYamlText: string): ParsedHappYaml {
  const lines = happYamlText.split('\n');
  const rolesIdx = lines.findIndex(line => line.trim() === 'roles:');
  if (rolesIdx === -1) {
    throw new Error('coordinator-candidate: happ.yaml has no top-level "roles:" key');
  }
  const header = lines.slice(0, rolesIdx + 1);

  const roleStarts: { index: number; name: string }[] = [];
  for (let i = rolesIdx + 1; i < lines.length; i++) {
    const match = ROLE_HEADER_RE.exec(lines[i]);
    if (match) roleStarts.push({ index: i, name: match[1] });
  }
  if (roleStarts.length === 0) {
    throw new Error('coordinator-candidate: happ.yaml "roles:" has no "- name:" entries');
  }

  let trailerStart = lines.length;
  // Non-null: the `roleStarts.length === 0` guard above already threw.
  const lastRoleStart = roleStarts.at(-1) as { index: number; name: string };
  for (let i = lastRoleStart.index + 1; i < lines.length; i++) {
    if (/^\S/.test(lines[i])) {
      trailerStart = i;
      break;
    }
  }

  const roleBlocks = new Map<string, string[]>();
  roleStarts.forEach(({ index, name }, position) => {
    const end = position + 1 < roleStarts.length ? roleStarts[position + 1].index : trailerStart;
    roleBlocks.set(name, lines.slice(index, end));
  });

  return { header, roleBlocks, trailer: lines.slice(trailerStart) };
}

/** The `.dna` filename an unpacked happ.yaml's role block declares under `dna: path:`. */
function roleDnaFileName(roleBlock: string[], role: string): string {
  const pathLine = roleBlock.find(line => /^\s+path:\s*\S/.test(line));
  if (!pathLine) {
    throw new Error(`coordinator-candidate: role "${role}"'s happ.yaml block names no dna path`);
  }
  return pathLine
    .trim()
    .replace(/^path:\s*/, '')
    .replace(/^['"]|['"]$/g, '');
}

/** The wasm filename an unpacked dna.yaml declares under `integrity:` or `coordinator:`. */
function zomeWasmFileName(dnaYamlLines: string[], section: 'integrity' | 'coordinator'): string {
  const sectionIdx = dnaYamlLines.findIndex(line => line.trim() === `${section}:`);
  if (sectionIdx === -1) {
    throw new Error(`coordinator-candidate: dna.yaml has no top-level "${section}:" key`);
  }
  for (let i = sectionIdx + 1; i < dnaYamlLines.length; i++) {
    const line = dnaYamlLines[i];
    if (/^\S/.test(line)) break; // next top-level key — section ended with no path
    const match = /^\s+path:\s*(\S+)/.exec(line);
    if (match) return match[1].replace(/^['"]|['"]$/g, '');
  }
  throw new Error(`coordinator-candidate: dna.yaml's "${section}:" section names no zome path`);
}

/** Renders a single-role happ.yaml: the baseline's own header + trailer, with
 * `roles:` narrowed to exactly the one role this candidate carries. */
function assembleSingleRoleHappYaml(happYaml: ParsedHappYaml, role: string): string {
  const roleBlock = happYaml.roleBlocks.get(role);
  if (!roleBlock) {
    throw new Error(
      `coordinator-candidate: no role "${role}" to assemble a single-role happ.yaml from ` +
        `(baseline has: ${[...happYaml.roleBlocks.keys()].join(', ')})`
    );
  }
  return [...happYaml.header, ...roleBlock, ...happYaml.trailer].join('\n');
}

// ---------------------------------------------------------------------------
// Extraction — reads ONLY the changed role's bytes out of the baseline
// bundle. No other role is unpacked, read, or referenced.
// ---------------------------------------------------------------------------

interface InstalledRoleZomes {
  integrityWasm: Buffer;
  coordinatorWasm: Buffer;
  /** Directory `hc dna unpack` wrote this role's dna.yaml + zome wasm into —
   * safe to mutate in place (scratch-only) as the pack source. */
  baselineDnaUnpack: string;
  coordinatorWasmFileName: string;
  /** This role's `.dna` filename as declared in the baseline happ.yaml. */
  dnaFileName: string;
  happYaml: ParsedHappYaml;
}

/**
 * Unpacks `baselineHapp` and returns ONE role's integrity + coordinator wasm
 * bytes — the currently-installed bytes for that role only, per this file's
 * module doc. Every other role in the baseline is never unpacked to its DNA
 * level at all.
 */
function extractInstalledRoleZomes(
  baselineHapp: string,
  scratchDir: string,
  role: string
): InstalledRoleZomes {
  const baselineHappUnpack = path.join(scratchDir, 'baseline-happ');
  runHc(['app', 'unpack', '--output', baselineHappUnpack, baselineHapp]);

  const happYamlText = readFileSync(path.join(baselineHappUnpack, 'happ.yaml'), 'utf8');
  const happYaml = parseHappYaml(happYamlText);
  const roleBlock = happYaml.roleBlocks.get(role);
  if (!roleBlock) {
    throw new Error(
      `coordinator-candidate: baseline happ has no role "${role}" ` +
        `(has: ${[...happYaml.roleBlocks.keys()].join(', ')})`
    );
  }
  const dnaFileName = roleDnaFileName(roleBlock, role);

  const baselineRoleDna = path.join(baselineHappUnpack, dnaFileName);
  const baselineDnaUnpack = path.join(scratchDir, `baseline-${role}-dna`);
  runHc(['dna', 'unpack', '--output', baselineDnaUnpack, baselineRoleDna]);

  const dnaYamlLines = readFileSync(path.join(baselineDnaUnpack, 'dna.yaml'), 'utf8').split('\n');
  const integrityWasmFileName = zomeWasmFileName(dnaYamlLines, 'integrity');
  const coordinatorWasmFileName = zomeWasmFileName(dnaYamlLines, 'coordinator');

  return {
    integrityWasm: readFileSync(path.join(baselineDnaUnpack, integrityWasmFileName)),
    coordinatorWasm: readFileSync(path.join(baselineDnaUnpack, coordinatorWasmFileName)),
    baselineDnaUnpack,
    coordinatorWasmFileName,
    dnaFileName,
    happYaml,
  };
}

/**
 * Overwrites the coordinator wasm INSIDE the already-unpacked baseline DNA
 * directory (scratch-only, safe to mutate) and repacks it — the integrity
 * wasm and dna.yaml are never touched, so the DNA hash moves only if the
 * coordinator's own wasm-hash inputs do (they don't: coordinator wasm is not
 * part of the DNA hash). This is the same unpack→edit→pack round trip this
 * file's module doc verified reproduces the source bundle's own `hc dna
 * hash` byte-for-byte.
 */
function packCandidateRoleDna(
  baselineDnaUnpack: string,
  coordinatorWasmFileName: string,
  markedCoordinatorWasm: Buffer,
  happDir: string,
  dnaFileName: string
): void {
  writeFileSync(path.join(baselineDnaUnpack, coordinatorWasmFileName), markedCoordinatorWasm);
  mkdirSync(happDir, { recursive: true });
  const candidateDna = path.join(happDir, dnaFileName);
  runHc(['dna', 'pack', baselineDnaUnpack, '-o', candidateDna]);
}

/** Writes a happ.yaml naming ONLY `role` into `happDir` — no other role's
 * `.dna` is copied in, so the resulting `.happ` cannot carry stale bytes for
 * a role nobody meant to change. */
function assembleCandidateHappDir(happDir: string, happYaml: ParsedHappYaml, role: string): void {
  writeFileSync(path.join(happDir, 'happ.yaml'), assembleSingleRoleHappYaml(happYaml, role));
}

/**
 * Mints (and memoizes) a single-role coordinator-only candidate. `markerPayload`
 * is baked into the coordinator wasm's trailing custom section (unless
 * `rollbackToBaseline` is set) so every run — and every distinct channel or
 * candidate within a run — produces a wasm hash nobody has installed yet.
 * `variantLabel` only names the scratch directory; identity comes from the
 * marker + role + rollback flag together (see `cache`).
 *
 * `variantOrOptions` accepts a bare string for backward compatibility with
 * existing call sites that only ever needed a scratch-dir label; new callers
 * should pass a `MintCandidateOptions` object to select `role` and/or
 * `rollbackToBaseline`.
 */
export function mintCoordinatorCandidate(
  baselineHapp: string,
  reportDir: string,
  runStamp: string,
  markerPayload: string,
  variantOrOptions?: string | MintCandidateOptions
): MintedCandidate {
  const options: MintCandidateOptions =
    typeof variantOrOptions === 'string'
      ? { variantLabel: variantOrOptions }
      : (variantOrOptions ?? {});
  const role = options.role ?? DEFAULT_ROLE;
  const rollbackToBaseline = options.rollbackToBaseline ?? false;
  const cacheKey = `${markerPayload}#role=${role}${rollbackToBaseline ? '#rollback' : ''}`;

  const memo = cache.get(cacheKey);
  if (memo) return memo;
  if (!existsSync(baselineHapp)) {
    throw new Error(`revert-target/baseline bundle missing: ${baselineHapp}`);
  }

  const variantSuffix = options.variantLabel ? `-${options.variantLabel}` : '';
  const scratchDir = path.join(reportDir, `candidate-${runStamp}${variantSuffix}`);
  mkdirSync(scratchDir, { recursive: true });

  const {
    integrityWasm,
    coordinatorWasm,
    baselineDnaUnpack,
    coordinatorWasmFileName,
    dnaFileName,
    happYaml,
  } = extractInstalledRoleZomes(baselineHapp, scratchDir, role);
  const baseCoordinatorSha256 = createHash('sha256').update(coordinatorWasm).digest('hex');
  const integrityWasmSha256 = createHash('sha256').update(integrityWasm).digest('hex');

  const markedCoordinatorWasm = rollbackToBaseline
    ? coordinatorWasm
    : appendCustomSection(coordinatorWasm, MARKER_SECTION_NAME, Buffer.from(markerPayload, 'utf8'));
  const coordinatorWasmSha256 = createHash('sha256').update(markedCoordinatorWasm).digest('hex');
  // Evidence for the "two consecutive mints, different channel ids, give
  // different sha256" check the 2026-09-02 diagnosis asked for — a run's log
  // alone proves whether THIS run's mint produced fresh bytes; comparing this
  // line across two runs' logs (or the `candidate-<runStamp>/` scratch dirs
  // on disk) proves it across runs. It also names the role and confirms the
  // integrity wasm was carried through unmodified (2026-09-06 fix).
  // eslint-disable-next-line no-console
  console.log(
    `[coordinator-candidate] run ${runStamp}: role=${role} rollbackToBaseline=${rollbackToBaseline}, ` +
      `integrity wasm sha256=${integrityWasmSha256} (unchanged from baseline), ` +
      `base coordinator wasm sha256=${baseCoordinatorSha256}` +
      (rollbackToBaseline
        ? ' (candidate coordinator wasm kept verbatim — no marker appended)'
        : `, marker section "${MARKER_SECTION_NAME}" payload="${markerPayload}"`) +
      `, candidate coordinator wasm sha256=${coordinatorWasmSha256}`
  );

  const happDir = path.join(scratchDir, 'happ');
  packCandidateRoleDna(
    baselineDnaUnpack,
    coordinatorWasmFileName,
    markedCoordinatorWasm,
    happDir,
    dnaFileName
  );
  assembleCandidateHappDir(happDir, happYaml, role);

  const candidateHappPath = path.join(scratchDir, 'elohim-candidate.happ');
  runHc(['app', 'pack', happDir, '-o', candidateHappPath]);

  const bytes = readFileSync(candidateHappPath);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  // eslint-disable-next-line no-console
  console.log(
    `[coordinator-candidate] run ${runStamp}: packed single-role (${role}) candidate .happ=${candidateHappPath} sha256=${sha256}`
  );
  const minted: MintedCandidate = {
    happPath: candidateHappPath,
    sha256,
    coordinatorWasmSha256,
    role,
    rollbackToBaseline,
  };
  cache.set(cacheKey, minted);
  return minted;
}

// ---------------------------------------------------------------------------
// CLI — a direct, human-runnable path to the same mint the cucumber steps
// use. `workspace-release.md`'s Mint step calls this directly so a developer
// never hand-assembles a full multi-role `.happ` via `just`/build.sh and
// hands it to the packager as `<your.happ>` — the exact mechanism that
// produced the 2026-09-06 mixed-lineage peer this file's module doc
// documents. Guarded by an argv[1] check so importing this module (the
// cucumber step files do) never triggers CLI parsing.
// ---------------------------------------------------------------------------

interface CliArgs {
  baselineHapp: string;
  reportDir: string;
  runStamp: string;
  marker: string;
  role: string;
  rollbackToBaseline: boolean;
  variant?: string;
}

const CLI_USAGE = `Usage: coordinator-candidate.ts --baseline-happ <path> --report-dir <dir> [options]

Mints a single-role coordinator-only candidate .happ from a baseline bundle.

  --baseline-happ <path>       the fleet's (or a household peer's) installed .happ — REQUIRED
  --report-dir <dir>           scratch/output root; the candidate lands at
                                <dir>/candidate-<run-stamp>[-<variant>]/elohim-candidate.happ — REQUIRED
  --run-stamp <stamp>          default: current UTC timestamp, compact form
  --marker <text>              custom-section payload for a forward candidate; ignored with
                                --rollback-to-baseline. Default: a stamp-derived string.
  --role <name>                the ONE role this candidate carries (default: ${DEFAULT_ROLE})
  --rollback-to-baseline       keep the coordinator wasm byte-identical to the baseline (no marker) —
                                mints a revert candidate for the named role
  --variant <label>            scratch-dir suffix only

Exit codes: 0 ok · 1 usage or mint failure.`;

function parseCliArgs(argv: string[]): CliArgs {
  const args: Partial<CliArgs> = { role: DEFAULT_ROLE, rollbackToBaseline: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const next = (): string => {
      i += 1;
      const value = argv.at(i);
      if (value === undefined) throw new Error(`${arg} requires a value`);
      return value;
    };
    switch (arg) {
      case '--baseline-happ':
        args.baselineHapp = next();
        break;
      case '--report-dir':
        args.reportDir = next();
        break;
      case '--run-stamp':
        args.runStamp = next();
        break;
      case '--marker':
        args.marker = next();
        break;
      case '--role':
        args.role = next();
        break;
      case '--variant':
        args.variant = next();
        break;
      case '--rollback-to-baseline':
        args.rollbackToBaseline = true;
        break;
      case '-h':
      case '--help':
        // eslint-disable-next-line no-console
        console.log(CLI_USAGE);
        process.exit(0);
        break;
      default:
        throw new Error(`unrecognized argument: ${arg}\n\n${CLI_USAGE}`);
    }
  }
  if (!args.baselineHapp) throw new Error(`--baseline-happ is required\n\n${CLI_USAGE}`);
  if (!args.reportDir) throw new Error(`--report-dir is required\n\n${CLI_USAGE}`);
  const runStamp = args.runStamp ?? new Date().toISOString().replace(/\D/g, '').slice(0, 14);
  const role = args.role ?? DEFAULT_ROLE;
  const rollbackToBaseline = args.rollbackToBaseline ?? false;
  const marker =
    args.marker ??
    (rollbackToBaseline ? `rollback-${role}-${runStamp}` : `mint-${role}-${runStamp}`);
  return {
    baselineHapp: args.baselineHapp,
    reportDir: args.reportDir,
    runStamp,
    marker,
    role,
    rollbackToBaseline,
    variant: args.variant,
  };
}

function isMainModule(): boolean {
  try {
    const argv1 = process.argv.at(1);
    return argv1 !== undefined && import.meta.url === pathToFileURL(argv1).href;
  } catch {
    return false;
  }
}

if (isMainModule()) {
  try {
    const args = parseCliArgs(process.argv.slice(2));
    const minted = mintCoordinatorCandidate(
      path.resolve(args.baselineHapp),
      path.resolve(args.reportDir),
      args.runStamp,
      args.marker,
      { role: args.role, rollbackToBaseline: args.rollbackToBaseline, variantLabel: args.variant }
    );
    // eslint-disable-next-line no-console
    console.log(JSON.stringify(minted, null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
