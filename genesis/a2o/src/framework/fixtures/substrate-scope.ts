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
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

export type CapStatus = 'available' | 'unavailable' | 'unknown';

/** Resolve the durable home (cluster-state.yaml). ELOHIM_CLUSTER_STATE_PATH_OVERRIDE lets tests
 * point at a fixture without touching the real file (mirrors humans.ts's override). */
function clusterStatePath(): string {
  const override = process.env.ELOHIM_CLUSTER_STATE_PATH_OVERRIDE;
  if (override) return override;
  // genesis/a2o/src/framework/fixtures/ → genesis/manifests/cluster-state.yaml
  return resolve(
    dirname(fileURLToPath(import.meta.url)),
    '../../../../manifests/cluster-state.yaml'
  );
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
      if (avail) return avail[1] === 'true' ? 'available' : 'unavailable';
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
  if (raw === 'available' || raw === 'unavailable') return raw;
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
  if (override) return override === 'available';
  const status = clusterCapStatus(cap);
  if (status === null) return true; // unreadable durable home → fail-open
  if (status === 'unknown') return true; // cap not declared in cluster-state → not a gate
  return status === 'available';
}

const REQUIRES_TAG = /^@requires:([a-z0-9][a-z0-9-]*)$/;

/** Extract cap names from a scenario/feature's `@requires:<cap>` gherkin tags. */
export function requiredCapsFromTags(tags: string[]): string[] {
  const caps: string[] = [];
  for (const t of tags) {
    const m = REQUIRES_TAG.exec(t.trim());
    if (m) caps.push(m[1]);
  }
  return caps;
}

/**
 * The cluster-TRACKED required caps that are unavailable for this run — i.e. the reason(s) a
 * scenario should be held. Caps not declared in cluster-state (e.g. `@requires:doorway`,
 * `@requires:seeded-content` — fixture preconditions handled elsewhere) are NOT substrate gates and
 * are ignored. Empty result ⇒ the scenario is in scope.
 *
 * Reads cluster-state.yaml at most ONCE per call (one snapshot → the vocabulary and every cap's
 * status are computed from the same bytes; no N+1 reads, no mid-call inconsistency). Invariant: a
 * cap used in `@requires:` must stay DECLARED in cluster-state — if a tracked cap is deleted/renamed
 * out of the file it silently demotes to a fixture precondition here (fail-open); scope-reconcile's
 * VOCAB-drift warning is what catches that on the planning side.
 */
export function unavailableRequiredCaps(tags: string[]): string[] {
  const caps = requiredCapsFromTags(tags);
  if (caps.length === 0) return [];
  let raw: string;
  try {
    raw = readFileSync(clusterStatePath(), 'utf-8');
  } catch {
    return []; // unreadable durable home → fail-open (never silently gate)
  }
  const known = knownResourcesFromText(raw);
  return caps.filter(c => {
    if (!known.has(c)) return false; // not a substrate cap → fixture precondition, not a gate
    const status = capEnvOverride(c) ?? parseResourceAvailability(raw, c);
    return status !== 'available';
  });
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

/** Reset substrate-skip tracking. Test runners call this at start-of-run. */
export function resetSubstrateSkips(): void {
  _skipped = new Map();
}
