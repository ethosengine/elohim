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
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

/** genesis/a2o/steps/delivery -> repo root (4 levels up) — mirrors the main steps file. */
const REPO_ROOT = fileURLToPath(new URL('../../../../', import.meta.url));

const HC_BIN = process.env['ELOHIM_HC_BIN'] ?? '/opt/holochain/bin/hc';

const ELOHIM_DNA_DIR = path.join(REPO_ROOT, 'elohim/holochain/dna/elohim');
const SOURCE_DNA_YAML = path.join(ELOHIM_DNA_DIR, 'dna.yaml');
const WORKDIR_DIR = path.join(ELOHIM_DNA_DIR, 'workdir');
const WORKDIR_HAPP_YAML = path.join(WORKDIR_DIR, 'happ.yaml');
const NODE_REGISTRY_DNA_SRC = path.join(
  REPO_ROOT,
  'elohim/holochain/dna/node-registry/node-registry.dna'
);

const LAMAD_DNA_FILENAME = 'lamad.dna';
const INTEGRITY_WASM_FILENAME = 'content_store_integrity.wasm';
const COORDINATOR_WASM_FILENAME = 'content_store.wasm';
const MARKER_SECTION_NAME = 'elohim.coord-build-marker';

export interface MintedCandidate {
  /** Absolute path to the freshly-packed `.happ` bundle. */
  happPath: string;
  /** sha256 (hex) of the packed bundle's bytes — for the blob PUT. */
  sha256: string;
}

/** Memoized per-process — every scenario in one run shares the same candidate. */
let cached: MintedCandidate | undefined;

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

/** Rewrites the node_registry role's cross-DNA-dir path to a local sibling copy. */
function localizeNodeRegistryPath(happYaml: string): string {
  const before = 'path: "../../node-registry/node-registry.dna"';
  const after = 'path: "node-registry.dna"';
  if (!happYaml.includes(before)) {
    throw new Error(`workdir happ.yaml no longer contains the expected node_registry path line`);
  }
  return happYaml.replace(before, after);
}

/**
 * Unpacks `BASELINE_HAPP`'s lamad role and returns its integrity +
 * coordinator wasm bytes — the currently-installed bytes, per this file's
 * module doc.
 */
function extractInstalledLamadZomes(
  baselineHapp: string,
  scratchDir: string
): { integrityWasm: Buffer; coordinatorWasm: Buffer } {
  const baselineHappUnpack = path.join(scratchDir, 'baseline-happ');
  runHc(['app', 'unpack', '--output', baselineHappUnpack, baselineHapp]);

  const baselineLamadDna = path.join(baselineHappUnpack, LAMAD_DNA_FILENAME);
  const baselineDnaUnpack = path.join(scratchDir, 'baseline-lamad-dna');
  runHc(['dna', 'unpack', '--output', baselineDnaUnpack, baselineLamadDna]);

  return {
    integrityWasm: readFileSync(path.join(baselineDnaUnpack, INTEGRITY_WASM_FILENAME)),
    coordinatorWasm: readFileSync(path.join(baselineDnaUnpack, COORDINATOR_WASM_FILENAME)),
  };
}

function packCandidateLamadDna(
  scratchDir: string,
  integrityWasm: Buffer,
  markedCoordinatorWasm: Buffer
): string {
  const dnaWorkDir = path.join(scratchDir, 'dna-work');
  const releaseDir = path.join(dnaWorkDir, 'target/wasm32-unknown-unknown/release');
  mkdirSync(releaseDir, { recursive: true });
  copyFileSync(SOURCE_DNA_YAML, path.join(dnaWorkDir, 'dna.yaml'));
  writeFileSync(path.join(releaseDir, INTEGRITY_WASM_FILENAME), integrityWasm);
  writeFileSync(path.join(releaseDir, COORDINATOR_WASM_FILENAME), markedCoordinatorWasm);

  const happDir = path.join(scratchDir, 'happ');
  mkdirSync(happDir, { recursive: true });
  const candidateLamadDna = path.join(happDir, LAMAD_DNA_FILENAME);
  runHc(['dna', 'pack', dnaWorkDir, '-o', candidateLamadDna]);
  return happDir;
}

function assembleCandidateHappDir(happDir: string): void {
  const happYaml = localizeNodeRegistryPath(readFileSync(WORKDIR_HAPP_YAML, 'utf8'));
  writeFileSync(path.join(happDir, 'happ.yaml'), happYaml);
  for (const dnaFile of ['imagodei.dna', 'infrastructure.dna', 'mishpat.dna']) {
    copyFileSync(path.join(WORKDIR_DIR, dnaFile), path.join(happDir, dnaFile));
  }
  copyFileSync(NODE_REGISTRY_DNA_SRC, path.join(happDir, 'node-registry.dna'));
}

/**
 * Mints (and memoizes) this run's coordinator-only candidate. `markerPayload`
 * is baked into the coordinator wasm's trailing custom section so every run
 * — and every distinct channel within a run — produces a wasm hash nobody
 * has installed yet.
 */
export function mintCoordinatorCandidate(
  baselineHapp: string,
  reportDir: string,
  runStamp: string,
  markerPayload: string
): MintedCandidate {
  if (cached) return cached;
  if (!existsSync(baselineHapp)) {
    throw new Error(`revert-target/baseline bundle missing: ${baselineHapp}`);
  }

  const scratchDir = path.join(reportDir, `candidate-${runStamp}`);
  mkdirSync(scratchDir, { recursive: true });

  const { integrityWasm, coordinatorWasm } = extractInstalledLamadZomes(baselineHapp, scratchDir);
  const markedCoordinatorWasm = appendCustomSection(
    coordinatorWasm,
    MARKER_SECTION_NAME,
    Buffer.from(markerPayload, 'utf8')
  );

  const happDir = packCandidateLamadDna(scratchDir, integrityWasm, markedCoordinatorWasm);
  assembleCandidateHappDir(happDir);

  const candidateHappPath = path.join(scratchDir, 'elohim-candidate.happ');
  runHc(['app', 'pack', happDir, '-o', candidateHappPath]);

  const bytes = readFileSync(candidateHappPath);
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  cached = { happPath: candidateHappPath, sha256 };
  return cached;
}
