/**
 * Package a runtime artifact as an EPR object on the content dataplane.
 *
 * A release is blob bytes plus a validated, content-addressed manifest, so every
 * substrate primitive (resiliency, replication, reach) comes along for free.
 * This tool does the LOCAL half: PUT the artifact bytes to a storage peer's blob
 * route, prove they come back by address, derive what the artifact applies to,
 * fill provenance, validate against
 * `elohim/rakia/schemas/v1/release-manifest.schema.json`, and emit the manifest.
 *
 * It authors NOTHING to the DHT. Declaring a release manifest canonical on its
 * channel is the ceremony driver's act, not this tool's.
 *
 * Packaging is not verification. A deliberately envelope-broken input still
 * packages — the manifest records what the artifact actually is, and the
 * adoption controller's verify step is the floor that refuses it.
 *
 * The one thing it will NOT invent is the channel's adoption discipline. Soak
 * budget and attestation threshold are either DECLARED (`--soak-secs` +
 * `--attestation-threshold`) or INHERITED from the channel
 * (`--inherit-discipline-from <storage base url>`); with neither, packaging
 * refuses. The retired `soakSecs 900` / `attestationThreshold 2` defaults
 * wedged a three-device household twice in one night — one attester
 * archetype, a threshold of two — and a number nobody typed is not a
 * discipline. An inherited release records the channel's own rule verbatim in
 * `adoptionDiscipline.channelDiscipline`, separately from its own effective
 * numbers, so a revert's act-specific `attestationThreshold: 0` can never
 * become the channel's rule for the releases that follow it. See
 * `AdoptionDiscipline`, `readChannelDiscipline` and `resolveAdoptionDiscipline`.
 *
 * Spec: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md §5, §8.
 *
 * Run from genesis/a2o:
 *   pnpm exec tsx scripts/epr-release-package.ts --artifact <path> --artifact-class <class> …
 *   pnpm exec tsx scripts/epr-release-package.ts --validate <manifest.json> …
 */

import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import * as AjvNs from 'ajv/dist/2020.js';

interface AjvOptions {
  strict: boolean;
  allErrors: boolean;
}
const AjvCtor: new (opts: AjvOptions) => AjvNs.default =
  (AjvNs as unknown as { default: new (opts: AjvOptions) => AjvNs.default }).default ??
  (AjvNs as unknown as new (opts: AjvOptions) => AjvNs.default);

const REPO_ROOT = fileURLToPath(new URL('../../..', import.meta.url));
const SCHEMA_PATH = path.join(REPO_ROOT, 'elohim/rakia/schemas/v1/release-manifest.schema.json');

const DEFAULT_PEER = 'http://localhost:8090';
const DEFAULT_NETWORK = 'elohim';
const DEFAULT_CHANNEL = 'commons';
const DEFAULT_REACH = 'commons';
const DEFAULT_AGENT_ID = 'did:elohim:release-packager';
const DEFAULT_REQUEST_TIMEOUT_MS = 30_000;

// Absolute paths so the probes cannot be shadowed by a writable PATH entry —
// the convention late-joiner-receipt.ts already uses for its BASH/FLOCK binaries.
// Both probes are optional: `--git-commit` and `--toolchain` override them, and
// the toolchain probe reports "unknown" rather than failing the run.
const GIT_BIN = process.env['GIT_BIN'] ?? '/usr/bin/git';
const RUSTC_BIN = process.env['RUSTC_BIN'] ?? '/opt/rust/cargo/bin/rustc';

const ARTIFACT_CLASSES = [
  'coordinator-bundle',
  'config-epr',
  'storage-binary',
  'happ-bundle',
  'happ-lineage',
] as const;
type ArtifactClass = (typeof ARTIFACT_CLASSES)[number];

/**
 * Channel-id segment each artifact class conventionally publishes under
 * (spec §3). Advisory: the packager warns on a mismatch instead of refusing,
 * because only three of the four segments are named upstream and a channel id
 * given explicitly is the author's declaration, not a typo to be corrected.
 */
const CLASS_CHANNEL_SEGMENT: Record<ArtifactClass, string> = {
  'coordinator-bundle': 'coordinators',
  'config-epr': 'config',
  'storage-binary': 'storage-binary',
  'happ-bundle': 'happ',
  'happ-lineage': 'happ',
};

const USAGE = `Usage: epr-release-package.ts --artifact <path> --artifact-class <class> [options]
       epr-release-package.ts --validate <manifest.json> [<manifest.json>…]

Artifact:
  --artifact <path>             file to package; repeat for a multi-blob release
  --artifact-class <class>      ${ARTIFACT_CLASSES.join(' | ')}

Channel and reach:
  --channel-id <id>             full runtime:<class>:<network>:<name> id
  --network <name>              network segment (default: ${DEFAULT_NETWORK})
  --channel <name>              channel-name segment (default: ${DEFAULT_CHANNEL})
  --declared-reach <reach>      audience for this release (default: ${DEFAULT_REACH})

Compatibility envelope (spec §8):
  --wire-epoch <n>              protocol wire epoch this release speaks; repeatable (default: 0)
  --lineage-parent <cid>        previous release CID on this channel (default: null)
  --additive-only <true|false>  additive-wire floor assertion (default: true)

appliesTo (what installed reality this release binds to):
  --applies-to-from <url>       derive roles from a peer's GET /version passport
  --applies-to <json|@file>     literal { "roles": { … } } or { role: … } map
  --applies-to-role <name>      restrict the derived roles; repeatable

Provenance:
  --builder-agent <id>          who built the artifact (default: $USER@$HOSTNAME)
  --toolchain <string>          toolchain identity (default: probed rustc, else "unknown")
  --build-info <json|@file>     the artifact's OWN build info
  --build-info-from <url>       read it from a runtime's GET /version envelope
  --git-commit <sha>            override the probed HEAD commit

Adoption discipline (spec §5) — declared or inherited, never defaulted:
  --soak-secs <n>               green-run budget before a peer may attest
  --attestation-threshold <n>   independent attestations to earn
  --canary <name>               ordered rollout wave; repeatable
  --inherit-discipline-from <url>
                                copy the discipline the household registered for this channel
                                from that storage peer: the channel root's own registered
                                discipline, else the channelDiscipline the head release manifest
                                carries forward, else that head's own adoptionDiscipline (refused
                                when the head is revert-shaped — threshold 0 is the revert ACT's
                                rule, not the channel's). --soak-secs/--attestation-threshold/
                                --canary still override the EFFECTIVE numbers; the channel's rule
                                is recorded verbatim as adoptionDiscipline.channelDiscipline

happ-lineage (spec 2026-09-03-holochain-evolution-epic-design §4):
  --migrate-from <role>=<dnaHash>
                                the DNA hash this role's release migrates FROM; repeatable
  --lineage <dnaHash,...>       comma-separated ancestry the v2 DNA properties declare; applied
                                to every role named by --migrate-from
  --path-commitment <cid>       the migrates-lineage commitment (entry hash, uhCEk…) that
                                notarizes this path; required when --artifact-class is happ-lineage
  --constitution-root <root>    the constitution the crossing is notarized under; applied to every
                                role named by --migrate-from. Checked against the adopting peer's
                                INSTALLED v1 root when that cell declares one, and against this
                                declaration when it does not. Omitted on both sides, the root check
                                is skipped and the receipt says so (root: undeclared).

Blob plane:
  --peer <url>                  storage peer for the blob PUT (default: ${DEFAULT_PEER})
  --agent-id <id>               x-agent-id on the PUT (default: ${DEFAULT_AGENT_ID})
  --no-put                      package offline: address the bytes, skip PUT and round-trip
  --request-timeout <ms>        per-request timeout (default: ${DEFAULT_REQUEST_TIMEOUT_MS})

Output:
  --out <path>                  write the manifest here (default: stdout)
  --compact                     emit single-line JSON
  --strict                      fail on properties the schema does not name
  --notes <text>                human-readable release note
  -h, --help                    show this help

Modes:
  --validate <file>…            validate existing manifests against the schema and exit

Exit codes: 0 ok · 2 manifest invalid or blob round-trip failed · 64 usage · 1 unexpected.`;

