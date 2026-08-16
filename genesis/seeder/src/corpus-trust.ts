/**
 * Corpus trust declaration — the genesis-side half of the declared-trust bootstrap
 * (minutes-quiesce sprint Q5; head-plane trust-gradient program T10).
 *
 * Spec: genesis/docs/superpowers/plans/2026-08-16-minutes-quiesce-fixture-trust-swarm-plan.md §3 W2.
 * Runtime vocabulary: elohim/elohim-storage/src/trust/stage.rs (`NetworkStage`,
 * `StakesProvenance`) — READ ONLY from here; this module never imports Rust.
 *
 * ## Why this exists
 *
 * Genesis fixture corpora are staging/preproduction artifacts. A peer that adopts
 * another peer's advertised fixture corpus wholesale (per-peer ceremony instead of
 * per-EPR) needs a *declared* reason to price that adoption cheaply. The reason is
 * declared ON THE ARTIFACT (`corpus.json` at the corpus root), verified at seed
 * time, and minted ONCE per deploy/environment as a stakes declaration.
 *
 * ## The three invariants
 *
 * 1. **Declared, never inferred.** The grant is read off the corpus descriptor.
 *    It is never derived from repo origin, hostname, `DEV_MODE`, or "we're on
 *    localhost". `NetworkStage::Simulacra` is reached only by positive declaration.
 * 2. **Fail-closed on absence.** No `corpus.json` ⇒ no declaration ⇒ nothing is
 *    minted ⇒ the runtime resolves `NetworkStage::Bootstrap` via
 *    `StakesProvenance::BootstrapDefault`. Absence is silent-but-safe.
 * 3. **Hard error on malformation.** A declaration that EXISTS must be valid.
 *    Unknown stage / missing grantor / preproduction-trust-in-production are hard
 *    errors naming the offending field — never a silent downgrade.
 *
 * ## Declaration style
 *
 * This extends the realism-ladder convention from
 * `genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md` §5
 * ("every seed module declares its rung and why") — same declaration style, now
 * machine-readable, with the trust grant living in the same block as the rung.
 */

import { createHash } from 'node:crypto';
import * as fs from 'node:fs';
import * as path from 'node:path';

/** Canonical filename of a corpus declaration, at the corpus directory root. */
export const CORPUS_DECLARATION_FILE = 'corpus.json';

/**
 * The declared operating stage, mirroring `NetworkStage` in
 * `elohim/elohim-storage/src/trust/stage.rs`.
 *
 * Wire form is the lowercase variant name. `"dev"`, `"dev-mode"`, `"staging"` and
 * friends are NOT vocabulary — a stage is one of these four or the declaration is
 * rejected.
 */
export const NETWORK_STAGES = ['simulacra', 'bootstrap', 'coordinated', 'enforced'] as const;
export type NetworkStage = (typeof NETWORK_STAGES)[number];

/** The environment tier a corpus is authored for. */
export const ENVIRONMENT_TIERS = ['preproduction', 'production'] as const;
export type EnvironmentTier = (typeof ENVIRONMENT_TIERS)[number];

/** The realism-ladder rungs (qahal-epr-household-lattice §5). */
export const REALISM_RUNGS = [0, 1, 2, 3] as const;
export type RealismRung = (typeof REALISM_RUNGS)[number];

/** `realism:` block — the existing convention, made machine-readable. */
export interface RealismDeclaration {
  rung: RealismRung;
  why: string;
}

/** `trustBootstrap:` block — the trust a deploy must mint to bootstrap this corpus. */
export interface TrustBootstrapDeclaration {
  /** Declared stage. `simulacra` is legal ONLY under `environment: preproduction`. */
  stage: NetworkStage;
  /** Steward who grants it — a real persona id (e.g. `human-adam-firstman`). */
  grantor: string;
  /** Scope of the grant. Must equal the declaring corpus's `id`. */
  scope: string;
}

