/**
 * substrate-scope.ts — the generic substrate-availability primitive for the a2o test bench.
 *
 * The Elohim "scope reconciler" is a cybernetic control loop over the agentic-memory corpus:
 * genesis/manifests/cluster-state.yaml is the SENSOR (declared substrate reality), each artifact's
 * `@requires:<cap>` tag is its SETPOINT (which dependency points it needs), and the reconciler holds
 * or skips artifacts whose required capability is unavailable so agentic developers operate only on
 * the in-scope focus form. This module is the RUNTIME arm of that loop: given a scenario's tags, it
 * decides — from the same durable home the planning arm reads — whether every required cap is
 * available for this test run.
 *
 * NOTHING here is shem-specific. `shem` is one cap among many (`alpha-cluster-6peer`,
 * `harbor-registry`, and any future dependency point declared in cluster-state.yaml). humans.ts
 * layers the shem-specific RemoteCompute API on top of these primitives; the Before hook in
 * common.steps.ts gates any `@requires:<cap>` scenario generically.
 *
 * Mirrors the planning arm: .claude/scripts/_lib/env_scope.py (gap_blocked) and
 * .claude/scripts/memory-kit/scope-reconcile.py (_parse_cluster). The line-based YAML parse is
 * deliberate — a2o has no YAML dependency and the file's shape is stable.
 */

import { readFileSync } from 'node:fs';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export type CapStatus = 'available' | 'unavailable' | 'unknown';

/** The three acts of the suite (genesis/a2o/LAYERS.md). `host` is the no-substrate lane. */
export type ActId = 'i' | 'ii' | 'iii' | 'host';

const AVAILABLE = 'available';
const LIVE_CLUSTER_STATE = 'cluster-state.yaml';

/**
 * Which cluster-state file declares each act's BASELINE — the caps that act provides BY DEFINITION.
 * `host` has none: 101 scenarios assert only on repo JSON or the `epr` CLI and need no substrate at
 * all, so they run in any lane, laptop included.
 */
const ACT_BASELINE_FILES: Record<ActId, string | null> = {
  i: 'cluster-state.act1-household.yaml',
  ii: 'cluster-state.act2-neighbourhood.yaml',
  iii: LIVE_CLUSTER_STATE,
  host: null,
};

/** genesis/manifests/ — the directory holding the live cluster-state AND the act lane contracts.
 * ELOHIM_MANIFESTS_DIR_OVERRIDE lets tests supply a synthetic three-file set. */
function manifestsDir(): string {
  const override = process.env.ELOHIM_MANIFESTS_DIR_OVERRIDE;
  if (override) return override;
  // genesis/a2o/src/framework/fixtures/ → genesis/manifests/
  return resolve(dirname(fileURLToPath(import.meta.url)), '../../../../manifests');
}

/** Resolve the ACTIVE LANE's cluster-state. ELOHIM_CLUSTER_STATE_PATH_OVERRIDE is how a lane opts
 * into an act contract (the mesh stage points it at cluster-state.act1-household.yaml); unset, the
 * run reads the live cluster-state.yaml and today's gating behaviour is unchanged. */
function clusterStatePath(): string {
  const override = process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE;
  if (override) return override;
  return join(manifestsDir(), LIVE_CLUSTER_STATE);
}

/**
 * Parse one resource's availability from cluster-state.yaml text. Walks `resources:` →
 * `  <name>:` (2-space indent) → `    available: <v>` (4-space). A cap is 'available' iff its
 * value is exactly `true`; `false`/`degraded` → 'unavailable' (the conservative read — a degraded
 * capability is not safe to assert against); the resource being absent or lacking an `available:`
 * line → 'unknown' (the caller decides what to do with an undeclared cap).
 */
