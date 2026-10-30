/**
 * Household receipt → signed brit ValidationAttestation (native-delivery sprint, lane F2).
 *
 * A household run (`just test mesh`) already writes a sprint report keyed by `env.sut`, the
 * content identity of the source it ran. This module turns that report into one
 * `ValidationAttestationContentNode` per measured concern, written with `brit-build-ref validate
 * put` under `refs/notes/brit/validate/a2o-household/<concern>/<sut>` and signed by the workspace
 * key (`<git-common-dir>/brit/agent-key`); the berth mooring that names who ran it rides in the
 * summary. The pre-push T2 reader (genesis/orchestrator/scripts/serving-receipt.mjs) admits such
 * an attestation as the receipt — reach `trusted`, the household form of verification (F1's
 * mapping: Verified on the household → trusted) — in preference to a report file's contents.
 *
 * The attestation's `artifactCid` is a raw (0x55) sha2-256 CIDv1 over the SAME canonical
 * `name=value` text `hashSutParts` hashes, so its short fingerprint IS `env.sut`: the attestation
 * is keyed by the tree it verified, not by a timestamp.
 *
 * Two brit-build-ref lineages answer on a host, and this module speaks to both: the pinned
 * 76ba95d8a0 line (no `signer`; `validate get` prints stored bytes unverified) and the
 * verified-reads line (`signer --json`; `get` prints only a record that decodes strictly,
 * verifies against its own signer and binds to its ref — anything else is a stderr
 * `unverifiable: <ref> <reason>` line, exit 0). `resolveWorkspaceIdentity` tells them apart by the
 * clap usage error (exit 2) the pinned build prints for `signer`. A record brit reports
 * unverifiable is ABSENT to every reader here and warned once, naming the ref and the reason;
 * under the pinned build the reader verifies the stored bytes itself and says so.
 *
 * This file is imported both by build-sprint-report.ts (tsx) and by serving-receipt.mjs (node's
 * type stripping), and its session adapter both use only node builtins at runtime.
 *
 * Ownership of the signer-preference rule (`signer --json` first; the seed file only under the
 * pinned brit): genesis/orchestrator/gate-evidence.mjs `signerIdentity` owns it for every
 * orchestrator reader (delivery-series.mjs delegates). `resolveWorkspaceIdentity` below is the
 * a2o-side port of that one rule, kept as a separate function only because this module must stay
 * node-builtins-only and must not import the orchestrator package; its test pins the same
 * streams, and a change to the rule lands in both places together. `classifyPutRefusal` is the
 * same kind of port of genesis/orchestrator/scripts/brit-attest.mjs `expectedRefusal` (brit's two
 * expected `build put` refusals), with this module's hyphenated kinds; the two change together.
 */

import { execFileSync, spawnSync } from 'node:child_process';
import { createHash, createPrivateKey, createPublicKey, verify as cryptoVerify } from 'node:crypto';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { basename, delimiter, dirname, join } from 'node:path';

import { resolve as resolveSession } from '../../src/framework/session.ts';

export const ATTESTATION_STEP = 'a2o-household';
export const ATTESTATION_REF_PREFIX = `refs/notes/brit/validate/${ATTESTATION_STEP}/`;
export const RECEIPT_SCHEMA = 'a2o-household-receipt@1';
export const VALIDATOR_VERSION = 'a2o/build-sprint-report@1';

type Parts = Record<string, string>;

export interface ReceiptScenario {
  name: string;
  status: string;
  surface: string;
  durationMs: number;
}

export interface Mooring {
  session: string;
  principal: string | null;
  recipient: Record<string, unknown>;
}

/** What the attestation's `resultSummary` carries (JSON text). */
export interface ReceiptSummary {
  schema: typeof RECEIPT_SCHEMA;
  concern: string;
  sut: string;
  sutParts: Parts;
  lane: string;
  processControl: boolean;
  runId: string;
  report: string;
  /** Protocol reach this verification earns (F1: Verified on the household → trusted). */
  reach: 'trusted';
  moored: Mooring | null;
  scenarios: ReceiptScenario[];
  /**
   * Present only when the stage ran on a peer: the DHT chain this workspace verified before it
   * signed. The reader re-verifies it against the requester's own task record
   * (`GET /api/v1/compute/tasks/<requestActionHash>`); the brit signature vouches only that this
   * workspace verified the chain and decoded the report, never for the provider.
   */
  peer?: PeerStage;
}

/**
 * A peer-executed stage's receipt chain. Every key here is a Holochain identity or a content
 * digest the provider's completion carries; none is ever compared to a brit key.
 */
export interface PeerStage {
  /** H = household stand-in (jessica); A = a real peer (adam). */
  rung: 'H' | 'A';
  provider: string;
  requester: string;
  grantActionHash: string;
  grantCid: string;
  scope: string;
  requestActionHash: string;
  completionActionHash: string;
  receiptCid: string;
  taskCid: string;
  featureSha256: string;
  reportSha256: string;
}

/**
 * The requester storage's task record (`GET /api/v1/compute/tasks/<requestActionHash>`). Storage
 * re-derives `taskCid` from the envelope before it answers (`verify_status`), so the task
 * identity here is the envelope's content address, not a claim.
 */
