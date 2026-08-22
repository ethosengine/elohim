/**
 * The `env` block and `gitCommit` of a sprint report (Law II, guidestar
 * 2026-08-22 §3).
 *
 * "Localdev and CI are not two kinds of thing, two tiers, or two authorities.
 * They are one Process applying one measure to one subject, differing in
 * exactly one component of the derivation triple." This module builds that
 * component, so a local↔fleet divergence is a `diff` of two reports rather
 * than an argument in a human's head.
 *
 * Every field here ships a DEFAULT the run emits and a human can later
 * correct — the declaration load must never become the evening. What cannot be
 * observed is NAMED in `unknown` rather than guessed or omitted.
 */

import { readFileSync } from 'node:fs';

import { computeSut, createSutProbe, type SutComponent, type SutProbe } from './sut.js';

export const LANE_HOUSEHOLD = 'household';
export const LANE_ALPHA_FLEET = 'alpha-fleet';
export const LANE_UNKNOWN = 'unknown';

/** The four declared network stages (genesis/seeder/src/corpus-trust.ts NETWORK_STAGES). */
export const NETWORK_STAGES = ['simulacra', 'bootstrap', 'coordinated', 'enforced'];

export interface RunEnv {
  lane: string;
  peers: number;
  processControl: boolean;
  networkStage: string;
  sut: string;
  sutParts: Record<string, string>;
  unknown: string[];
}

export interface RunEnvProbe extends SutProbe {
  /** File contents, or null when unreadable. */
  readText(path: string): string | null;
}

/**
 * Default lane for a cucumber profile — a documented default, not a claim.
 * `--lane` (or A2O_LANE) overrides it, and any lane not in this table resolves
 * to "unknown" rather than being guessed into one of the two real ones.
 */
export function laneForProfile(profile: string): string {
  switch (profile) {
    case 'mesh':
    case 'local':
    case 'household':
      return LANE_HOUSEHOLD;
    case 'dataplane':
    case 'alpha':
    case 'delivery':
    case 'delivery-browser':
    case 'browser':
    case 'testnet':
      return LANE_ALPHA_FLEET;
    default:
      return LANE_UNKNOWN;
  }
}

/** First value that is set and not blank — a blank declaration declares nothing. */
function firstDeclared(...values: (string | undefined)[]): string | undefined {
  for (const value of values) {
    const trimmed = value?.trim();
    if (trimmed) return trimmed;
  }
  return undefined;
}

export function resolveLane(
  env: Readonly<Record<string, string | undefined>>,
  profile: string,
  explicit?: string
): string {
  return firstDeclared(explicit, env['A2O_LANE']) ?? laneForProfile(profile);
}

/**
 * The commit this run measured. GIT_COMMIT when CI sets it, else
 * `git rev-parse HEAD`, else UNDEFINED — an unknown commit is omitted, never
 * rendered as a present one. Matches CoverageGapReport.gitCommit's field shape
 * (scripts/types.ts) rather than minting a second convention.
 */
export function resolveGitCommit(probe: SutProbe): string | undefined {
  // GIT_COMMIT first, and only then git — CI already knows, and a spawn per
  // report is a cost with no answer attached.
  const declared = firstDeclared(probe.env['GIT_COMMIT']);
  if (declared) return declared;
  return firstDeclared(probe.git(['rev-parse', 'HEAD']) ?? undefined);
}

interface FixtureShape {
  processControl?: boolean;
  storagePeers?: Record<string, { url?: string } | undefined>;
}

function readFixture(probe: RunEnvProbe): FixtureShape | null {
  const path = probe.env['E2E_HOUSEHOLD_FIXTURE_PATH']?.trim();
  if (!path) return null;
  const raw = probe.readText(path);
  if (raw === null) return null;
  try {
    return JSON.parse(raw) as FixtureShape;
  } catch {
    return null;
  }
}

function optionalInteger(value: string | undefined): number | undefined {
  if (value === undefined || value.trim() === '') return undefined;
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : undefined;
}

function parseBoolean(value: string | undefined): boolean | undefined {
  if (value === undefined) return undefined;
  const normalized = value.trim().toLowerCase();
  if (['1', 'true', 'yes'].includes(normalized)) return true;
  if (['0', 'false', 'no'].includes(normalized)) return false;
  return undefined;
}

export interface RunEnvOptions {
  profile: string;
  /** From `--lane`; falls back to A2O_LANE, then the profile table. */
  lane?: string;
  components?: readonly SutComponent[];
}

export function computeRunEnv(probe: RunEnvProbe, options: RunEnvOptions): RunEnv {
  const lane = resolveLane(probe.env, options.profile, options.lane);
  const fixture = readFixture(probe);
  const unknown: string[] = [];

  // peers — the household fixture names the peers this run addressed, with
  // real URLs. A deployed fleet's peer count cannot be observed from the repo
  // without a live probe, so it is declared unknown rather than guessed at 1.
  const declaredPeers = optionalInteger(probe.env['A2O_PEER_COUNT']);
  const fixturePeers = Object.values(fixture?.storagePeers ?? {}).filter(peer =>
    Boolean(peer?.url)
  ).length;
  const peers = declaredPeers ?? fixturePeers;
  if (declaredPeers === undefined && fixturePeers === 0) unknown.push('peers');

  // processControl — the fixture already declares this (it is what makes a
  // destructive chapter runnable at all); the lane is the fallback.
  const processControl =
    parseBoolean(probe.env['A2O_PROCESS_CONTROL']) ??
    fixture?.processControl ??
    lane === LANE_HOUSEHOLD;

  // networkStage — advertised by a doorway's freshness policy, which this
  // builder does not call. A declaration wins; otherwise say so.
  const declaredStage = [
    probe.env['E2E_NETWORK_STAGE'],
    probe.env['A2O_NETWORK_STAGE'],
    probe.env['ELOHIM_NETWORK_STAGE'],
  ]
    .map(value => value?.trim())
    .find(value => value && NETWORK_STAGES.includes(value));
  const networkStage = declaredStage ?? LANE_UNKNOWN;
  if (!declaredStage) unknown.push('networkStage');

  // A run without process control is running against a deployed substrate
  // whose cluster state, PVC pressure and live DNS are operator-owned and not
  // derivable from this repo. Law II names them; they are declared unknown in
  // the key rather than omitted from it.
  if (!processControl) unknown.push('clusterState', 'pvcPressure', 'liveDns');

  const identity = computeSut(probe, options.components);

  return {
    lane,
    peers,
    processControl,
    networkStage,
    sut: identity.sut,
    sutParts: identity.sutParts,
    unknown: [...unknown, ...identity.unknown],
  };
}

export function createRunEnvProbe(
  repoRoot: string,
  env: Readonly<Record<string, string | undefined>> = process.env
): RunEnvProbe {
  const base = createSutProbe(repoRoot, env);
  return {
    ...base,
    readText(path: string): string | null {
      try {
        return readFileSync(path, 'utf8');
      } catch {
        return null;
      }
    },
  };
}