export function parseResourceAvailability(raw: string, cap: string): CapStatus {
  // The only 2-space `<name>:` keys in cluster-state.yaml are the resource entries, so tracking the
  // cap's block by that indent is sufficient (and keeps this branch-light). 4-space `available:` is
  // the value line; deeper-indented role/note continuations are ignored.
  let inBlock = false;
  for (const line of raw.split('\n')) {
    const head = /^ {2}([A-Za-z0-9_-]+):\s*$/.exec(line);
    if (head) {
      inBlock = head[1] === cap;
    } else if (inBlock) {
      const avail = /^ {4}available:\s*(\S+)/.exec(line);
      if (avail) return avail[1] === 'true' ? AVAILABLE : 'unavailable';
    }
  }
  return 'unknown';
}

/** Resource names declared in cluster-state.yaml text (pure — no I/O). Extracted so a caller that
 * already holds the file text can compute the vocabulary without a second read. */
function knownResourcesFromText(raw: string): Set<string> {
  const names = new Set<string>();
  let inResources = false;
  for (const line of raw.split('\n')) {
    if (/^resources:\s*$/.test(line)) {
      inResources = true;
      continue;
    }
    if (!inResources) continue;
    if (/^[A-Za-z]/.test(line)) break; // left the resources block
    const head = /^ {2}([A-Za-z0-9_-]+):\s*$/.exec(line);
    if (head) names.add(head[1]);
  }
  return names;
}

/** Resource names declared AVAILABLE in a cluster-state text (pure). An act's baseline is exactly
 * this set for that act's lane contract — which is why `available: false` in an act file is a lane
 * statement ("this act does not provide it"), never a health report. */
function availableResourcesFromText(raw: string): Set<string> {
  const names = new Set<string>();
  for (const name of knownResourcesFromText(raw)) {
    if (parseResourceAvailability(raw, name) === AVAILABLE) names.add(name);
  }
  return names;
}

/** All resource names declared in cluster-state.yaml (the cap vocabulary the planning arm uses).
 * A `@requires:<cap>` tag whose cap is NOT in this set is a fixture precondition (e.g. doorway,
 * seeded-content), not a hardware-availability gate — see unavailableRequiredCaps(). */
export function knownResources(): Set<string> {
  try {
    return knownResourcesFromText(readFileSync(clusterStatePath(), 'utf-8'));
  } catch {
    return new Set(); // unreadable durable home → empty vocabulary (nothing gates)
  }
}

/** Status of a cap from the durable home; null when cluster-state.yaml is unreadable
 * (e.g. a published consumer outside the repo) — the caller then fails open. */
export function clusterCapStatus(cap: string): CapStatus | null {
  try {
    return parseResourceAvailability(readFileSync(clusterStatePath(), 'utf-8'), cap);
  } catch {
    return null;
  }
}

/** The env var name carrying a per-cap runtime override (the CI/operator channel). */
function capVarName(cap: string): string {
  return `ELOHIM_CAP_${cap.toUpperCase().replace(/[^A-Z0-9]+/g, '_')}_STATUS`;
}

/**
 * Per-cap env override — the runtime channel CI's Probe Substrate stage / the operator sets so a
 * live probe can win over the durable declaration for one run. `shem` uses ONLY its historical var
 * `ELOHIM_REMOTE_COMPUTE_STATUS` (the SAME var humans.ts reads — so the two shem checks can never
 * disagree on the override channel); every other cap uses `ELOHIM_CAP_<UPPER_SNAKE>_STATUS`. Only an
 * explicit available|unavailable counts as an override; anything else → null (derive).
 */
export function capEnvOverride(cap: string): CapStatus | null {
  const envVar = cap === 'shem' ? 'ELOHIM_REMOTE_COMPUTE_STATUS' : capVarName(cap);
  const raw = (process.env[envVar] ?? '').toLowerCase().trim();
  if (raw === AVAILABLE || raw === 'unavailable') return raw;
  return null;
}