export interface PeerTaskStatus {
  taskCid?: string;
  requestActionHash?: string;
  state?: string;
  requester?: string;
  provider?: string;
  grantActionHash?: string;
  envelope?: { project?: string; dna?: { sha256?: string } };
  completion?: { actionHash?: string; receiptCid?: string } | null;
  refusal?: unknown;
  observed?: {
    verified?: boolean;
    grantCid?: string;
    scope?: string;
    grantProvider?: string;
    grantRecipient?: string;
    fulfilledEventId?: string;
    refused?: string;
  } | null;
}

/** The grant scope a measure stage runs under (the event class, never the project prefix). */
export const PEER_STAGE_SCOPE = 'measure-stage';
export const PEER_STAGE_PROJECT_PREFIX = 'a2o-stage:';

export interface ConcernAttestation {
  check: string;
  artifact: string;
  /** Three-valued, as every verification outcome is: brit's `warn` is never emitted here. */
  result: 'pass' | 'fail' | 'skip';
  summary: ReceiptSummary;
}

// ─── source identity ────────────────────────────────────────────────────────

/** The exact text `hashSutParts` (lib/sut.ts) digests; the parity test pins the two together. */
export function canonicalSutParts(parts: Parts): string {
  return Object.keys(parts)
    .sort((a, b) => a.localeCompare(b))
    .map(key => `${key}=${parts[key]}`)
    .join('\n');
}

function sha256(input: string | Uint8Array): Buffer {
  return createHash('sha256').update(input).digest();
}

/** `sha256:<16 hex>` — equal to `hashSutParts(parts)`. */
export function sutOf(parts: Parts): string {
  return `sha256:${sha256(canonicalSutParts(parts)).toString('hex').slice(0, 16)}`;
}

/** CIDv1 bytes (no multibase prefix): version 1, raw codec 0x55, sha2-256 multihash. */
export function sutArtifactCidBytes(parts: Parts): Uint8Array {
  return Uint8Array.from([0x01, 0x55, 0x12, 0x20, ...sha256(canonicalSutParts(parts))]);
}

const BASE32 = 'abcdefghijklmnopqrstuvwxyz234567';