class UsageError extends Error {}

/** The packaging contract broke: invalid manifest, or bytes that did not come back. */
class PackagingFailure extends Error {
  constructor(
    message: string,
    readonly detail: string[] = []
  ) {
    super(message);
  }
}

// ---------------------------------------------------------------------------
// Content addressing — CIDv1 raw / sha2-256 / base32-lower
// ---------------------------------------------------------------------------

const BASE32_ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

/** RFC 4648 base32, lower-case, unpadded — multibase 'b'. */
export function base32Encode(bytes: Uint8Array): string {
  let out = '';
  let buffer = 0;
  let bits = 0;
  for (const byte of bytes) {
    buffer = (buffer << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      bits -= 5;
      out += BASE32_ALPHABET[(buffer >> bits) & 0x1f];
    }
  }
  if (bits > 0) out += BASE32_ALPHABET[(buffer << (5 - bits)) & 0x1f];
  return out;
}

/**
 * The canonical blob address: `Cid::new_v1(0x55, Sha2_256(bytes))` rendered as
 * multibase base32-lower — the same `bafkrei…` form `elohim-storage/src/epr_codec.rs`
 * mints. A bare `sha256-<hex>` is the legacy blob-path form and never an address.
 */
export function blobCid(bytes: Buffer): string {
  const digest = createHash('sha256').update(bytes).digest();
  const multihash = Buffer.concat([Buffer.from([0x12, 0x20]), digest]);
  const cidBytes = Buffer.concat([Buffer.from([0x01, 0x55]), multihash]);
  return `b${base32Encode(cidBytes)}`;
}

export function sha256Hex(bytes: Buffer): string {
  return createHash('sha256').update(bytes).digest('hex');
}

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

interface Options {
  help: boolean;
  validate: string[];
  artifacts: string[];
  artifactClass: ArtifactClass | null;
  channelId: string | null;
  network: string;
  channel: string;
  declaredReach: string;
  wireEpochs: number[];
  lineageParent: string | null;
  additiveOnly: boolean;
  appliesToFrom: string | null;
  appliesToLiteral: string | null;
  appliesToRoles: string[];
  builderAgent: string | null;
  toolchain: string | null;
  buildInfo: string | null;
  buildInfoFrom: string | null;
  gitCommit: string | null;
  /** null = not declared on the command line; see `resolveAdoptionDiscipline`. */
  soakSecs: number | null;
  /** null = not declared on the command line; see `resolveAdoptionDiscipline`. */
  attestationThreshold: number | null;
  canaryOrder: string[];
  inheritDisciplineFrom: string | null;
  /** role name -> the DNA hash that role's happ-lineage release migrates FROM. */
  migrateFrom: Record<string, string>;
  /** The ancestry (dnaHash list) applied to every role named in `migrateFrom`. */
  lineage: string[];
  /** The notarized migrates-lineage commitment (entry hash) for a happ-lineage release. */
  pathCommitment: string | null;
  /** The constitution the crossing is notarized under, applied to every role named in `migrateFrom`. */
  constitutionRoot: string | null;
  peer: string;
  agentId: string;
  put: boolean;
  requestTimeoutMs: number;
  out: string | null;
  compact: boolean;
  strict: boolean;
  notes: string | null;
}

/**
 * The flags whose value is a peer base URL, and the `Options` field each
 * fills. One switch arm parses all three (`parseHttpUrl` is the same
 * validation for each), which also keeps the argv switch under the lint's
 * case ceiling.
 */
const URL_VALUED_FLAGS = {
  '--applies-to-from': 'appliesToFrom',
  '--build-info-from': 'buildInfoFrom',
  '--inherit-discipline-from': 'inheritDisciplineFrom',
} as const satisfies Record<string, keyof Options>;

/**
 * The happ-lineage flags, folded into one shared switch arm the same way
 * `URL_VALUED_FLAGS` folds its three — flag -> the `Options` field it fills,
 * so the switch stays under the lint's non-empty-case ceiling. Unlike the
 * URL flags these three don't share one parse function (role=value map
 * entry, comma-split repeatable list, plain string), so the shared arm
 * branches on the resolved field name rather than applying one function —
 * the repeatable/comma-split semantics stay inside that one arm, not spread
 * across three.
 */
const LINEAGE_FLAGS = {
  '--migrate-from': 'migrateFrom',
  '--lineage': 'lineage',
  '--path-commitment': 'pathCommitment',
  '--constitution-root': 'constitutionRoot',
} as const satisfies Record<string, keyof Options>;

function requiredValue(args: string[], index: number, flag: string): string {
  const value = args[index + 1];
  if (!value || value.startsWith('--')) throw new UsageError(`${flag} requires a value`);
  return value;
}

function nonNegativeInteger(value: string, flag: string): number {
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < 0) {
    throw new UsageError(`${flag} expects a non-negative integer, got: ${value}`);
  }
  return parsed;
}

function parseBoolean(value: string, flag: string): boolean {
  if (value === 'true') return true;
  if (value === 'false') return false;
  throw new UsageError(`${flag} expects true or false, got: ${value}`);
}