/**
 * Is <cap> available for this run? Precedence (mirrors humans.ts's two-homes rule so the runtime
 * can never silently disagree with the durable home):
 *   1. explicit env override (CI probe / operator) wins;
 *   2. else cluster-state.yaml — only `available: true` counts as available;
 *   3. fail-open (true) ONLY when the cap is unknown to cluster-state or the file is unreadable —
 *      an undeclared capability never silently holds a run.
 */
export function isCapAvailable(cap: string): boolean {
  const override = capEnvOverride(cap);
  if (override) return override === AVAILABLE;
  const status = clusterCapStatus(cap);
  if (status === null) return true; // unreadable durable home → fail-open
  if (status === 'unknown') return true; // cap not declared in cluster-state → not a gate
  return status === AVAILABLE;
}

/** The cap that says a lane OWNS its substrate (may kill peers, restart conductors, write pins,
 * delete blobs). TRUE only in the Act I household contract; FALSE on every shared fleet. */
export const OWNED_SUBSTRATE_CAP = 'owned-substrate';

/** The operator switch every destructive gate honours: `1` forces destructive steps ON, `0`
 * forces them OFF, anything else defers to the declared cap. */
export const DESTRUCTIVE_ENV = 'A2O_ALLOW_DESTRUCTIVE';

/** The one sentence a held destructive step prints — so every gate names the same two ways in. */
export const DESTRUCTIVE_HELD_HINT =
  `Set ${DESTRUCTIVE_ENV}=1, or run on a lane whose cluster-state declares ` +
  `${OWNED_SUBSTRATE_CAP} available (the Act I household contract, ` +
  'ELOHIM_CLUSTER_STATE_PATH_OVERRIDE=genesis/manifests/cluster-state.act1-household.yaml), to enable.';

/**
 * May this run mutate the substrate? ONE answer for every destructive gate under steps/.
 *
 * Before this helper each step file re-read `A2O_ALLOW_DESTRUCTIVE` on its own, so a lane that
 * DECLARED `owned-substrate: available` (the household mesh — the only lane where destructive
 * chapters can run at all) still held every destructive step unless the operator also remembered
 * a second switch. Two homes for one fact is how saga chapter 11 sat skipped on a mesh that owned
 * itself (2026-08-22). The declared cap is the authority; the env var is the operator override.
 *
 * Precedence:
 *   1. `A2O_ALLOW_DESTRUCTIVE=1` → on; `=0` → off (explicit operator intent wins either way);
 *   2. else the per-cap runtime override `ELOHIM_CAP_OWNED_SUBSTRATE_STATUS` (CI probe channel);
 *   3. else the active lane's cluster-state declares `owned-substrate` `available: true` → on;
 *   4. else OFF. Deliberately NOT fail-open, unlike isCapAvailable(): an unreadable or silent
 *      cluster-state must never switch kill/restart/delete ON by accident. The shared fleet
 *      declares the cap false on purpose; a lane that says nothing is treated the same way.
 */
export function destructiveAllowed(): boolean {
  const raw = (process.env[DESTRUCTIVE_ENV] ?? '').trim();
  if (raw === '1') return true;
  if (raw === '0') return false;
  const override = capEnvOverride(OWNED_SUBSTRATE_CAP);
  if (override) return override === AVAILABLE;
  return clusterCapStatus(OWNED_SUBSTRATE_CAP) === AVAILABLE;
}

const REQUIRES_TAG = /^@requires:([a-z0-9][a-z0-9-]*)$/;
const ACT_TAG = /^@act:(i|ii|iii|host)$/;

/** Extract cap names from a scenario/feature's `@requires:<cap>` gherkin tags. */
export function requiredCapsFromTags(tags: string[]): string[] {
  const caps: string[] = [];
  for (const t of tags) {
    const m = REQUIRES_TAG.exec(t.trim());
    if (m) caps.push(m[1]);
  }
  return caps;
}