/** Multibase base32-lower text of CID bytes (`bafkrei…` for a raw sha2-256 CIDv1). */
export function cidToString(bytes: Uint8Array): string {
  let out = 'b';
  let buffer = 0;
  let bits = 0;
  for (const byte of bytes) {
    buffer = (buffer << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += BASE32[(buffer >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += BASE32[(buffer << (5 - bits)) & 31];
  return out;
}

/** The short form of a raw sha2-256 CID's digest — `sha256:<16 hex>`, or null for another shape. */
export function cidShortFingerprint(bytes: readonly number[] | Uint8Array): string | null {
  const b = Array.from(bytes);
  if (b.length !== 36 || b[0] !== 0x01 || b[1] !== 0x55 || b[2] !== 0x12 || b[3] !== 0x20) {
    return null;
  }
  return `sha256:${Buffer.from(b.slice(4)).toString('hex').slice(0, 16)}`;
}

export function checkNameFor(concern: string, sut: string): string {
  return `${concern}/${sut}`;
}

// ─── writer side ────────────────────────────────────────────────────────────

interface ReportLike {
  runId?: string;
  env?: { lane?: string; processControl?: boolean; sut?: string; sutParts?: Parts };
  summary?: {
    byConcern?: Record<
      string,
      {
        failed?: number;
        scenarios?: { name: string; status: string; surface: string; durationMs?: number }[];
      }
    >;
  };
}

/** failed if any station failed; passed only if every one passed; otherwise not proven (skip). */
function resultFor(scenarios: ReceiptScenario[]): ConcernAttestation['result'] {
  if (scenarios.some(s => s.status === 'failed')) return 'fail';
  if (scenarios.length > 0 && scenarios.every(s => s.status === 'passed')) return 'pass';
  return 'skip';
}

/**
 * One attestation per measured concern of a household, process-controlled run. Anything else —
 * a fleet run, a run whose `sut` does not re-derive from its own parts — yields none: it cannot
 * say which tree it verified.
 */
export function attestationsFromReport(
  report: ReportLike,
  reportName: string,
  moored: Mooring | null
): ConcernAttestation[] {
  const env = report.env;
  if (env?.lane !== 'household' || env.processControl !== true) return [];
  const parts = env.sutParts ?? {};
  if (!env.sut || Object.keys(parts).length === 0 || sutOf(parts) !== env.sut) return [];
  const artifact = cidToString(sutArtifactCidBytes(parts));
  return Object.entries(report.summary?.byConcern ?? {})
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([concern, rollup]) => {
      const scenarios: ReceiptScenario[] = (rollup.scenarios ?? []).map(s => ({
        name: s.name,
        status: s.status,
        surface: s.surface,
        durationMs: s.durationMs ?? 0,
      }));
      return {
        check: checkNameFor(concern, env.sut as string),
        artifact,
        result: resultFor(scenarios),
        summary: {
          schema: RECEIPT_SCHEMA,
          concern,
          sut: env.sut as string,
          sutParts: parts,
          lane: 'household',
          processControl: true,
          runId: report.runId ?? '',
          report: reportName,
          reach: 'trusted',
          moored,
          scenarios,
        },
      };
    });
}

type Env = Readonly<Record<string, string | undefined>>;

/** The berth mooring of this session (genesis/agentic/bin/berth), or null when none is on record. */
export function resolveMooring(env: Env): Mooring | null {
  const session = resolveSession({ env }).session;
  if (!session) return null;
  // eslint-disable-next-line @typescript-eslint/prefer-nullish-coalescing
  const dir = env.BERTH_DIR || join(env.CLAUDE_CONFIG_DIR || '/projects/.claude-config', 'berth');
  const safe = [...session].map(c => (/[A-Za-z0-9_.-]/.test(c) ? c : '_')).join('');
  try {
    const m = JSON.parse(readFileSync(join(dir, 'moorings', `${safe}.json`), 'utf8'));
    return {
      session,
      principal: typeof m.principal === 'string' ? m.principal : null,
      recipient: m.recipient && typeof m.recipient === 'object' ? m.recipient : {},
    };
  } catch {
    return null;
  }
}

/** An executable named by `explicit`, else found on PATH; null when absent. */
export function resolveBinary(
  name: string,
  explicit: string | undefined,
  path: string | undefined
) {
  const candidates = explicit ? [explicit] : (path ?? '').split(delimiter).map(d => join(d, name));
  for (const c of candidates) {
    try {
      if (c && statSync(c).isFile()) return c;
    } catch {
      /* not here */
    }
  }
  return null;
}

/** The workspace's git common dir (shared by every worktree), or null. */
export function gitCommonDir(repoRoot: string): string | null {
  try {
    // eslint-disable-next-line sonarjs/no-os-command-from-path -- git is a standard system tool
    return execFileSync('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], {
      cwd: repoRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();
  } catch {
    return null;
  }
}

export type Runner = (
  cmd: string,
  args: string[],
  cwd: string
) => { status: number | null; stdout: string; stderr: string };

const defaultRunner: Runner = (cmd, args, cwd) => {
  const r = spawnSync(cmd, args, { cwd, encoding: 'utf8', timeout: 120_000 });
  return { status: r.status, stdout: r.stdout ?? '', stderr: r.stderr ?? String(r.error ?? '') };
};

// ─── which brit-build-ref answers ────────────────────────────────────────────

/**
 * The brit-build-ref lineage on this host. `verified-reads` knows `signer` and its `get` prints
 * only a verified record; `pinned` (76ba95d8a0) prints a clap usage error for `signer` and its
 * `validate get` prints stored bytes unverified; `absent` is no binary at all.
 */
export interface BritProbe {
  kind: typeof VERIFIED_READS | 'pinned' | 'absent';
  bin: string | null;
}

/** The first stderr line clap prints (exit 2) for a subcommand a binary does not know. */
const UNKNOWN_SUBCOMMAND = /^error: unrecognized subcommand '([^']+)'/;

function firstLine(text: string): string {
  return text.trim().split('\n')[0] ?? '';
}

/** Who answered for the workspace key, and what it said. */
export interface WorkspaceIdentity {
  /** Hex ed25519 public key brit signs with, or null when none is on record. */
  agentId: string | null;
  /** `brit signer` is the binary's own answer; the file is read only when brit cannot be asked. */
  source: typeof SIGNER | typeof KEY_FILE;
  brit: BritProbe;
  /** Why a brit that knows `signer` named no key (its first stderr line); absent otherwise. */
  note?: string;
}

const SIGNER = 'brit signer';
const KEY_FILE = 'agent-key file';
const VERIFIED_READS = 'verified-reads';

/**
 * The workspace's signing identity, preferring `brit-build-ref signer --json` (never mints a key;
 * `{"agentId":null}` is no key on record, exit 1 is a key brit refuses to load — neither is
 * second-guessed by reading the file). The seed file is read only when the pinned brit answers
 * `signer` with clap's usage error, or when no brit-build-ref is on this host at all. The binary
 * is `BRIT_BUILD_REF_BIN`, else `BRIT_BUILD_REF`, else the first on `PATH`.
 */
export function resolveWorkspaceIdentity(
  repoRoot: string,
  env: Env,
  run: Runner = defaultRunner
): WorkspaceIdentity {
  // BRIT_BUILD_REF_BIN is the orchestrator's name for the same binary (gate-evidence.mjs).
  const explicit = env.BRIT_BUILD_REF_BIN ?? env.BRIT_BUILD_REF;
  const bin = resolveBinary('brit-build-ref', explicit, env.PATH);
  if (bin) {
    const r = run(bin, ['--repo', repoRoot, 'signer', '--json'], repoRoot);
    const err = firstLine(r.stderr);
    const unknown = r.status === 2 && UNKNOWN_SUBCOMMAND.exec(err)?.[1] === 'signer';
    if (!unknown) {
      const brit: BritProbe = { kind: VERIFIED_READS, bin };
      if (r.status !== 0) {
        const note = err || `signer --json exited ${r.status ?? 'without a status'}`;
        return { agentId: null, source: SIGNER, brit, note };
      }
      let id: unknown;
      try {
        id = (JSON.parse(r.stdout) as { agentId?: unknown })?.agentId;
      } catch {
        return { agentId: null, source: SIGNER, brit, note: 'signer --json printed no JSON' };
      }
      if (id === null) return { agentId: null, source: SIGNER, brit };
      if (typeof id === 'string' && /^[0-9a-f]{64}$/.test(id)) {
        return { agentId: id, source: SIGNER, brit };
      }
      return { agentId: null, source: SIGNER, brit, note: 'signer --json named no 64-hex agentId' };
    }
  }
  const brit: BritProbe = { kind: bin ? 'pinned' : 'absent', bin };
  const common = gitCommonDir(repoRoot);
  return { agentId: common ? workspaceAgentId(common) : null, source: KEY_FILE, brit };
}

/** What a reader that verified the bytes itself says about its verifier. */
function localVerifier(kind: BritProbe['kind']): string {
  return kind === 'pinned'
    ? 'verified locally; the pinned brit-build-ref has no verified read'
    : 'verified locally; brit-build-ref is not on this host';
}

// ─── ref segments ───────────────────────────────────────────────────────────

/** The characters brit's `encode_segment` percent-encodes in a ref path segment. */
const SEGMENT_ESCAPES: [string, string][] = [
  [':', '%3A'],
  ['@', '%40'],
  ['?', '%3F'],
  ['*', '%2A'],
  ['[', '%5B'],
  ['\\', '%5C'],
  ['^', '%5E'],
  ['~', '%7E'],
  [' ', '%20'],
];

/** `encode_segment` (brit-epr refs.rs): the ref path brit writes a step or check under. */
export function encodeRefSegment(s: string): string {
  let out = '';
  for (const c of s) {
    const code = c.codePointAt(0) ?? 0;
    const esc = SEGMENT_ESCAPES.find(([plain]) => plain === c)?.[1];
    if (esc) out += esc;
    else if (code < 0x20 || code === 0x7f)
      out += `%${code.toString(16).toUpperCase().padStart(2, '0')}`;
    else out += c;
  }
  return out.replaceAll('..', '%2E%2E');
}

/** `decode_segment`: the check a validate ref's path names. */
export function decodeRefSegment(s: string): string {
  let out = s;
  for (const [plain, esc] of SEGMENT_ESCAPES) out = out.replaceAll(esc, plain);
  return out.replaceAll('%2E%2E', '..');
}

/** The ref `validate put --step a2o-household --check <check>` writes. */
export function attestationRef(check: string): string {
  return `${ATTESTATION_REF_PREFIX}${encodeRefSegment(check)}`;
}

/**
 * One `brit-build-ref validate put` of `attestation` under the workspace key of `workspace` (the
 * directory holding the `.git` common dir). `cwd` defaults to the workspace; the household writer
 * runs it from its repo root. Returns the runner's result; never throws on a non-zero exit.
 */
/** One of the two refusals `brit-build-ref validate put` prints on the verified-read line. */
export interface PutRefusal {
  kind: 'concurrent-update' | 'would-not-verify';
  /** The ref brit named, else `fallbackRef`. */
  ref: string;
  reason: string;
  /** The first stderr line, verbatim. */
  line: string;
}

/**
 * The two refusals `brit-build-ref … put` prints (brit 7e4a83115e), read from the FIRST stderr
 * line only (`Error: …` is anyhow's main). Both are the writer's contract working — a ref another
 * writer moved, or bytes that would not read back — never a failure of the caller: the report on
 * disk stays the evidence and the put is deferred. One owner for every writer (this module's
 * publishHouseholdEvidence and genesis/agentic/compute/stage/collect-stage.mjs).
 *
 *   Error: REFUSED concurrent update: ref=<ref> expected=<oid> observed=<oid> attempts=<n>
 *   Error: refusing to write <ref>: <reason>
 */
export function classifyPutRefusal(
  stderr: string | undefined,
  fallbackRef: string
): PutRefusal | null {
  const line = firstLine(stderr ?? '');
  const concurrent = /^Error: REFUSED concurrent update: (.*)$/.exec(line);
  if (concurrent) {
    const ref = /\bref=(\S+)/.exec(concurrent[1])?.[1] ?? fallbackRef;
    return { kind: 'concurrent-update', ref, reason: concurrent[1], line };
  }
  const wouldNotVerify = /^Error: refusing to write (\S+): (.*)$/.exec(line);
  if (wouldNotVerify) {
    return {
      kind: 'would-not-verify',
      ref: wouldNotVerify[1] || fallbackRef,
      reason: wouldNotVerify[2],
      line,
    };
  }
  return null;
}

export function putAttestation(
  brit: string,
  workspace: string,
  attestation: ConcernAttestation,
  run: Runner = defaultRunner,
  cwd: string = workspace
): ReturnType<Runner> {
  return run(
    brit,
    [
      '--repo',
      workspace,
      'validate',
      'put',
      '--step',
      ATTESTATION_STEP,
      '--check',
      attestation.check,
      '--artifact',
      attestation.artifact,
      '--result',
      attestation.result,
      '--summary',
      JSON.stringify(attestation.summary),
      '--validator-version',
      VALIDATOR_VERSION,
    ],
    cwd
  );
}

/**
 * After a household report is written: `epr flow fulfill <report>` (when `epr` exists) and one
 * `brit-build-ref validate put` per concern (when brit-build-ref exists). Never throws and never
 * changes the run's verdict — the report on disk is the evidence; these are its projections.
 * Returns the lines it printed.
 */
export function publishHouseholdEvidence(opts: {
  report: ReportLike;
  reportPath: string;
  repoRoot: string;
  env: Env;
  run?: Runner;
  log?: (line: string) => void;
}): string[] {
  const lines: string[] = [];
  const log = (line: string) => {
    lines.push(line);
    (opts.log ?? console.log)(line);
  };
  const run = opts.run ?? defaultRunner;
  const { report, reportPath, repoRoot, env } = opts;
  if (env.A2O_POST_REPORT === '0') return lines;
  if (report.env?.lane !== 'household') return lines;

  const epr = env.EPR_BIN ?? resolveBinary('epr', undefined, env.PATH);
  if (epr) {
    const r = run(epr, ['flow', 'fulfill', reportPath, '--root', repoRoot], repoRoot);
    log(
      r.status === 0
        ? `epr flow fulfill: ${r.stdout.trim().split('\n').pop() ?? 'ok'}`
        : `epr flow fulfill FAILED (exit ${r.status}): ${r.stderr.trim().split('\n').pop() ?? ''}`
    );
  } else {
    log('epr flow fulfill skipped: no epr binary on this host (EPR_BIN or PATH)');
  }

  const identity = resolveWorkspaceIdentity(repoRoot, env, run);
  const brit = identity.brit.bin;
  if (!brit) {
    log(
      'validation attestation skipped: brit-build-ref is not built on this host ' +
        '(BRIT_BUILD_REF or PATH; cargo build -p brit-build-ref in elohim/brit)'
    );
    return lines;
  }
  const common = gitCommonDir(repoRoot);
  if (!common || basename(common) !== '.git') {
    log('validation attestation skipped: no .git common dir to hold the workspace key');
    return lines;
  }
  const workspace = dirname(common);
  const moored = resolveMooring(env);
  const attestations = attestationsFromReport(report, basename(reportPath), moored);
  if (attestations.length === 0) {
    log('validation attestation skipped: the report does not name a household source identity');
    return lines;
  }
  let written = 0;
  let kept = 0;
  for (const a of attestations) {
    // Read first: what is at the ref decides whether this put is unneeded, or replaces a record.
    const prior = readAttestationRef(brit, identity.brit.kind, workspace, a.check, run, repoRoot);
    if (prior.ok) {
      const same =
        prior.node.resultSummary === JSON.stringify(a.summary) &&
        prior.node.result === a.result &&
        (identity.agentId === null || prior.node.validatorId === identity.agentId);
      if (same) {
        kept += 1;
        log(
          `validation attestation already on record for ${a.check} ` +
            `(run ${a.summary.runId}, verified by ${prior.verifiedBy}); not rewritten`
        );
        continue;
      }
    } else if (prior.reason !== null) {
      log(`household attestation unverifiable: ${prior.reason}; replacing it`);
    }
    const r = putAttestation(brit, workspace, a, run, repoRoot);
    if (r.status === 0) {
      written += 1;
      continue;
    }
    const refusal = classifyPutRefusal(r.stderr, attestationRef(a.check));
    if (refusal) {
      log(
        `validation attestation deferred for ${a.check} (${refusal.kind} on ${refusal.ref}): ${refusal.reason}`
      );
    } else {
      log(`validation attestation FAILED for ${a.check} (exit ${r.status}): ${r.stderr.trim()}`);
    }
  }
  const who = moored ? `${moored.principal ?? '?'} via ${moored.session}` : 'no berth mooring';
  const note = identity.note ? `: ${identity.note}` : '';
  const key = identity.agentId
    ? `workspace key ${identity.agentId.slice(0, 12)}… via ${identity.source}`
    : `no workspace key via ${identity.source}${note}`;
  log(
    `validation attestations: ${written + kept}/${attestations.length} written under ` +
      `${ATTESTATION_REF_PREFIX}<concern>/${attestations[0].summary.sut} (${key}; ${who})` +
      (kept ? `; ${kept} already on record` : '')
  );
  return lines;
}

// ─── reader side ────────────────────────────────────────────────────────────

/** The JSON a validate ref blob holds (serde camelCase of ValidationAttestationContentNode). */
export interface ValidationNodeJson {
  artifactCid: number[];
  checkName: string;
  findingsCid: number[] | null;
  result: string;
  resultSummary: string;
  signature: string;
  ttlSec: number | null;
  validatedAt: string;
  validatorId: string;
  validatorVersion: string;
}

const NODE_KEYS = [
  'artifactCid',
  'checkName',
  'findingsCid',
  'result',
  'resultSummary',
  'signature',
  'ttlSec',
  'validatedAt',
  'validatorId',
  'validatorVersion',
];

function cborHead(major: number, n: number): Buffer {
  if (n < 24) return Buffer.from([(major << 5) | n]);
  if (n < 0x100) return Buffer.from([(major << 5) | 24, n]);
  if (n < 0x10000) return Buffer.from([(major << 5) | 25, n >> 8, n & 0xff]);
  if (n < 0x100000000) {
    const b = Buffer.alloc(5);
    b[0] = (major << 5) | 26;
    b.writeUInt32BE(n, 1);
    return b;
  }
  const b = Buffer.alloc(9);
  b[0] = (major << 5) | 27;
  b.writeBigUInt64BE(BigInt(n), 1);
  return b;
}

function cborText(s: string): Buffer {
  const bytes = Buffer.from(s, 'utf8');
  return Buffer.concat([cborHead(3, bytes.length), bytes]);
}

function cborCid(bytes: number[]): Buffer {
  // DAG-CBOR link: tag 42 over a byte string of 0x00 (identity multibase) + CID bytes.
  const body = Buffer.from([0x00, ...bytes]);
  return Buffer.concat([Buffer.from([0xd8, 0x2a]), cborHead(2, body.length), body]);
}

/**
 * Canonical DAG-CBOR of a validation node — the bytes brit signs (with `signature: ""`) and
 * stores (with the signature). Keys sort by encoded length, then bytewise (RFC 8949 §4.2.3).
 * An unexpected key or type throws: an unknown shape is never "verified".
 */
export function encodeValidationNode(node: ValidationNodeJson): Buffer {
  const keys = Object.keys(node);
  if (keys.length !== NODE_KEYS.length || !NODE_KEYS.every(k => keys.includes(k))) {
    throw new Error('validation node has an unexpected field set');
  }
  const record = node as unknown as Record<string, unknown>;
  const ordered = [...NODE_KEYS].sort(
    (a, b) => a.length - b.length || (a < b ? -1 : a > b ? 1 : 0)
  );
  const parts: Buffer[] = [cborHead(5, ordered.length)];
  for (const key of ordered) {
    parts.push(cborText(key));
    const value = record[key];
    if (value === null) parts.push(Buffer.from([0xf6]));
    else if (key === 'artifactCid' || key === 'findingsCid') {
      if (!Array.isArray(value) || !value.every(n => Number.isInteger(n) && n >= 0 && n < 256)) {
        throw new Error(`${key} is not CID bytes`);
      }
      parts.push(cborCid(value as number[]));
    } else if (key === 'ttlSec') {
      if (!Number.isSafeInteger(value) || (value as number) < 0) throw new Error('ttlSec');
      parts.push(cborHead(0, value as number));
    } else if (typeof value === 'string') parts.push(cborText(value));
    else throw new Error(`${key} has an unexpected type`);
  }
  return Buffer.concat(parts);
}

const SPKI_ED25519 = Buffer.from('302a300506032b6570032100', 'hex');
const PKCS8_ED25519 = Buffer.from('302e020100300506032b657004220420', 'hex');

/** ed25519 over the unsigned node's canonical bytes, against the node's own validatorId. */
export function verifyValidationNode(node: ValidationNodeJson): boolean {
  try {
    const pub = Buffer.from(node.validatorId, 'hex');
    const sig = Buffer.from(node.signature, 'hex');
    if (pub.length !== 32 || sig.length !== 64) return false;
    const key = createPublicKey({
      key: Buffer.concat([SPKI_ED25519, pub]),
      format: 'der',
      type: 'spki',
    });
    return cryptoVerify(null, encodeValidationNode({ ...node, signature: '' }), key, sig);
  } catch {
    return false;
  }
}

/** Hex public key of the workspace key brit signs with (`<common-dir>/brit/agent-key`), or null. */
export function workspaceAgentId(commonDir: string): string | null {
  const path = join(commonDir, 'brit', 'agent-key');
  if (!existsSync(path)) return null;
  try {
    const seed = readFileSync(path);
    if (seed.length !== 32) return null;
    const priv = createPrivateKey({
      key: Buffer.concat([PKCS8_ED25519, seed]),
      format: 'der',
      type: 'pkcs8',
    });
    const der = createPublicKey(priv).export({ format: 'der', type: 'spki' });
    return Buffer.from(der).subarray(SPKI_ED25519.length).toString('hex');
  } catch {
    return null;
  }
}

/** One verified read of a validate ref: the node, or why it is absent to this reader. */
type AttestationRead =
  | { ok: true; node: ValidationNodeJson; verifiedBy: 'brit' | 'local' }
  | { ok: false; reason: string | null };

/**
 * `brit-build-ref validate get` for `check`: stdout is the verified record; otherwise the
 * `unverifiable:` stderr line (`<ref> <reason>`) is the reason, a bare miss is `null` (nothing
 * stored), and a non-zero exit or non-JSON stdout is a reason of its own — never a hit.
 */
function britValidateGet(
  bin: string,
  repo: string,
  check: string,
  run: Runner,
  cwd: string
): AttestationRead {
  const r = run(
    bin,
    ['--repo', repo, 'validate', 'get', '--step', ATTESTATION_STEP, '--check', check],
    cwd
  );
  const ref = attestationRef(check);
  if (r.status !== 0) {
    const exit = r.status ?? 'without a status';
    return {
      ok: false,
      reason: `${ref} brit-build-ref validate get exited ${exit}: ${firstLine(r.stderr)}`,
    };
  }
  if (r.stdout.trim()) {
    try {
      return { ok: true, node: JSON.parse(r.stdout), verifiedBy: 'brit' };
    } catch {
      return { ok: false, reason: `${ref} brit-build-ref validate get printed no JSON` };
    }
  }
  const unverifiable = r.stderr.split('\n').find(l => l.startsWith('unverifiable: '));
  if (unverifiable) {
    const rest = unverifiable.slice('unverifiable: '.length);
    return { ok: false, reason: `${rest} (brit-build-ref verified read)` };
  }
  return { ok: false, reason: null };
}

/**
 * The reader's own verification of stored bytes — the pinned brit prints them unverified — with
 * brit's reasons: strict shape, the signature against the node's own `validatorId`, and the
 * check the ref names.
 */
function localVerifiedRead(
  bytes: string | null,
  ref: string,
  check: string,
  kind: BritProbe['kind']
): AttestationRead {
  const refuse = (why: string): AttestationRead => ({
    ok: false,
    reason: `${ref} ${why} (${localVerifier(kind)})`,
  });
  if (bytes === null) return { ok: false, reason: null };
  let node: ValidationNodeJson;
  try {
    node = JSON.parse(bytes);
  } catch (error) {
    return refuse(`does not decode: ${(error as Error).message}`);
  }
  if (!node || typeof node !== 'object') return refuse('does not decode: not an object');
  if (!verifyValidationNode(node)) {
    const id = JSON.stringify(String(node.validatorId));
    return refuse(`signature does not verify against agentId ${id}`);
  }
  if (node.checkName !== check) {
    const stored = JSON.stringify(node.checkName);
    return refuse(`checkName ${stored} does not match this ref's ${JSON.stringify(check)}`);
  }
  return { ok: true, node, verifiedBy: 'local' };
}

function gitText(repoRoot: string, args: string[]): string {
  // eslint-disable-next-line sonarjs/no-os-command-from-path -- git is a standard system tool
  return execFileSync('git', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    maxBuffer: 256 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'ignore'],
  });
}