function parseHttpUrl(rawUrl: string, flag: string): string {
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    throw new UsageError(`${flag} expects an HTTP(S) URL, got: ${rawUrl}`);
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    throw new UsageError(`${flag} expects an HTTP(S) URL, got: ${rawUrl}`);
  }
  return url.toString().replace(/\/$/, '');
}

/** Splits `role=value` for `--migrate-from`; either side empty is a usage error. */
function parseRoleEquals(raw: string, flag: string): [string, string] {
  const eq = raw.indexOf('=');
  const role = eq < 0 ? '' : raw.slice(0, eq);
  const value = eq < 0 ? '' : raw.slice(eq + 1);
  if (!role || !value) {
    throw new UsageError(`${flag} expects role=value, got: ${raw}`);
  }
  return [role, value];
}

function parseArtifactClass(value: string): ArtifactClass {
  const found = ARTIFACT_CLASSES.find(candidate => candidate === value);
  if (!found) {
    throw new UsageError(
      `--artifact-class expects one of ${ARTIFACT_CLASSES.join(' | ')}, got: ${value}`
    );
  }
  return found;
}

function parseArgs(argv: string[]): Options {
  const options: Options = {
    help: false,
    validate: [],
    artifacts: [],
    artifactClass: null,
    channelId: null,
    network: DEFAULT_NETWORK,
    channel: DEFAULT_CHANNEL,
    declaredReach: DEFAULT_REACH,
    wireEpochs: [],
    lineageParent: null,
    additiveOnly: true,
    appliesToFrom: null,
    appliesToLiteral: null,
    appliesToRoles: [],
    builderAgent: null,
    toolchain: null,
    buildInfo: null,
    buildInfoFrom: null,
    gitCommit: null,
    soakSecs: null,
    attestationThreshold: null,
    canaryOrder: [],
    inheritDisciplineFrom: null,
    migrateFrom: {},
    lineage: [],
    pathCommitment: null,
    constitutionRoot: null,
    peer: parseHttpUrl(process.env['RELEASE_PEER_URL'] ?? DEFAULT_PEER, '--peer'),
    agentId: DEFAULT_AGENT_ID,
    put: true,
    requestTimeoutMs: DEFAULT_REQUEST_TIMEOUT_MS,
    out: null,
    compact: false,
    strict: false,
    notes: null,
  };

  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    switch (arg) {
      case '-h':
      case '--help':
        options.help = true;
        break;
      case '--validate':
        // Consume every following non-flag token: `--validate a.json b.json`.
        while (index + 1 < argv.length && !argv[index + 1].startsWith('--')) {
          options.validate.push(path.resolve(argv[++index]));
        }
        if (options.validate.length === 0) throw new UsageError('--validate requires a file');
        break;
      case '--artifact':
        options.artifacts.push(path.resolve(requiredValue(argv, index, arg)));
        index++;
        break;
      case '--artifact-class':
        options.artifactClass = parseArtifactClass(requiredValue(argv, index, arg));
        index++;
        break;
      case '--channel-id':
        options.channelId = requiredValue(argv, index, arg);
        index++;
        break;
      case '--network':
        options.network = requiredValue(argv, index, arg);
        index++;
        break;
      case '--channel':
        options.channel = requiredValue(argv, index, arg);
        index++;
        break;
      case '--declared-reach':
        options.declaredReach = requiredValue(argv, index, arg);
        index++;
        break;
      case '--wire-epoch':
        options.wireEpochs.push(nonNegativeInteger(requiredValue(argv, index, arg), arg));
        index++;
        break;
      case '--lineage-parent':
        options.lineageParent = requiredValue(argv, index, arg);
        index++;
        break;
      case '--additive-only':
        options.additiveOnly = parseBoolean(requiredValue(argv, index, arg), arg);
        index++;
        break;
      // Every flag whose value is a peer base URL shares one arm (see
      // `URL_VALUED_FLAGS`) — three separate cases would push this switch past
      // the 30-case lint ceiling for no reader benefit.
      case '--applies-to-from':
      case '--build-info-from':
      case '--inherit-discipline-from':
        options[URL_VALUED_FLAGS[arg]] = parseHttpUrl(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--applies-to':
        options.appliesToLiteral = requiredValue(argv, index, arg);
        index++;
        break;
      case '--applies-to-role':
        options.appliesToRoles.push(requiredValue(argv, index, arg));
        index++;
        break;
      case '--builder-agent':
        options.builderAgent = requiredValue(argv, index, arg);
        index++;
        break;
      case '--toolchain':
        options.toolchain = requiredValue(argv, index, arg);
        index++;
        break;
      case '--build-info':
        options.buildInfo = requiredValue(argv, index, arg);
        index++;
        break;
      case '--git-commit':
        options.gitCommit = requiredValue(argv, index, arg);
        index++;
        break;
      case '--soak-secs':
        options.soakSecs = nonNegativeInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--attestation-threshold':
        options.attestationThreshold = nonNegativeInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--canary':
        options.canaryOrder.push(requiredValue(argv, index, arg));
        index++;
        break;
      // Every happ-lineage flag shares one arm (see `LINEAGE_FLAGS`) — three
      // separate cases would push this switch past the 30-case lint ceiling.
      case '--migrate-from':
      case '--lineage':
      case '--path-commitment':
      case '--constitution-root': {
        const value = requiredValue(argv, index, arg);
        const field = LINEAGE_FLAGS[arg];
        if (field === 'migrateFrom') {
          const [role, dnaHash] = parseRoleEquals(value, arg);
          options.migrateFrom[role] = dnaHash;
        } else if (field === 'lineage') {
          options.lineage.push(
            ...value
              .split(',')
              .map(entry => entry.trim())
              .filter(entry => entry.length > 0)
          );
        } else if (field === 'constitutionRoot') {
          options.constitutionRoot = value;
        } else {
          options.pathCommitment = value;
        }
        index++;
        break;
      }
      case '--peer':
        options.peer = parseHttpUrl(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--agent-id':
        options.agentId = requiredValue(argv, index, arg);
        index++;
        break;
      case '--no-put':
        options.put = false;
        break;
      case '--request-timeout':
        options.requestTimeoutMs = nonNegativeInteger(requiredValue(argv, index, arg), arg);
        index++;
        break;
      case '--out':
        options.out = path.resolve(requiredValue(argv, index, arg));
        index++;
        break;
      case '--compact':
        options.compact = true;
        break;
      case '--strict':
        options.strict = true;
        break;
      case '--notes':
        options.notes = requiredValue(argv, index, arg);
        index++;
        break;
      default:
        throw new UsageError(`unknown option: ${arg}`);
    }
  }

  if (options.help || options.validate.length > 0) return options;
  if (options.artifacts.length === 0) throw new UsageError('--artifact is required');
  if (!options.artifactClass) throw new UsageError('--artifact-class is required');
  if (options.artifactClass === 'happ-lineage' && !options.pathCommitment) {
    throw new UsageError(
      '--path-commitment <cid> is required when --artifact-class is happ-lineage — a lineage ' +
        'crossing is adoptable only when a notarized migrates-lineage commitment names it'
    );
  }
  if (options.wireEpochs.length === 0) options.wireEpochs.push(0);
  return options;
}

// ---------------------------------------------------------------------------
// Manifest shape (mirrors elohim/rakia/schemas/v1/release-manifest.schema.json)
// ---------------------------------------------------------------------------

interface ArtifactEntry {
  blobCid: string;
  bytes: number;
  sha256: string;
  filename: string;
  mimeType?: string;
}

interface RoleBinding {
  dnaHash: string;
  coordinatorWasmHashes: string[];
  coordinatorZomes?: Record<string, string>;
  /** happ-lineage only: the DNA hash this role's release migrates FROM. */
  migrateFrom?: string;
  /** happ-lineage only: the ancestry the v2 DNA properties declare. */
  lineage?: string[];
  /**
   * happ-lineage only: the constitution this crossing is notarized under — the
   * same root the path's migrates-lineage commitment carries (epic §4.1).
   * `verify_path` checks the path against the peer's INSTALLED v1 root when
   * that cell declares one, and against THIS declaration when it does not.
   */
  constitutionRoot?: string;
}

interface ReleaseManifest {
  kind: 'release-manifest';
  manifestVersion: '1.0';
  channelId: string;
  artifactClass: ArtifactClass;
  artifacts: ArtifactEntry[];
  appliesTo: { roles: Record<string, RoleBinding> };
  envelope: { wireEpochs: number[]; lineageParentCid: string | null; additiveOnly: boolean };
  provenance: {
    builderAgent: string;
    toolchain: string;
    buildInfo: Record<string, unknown>;
    builtFrom: { gitCommit: string; gitBranch?: string; dirty?: boolean };
  };
  declaredReach: string;
  adoptionDiscipline: AdoptionDiscipline;
  notes?: string;
}

/**
 * The channel's rule for how attestation evidence is counted (spec §5). It is
 * DECLARED (`--soak-secs` + `--attestation-threshold`) or INHERITED from the
 * channel (`--inherit-discipline-from`) — never defaulted, because a number
 * nobody typed is not a discipline: the retired `soakSecs 900 /
 * attestationThreshold 2` defaults silently wedged a three-device household
 * whose only attester archetype could never reach two.
 */
interface AdoptionDiscipline {
  soakSecs: number;
  attestationThreshold: number;
  canaryOrder: string[];
  /**
   * The record this discipline was copied from — the channel head's
   * `dhtAnchorHash` on the peer named by `--inherit-discipline-from`.
   * Additive and open-schema-legal (§8.2); see `PRODUCER_ADDITIVE_KEYS`.
   */
  inheritedFrom?: string;
  /**
   * THE CHANNEL'S OWN RULE, carried forward verbatim by every release that
   * inherits it — distinct from the three fields above, which are THIS
   * release's effective rule.
   *
   * The two diverge on a revert: a revert declares `attestationThreshold: 0`
   * because nobody attests a release the fleet is being asked to LEAVE, and
   * that zero is a property of the revert ACT, not of the channel. Without
   * this field the zero would become the channel's rule for every later
   * release, because the only record any reader can reach is the current head
   * (see `readChannelDiscipline` for why no root or by-cid read exists).
   */
  channelDiscipline?: {
    soakSecs: number;
    attestationThreshold: number;
    canaryOrder: string[];
  };
  /**
   * happ-lineage only: the notarized migrates-lineage commitment (entry hash)
   * that this release's crossing walks (spec 2026-09-03-holochain-evolution-
   * epic-design §4). Required by the schema's root `if/then` when
   * `artifactClass` is happ-lineage.
   */
  path?: { commitmentCid: string };
}

// ---------------------------------------------------------------------------
// Schema validation + open-schema strict lint
// ---------------------------------------------------------------------------

type JsonObject = Record<string, unknown>;

function loadSchema(): JsonObject {
  return JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')) as JsonObject;
}

function validateAgainstSchema(schema: JsonObject, manifest: unknown): string[] {
  const ajv = new AjvCtor({ strict: false, allErrors: true });
  const validate = ajv.compile(schema);
  if (validate(manifest)) return [];
  return (validate.errors ?? []).map(
    error => `${error.instancePath || '/'} ${error.message ?? 'invalid'}`
  );
}

function resolveRef(schema: JsonObject, node: JsonObject): JsonObject {
  const ref = node['$ref'];
  if (typeof ref !== 'string' || !ref.startsWith('#/$defs/')) return node;
  const defs = schema['$defs'] as Record<string, JsonObject> | undefined;
  return defs?.[ref.slice('#/$defs/'.length)] ?? node;
}

/**
 * Keys this packager emits DELIBERATELY that the schema does not (yet) name.
 * The schema is open by design (§8.2) so they validate; `--strict` is a typo
 * lint for the author, and a key the author meant to write is not a typo.
 * Declare it here and add it to `elohim/rakia/schemas/v1/release-manifest.schema.json`
 * the next time that schema is touched — this set is the producer's promise,
 * not a second schema.
 */
const PRODUCER_ADDITIVE_KEYS = new Set([
  '/adoptionDiscipline/inheritedFrom',
  '/adoptionDiscipline/channelDiscipline',
]);

/**
 * The schema is deliberately OPEN so mixed-version peers tolerate fields they
 * do not know (spec §8.2). That tolerance also swallows a producer's typo, so
 * `--strict` walks the schema and reports every property the schema does not
 * name — a lint for the author, never a rule for the reader.
 */
function lintUnknownKeys(
  schema: JsonObject,
  node: JsonObject,
  value: unknown,
  pointer: string,
  found: string[]
): void {
  const resolved = resolveRef(schema, node);
  if (Array.isArray(value)) {
    const items = resolved['items'];
    if (items && typeof items === 'object') {
      value.forEach((entry, i) =>
        lintUnknownKeys(schema, items as JsonObject, entry, `${pointer}/${i}`, found)
      );
    }
    return;
  }
  if (typeof value !== 'object' || value === null) return;

  const properties = resolved['properties'] as Record<string, JsonObject> | undefined;
  const additional = resolved['additionalProperties'];
  for (const [key, child] of Object.entries(value)) {
    const declared = properties?.[key];
    if (declared) {
      lintUnknownKeys(schema, declared, child, `${pointer}/${key}`, found);
    } else if (additional && typeof additional === 'object') {
      lintUnknownKeys(schema, additional as JsonObject, child, `${pointer}/${key}`, found);
    } else if (properties && !PRODUCER_ADDITIVE_KEYS.has(`${pointer}/${key}`)) {
      found.push(`${pointer}/${key}`);
    }
  }
}

// ---------------------------------------------------------------------------
// Inputs the packager reads rather than invents
// ---------------------------------------------------------------------------

/**
 * `fetch` rejects with an opaque TypeError when a peer is down. A packager that
 * prints a stack trace there reads as a bug in the tool rather than an absent
 * peer, so every network reach reports the URL it could not hold.
 */
async function reach(url: string, init: RequestInit): Promise<Response> {
  try {
    return await fetch(url, init);
  } catch (error) {
    throw new PackagingFailure(`could not reach ${url}`, [
      error instanceof Error ? error.message : String(error),
    ]);
  }
}

async function getJson(url: string, timeoutMs: number): Promise<JsonObject> {
  const response = await reach(url, { signal: AbortSignal.timeout(timeoutMs) });
  if (!response.ok) throw new PackagingFailure(`GET ${url} returned ${response.status}`);
  return (await response.json()) as JsonObject;
}

/** `@path` reads a file; anything else is parsed as literal JSON. */
function readJsonArgument(raw: string, flag: string): JsonObject {
  const text = raw.startsWith('@') ? readFileSync(path.resolve(raw.slice(1)), 'utf8') : raw;
  try {
    const parsed: unknown = JSON.parse(text);
    if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
      throw new Error('not a JSON object');
    }
    return parsed as JsonObject;
  } catch (error) {
    throw new UsageError(`${flag} could not be read as a JSON object: ${String(error)}`);
  }
}