/** A parsed, validated corpus declaration. */
export interface CorpusDeclaration {
  /** Corpus id — the grant scope. `genesis-lamad`, `genesis-humans`, ... */
  id: string;
  /** Human-readable corpus label. */
  title: string;
  environment: EnvironmentTier;
  realism: RealismDeclaration;
  /** Absent ⇒ nothing is minted (runtime fail-closed to `Bootstrap`). */
  trustBootstrap?: TrustBootstrapDeclaration;
}

/** A validated declaration plus where it came from (needed for provenance). */
export interface LoadedCorpusDeclaration {
  declaration: CorpusDeclaration;
  /** Absolute path of the `corpus.json` that was read. */
  sourcePath: string;
  /** sha256 of the declaration file's exact bytes — the provenance fingerprint. */
  sha256: string;
}

/**
 * Thrown when a declaration exists but is invalid. Always names the field.
 * Never thrown for a MISSING declaration — that path is fail-closed-by-absence.
 */
export class CorpusDeclarationError extends Error {
  constructor(
    readonly sourcePath: string,
    readonly field: string,
    detail: string,
  ) {
    super(`${sourcePath}: '${field}' ${detail}`);
    this.name = 'CorpusDeclarationError';
  }
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

function requireNonEmptyString(
  raw: Record<string, unknown>,
  field: string,
  sourcePath: string,
): string {
  const v = raw[field];
  if (typeof v !== 'string' || v.trim() === '') {
    throw new CorpusDeclarationError(sourcePath, field, 'is required and must be a non-empty string');
  }
  return v.trim();
}

/**
 * Parse + validate a corpus declaration from raw JSON text.
 *
 * @param text       exact file bytes as UTF-8
 * @param sourcePath path used in every error message (need not exist on disk)
 * @throws CorpusDeclarationError for any structural or vocabulary violation
 */
export function parseCorpusDeclaration(text: string, sourcePath: string): CorpusDeclaration {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (e) {
    throw new CorpusDeclarationError(
      sourcePath,
      'corpus.json',
      `is not valid JSON: ${e instanceof Error ? e.message : String(e)}`,
    );
  }
  if (!isPlainObject(raw)) {
    throw new CorpusDeclarationError(sourcePath, 'corpus.json', 'must be a JSON object');
  }

  const id = requireNonEmptyString(raw, 'id', sourcePath);
  const title = requireNonEmptyString(raw, 'title', sourcePath);

  const environment = raw['environment'];
  if (typeof environment !== 'string' || !(ENVIRONMENT_TIERS as readonly string[]).includes(environment)) {
    throw new CorpusDeclarationError(
      sourcePath,
      'environment',
      `must be one of: ${ENVIRONMENT_TIERS.join(', ')} (got ${JSON.stringify(environment)})`,
    );
  }

  const realismRaw = raw['realism'];
  if (!isPlainObject(realismRaw)) {
    throw new CorpusDeclarationError(
      sourcePath,
      'realism',
      'is required — every seed corpus declares its rung and why (qahal-epr-household-lattice §5)',
    );
  }
  const rung = realismRaw['rung'];
  if (typeof rung !== 'number' || !(REALISM_RUNGS as readonly number[]).includes(rung)) {
    throw new CorpusDeclarationError(
      sourcePath,
      'realism.rung',
      `must be one of: ${REALISM_RUNGS.join(', ')} (got ${JSON.stringify(rung)})`,
    );
  }
  const why = realismRaw['why'];
  if (typeof why !== 'string' || why.trim() === '') {
    throw new CorpusDeclarationError(sourcePath, 'realism.why', 'is required — declare the rung AND why');
  }

  const declaration: CorpusDeclaration = {
    id,
    title,
    environment: environment as EnvironmentTier,
    realism: { rung: rung as RealismRung, why: why.trim() },
  };

  const trustRaw = raw['trustBootstrap'];
  if (trustRaw === undefined || trustRaw === null) {
    // Fail-closed by absence: no grant declared, so no grant is minted.
    return declaration;
  }
  if (!isPlainObject(trustRaw)) {
    throw new CorpusDeclarationError(sourcePath, 'trustBootstrap', 'must be a JSON object when present');
  }

  const stage = trustRaw['stage'];
  if (typeof stage !== 'string' || !(NETWORK_STAGES as readonly string[]).includes(stage)) {
    throw new CorpusDeclarationError(
      sourcePath,
      'trustBootstrap.stage',
      `must be one of: ${NETWORK_STAGES.join(', ')} (got ${JSON.stringify(stage)}). ` +
        'These are the NetworkStage variants (elohim-storage src/trust/stage.rs); ' +
        '"dev", "dev-mode" and "staging" are not vocabulary.',
    );
  }

  const grantor = trustRaw['grantor'];
  if (typeof grantor !== 'string' || grantor.trim() === '') {
    throw new CorpusDeclarationError(
      sourcePath,
      'trustBootstrap.grantor',
      'is required — a grant is always granted BY a declared steward, never by the repo',
    );
  }

  const scopeRaw = trustRaw['scope'];
  const scope = scopeRaw === undefined ? id : scopeRaw;
  if (typeof scope !== 'string' || scope.trim() === '') {
    throw new CorpusDeclarationError(sourcePath, 'trustBootstrap.scope', 'must be a non-empty string when present');
  }
  if (scope.trim() !== id) {
    throw new CorpusDeclarationError(
      sourcePath,
      'trustBootstrap.scope',
      `must equal the corpus 'id' ('${id}') — a corpus may only grant trust over itself (got '${scope.trim()}')`,
    );
  }

  if (stage === 'simulacra' && declaration.environment !== 'preproduction') {
    throw new CorpusDeclarationError(
      sourcePath,
      'trustBootstrap.stage',
      `'simulacra' requires environment 'preproduction' (got '${declaration.environment}') — ` +
        'preproduction trust must never leak into an enforced-reach environment (plan §6).',
    );
  }

  declaration.trustBootstrap = { stage: stage as NetworkStage, grantor: grantor.trim(), scope: scope.trim() };
  return declaration;
}

/**
 * Load the declaration for a corpus directory.
 *
 * @returns `null` when no `corpus.json` exists — the fail-closed-by-absence path.
 * @throws CorpusDeclarationError when one exists but is invalid.
 */
export function loadCorpusDeclaration(corpusDir: string): LoadedCorpusDeclaration | null {
  const sourcePath = path.join(corpusDir, CORPUS_DECLARATION_FILE);
  if (!fs.existsSync(sourcePath)) return null;
  const bytes = fs.readFileSync(sourcePath);
  const declaration = parseCorpusDeclaration(bytes.toString('utf8'), sourcePath);
  return {
    declaration,
    sourcePath,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  };
}

// =============================================================================
// STAKES-DECLARATION-SEAM-Q6
// =============================================================================
// Everything below builds the artifact the RUNTIME half (Q5's sibling task Q6 —
// `ManifestStakesResolver` + `ELOHIM_NETWORK_STAKES`) consumes. No write route
// for a policy manifest exists today: elohim-storage's `manifests` table is
// populated only by `services::bootstrap_manifests::seed_if_empty` (embedded
// JSON, local-only) and there is no `POST /manifests` on storage or doorway
// (verified 2026-08-16). So the seeder emits the declaration to a file and logs
// it; the POST is Q6's to add.
//
// EXACT CONTRACT Q6 SHOULD ACCEPT (this is what `emitStakesDeclaration` writes,
// byte-for-byte the shape below):
//
// {
//   "schemaVersion": "1",
//   "manifestKind": "network-stakes",
//   "manifestCid": "stakes:genesis-lamad:sha256-<64 hex>",
//   "stakes": {
//     "stage": "simulacra",              // NetworkStage variant, lowercase
//     "scope": "genesis-lamad",          // == StakesResolver::stage_for(scope)
//     "grantor": "human-adam-firstman",  // declaring steward
//     "environment": "preproduction"
//   },
//   "corpus": { "corpusId": "genesis-lamad", "title": "Genesis Lamad Corpus",
//                "realismRung": 0, "itemsDeclared": 3432 },
//   "provenance": {
//     "declaredBy": "genesis/data/lamad/corpus.json",
//     "declarationSha256": "<64 hex>",   // sha256 of the corpus.json bytes
//     "mintedBy": "genesis-seeder",
//     "mintedAt": "2026-08-16T12:00:00.000Z",
//     "seedTarget": "http://localhost:8888"
//   },
//   "operatorConfig": { "ELOHIM_NETWORK_STAKES": "simulacra" }
// }
//
// Q6 mapping:
//   - `manifestCid`  -> `StakesProvenance::Manifest { cid }`
//   - `stakes.scope` -> the `scope` argument of `StakesResolver::stage_for`
//   - `stakes.stage` -> `NetworkStage` (exact lowercase match; anything else is
//                       a REJECT, not a downgrade — do not "best effort" parse)
//   - artifact absent / unparseable / stage unknown -> `NetworkStage::Bootstrap`
//     with `StakesProvenance::BootstrapDefault`. Fail closed, always.
//   - `operatorConfig.ELOHIM_NETWORK_STAKES` is the OperatorConfig path's value;
//     when both a manifest and the env var are present, Q6 decides precedence —
//     this artifact asserts nothing about it beyond carrying both.
//
// When Q6 lands a write route, the ONLY change here is: after
// `emitStakesDeclaration`, POST the same object to it and log the response.
// Nothing about the declaration, its validation, or its shape changes.
// =============================================================================

/** The machine-readable artifact the runtime (Q6) consumes. */
export interface StakesDeclarationArtifact {
  schemaVersion: '1';
  manifestKind: 'network-stakes';
  manifestCid: string;
  stakes: {
    stage: NetworkStage;
    scope: string;
    grantor: string;
    environment: EnvironmentTier;
  };
  corpus: {
    corpusId: string;
    title: string;
    realismRung: RealismRung;
    itemsDeclared: number | null;
  };
  provenance: {
    declaredBy: string;
    declarationSha256: string;
    mintedBy: 'genesis-seeder';
    mintedAt: string;
    seedTarget: string;
  };
  operatorConfig: { ELOHIM_NETWORK_STAKES: NetworkStage };
}

export interface MintContext {
  /** Where this seed run is writing (doorway base URL). */
  seedTarget: string;
  /** Repo-relative path recorded as `provenance.declaredBy`. */
  declaredBy: string;
  /** Item count of the corpus, when the caller knows it. */
  itemsDeclared?: number | null;
  /** ISO-8601 mint timestamp; defaults to now. Injectable for deterministic tests. */
  mintedAt?: string;
}

/**
 * Build the stakes declaration for a loaded corpus declaration.
 *
 * @returns `null` when the corpus declares no `trustBootstrap` — nothing minted.
 */
export function buildStakesDeclaration(
  loaded: LoadedCorpusDeclaration,
  ctx: MintContext,
): StakesDeclarationArtifact | null {
  const { declaration, sha256 } = loaded;
  const grant = declaration.trustBootstrap;
  if (!grant) return null;

  return {
    schemaVersion: '1',
    manifestKind: 'network-stakes',
    manifestCid: `stakes:${grant.scope}:sha256-${sha256}`,
    stakes: {
      stage: grant.stage,
      scope: grant.scope,
      grantor: grant.grantor,
      environment: declaration.environment,
    },
    corpus: {
      corpusId: declaration.id,
      title: declaration.title,
      realismRung: declaration.realism.rung,
      itemsDeclared: ctx.itemsDeclared ?? null,
    },
    provenance: {
      declaredBy: ctx.declaredBy,
      declarationSha256: sha256,
      mintedBy: 'genesis-seeder',
      mintedAt: ctx.mintedAt ?? new Date().toISOString(),
      seedTarget: ctx.seedTarget,
    },
    operatorConfig: { ELOHIM_NETWORK_STAKES: grant.stage },
  };
}

/** The one log line the plan asks for. Kept as a pure function so tests can pin it. */
export function stakesDeclarationLogLine(artifact: StakesDeclarationArtifact): string {
  return `stakes declaration: ${artifact.stakes.stage} for corpus ${artifact.stakes.scope} by ${artifact.stakes.grantor}`;
}

/** Write the artifact to `outputPath` (pretty JSON + trailing newline). */
export function emitStakesDeclaration(artifact: StakesDeclarationArtifact, outputPath: string): void {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
}