/** The stored blob at `ref`/`oid`, or null when git lists nothing there. */
function storedBytes(repoRoot: string, object: string): string | null {
  try {
    return gitText(repoRoot, ['cat-file', 'blob', object]);
  } catch {
    return null;
  }
}

/** The validate ref for `check` through the verified read the host's brit affords. */
function readAttestationRef(
  bin: string,
  kind: BritProbe['kind'],
  repo: string,
  check: string,
  run: Runner,
  cwd: string
): AttestationRead {
  if (kind === VERIFIED_READS) return britValidateGet(bin, repo, check, run, cwd);
  const ref = attestationRef(check);
  return localVerifiedRead(storedBytes(repo, ref), ref, check, kind);
}

export interface ReadOptions {
  /** Which brit answers (from `resolveWorkspaceIdentity`); absent reads verify locally. */
  brit?: BritProbe;
  run?: Runner;
  /** Receives one line per record that is absent to this reader, naming the ref and the reason. */
  warn?: (line: string) => void;
}

export interface HouseholdAttestation {
  ref: string;
  node: ValidationNodeJson;
  /** `brit`: brit-build-ref's verified read printed it; `local`: this module verified the bytes. */
  verifiedBy: 'brit' | 'local';
}

/**
 * Every household validate ref for `concern` that verifies. With a verified-reads brit each ref
 * goes through `validate get` and only what brit prints is returned; otherwise the stored bytes
 * are verified here. A record that does not verify is absent and warned once:
 * `household attestation unverifiable: <ref> <reason> (<verifier>)`.
 */