interface PassportRole {
  role?: unknown;
  dnaHash?: unknown;
  coordinatorWasmHashes?: unknown;
}

/**
 * Derive `appliesTo.roles` from a live peer's runtime passport.
 *
 * The honest source for per-role DNA hashes is the conductor that installed the
 * bundle. Re-deriving them from a `.happ` in TypeScript would mean reproducing
 * Holochain's `DnaDef` serialization and blake2b hashing byte-exactly — a
 * second implementation of a consensus-critical hash, with no way to detect its
 * own drift. `GET /version` reports the same hashes `happ_manager::bundle_dna_hashes`
 * resolves, already authoritative, so the packager reads them instead of
 * recomputing them.
 */
function roleBindingFrom(entry: PassportRole): [string, RoleBinding] | null {
  const role = typeof entry.role === 'string' ? entry.role : null;
  const dnaHash = typeof entry.dnaHash === 'string' ? entry.dnaHash : null;
  if (!role || !dnaHash) return null;
  const zomes =
    typeof entry.coordinatorWasmHashes === 'object' && entry.coordinatorWasmHashes !== null
      ? (entry.coordinatorWasmHashes as Record<string, string>)
      : {};
  return [
    role,
    {
      dnaHash,
      coordinatorWasmHashes: [...new Set(Object.values(zomes))].sort((a, b) => a.localeCompare(b)),
      ...(Object.keys(zomes).length > 0 ? { coordinatorZomes: zomes } : {}),
    },
  ];
}