/** Every `@act:<id>` a scenario declares, in tag order (normally exactly one). */
function actsFromTags(tags: string[]): ActId[] {
  const acts: ActId[] = [];
  for (const t of tags) {
    const m = ACT_TAG.exec(t.trim());
    if (m) acts.push(m[1] as ActId);
  }
  return acts;
}

/**
 * The act a scenario declares, or null when it declares none (in which case only its explicit
 * `@requires:` caps gate it — the pre-layering behaviour). A scenario belongs to exactly ONE act;
 * two act tags is an authoring error and is warned about at the gate.
 */
export function actFromTags(tags: string[]): ActId | null {
  return actsFromTags(tags)[0] ?? null;
}

/**
 * The BASELINE caps of an act: the caps its own lane contract declares available. This is the whole
 * point of the layering — a scenario declares its act, and the caps follow, so `@requires:` is only
 * ever emitted for a cap OUTSIDE the act's baseline.
 *
 * `host` → no caps (no substrate at all). `iii` → the live cluster-state's available caps plus
 * `shem`, whose lane the commons is. Unreadable contract → empty baseline (fail-open: a missing
 * file must never invent a gate).
 */
export function actBaselineCaps(act: ActId): string[] {
  const file = ACT_BASELINE_FILES[act];
  if (file === null) return [];
  let raw: string;
  try {
    raw = readFileSync(join(manifestsDir(), file), 'utf-8');
  } catch {
    return [];
  }
  const caps = availableResourcesFromText(raw);
  if (act === 'iii') caps.add('shem'); // Act III's stage, by definition
  return [...caps].sort((a, b) => a.localeCompare(b));
}

// --- loud-once diagnostics -------------------------------------------------
// An undeclared cap used to be a SILENT no-op: `@requires:invented-thing` read like a gate and held
// nothing. It still fails open (that is the safe default), but it now says so — once per run per
// cap, so a 900-scenario suite does not drown in it.

let _warnedOnce = new Set<string>();

function warnOnce(key: string, message: string): void {
  if (_warnedOnce.has(key)) return;
  _warnedOnce.add(key);
  console.warn(message);
}

function warnUndeclaredCap(cap: string): void {
  warnOnce(
    `cap:${cap}`,
    `  ⚠️  UNDECLARED CAP: @requires:${cap} GATES NOTHING — "${cap}" is not a resource in ` +
      `${clusterStatePath()}. Declare it there with an \`available:\` line (see ` +
      `genesis/a2o/LAYERS.md), or drop the tag: a tag that reads like a gate and holds nothing is ` +
      `worse than no tag at all.`
  );
}

function warnMultipleActs(acts: ActId[]): void {
  warnOnce(
    `acts:${acts.join(',')}`,
    `  ⚠️  MULTIPLE ACT TAGS: a scenario declares @act:${acts.join(' and @act:')} — a scenario ` +
      `belongs to exactly ONE act (genesis/a2o/LAYERS.md). Gating on the first (@act:${acts[0]}).`
  );
}

/** Caps warned about as undeclared during this run (sorted) — the machine-readable form of the
 * loud warning, for a runner that wants to report tag drift. */
export function undeclaredCapsWarned(): string[] {
  return [..._warnedOnce]
    .filter(k => k.startsWith('cap:'))
    .map(k => k.slice(4))
    .sort((a, b) => a.localeCompare(b));
}

/** Reset the once-per-run diagnostic ledger. Folded into resetSubstrateSkips() so the existing
 * BeforeAll hook clears it without needing a second call site. */
export function resetUndeclaredCapWarnings(): void {
  _warnedOnce = new Set();
}