export function readHouseholdAttestations(
  repoRoot: string,
  concern: string,
  opts: ReadOptions = {}
): HouseholdAttestation[] {
  const brit = opts.brit ?? { kind: 'absent', bin: null };
  const run = opts.run ?? defaultRunner;
  const warn = opts.warn ?? ((_line: string) => undefined);
  let listing: string;
  try {
    listing = gitText(repoRoot, [
      'for-each-ref',
      '--format=%(objectname) %(objecttype) %(refname)',
      `${ATTESTATION_REF_PREFIX}${encodeRefSegment(concern)}/`,
    ]);
  } catch {
    return [];
  }
  const refs = listing
    .split('\n')
    .map(l => l.split(' '))
    .filter(([oid, type, ref]) => oid && type === 'blob' && ref);
  const out: HouseholdAttestation[] = [];
  for (const [oid, , ref] of refs) {
    const check = decodeRefSegment(ref.slice(ATTESTATION_REF_PREFIX.length));
    const read =
      brit.kind === VERIFIED_READS && brit.bin
        ? britValidateGet(brit.bin, repoRoot, check, run, repoRoot)
        : localVerifiedRead(storedBytes(repoRoot, oid), ref, check, brit.kind);
    if (read.ok) out.push({ ref, node: read.node, verifiedBy: read.verifiedBy });
    else if (read.reason === null) {
      warn(`household attestation unverifiable: ${ref} listed but nothing is stored there`);
    } else warn(`household attestation unverifiable: ${read.reason}`);
  }
  return out;
}