function rolesFromPassport(passport: JsonObject, only: string[]): Record<string, RoleBinding> {
  const runtime = (passport['passport'] ?? passport) as JsonObject;
  const happ = runtime['happ'] as JsonObject | undefined;
  const roles = happ?.['roles'];
  if (!Array.isArray(roles) || roles.length === 0) {
    const reason = typeof happ?.['error'] === 'string' ? ` (${String(happ['error'])})` : '';
    throw new PackagingFailure(`runtime passport reported no installed hApp roles${reason}`);
  }

  const out: Record<string, RoleBinding> = {};
  for (const entry of roles as PassportRole[]) {
    const binding = roleBindingFrom(entry);
    if (!binding) continue;
    if (only.length > 0 && !only.includes(binding[0])) continue;
    out[binding[0]] = binding[1];
  }

  const missing = only.filter(role => !(role in out));
  if (missing.length > 0) {
    throw new PackagingFailure(`passport has no role(s): ${missing.join(', ')}`);
  }
  if (Object.keys(out).length === 0) {
    throw new PackagingFailure('runtime passport yielded no usable roles');
  }
  return out;
}

function normaliseAppliesTo(raw: JsonObject): { roles: Record<string, RoleBinding> } {
  const roles = (raw['roles'] ?? raw) as Record<string, RoleBinding>;
  if (typeof roles !== 'object' || Object.keys(roles).length === 0) {
    throw new UsageError('--applies-to needs a non-empty { "roles": { … } } map');
  }
  return { roles };
}

/**
 * Applies `--migrate-from <role>=<dnaHash>` / `--lineage <dnaHash,...>` onto
 * the resolved role bindings for a happ-lineage release. `--lineage` is one
 * shared ancestry list, applied to every role named by `--migrate-from`
 * (spec 2026-09-03-holochain-evolution-epic-design §4); role-level
 * requiredness (a happ-lineage role must carry both) is left to Rust
 * (Task 4) — an omitted role here surfaces as a schema failure at emit time
 * (roleBinding is a shared $defs entry every artifactClass reuses).
 */
function applyLineageBindings(
  appliesTo: { roles: Record<string, RoleBinding> },
  options: Options
): { roles: Record<string, RoleBinding> } {
  if (Object.keys(options.migrateFrom).length === 0) return appliesTo;
  const roles: Record<string, RoleBinding> = { ...appliesTo.roles };
  for (const [role, migrateFrom] of Object.entries(options.migrateFrom)) {
    const existing = roles[role];
    if (!existing) {
      throw new UsageError(
        `--migrate-from names role "${role}", which --applies-to/--applies-to-from did not resolve`
      );
    }
    roles[role] = {
      ...existing,
      migrateFrom,
      ...(options.lineage.length > 0 ? { lineage: options.lineage } : {}),
      ...(options.constitutionRoot ? { constitutionRoot: options.constitutionRoot } : {}),
    };
  }
  return { roles };
}

function git(args: string[]): string | null {
  try {
    return execFileSync(GIT_BIN, args, { cwd: REPO_ROOT, encoding: 'utf8' }).trim();
  } catch {
    return null;
  }
}

function probeToolchain(): string {
  try {
    return execFileSync(RUSTC_BIN, ['--version'], { encoding: 'utf8' }).trim();
  } catch {
    return 'unknown';
  }
}

// ---------------------------------------------------------------------------
// Blob plane
// ---------------------------------------------------------------------------

interface BlobResult {
  entry: ArtifactEntry;
  putStatus: number | null;
  storedAs: string | null;
  roundTripBytes: number | null;
}