/**
 * The reasons this scenario should be HELD for this run — empty ⇒ it is in scope.
 *
 * Two sources, one rule (`genesis/a2o/LAYERS.md`): a scenario runs when its ACT's baseline caps are
 * all available in the ACTIVE LANE **and** every explicit `@requires:` cap is available. Act-baseline
 * holds are reported as `<cap> (act <id> baseline)` so the skip line names the act that priced the
 * scenario, not just an orphan cap name.
 *
 * Fail-open, everywhere it matters: an unreadable lane file, a baseline cap the lane does not
 * declare, and an undeclared `@requires:` cap all let the scenario run — the last of those now warns
 * loudly (once per run) instead of passing silently. That is what keeps a lane which has NOT opted
 * into an act contract behaving exactly as it did before the layering.
 *
 * Reads the lane file at most ONCE per call (one snapshot → vocabulary and every cap's status come
 * from the same bytes; no N+1 reads, no mid-call inconsistency).
 */
function heldBaselineCaps(
  act: ActId,
  known: Set<string>,
  laneStatus: (cap: string) => CapStatus
): string[] {
  // A baseline cap the LANE never declares is not a lane gate — the same fail-open rule as
  // @requires. A lane opts into an act contract by declaring the vocabulary, not by omitting it.
  return actBaselineCaps(act)
    .filter(cap => known.has(cap) && laneStatus(cap) !== AVAILABLE)
    .map(cap => `${cap} (act ${act} baseline)`);
}

function heldExplicitCaps(
  caps: string[],
  known: Set<string>,
  laneStatus: (cap: string) => CapStatus
): string[] {
  const held: string[] = [];
  for (const cap of caps) {
    if (known.has(cap)) {
      if (laneStatus(cap) !== AVAILABLE) held.push(cap);
    } else {
      warnUndeclaredCap(cap); // fixture precondition OR tag drift — say so, but never gate
    }
  }
  return held;
}

export function unavailableRequiredCaps(tags: string[]): string[] {
  const explicit = requiredCapsFromTags(tags);
  const acts = actsFromTags(tags);
  if (explicit.length === 0 && acts.length === 0) return [];
  if (acts.length > 1) warnMultipleActs(acts);
  let raw: string;
  try {
    raw = readFileSync(clusterStatePath(), 'utf-8');
  } catch {
    return []; // unreadable durable home → fail-open (never silently gate)
  }
  const known = knownResourcesFromText(raw);
  const laneStatus = (cap: string): CapStatus =>
    capEnvOverride(cap) ?? parseResourceAvailability(raw, cap);
  const baseline = acts.length > 0 ? heldBaselineCaps(acts[0], known, laneStatus) : [];
  return [...baseline, ...heldExplicitCaps(explicit, known, laneStatus)];
}

// ---------------------------------------------------------------------------
// Skip tracking (parallels autoSkippedHumans in humans.ts) — lets the runner
// surface a reduced-scope summary and mark the build UNSTABLE, not FAILED.
// ---------------------------------------------------------------------------

// Keyed on `<uri>::<scenario>` (composite) so two scenarios that share the same `Scenario:` text in
// different feature files do NOT overwrite each other in the reduced-scope report.
let _skipped = new Map<string, { scenario: string; caps: string[] }>();

/** Record that a scenario was held because <caps> are unavailable. `uri` (the feature-file path)
 * disambiguates same-named scenarios across files; pass scenario.pickle.uri at the call site. */
export function noteSubstrateSkip(scenario: string, caps: string[], uri = ''): void {
  _skipped.set(`${uri}::${scenario}`, { scenario, caps: [...caps] });
}

/** Snapshot of scenarios held this run by substrate-cap gating (sorted by name, copy). */
export function substrateSkippedScenarios(): { scenario: string; caps: string[] }[] {
  return [..._skipped.values()].sort((a, b) => a.scenario.localeCompare(b.scenario));
}

/** Reset substrate-skip tracking AND the once-per-run diagnostic ledger. Test runners call this at
 * start-of-run (steps/common.steps.ts's BeforeAll), so the loud undeclared-cap warning fires once
 * per RUN, not once per process. */
export function resetSubstrateSkips(): void {
  _skipped = new Map();
  resetUndeclaredCapWarnings();
}