export type Admission = { ok: true; summary: ReceiptSummary } | { ok: false; reason: string };

export type PeerVerdict = { ok: true } | { ok: false; reason: string };

const PEER_KEYS: (keyof PeerStage)[] = [
  'provider',
  'requester',
  'grantActionHash',
  'grantCid',
  'scope',
  'requestActionHash',
  'completionActionHash',
  'receiptCid',
  'taskCid',
  'featureSha256',
  'reportSha256',
];

/**
 * Whether the requester's own task record carries the chain a peer summary claims. Every
 * comparison is Holochain-to-Holochain or digest-to-digest: the provider's claims are signatures
 * the requester's storage verified (the completion is authored by `task.provider`; `observed`
 * re-reads the grant with its author pinned to that provider). Pure — the caller fetches.
 */
export function peerChainVerdict(peer: PeerStage, status: PeerTaskStatus | undefined): PeerVerdict {
  if (
    !peer ||
    typeof peer !== 'object' ||
    (peer.rung !== 'H' && peer.rung !== 'A') ||
    !PEER_KEYS.every(k => typeof peer[k] === 'string' && peer[k] !== '')
  ) {
    return { ok: false, reason: 'peer block is malformed' };
  }
  if (!status || typeof status !== 'object') {
    return { ok: false, reason: 'peer chain unverifiable (requester storage unreachable)' };
  }
  if (status.requestActionHash !== peer.requestActionHash) {
    return { ok: false, reason: 'peer request is not the task record the requester returned' };
  }
  if (status.taskCid !== peer.taskCid) {
    return { ok: false, reason: 'peer task CID is not the task record' };
  }
  if (status.state !== 'completed') {
    return { ok: false, reason: `peer task is ${status.state ?? 'in no state'}, not completed` };
  }
  if (status.completion?.actionHash !== peer.completionActionHash) {
    return { ok: false, reason: 'peer completion is not the one the task record carries' };
  }
  if (status.completion?.receiptCid !== peer.receiptCid) {
    return { ok: false, reason: 'peer receipt CID is not the completion receipt' };
  }
  if (status.provider !== peer.provider) {
    return { ok: false, reason: 'peer provider is not the task provider' };
  }
  if (status.requester !== peer.requester) {
    return { ok: false, reason: 'peer requester is not the task requester' };
  }
  if (status.grantActionHash !== peer.grantActionHash) {
    return { ok: false, reason: 'peer grant is not the task grant' };
  }
  const observed = status.observed;
  if (!observed) return { ok: false, reason: 'peer grant not observed by requester storage' };
  if (observed.verified !== true) {
    const why = observed.refused ? ': ' + observed.refused : '';
    return { ok: false, reason: 'peer grant not verified by requester storage' + why };
  }
  if (observed.scope !== PEER_STAGE_SCOPE || peer.scope !== PEER_STAGE_SCOPE) {
    return {
      ok: false,
      reason: `peer grant scope is ${observed.scope ?? 'absent'}, not ${PEER_STAGE_SCOPE}`,
    };
  }
  if (observed.grantCid !== peer.grantCid) {
    return { ok: false, reason: 'peer grant CID is not the observed grant' };
  }
  if (observed.grantProvider !== peer.provider) {
    return { ok: false, reason: 'peer provider is not the grant provider' };
  }
  if (observed.grantRecipient !== peer.requester) {
    return { ok: false, reason: 'peer requester is not the grant recipient' };
  }
  if (!status.envelope?.project?.startsWith(PEER_STAGE_PROJECT_PREFIX)) {
    return { ok: false, reason: `peer task is not an ${PEER_STAGE_PROJECT_PREFIX} stage` };
  }
  if (status.envelope.dna?.sha256 !== peer.featureSha256) {
    return { ok: false, reason: 'peer feature digest is not the task payload' };
  }
  return { ok: true };
}