function mimeFor(file: string, artifactClass: ArtifactClass): string {
  if (artifactClass === 'config-epr' || file.endsWith('.json')) return 'application/json';
  return 'application/octet-stream';
}

async function putAndVerify(
  file: string,
  bytes: Buffer,
  entry: ArtifactEntry,
  options: Options
): Promise<BlobResult> {
  const putPath = `${options.peer}/blob/sha256-${entry.sha256}`;
  const putResponse = await reach(putPath, {
    method: 'PUT',
    headers: {
      'content-type': entry.mimeType ?? 'application/octet-stream',
      'x-agent-id': options.agentId,
    },
    body: new Uint8Array(bytes),
    signal: AbortSignal.timeout(options.requestTimeoutMs),
  });
  if (!putResponse.ok) {
    throw new PackagingFailure(
      `blob PUT for ${path.basename(file)} returned ${putResponse.status}`,
      [(await putResponse.text()).slice(0, 400)]
    );
  }
  const putBody = (await putResponse.json()) as { blobHash?: unknown };
  const storedAs =
    typeof putBody.blobHash === 'string' ? putBody.blobHash : `sha256-${entry.sha256}`;

  // The interface contract: the bytes must be fetchable by address from the peer
  // they were PUT to BEFORE this tool exits 0. Nothing downstream can recover a
  // manifest that points at bytes no peer will serve.
  const getResponse = await reach(`${options.peer}/blob/${storedAs}`, {
    signal: AbortSignal.timeout(options.requestTimeoutMs),
  });
  if (!getResponse.ok) {
    throw new PackagingFailure(
      `blob round-trip for ${path.basename(file)} failed: GET returned ${getResponse.status}`
    );
  }
  const fetched = Buffer.from(await getResponse.arrayBuffer());
  const fetchedSha = sha256Hex(fetched);
  if (fetchedSha !== entry.sha256) {
    throw new PackagingFailure(
      `blob round-trip for ${path.basename(file)} returned different bytes`,
      [`expected sha256 ${entry.sha256}`, `observed sha256 ${fetchedSha}`]
    );
  }
  return { entry, putStatus: putResponse.status, storedAs, roundTripBytes: fetched.length };
}