/**
 * Structural admission of one attestation as a household receipt: signed by the workspace key,
 * a pass, keyed by a source identity that re-derives from the parts it carries, and whose
 * artifact CID is that identity. A summary carrying `peer` is admitted only when
 * `peerStatus(peer.requestActionHash)` — the requester's own task record — verifies the chain
 * (`peerChainVerdict`); the artifact stays the SUT either way. Whether the parts equal the
 * CURRENT tree, and whether every station passed, is the caller's receipt rule.
 */
export function admitAttestation(
  node: ValidationNodeJson,
  concern: string,
  workspaceId: string | null,
  peerStatus?: (requestActionHash: string) => PeerTaskStatus | undefined
): Admission {
  if (!workspaceId) return { ok: false, reason: 'no workspace key' };
  if (node.validatorId !== workspaceId) return { ok: false, reason: 'not the workspace key' };
  if (!verifyValidationNode(node)) return { ok: false, reason: 'signature does not verify' };
  if (node.result !== 'pass') return { ok: false, reason: `result ${node.result}` };
  let summary: ReceiptSummary;
  try {
    summary = JSON.parse(node.resultSummary);
  } catch {
    return { ok: false, reason: 'summary is not JSON' };
  }
  if (summary?.schema !== RECEIPT_SCHEMA || summary.concern !== concern) {
    return { ok: false, reason: 'summary names another schema or concern' };
  }
  if (summary.reach !== 'trusted') return { ok: false, reason: 'reach is not trusted' };
  const parts = summary.sutParts ?? {};
  if (sutOf(parts) !== summary.sut) return { ok: false, reason: 'sut does not re-derive' };
  if (cidShortFingerprint(node.artifactCid) !== summary.sut) {
    return { ok: false, reason: 'artifact CID is not the sut' };
  }
  if (node.checkName !== checkNameFor(concern, summary.sut)) {
    return { ok: false, reason: 'check name is not keyed by the sut' };
  }
  if (summary.peer !== undefined) {
    const peer = summary.peer;
    const hash = peer && typeof peer === 'object' ? peer.requestActionHash : undefined;
    const verdict = peerChainVerdict(
      peer,
      typeof hash === 'string' && hash ? peerStatus?.(hash) : undefined
    );
    if (!verdict.ok) return verdict;
  }
  return { ok: true, summary };
}