async function packageArtifact(
  file: string,
  options: Options,
  artifactClass: ArtifactClass
): Promise<BlobResult> {
  const stats = statSync(file);
  if (!stats.isFile()) throw new UsageError(`--artifact is not a file: ${file}`);
  const bytes = readFileSync(file);
  const entry: ArtifactEntry = {
    blobCid: blobCid(bytes),
    bytes: bytes.length,
    sha256: sha256Hex(bytes),
    filename: path.basename(file),
    mimeType: mimeFor(file, artifactClass),
  };
  if (!options.put) {
    return { entry, putStatus: null, storedAs: null, roundTripBytes: null };
  }
  return putAndVerify(file, bytes, entry, options);
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

function resolveChannelId(options: Options, artifactClass: ArtifactClass): string {
  if (options.channelId) {
    const segment = options.channelId.split(':')[1];
    const expected = CLASS_CHANNEL_SEGMENT[artifactClass];
    if (segment !== expected) {
      console.error(
        `warn: channel id segment "${segment ?? '(none)'}" is not the conventional "${expected}" for artifact class ${artifactClass}`
      );
    }
    return options.channelId;
  }
  return `runtime:${CLASS_CHANNEL_SEGMENT[artifactClass]}:${options.network}:${options.channel}`;
}

async function resolveAppliesTo(options: Options): Promise<{ roles: Record<string, RoleBinding> }> {
  if (options.appliesToLiteral) {
    return normaliseAppliesTo(readJsonArgument(options.appliesToLiteral, '--applies-to'));
  }
  if (options.appliesToFrom) {
    const passport = await getJson(`${options.appliesToFrom}/version`, options.requestTimeoutMs);
    return { roles: rolesFromPassport(passport, options.appliesToRoles) };
  }
  throw new UsageError('one of --applies-to-from <url> or --applies-to <json|@file> is required');
}

// ---------------------------------------------------------------------------
// Adoption discipline — declared or inherited, never defaulted
// ---------------------------------------------------------------------------

const CHANNEL_ROOT_KIND = 'release-channel';
const RELEASE_MANIFEST_KIND = 'release-manifest';

/**
 * A revert declares `attestationThreshold: 0` because nobody attests a release
 * the fleet is being asked to LEAVE (measured: packaging a revert at the
 * forward threshold left every peer refusing `threshold_unmet` forever). No
 * field marks a manifest as a revert, so that zero IS the revert signature —
 * and it is a property of the revert ACT, never of the channel.
 */
const REVERT_ACT_THRESHOLD = 0;

/** The fields of a storage content row this tool reads for inheritance. */
interface ChannelContentItem {
  metadata?: {
    kind?: unknown;
    discipline?: unknown;
    manifest?: { adoptionDiscipline?: unknown };
  };
  dhtAnchorHash?: unknown;
}

interface InheritedDiscipline {
  soakSecs: number;
  attestationThreshold: number;
  canaryOrder: string[];
  /** What the discipline was read off — named in the stderr receipt. */
  source: string;
  /** The head record's action hash, when the row reports one. */
  anchor: string | null;
}

function disciplineShape(
  raw: unknown,
  where: string
): { soakSecs: number; attestationThreshold: number; canaryOrder: string[] } {
  if (typeof raw !== 'object' || raw === null) {
    throw new PackagingFailure(`${where} carries no adoption discipline to inherit`);
  }
  const record = raw as Record<string, unknown>;
  const soakSecs = record['soakSecs'];
  const attestationThreshold = record['attestationThreshold'];
  if (!Number.isSafeInteger(soakSecs) || !Number.isSafeInteger(attestationThreshold)) {
    throw new PackagingFailure(
      `${where} carries an adoption discipline without integer soakSecs/attestationThreshold`,
      [JSON.stringify(record).slice(0, 300)]
    );
  }
  const canaryOrder = Array.isArray(record['canaryOrder'])
    ? (record['canaryOrder'] as unknown[]).filter((v): v is string => typeof v === 'string')
    : [];
  return {
    soakSecs: soakSecs as number,
    attestationThreshold: attestationThreshold as number,
    canaryOrder,
  };
}

/**
 * Reads the discipline the household already registered for this channel from
 * one storage peer's `GET /db/content/{channelId}` — the channel's own content
 * row, whose `metadata` is either
 *
 *   - the channel ROOT record (`kind: "release-channel"`), carrying the
 *     `discipline` the steward registered at `channel create`; or
 *   - the channel's current HEAD release manifest (`kind: "release-manifest"`),
 *     carrying `adoptionDiscipline.channelDiscipline` (the channel's own rule,
 *     carried forward) and that release's own effective rule.
 *
 * ## Only ONE record is reachable, so the channel's rule must ride it
 *
 * There is no read path to any record but the current head. Measured
 * 2026-09-04: `GET /db/content/{id}` serves only the projected current
 * version (`update_content` overwrites the row, so the root's registered
 * `discipline` is gone after the first publish); storage registers no
 * versions/history route and `GET /db/dht/{hash}` is a 404 stub (its conductor
 * bridge is deferred); and no zome extern lists an id's versions, so even a
 * conductor cannot retrieve the root — `scripts/release-lineage-probe.ts`
 * §"Version-listing extern" documents exactly that gap. A `lineageParentCid`
 * walk is therefore impossible too: there is nothing to fetch a parent WITH.
 *
 * Hence `channelDiscipline`: every release that inherits carries the channel's
 * own rule forward verbatim, so the one reachable record always holds it. The
 * order this function applies:
 *
 *   1. root record  -> its registered `discipline` (the origin of the rule);
 *   2. head manifest with `channelDiscipline` -> that, the channel's rule
 *      carried forward, EVEN when the head is a revert whose own threshold is
 *      the revert act's 0;
 *   3. head manifest without one -> its own `adoptionDiscipline`, unless it is
 *      revert-shaped (threshold 0), which is an ACT's rule and would silently
 *      become the channel's — refuse instead of inheriting it.
 */
async function readChannelDiscipline(
  baseUrl: string,
  channelId: string,
  timeoutMs: number
): Promise<InheritedDiscipline> {
  const url = `${baseUrl}/db/content/${encodeURIComponent(channelId)}`;
  const row = (await getJson(url, timeoutMs)) as ChannelContentItem;
  const anchor = typeof row.dhtAnchorHash === 'string' ? row.dhtAnchorHash : null;
  const kind = row.metadata?.kind;

  if (kind === CHANNEL_ROOT_KIND) {
    const shape = disciplineShape(row.metadata?.discipline, `the channel root at ${url}`);
    return {
      ...shape,
      source: `the channel's registered discipline (root record) at ${url}`,
      anchor,
    };
  }

  if (kind === RELEASE_MANIFEST_KIND) {
    const head = row.metadata?.manifest?.adoptionDiscipline;
    const carried =
      typeof head === 'object' && head !== null
        ? (head as Record<string, unknown>)['channelDiscipline']
        : undefined;
    if (carried !== undefined) {
      const shape = disciplineShape(carried, `the channelDiscipline on ${url}`);
      return {
        ...shape,
        source: `the channel discipline carried forward by the head release manifest at ${url}`,
        anchor,
      };
    }
    const shape = disciplineShape(head, `the head release manifest on ${url}`);
    if (shape.attestationThreshold === REVERT_ACT_THRESHOLD) {
      throw new PackagingFailure(
        `the head of ${channelId} declares attestationThreshold ${REVERT_ACT_THRESHOLD} and carries no ` +
          `channelDiscipline, which is the signature of a REVERT — a threshold nobody could ever meet ` +
          `is a property of the revert act, not of the channel, and no read path exists to walk past ` +
          `the head to recover the channel's own rule`,
        [
          `head record: ${anchor ?? '(no anchor reported)'} at ${url}`,
          "declare this release's discipline explicitly with --soak-secs + --attestation-threshold,",
          'or inherit from a peer whose head for this channel is a real release rather than a revert.',
        ]
      );
    }
    return {
      ...shape,
      source: `the head release manifest's own adoptionDiscipline at ${url}`,
      anchor,
    };
  }

  throw new PackagingFailure(
    `${url} is not a release channel: its metadata.kind is ${JSON.stringify(kind ?? null)}, ` +
      `not "${CHANNEL_ROOT_KIND}" or "${RELEASE_MANIFEST_KIND}"`
  );
}

/**
 * Where a release's adoption discipline comes from, in order: the explicit
 * flags, then `--inherit-discipline-from`, and otherwise a REFUSAL. There is
 * no default — see `AdoptionDiscipline`'s own doc for the wedge the retired
 * `900 / 2` defaults caused.
 */
async function resolveAdoptionDiscipline(
  options: Options,
  channelId: string
): Promise<AdoptionDiscipline> {
  const declaredBoth = options.soakSecs !== null && options.attestationThreshold !== null;

  if (!options.inheritDisciplineFrom) {
    if (!declaredBoth) {
      throw new UsageError(
        `a release must carry an adoption discipline for channel ${channelId}: declare it with ` +
          `--soak-secs <n> AND --attestation-threshold <n>, or inherit the household's own with ` +
          `--inherit-discipline-from <storage base url> — this tool no longer defaults it, because ` +
          `a number nobody typed is not a discipline` +
          (options.soakSecs === null ? ' (--soak-secs was not given)' : '') +
          (options.attestationThreshold === null ? ' (--attestation-threshold was not given)' : '')
      );
    }
    return {
      soakSecs: options.soakSecs as number,
      attestationThreshold: options.attestationThreshold as number,
      canaryOrder: options.canaryOrder,
    };
  }

  const inherited = await readChannelDiscipline(
    options.inheritDisciplineFrom,
    channelId,
    options.requestTimeoutMs
  );
  const fromChannel: string[] = [];
  if (options.soakSecs === null) fromChannel.push('soakSecs');
  if (options.attestationThreshold === null) fromChannel.push('attestationThreshold');
  if (options.canaryOrder.length === 0) fromChannel.push('canaryOrder');

  // The channel's own rule, carried forward VERBATIM — never the effective
  // numbers below, which may carry this release's own act-specific override
  // (a revert's threshold 0 being the whole reason the two must stay apart).
  const channelDiscipline = {
    soakSecs: inherited.soakSecs,
    attestationThreshold: inherited.attestationThreshold,
    canaryOrder: inherited.canaryOrder,
  };
  const discipline: AdoptionDiscipline = {
    soakSecs: options.soakSecs ?? inherited.soakSecs,
    attestationThreshold: options.attestationThreshold ?? inherited.attestationThreshold,
    canaryOrder: options.canaryOrder.length > 0 ? options.canaryOrder : inherited.canaryOrder,
    channelDiscipline,
    ...(inherited.anchor ? { inheritedFrom: inherited.anchor } : {}),
  };
  console.error(
    `inherited ${fromChannel.length > 0 ? fromChannel.join('+') : 'nothing (every field overridden)'} ` +
      `from ${inherited.source}` +
      (inherited.anchor ? ` (record ${inherited.anchor})` : ' (row reports no record anchor)') +
      `: effective soakSecs=${discipline.soakSecs} attestationThreshold=${discipline.attestationThreshold} ` +
      `canaryOrder=${JSON.stringify(discipline.canaryOrder)}; channelDiscipline carried forward=` +
      JSON.stringify(channelDiscipline)
  );
  return discipline;
}

async function resolveBuildInfo(options: Options): Promise<Record<string, unknown>> {
  if (options.buildInfo) return readJsonArgument(options.buildInfo, '--build-info');
  if (options.buildInfoFrom) {
    const version = await getJson(`${options.buildInfoFrom}/version`, options.requestTimeoutMs);
    // The artifact's own build envelope, minus the runtime passport that wraps it.
    const build = { ...version };
    delete build['passport'];
    return build;
  }
  return {};
}

async function assembleManifest(options: Options): Promise<{
  manifest: ReleaseManifest;
  blobs: BlobResult[];
}> {
  const artifactClass = options.artifactClass as ArtifactClass;
  const channelId = resolveChannelId(options, artifactClass);
  // Resolved BEFORE the blob plane is touched: a missing discipline is a
  // usage refusal, and refusing it after PUTting a nine-megabyte bundle
  // would make the tool look slow rather than strict.
  const adoptionDiscipline = await resolveAdoptionDiscipline(options, channelId);

  const blobs: BlobResult[] = [];
  for (const file of options.artifacts) {
    blobs.push(await packageArtifact(file, options, artifactClass));
  }

  const appliesTo = applyLineageBindings(await resolveAppliesTo(options), options);
  const buildInfo = await resolveBuildInfo(options);
  const gitCommit = options.gitCommit ?? git(['rev-parse', 'HEAD']);
  if (!gitCommit) {
    throw new UsageError('could not probe the source commit; pass --git-commit <sha>');
  }
  const branch = git(['rev-parse', '--abbrev-ref', 'HEAD']);
  const porcelain = git(['status', '--porcelain']);

  const manifest: ReleaseManifest = {
    kind: 'release-manifest',
    manifestVersion: '1.0',
    channelId,
    artifactClass,
    artifacts: blobs.map(blob => blob.entry),
    appliesTo,
    envelope: {
      wireEpochs: [...new Set(options.wireEpochs)].sort((a, b) => a - b),
      lineageParentCid: options.lineageParent,
      additiveOnly: options.additiveOnly,
    },
    provenance: {
      builderAgent:
        options.builderAgent ??
        `${process.env['USER'] ?? 'unknown'}@${process.env['HOSTNAME'] ?? 'unknown'}`,
      toolchain: options.toolchain ?? probeToolchain(),
      buildInfo,
      builtFrom: {
        gitCommit,
        ...(branch && branch !== 'HEAD' ? { gitBranch: branch } : {}),
        ...(porcelain === null ? {} : { dirty: porcelain.length > 0 }),
      },
    },
    declaredReach: options.declaredReach,
    adoptionDiscipline: {
      ...adoptionDiscipline,
      ...(options.pathCommitment ? { path: { commitmentCid: options.pathCommitment } } : {}),
    },
    ...(options.notes ? { notes: options.notes } : {}),
  };

  return { manifest, blobs };
}

// ---------------------------------------------------------------------------
// Modes
// ---------------------------------------------------------------------------

function runValidate(files: string[], strict: boolean): void {
  const schema = loadSchema();
  const failures: string[] = [];
  for (const file of files) {
    const manifest: unknown = JSON.parse(readFileSync(file, 'utf8'));
    const errors = validateAgainstSchema(schema, manifest);
    const unknown: string[] = [];
    if (strict) lintUnknownKeys(schema, schema, manifest, '', unknown);
    if (errors.length === 0 && unknown.length === 0) {
      console.log(`PASS ${path.relative(REPO_ROOT, file)}`);
      continue;
    }
    failures.push(file);
    console.error(`FAIL ${path.relative(REPO_ROOT, file)}`);
    for (const error of errors) console.error(`  schema: ${error}`);
    for (const key of unknown) console.error(`  unknown property: ${key}`);
  }
  if (failures.length > 0) {
    throw new PackagingFailure(`${failures.length}/${files.length} manifest(s) invalid`);
  }
  console.log(
    `${files.length}/${files.length} manifests validate against the release-manifest schema`
  );
}

async function runPackage(options: Options): Promise<void> {
  const { manifest, blobs } = await assembleManifest(options);
  const schema = loadSchema();
  const errors = validateAgainstSchema(schema, manifest);
  const unknown: string[] = [];
  if (options.strict) lintUnknownKeys(schema, schema, manifest, '', unknown);
  if (errors.length > 0 || unknown.length > 0) {
    throw new PackagingFailure('emitted manifest does not validate', [
      ...errors.map(error => `schema: ${error}`),
      ...unknown.map(key => `unknown property: ${key}`),
    ]);
  }

  const json = options.compact
    ? JSON.stringify(manifest)
    : `${JSON.stringify(manifest, null, 2)}\n`;
  if (options.out) {
    mkdirSync(path.dirname(options.out), { recursive: true });
    writeFileSync(options.out, json);
  } else {
    process.stdout.write(json);
  }

  for (const blob of blobs) {
    if (blob.roundTripBytes === null) {
      console.error(
        `addressed ${blob.entry.filename}: ${blob.entry.blobCid} (${blob.entry.bytes} bytes) — --no-put, no round-trip`
      );
      continue;
    }
    console.error(
      `round-trip ${blob.entry.filename}: PUT ${blob.putStatus} as ${blob.storedAs}; ` +
        `GET returned ${blob.roundTripBytes} bytes matching sha256 ${blob.entry.sha256.slice(0, 12)}…; cid ${blob.entry.blobCid}`
    );
  }
  console.error(
    `PASS ${manifest.artifactClass} release for ${manifest.channelId}: ` +
      `${manifest.artifacts.length} blob(s), ${Object.keys(manifest.appliesTo.roles).length} role(s), ` +
      `reach ${manifest.declaredReach}` +
      (options.out ? `; manifest at ${path.relative(REPO_ROOT, options.out)}` : '')
  );
}

try {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    console.log(USAGE);
    process.exitCode = 0;
  } else if (options.validate.length > 0) {
    runValidate(options.validate, options.strict);
    process.exitCode = 0;
  } else {
    await runPackage(options);
    process.exitCode = 0;
  }
} catch (error) {
  if (error instanceof UsageError) {
    console.error(`${error.message}\n\n${USAGE}`);
    process.exitCode = 64;
  } else if (error instanceof PackagingFailure) {
    console.error(`PACKAGING FAILED: ${error.message}`);
    for (const line of error.detail) console.error(`  ${line}`);
    process.exitCode = 2;
  } else {
    console.error(error instanceof Error ? error.stack : String(error));
    process.exitCode = 1;
  }
}
