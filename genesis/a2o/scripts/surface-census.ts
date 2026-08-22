/**
 * `surface-census` — mechanical WIRE / FIXTURE / IMPLEMENT-BOUNDED /
 * IMPLEMENT-DESIGN / STRUCTURAL / DEFECT-STALE classifier for a2o scenarios.
 *
 * Read this alongside `LAYERS.md`, `layering/wip-inventory.md` (the bind
 * classification method this script reuses), `layering/burndown.md` and
 * `layering/code-reds.md` — this script answers a DIFFERENT question than
 * wip-inventory (which only says "is this step bound"): for every `@wip` or
 * currently-failed Act I/host scenario it asks "what does the mesh actually
 * need to turn this green" and answers mechanically, from live probes, not
 * from reading the feature prose.
 *
 * Method (mirrors the task spec):
 *   1. Discover feature files that carry `@wip` or that failed in the latest
 *      mesh report — the population this classifier exists to serve.
 *   2. Bind status per scenario via a SCOPED `cucumber-js --dry-run` (JSON
 *      formatter), cached per feature file. Never `-p local`; the config is a
 *      generated empty profile so cucumber does not merge in the whole tree
 *      (see LAYERS.md § The scoping rule).
 *   3. For every step (bound or not) — the rendered step TEXT already carries
 *      literal args (`When I query "/db/p2p/conductor-diagnostics" on peer
 *      "alpha-A"`), which is the primary surface source. For BOUND steps,
 *      also scan a heuristically-extracted slice of the step definition's own
 *      source (same file, one hop into a same-file helper) — some surfaces
 *      are baked into glue rather than named in the Gherkin.
 *   4. PROBE the live mesh read-only (GET only, never POST/PATCH/DELETE) for
 *      existence of every distinct surface found, deduped across the whole
 *      population so the mesh is hit once per surface, not once per scenario.
 *   5. Classify per the rubric in `classifyScenario()`.
 *   6. Write `layering/surface-census.md`.
 *
 * Re-runnable: `pnpm census`. Idempotent: read-only against the mesh (GET
 * only); dry-run JSON is cached under `reports/census-cache/` (gitignored),
 * keyed by feature-file mtime — `--fresh` forces a clean run.
 */

import { execFile } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

// ---------------------------------------------------------------------------
// paths & config
// ---------------------------------------------------------------------------

const HERE = fileURLToPath(new URL('.', import.meta.url));
const A2O_ROOT = join(HERE, '..');
const FEATURES_DIR = join(A2O_ROOT, 'features');
const CACHE_DIR = join(A2O_ROOT, 'reports', 'census-cache');
const EMPTY_CONFIG_PATH = join(CACHE_DIR, 'empty.cucumber.mjs');
const OUT_PATH = join(A2O_ROOT, 'layering', 'surface-census.md');
const PROLOGUE_SCRIPT = join(A2O_ROOT, '..', '..', 'app/elohim-app/scripts/hc-mesh-prologue.sh');
const HC_MESH_SCRIPT = join(A2O_ROOT, '..', '..', 'app/elohim-app/scripts/hc-mesh.sh');

const DOORWAY_URL = process.env['CENSUS_DOORWAY_URL'] ?? 'http://localhost:8888';
const STORAGE_URL = process.env['CENSUS_STORAGE_URL'] ?? 'http://localhost:8090';
// /tmp/elohim-local-mesh is the documented mesh-lane handoff location (hc-mesh-prologue.sh writes
// the fixture there; `just mesh` writes reports there) — not an arbitrary publicly-writable path.
const DEFAULT_FIXTURE_PATH = '/tmp/elohim-local-mesh/household-fixture.json'; // eslint-disable-line sonarjs/publicly-writable-directories
const HOUSEHOLD_FIXTURE_PATH = process.env['E2E_HOUSEHOLD_FIXTURE_PATH'] ?? DEFAULT_FIXTURE_PATH;
// Default candidate reports, newest/most-specific first — ALL non-empty ones
// found are merged (not "first wins"), so a saga-lane report and a mesh-lane
// report covering disjoint scenario sets both contribute DEFECT-STALE signal.
const DEFAULT_REPORT_CANDIDATES = [
  // wave1-saga.json (2026-08-21) was a VOID run — `--profile saga` launched
  // without the `just test mesh` env block, so all 25 scenarios self-skipped
  // in 0.2s with zero evidence. NOT listed here; wave1b-saga.json is the
  // real re-measurement after the corpus repair + doorway restart. A void
  // report is also detected generically at load time (see loadLatestReport)
  // in case a future run repeats the mistake under a different name.
  // eslint-disable-next-line sonarjs/publicly-writable-directories
  '/tmp/elohim-local-mesh/reports/wave1b-saga.json',
  // eslint-disable-next-line sonarjs/publicly-writable-directories
  '/tmp/elohim-local-mesh/reports/wave1b-lamad-ssr.json',
  // eslint-disable-next-line sonarjs/publicly-writable-directories
  '/tmp/elohim-local-mesh/reports/wave1b-mesh.json',
  // eslint-disable-next-line sonarjs/publicly-writable-directories
  '/tmp/elohim-local-mesh/reports/wave1.json',
  // eslint-disable-next-line sonarjs/publicly-writable-directories
  '/tmp/elohim-local-mesh/reports/inventory2.json',
];
const CONCURRENCY = Number(process.env['CENSUS_CONCURRENCY'] ?? 4);
const PROBE_TIMEOUT_MS = 3000;

const FRESH = process.argv.includes('--fresh');
const LIMIT_ARG = process.argv.find(a => a.startsWith('--limit='));
const LIMIT = LIMIT_ARG ? Number(LIMIT_ARG.slice('--limit='.length)) : undefined;
// --report <path> is repeatable and REPLACES the defaults entirely (an
// explicit ask for exactly these reports); omit it to merge the defaults.
const REPORT_ARGS = process.argv
  .filter(a => a.startsWith('--report='))
  .map(a => a.slice('--report='.length));
const REPORT_CANDIDATES = REPORT_ARGS.length > 0 ? REPORT_ARGS : DEFAULT_REPORT_CANDIDATES;

// ---------------------------------------------------------------------------
// small shared helpers
// ---------------------------------------------------------------------------

function log(line: string): void {
  console.log(line);
}

function readTextCached(cache: Map<string, string>, path: string): string {
  const cached = cache.get(path);
  if (cached !== undefined) return cached;
  const text = existsSync(path) ? readFileSync(path, 'utf8') : '';
  cache.set(path, text);
  return text;
}

/** Uniform-domain dedupe key for a Set<string> collector. */
function addAll(target: Set<string>, values: Iterable<string>): void {
  for (const v of values) target.add(v);
}

// ---------------------------------------------------------------------------
// step 1/2 — feature discovery + scoped dry-run (cached)
// ---------------------------------------------------------------------------

function discoverFeatureFiles(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) out.push(...discoverFeatureFiles(full));
    else if (entry.isFile() && entry.name.endsWith('.feature')) out.push(full);
  }
  return out;
}

/** Cheap pre-filter: only feature files worth a dry-run (carry @wip, or
 * appear in the latest report — checked once the report is loaded). */
function grepHasWip(path: string): boolean {
  return readFileSync(path, 'utf8').includes('@wip');
}

interface RawTag {
  name: string;
}
interface RawStepResult {
  status: string;
  error_message?: string;
}
interface RawStep {
  keyword: string;
  name: string;
  match?: { location: string };
  result?: RawStepResult;
}
interface RawElement {
  id: string;
  name: string;
  type: string;
  line: number;
  tags?: RawTag[];
  steps?: RawStep[];
}
interface RawFeature {
  uri: string;
  elements?: RawElement[];
}

function ensureEmptyConfig(): void {
  mkdirSync(CACHE_DIR, { recursive: true });
  writeFileSync(EMPTY_CONFIG_PATH, 'export default {};\n');
}

function cachePathFor(featureAbs: string): string {
  const rel = relative(A2O_ROOT, featureAbs).replaceAll(/[/\\]/g, '__');
  return join(CACHE_DIR, `${rel}.json`);
}

function cacheIsFresh(featureAbs: string, cachePath: string): boolean {
  if (FRESH || !existsSync(cachePath)) return false;
  return statSync(cachePath).mtimeMs >= statSync(featureAbs).mtimeMs;
}

async function runDryRun(featureAbs: string): Promise<RawFeature[]> {
  const cachePath = cachePathFor(featureAbs);
  if (cacheIsFresh(featureAbs, cachePath)) {
    return JSON.parse(readFileSync(cachePath, 'utf8')) as RawFeature[];
  }
  const featureRel = relative(A2O_ROOT, featureAbs);
  const configRel = relative(A2O_ROOT, EMPTY_CONFIG_PATH);
  const { stdout } = await execFileAsync(
    join(A2O_ROOT, 'node_modules', '.bin', 'cucumber-js'),
    [
      '--dry-run',
      featureRel,
      '--config',
      configRel,
      '--require-module',
      'tsx',
      '--require',
      'steps/**/*.ts',
      '--format',
      'json',
    ],
    { cwd: A2O_ROOT, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }
  );
  const parsed = JSON.parse(stdout) as RawFeature[];
  writeFileSync(cachePath, JSON.stringify(parsed));
  return parsed;
}

/** Bounded-concurrency map — dry-run spawns are process-heavy (tsx loads the
 * whole `steps/**` tree each time) and MUST be awaited async (never
 * `execFileSync`) or the subprocess blocks the single JS thread and no other
 * worker gets a turn — the "concurrency" would be concurrency in name only. */
async function mapWithConcurrency<T, R>(
  items: T[],
  limit: number,
  fn: (item: T) => Promise<R>
): Promise<R[]> {
  const results: R[] = [];
  let next = 0;
  async function worker(): Promise<void> {
    for (;;) {
      const i = next++;
      if (i >= items.length) return;
      results[i] = await fn(items[i]);
    }
  }
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
}

// ---------------------------------------------------------------------------
// scenario model — outline rows collapsed by `id`
// ---------------------------------------------------------------------------

interface StepInfo {
  keyword: string;
  text: string;
  matchLocation: string | null;
  bound: boolean;
}

interface CensusScenario {
  featureUri: string;
  scenarioId: string;
  name: string;
  line: number;
  tags: string[];
  steps: StepInfo[];
}

function isHookKeyword(keyword: string): boolean {
  const k = keyword.trim();
  return k === 'Before' || k === 'After' || k === '';
}

function toSteps(raw: RawStep[]): StepInfo[] {
  return raw
    .filter(s => !isHookKeyword(s.keyword))
    .map(s => ({
      keyword: s.keyword.trim(),
      text: s.name,
      matchLocation: s.match?.location ?? null,
      bound: s.match !== undefined,
    }));
}

function collectScenarios(features: RawFeature[]): CensusScenario[] {
  const byId = new Map<string, CensusScenario>();
  for (const feature of features) {
    for (const el of feature.elements ?? []) {
      if (el.type !== 'scenario') continue;
      if (byId.has(el.id)) continue; // Scenario Outline rows share one id — keep the first
      byId.set(el.id, {
        featureUri: feature.uri,
        scenarioId: el.id,
        name: el.name,
        line: el.line,
        tags: (el.tags ?? []).map(t => t.name),
        steps: toSteps(el.steps ?? []),
      });
    }
  }
  return [...byId.values()];
}

type BindClass = 'a' | 'b' | 'c';

function bindClass(scenario: CensusScenario): BindClass {
  const bound = scenario.steps.filter(s => s.bound).length;
  if (bound === scenario.steps.length) return 'a';
  if (bound === 0) return 'c';
  return 'b';
}

function actTag(scenario: CensusScenario): string | null {
  const tag = scenario.tags.find(t => t.startsWith('@act:'));
  return tag ? tag.slice('@act:'.length) : null;
}

function isInScope(scenario: CensusScenario): boolean {
  const act = actTag(scenario);
  return act === 'i' || act === 'host';
}

// ---------------------------------------------------------------------------
// step 3 — surface extraction
// ---------------------------------------------------------------------------

/** A field name paired with the HTTP surface its step queried, WHEN a
 * `When I query "<path>" on peer …` step precedes it in the same scenario —
 * `path: null` means no such step was seen and the field can only be
 * checked against whatever sample the scenario's surfaces happen to offer. */
interface FieldRef {
  path: string | null;
  field: string;
}

interface Surfaces {
  httpPaths: Set<string>;
  metrics: Set<string>;
  fieldRefs: FieldRef[];
  envVars: Set<string>;
  meshActions: Set<string>;
}

function emptySurfaces(): Surfaces {
  return {
    httpPaths: new Set(),
    metrics: new Set(),
    fieldRefs: [],
    envVars: new Set(),
    meshActions: new Set(),
  };
}

const HTTP_PATH_RE =
  /\/(?:db|api\/v1|api|admin|p2p|sync\/v1|sync|auth|health|threshold|status\.json)(?:\/[\w.-]+)*/g;
const METRIC_RE = /\b(?:elohim|doorway)_\w+\b/g;
const ENV_VAR_RE = /\b(?:E2E|ELOHIM)_[A-Z0-9_]+\b/g;
const FIELD_RE = /\bfield\s+"([^"]+)"/gi;
const UNDER_RE = /\bunder\s+"([^"]+)"/i;
const PARENT_FIELD_RE = /response field\s+"([^"]+)"\s+has field/i;
const MESH_ACTION_RE = /\b([A-Z_][A-Z0-9_]*)\s*,\s*(?:['"])([a-z][a-z0-9-]*)(?:['"])/g;

function extractFields(text: string): string[] {
  const under = UNDER_RE.exec(text)?.[1];
  const parent = PARENT_FIELD_RE.exec(text)?.[1];
  const out: string[] = [];
  for (const m of text.matchAll(FIELD_RE)) {
    const name = m[1] ?? '';
    if (under) out.push(`${under}[].${name}`);
    else if (parent) out.push(`${parent}.${name}`);
    else out.push(name);
  }
  return out;
}

/** hc-mesh.sh/hc-mesh-*.sh `case` action reached via `[SCRIPT_CONST, 'action', ...]`. */
function extractMeshActions(text: string, fileText: string): string[] {
  const out: string[] = [];
  for (const m of text.matchAll(MESH_ACTION_RE)) {
    const constName = m[1] ?? '';
    const action = m[2] ?? '';
    const defLine = fileText
      .split('\n')
      .find(l => l.includes(constName) && l.includes('=') && /hc-mesh/i.test(l));
    if (defLine) out.push(`hc-mesh::${action}`);
  }
  return out;
}

/** httpPaths/metrics/envVars/meshActions only — fields are handled
 * separately in `extractSurfaces` because they need step-sequence context
 * (which surface a `has field "x"` step is talking about). */
function scanText(text: string, fileText: string, surfaces: Surfaces): void {
  addAll(surfaces.httpPaths, text.match(HTTP_PATH_RE) ?? []);
  addAll(surfaces.metrics, text.match(METRIC_RE) ?? []);
  addAll(surfaces.envVars, text.match(ENV_VAR_RE) ?? []);
  addAll(surfaces.meshActions, extractMeshActions(text, fileText));
}

/** Top-level declaration boundaries in this codebase's prettier-formatted step
 * files (single quotes, col-0 top-level statements) — used to bound a
 * heuristic "extract this function's source" scan without a real parser. Kept
 * as separate small regexes (never combined into one alternation) so each
 * stays under the linter's regex-complexity ceiling. */
const TOP_LEVEL_BOUNDARY_RES = [
  /^(?:Given|When|Then|Before|After|BeforeAll|AfterAll)\(/,
  /^(?:export\s+)?(?:async\s+)?function\s/,
  /^const\s+\w+\s*=/,
];

function isTopLevelBoundary(line: string): boolean {
  return TOP_LEVEL_BOUNDARY_RES.some(re => re.test(line));
}

const MAX_BLOCK_LINES = 150;
const MAX_HELPER_HOPS = 1;

function extractBlock(lines: string[], startLine: number): string {
  const start = startLine - 1;
  let end = Math.min(lines.length, start + MAX_BLOCK_LINES);
  for (let i = start + 1; i < lines.length && i < start + MAX_BLOCK_LINES; i++) {
    if (isTopLevelBoundary(lines[i] ?? '')) {
      end = i;
      break;
    }
  }
  return lines.slice(start, end).join('\n');
}

const CALL_RE = /\b([a-z][a-zA-Z0-9]*)\(/g;
const SKIP_CALL_NAMES = new Set([
  'assert',
  'expect',
  'require',
  'describe',
  'it',
  'test',
  'console',
  'return',
  'await',
  'async',
  'function',
  'if',
  'for',
  'while',
  'switch',
  'catch',
  'typeof',
  'new',
  'this',
]);

/** One hop into a same-file helper the step body calls, so glue that
 * delegates (e.g. `establishGraduatedSteward` → `fetchHostingAccount`) still
 * surfaces the literals it hides. */
function followSameFileHelpers(body: string, fileText: string, fileLines: string[]): string[] {
  const blocks: string[] = [];
  const seen = new Set<string>();
  for (const m of body.matchAll(CALL_RE)) {
    const name = m[1] ?? '';
    if (SKIP_CALL_NAMES.has(name) || seen.has(name)) continue;
    seen.add(name);
    const defRe = new RegExp(String.raw`^(?:export\s+)?(?:async\s+)?function\s+${name}\s*\(`, 'm');
    const defMatch = defRe.exec(fileText);
    if (!defMatch) continue;
    const defLine = fileText.slice(0, defMatch.index).split('\n').length;
    blocks.push(extractBlock(fileLines, defLine));
    if (blocks.length >= 8) break;
  }
  return blocks;
}

interface FileCaches {
  text: Map<string, string>;
  lines: Map<string, string[]>;
  blockByLocation: Map<string, string[]>;
}

function newFileCaches(): FileCaches {
  return { text: new Map(), lines: new Map(), blockByLocation: new Map() };
}

/** Extracted block(s) for one bound step's `match.location`, cached globally
 * since many scenarios share a step (e.g. the doorway background step). */
function blocksForLocation(loc: string, caches: FileCaches): string[] {
  const cached = caches.blockByLocation.get(loc);
  if (cached) return cached;
  const [fileRel, lineStr] = loc.split(':');
  const filePath = join(A2O_ROOT, fileRel ?? '');
  const fileText = readTextCached(caches.text, filePath);
  if (fileText === '') {
    caches.blockByLocation.set(loc, []);
    return [];
  }
  const fileLines = caches.lines.get(filePath) ?? fileText.split('\n');
  caches.lines.set(filePath, fileLines);
  const startLine = Number(lineStr ?? '1');
  const primary = extractBlock(fileLines, startLine);
  const hops = MAX_HELPER_HOPS > 0 ? followSameFileHelpers(primary, fileText, fileLines) : [];
  const blocks = [primary, ...hops];
  caches.blockByLocation.set(loc, blocks);
  return blocks;
}

/** Last HTTP path mentioned in this step's text (dataplane.steps.ts's
 * generic `I query "<path>" on peer …` layer means the step ORDER, not just
 * the presence of a path, carries which surface a later `has field` step is
 * about). */
function lastPathIn(text: string): string | null {
  const matches = text.match(HTTP_PATH_RE);
  return matches && matches.length > 0 ? (matches.at(-1) as string) : null;
}

function extractSurfaces(scenario: CensusScenario, caches: FileCaches): Surfaces {
  const surfaces = emptySurfaces();
  let currentPath: string | null = null;
  for (const step of scenario.steps) {
    scanText(step.text, '', surfaces);
    const path = lastPathIn(step.text);
    if (path) currentPath = path;
    for (const field of extractFields(step.text)) {
      surfaces.fieldRefs.push({ path: currentPath, field });
    }
    if (step.bound && step.matchLocation) {
      // Fields are NOT scanned from glue source here (unlike http/metric/env/
      // mesh-action surfaces): step-def JSDoc routinely lists field-name
      // EXAMPLES for a shared parameterized step ("has field {string}") that
      // belong to OTHER scenarios reusing the same glue, and an unassociated
      // field falsely reads as ABSENT the moment it doesn't match whichever
      // sample body happens to be available. Only a field this scenario's own
      // Gherkin text actually asserts is trustworthy enough to report.
      for (const block of blocksForLocation(step.matchLocation, caches))
        scanText(block, block, surfaces);
    }
  }
  return surfaces;
}

// ---------------------------------------------------------------------------
// step 4 — live probing (read-only, GET only)
// ---------------------------------------------------------------------------

type ProbeStatus = 'EXISTS' | 'ABSENT' | 'UNKNOWN';

const DYNAMIC_SEGMENT_RE = /^(?:e2e-|bafk|sha256-)|^[0-9a-f]{16,}$|^\d+$|^.{24,}$/;

/** Truncate a probed path at the first segment that looks dynamic (a hash,
 * CID, uuid-ish token, or a bare e2e-* fixture id) — probe the static prefix. */
function normalizeForProbe(path: string): string {
  const segments = path.split('/');
  const kept: string[] = [];
  for (const seg of segments) {
    if (seg !== '' && DYNAMIC_SEGMENT_RE.test(seg)) break;
    kept.push(seg);
  }
  const normalized = kept.join('/');
  return normalized === '' ? path : normalized;
}

const ABSENT_BODY_RE = /Unknown API route|App not found/i;

/**
 * A 404 only counts as ABSENT when the body carries elohim-storage's own
 * "this route/app doesn't exist" shapes (`Unknown API route`, `App not
 * found` — verified live against elohim-storage's `/api/v1/*` and `/apps/*`
 * catch-alls). A 404 that doesn't match — including doorway's OWN top-level
 * `{"error":"Not Found",...}` for a path it never proxies — is ambiguous
 * (could mean "route absent", could mean "upstream unreachable right now")
 * and stays UNKNOWN rather than guessing either way. Only a genuinely
 * non-404 response counts as EXISTS.
 */
async function probeOneHost(base: string, path: string): Promise<ProbeStatus> {
  try {
    const res = await fetch(base + path, {
      method: 'GET',
      signal: AbortSignal.timeout(PROBE_TIMEOUT_MS),
    });
    if (res.status !== 404) return 'EXISTS';
    const text = await res.text().catch(() => '');
    return ABSENT_BODY_RE.test(text) ? 'ABSENT' : 'UNKNOWN';
  } catch {
    return 'UNKNOWN';
  }
}

interface HttpProbeResult {
  status: ProbeStatus;
  detail: string;
  sampleBody: unknown;
}

async function fetchJsonSafe(url: string): Promise<unknown> {
  try {
    const res = await fetch(url, { signal: AbortSignal.timeout(PROBE_TIMEOUT_MS) });
    return (await res.json()) as unknown;
  } catch {
    return undefined;
  }
}

async function probeHttpPath(rawPath: string): Promise<HttpProbeResult> {
  const path = normalizeForProbe(rawPath);
  const [doorway, storage] = await Promise.all([
    probeOneHost(DOORWAY_URL, path),
    probeOneHost(STORAGE_URL, path),
  ]);
  let status: ProbeStatus = 'UNKNOWN';
  if (doorway === 'EXISTS' || storage === 'EXISTS') status = 'EXISTS';
  else if (doorway === 'ABSENT' && storage === 'ABSENT') status = 'ABSENT';
  const sampleBody = status === 'EXISTS' ? await fetchJsonSafe(`${DOORWAY_URL}${path}`) : undefined;
  return { status, detail: `doorway=${doorway} storage=${storage}`, sampleBody };
}

interface MeshHealth {
  doorway: boolean;
  storage: boolean;
}

/**
 * elohim-storage is the authoritative 404 shape for IMPLEMENT-DESIGN/BOUNDED
 * (`Unknown API route`, `App not found`) — doorway alone can only prove
 * EXISTS, never ABSENT (see `probeOneHost`). A report generated with storage
 * unreachable can under-count IMPLEMENT-DESIGN/IMPLEMENT-BOUNDED; surface
 * that plainly rather than let a silent zero read as "nothing missing".
 */
async function checkMeshHealth(): Promise<MeshHealth> {
  const probe = async (base: string): Promise<boolean> => {
    try {
      const res = await fetch(`${base}/health`, { signal: AbortSignal.timeout(PROBE_TIMEOUT_MS) });
      return res.ok;
    } catch {
      return false;
    }
  };
  const [doorway, storage] = await Promise.all([probe(DOORWAY_URL), probe(STORAGE_URL)]);
  return { doorway, storage };
}

async function fetchMetricsText(base: string): Promise<string | null> {
  try {
    const res = await fetch(`${base}/metrics`, { signal: AbortSignal.timeout(PROBE_TIMEOUT_MS) });
    return res.ok ? await res.text() : null;
  } catch {
    return null;
  }
}

function probeMetric(
  name: string,
  doorwayMetrics: string | null,
  storageMetrics: string | null
): ProbeStatus {
  if (doorwayMetrics?.includes(name) === true || storageMetrics?.includes(name) === true) {
    return 'EXISTS';
  }
  if (doorwayMetrics !== null || storageMetrics !== null) return 'ABSENT';
  return 'UNKNOWN';
}

function hasDotPath(obj: unknown, path: string): boolean {
  const parts = path.replaceAll('[]', '').split('.').filter(Boolean);
  let cur: unknown = obj;
  for (const part of parts) {
    if (Array.isArray(cur)) cur = cur[0];
    if (typeof cur !== 'object' || cur === null) return false;
    if (!(part in (cur as Record<string, unknown>))) return false;
    cur = (cur as Record<string, unknown>)[part];
  }
  return true;
}

interface ProbeCache {
  http: Map<string, HttpProbeResult>;
  metric: Map<string, ProbeStatus>;
  doorwayMetrics: string | null;
  storageMetrics: string | null;
}

async function probeAllSurfaces(all: Surfaces): Promise<ProbeCache> {
  const cache: ProbeCache = {
    http: new Map(),
    metric: new Map(),
    doorwayMetrics: await fetchMetricsText(DOORWAY_URL),
    storageMetrics: await fetchMetricsText(STORAGE_URL),
  };
  const paths = [...all.httpPaths];
  const httpResults = await mapWithConcurrency(paths, 8, probeHttpPath);
  paths.forEach((path, i) => cache.http.set(path, httpResults[i]));
  for (const metric of all.metrics) {
    cache.metric.set(metric, probeMetric(metric, cache.doorwayMetrics, cache.storageMetrics));
  }
  return cache;
}

// ---------------------------------------------------------------------------
// env / fixture grounding
// ---------------------------------------------------------------------------

const EXPORT_VAR_RE = /export\s+([A-Z][A-Z0-9_]*)=/g;
/** Peer names are interpolated dynamically in the prologue script
 * (`E2E_STORAGE_$(echo "$name" | tr ...)`) — declared here since the regex
 * can't see the runtime value. */
const DYNAMIC_PEER_ENV_VARS = ['E2E_STORAGE_MATTHEW', 'E2E_STORAGE_JESSICA', 'E2E_STORAGE_JAMES'];

function declaredEnvVars(caches: FileCaches): Set<string> {
  const declared = new Set<string>(DYNAMIC_PEER_ENV_VARS);
  for (const script of [PROLOGUE_SCRIPT, HC_MESH_SCRIPT]) {
    const text = readTextCached(caches.text, script);
    for (const m of text.matchAll(EXPORT_VAR_RE)) declared.add(m[1]);
  }
  return declared;
}

function loadHouseholdFixture(caches: FileCaches): unknown {
  const text = readTextCached(caches.text, HOUSEHOLD_FIXTURE_PATH);
  if (text === '') return undefined;
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
}

interface EnvGround {
  declared: Set<string>;
  fixtureText: string;
}

function checkEnvVar(name: string, ground: EnvGround): 'present' | 'missing' {
  const currentlySet = process.env[name] !== undefined && process.env[name] !== '';
  const declared = ground.declared.has(name);
  const fixtureMentioned = ground.fixtureText.includes(name);
  return currentlySet || declared || fixtureMentioned ? 'present' : 'missing';
}

// ---------------------------------------------------------------------------
// latest mesh report — pass/fail cross-reference
// ---------------------------------------------------------------------------

interface ReportEntry {
  status: string;
  errorMessage?: string;
}

function scenarioReportEntry(el: RawElement): ReportEntry {
  const steps = (el.steps ?? []).filter(s => !isHookKeyword(s.keyword));
  const failedStep = steps.find(s => s.result?.status === 'failed');
  const status = failedStep
    ? 'failed'
    : (steps.find(s => s.result?.status !== 'passed')?.result?.status ?? 'passed');
  return { status, errorMessage: failedStep?.result?.error_message };
}

function indexReportFeatures(features: RawFeature[]): Map<string, ReportEntry> {
  const byKey = new Map<string, ReportEntry>();
  for (const feature of features) {
    for (const el of feature.elements ?? []) {
      if (el.type !== 'scenario') continue;
      byKey.set(`${feature.uri}::${el.name}`, scenarioReportEntry(el));
    }
  }
  return byKey;
}

/** A report whose EVERY scenario is 'skipped' carries no evidence — it means
 * the harness never actually ran the suite (env block missing, profile
 * misconfigured, wrong working directory), not that every scenario is
 * legitimately gated. wave1-saga.json (2026-08-21) was exactly this: 25/25
 * skipped in 0.2s because `--profile saga` launched without the `just test
 * mesh` env exports. Detected generically (not by filename) so a future
 * mistake under a different name is still caught. */
function isVoidReport(byKey: Map<string, ReportEntry>): boolean {
  return byKey.size > 0 && [...byKey.values()].every(e => e.status === 'skipped');
}

/** Merges EVERY non-empty candidate report — a saga-lane report and a
 * mesh-lane report cover disjoint scenario sets, and both should contribute
 * DEFECT-STALE signal. Candidates are ordered newest/most-specific first, so
 * on a key collision the FIRST report to claim a key wins. */
function loadLatestReport(): { paths: string[]; byKey: Map<string, ReportEntry> } | null {
  const paths: string[] = [];
  const byKey = new Map<string, ReportEntry>();
  for (const path of REPORT_CANDIDATES) {
    if (!existsSync(path) || statSync(path).size === 0) continue;
    const raw = readFileSync(path, 'utf8');
    if (raw.trim() === '') continue;
    const features = JSON.parse(raw) as RawFeature[];
    const reportByKey = indexReportFeatures(features);
    if (isVoidReport(reportByKey)) {
      log(`Report ${path} is VOID (every scenario skipped — harness likely never ran) — ignored.`);
      continue;
    }
    for (const [key, entry] of reportByKey) {
      if (!byKey.has(key)) byKey.set(key, entry);
    }
    paths.push(path);
  }
  return paths.length > 0 ? { paths, byKey } : null;
}

function reportKey(scenario: CensusScenario): string {
  return `${scenario.featureUri}::${scenario.name}`;
}

// ---------------------------------------------------------------------------
// STRUCTURAL — topology-impossibility text scan
// ---------------------------------------------------------------------------

const STRUCTURAL_PATTERNS: { re: RegExp; label: string }[] = [
  {
    re: /@requires:shem\b/,
    label: 'needs shem (multi-tenant commons canvas — no household analog)',
  },
  {
    re: /@requires:alpha-cluster-6peer\b/,
    label: 'needs the 6-peer alpha cluster — household mesh has 3',
  },
  {
    re: /embedded-conductor-only/i,
    label: 'embedded-conductor-only path (household runs external conductors)',
  },
  {
    re: /two.?peer loss/i,
    label: 'two-of-three-peer loss on a 3-peer household (no quorum remains)',
  },
];

function structuralHit(scenario: CensusScenario, rawFeatureText: string): string | null {
  const haystack = `${scenario.tags.join(' ')}\n${rawFeatureText}`;
  for (const { re, label } of STRUCTURAL_PATTERNS) {
    if (re.test(haystack)) return label;
  }
  return null;
}

// ---------------------------------------------------------------------------
// step 5 — classification rubric
// ---------------------------------------------------------------------------

type CensusClass =
  | 'WIRE'
  | 'FIXTURE'
  | 'IMPLEMENT-BOUNDED'
  | 'IMPLEMENT-DESIGN'
  | 'STRUCTURAL'
  | 'DEFECT-STALE'
  | 'UNCLASSIFIED';

interface SurfaceVerdict {
  exists: string[];
  absent: string[];
  unknown: string[];
}

function partitionHttp(surfaces: Surfaces, probes: ProbeCache): SurfaceVerdict {
  const v: SurfaceVerdict = { exists: [], absent: [], unknown: [] };
  for (const path of surfaces.httpPaths) {
    const result = probes.http.get(path);
    if (result?.status === 'EXISTS') v.exists.push(path);
    else if (result?.status === 'ABSENT') v.absent.push(path);
    else v.unknown.push(path);
  }
  return v;
}

function partitionMetrics(surfaces: Surfaces, probes: ProbeCache): SurfaceVerdict {
  const v: SurfaceVerdict = { exists: [], absent: [], unknown: [] };
  for (const metric of surfaces.metrics) {
    const status = probes.metric.get(metric) ?? 'UNKNOWN';
    if (status === 'EXISTS') v.exists.push(metric);
    else if (status === 'ABSENT') v.absent.push(metric);
    else v.unknown.push(metric);
  }
  return v;
}

/** Fields only verify against a sample body already fetched while probing
 * HTTP surfaces — no field is probed as a second GET. A field whose step
 * followed a specific `I query "<path>" …` step checks against THAT path's
 * sample; an unassociated field (from step-def glue, or Gherkin that never
 * named a path) falls back to any sample this scenario's surfaces offer —
 * lower confidence, since it may not be the field's real owning endpoint. */
function partitionFields(surfaces: Surfaces, probes: ProbeCache): SurfaceVerdict {
  const v: SurfaceVerdict = { exists: [], absent: [], unknown: [] };
  // An unassociated field (no preceding `I query "<path>"` step) is only
  // resolved against a sample when the scenario names exactly ONE distinct
  // http surface — unambiguous. With two or more, guessing which one the
  // field belongs to is how a real field reads as falsely ABSENT.
  const distinctPaths = [...surfaces.httpPaths];
  const unambiguousSample =
    distinctPaths.length === 1 ? probes.http.get(distinctPaths[0])?.sampleBody : undefined;
  const seen = new Set<string>();
  for (const ref of surfaces.fieldRefs) {
    const key = `${ref.path ?? ''}::${ref.field}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const sample = ref.path ? probes.http.get(ref.path)?.sampleBody : unambiguousSample;
    if (sample === undefined) v.unknown.push(ref.field);
    else if (hasDotPath(sample, ref.field)) v.exists.push(ref.field);
    else v.absent.push(ref.field);
  }
  return v;
}

interface ClassificationResult {
  cls: CensusClass;
  closesIt: string;
  exists: string[];
  absent: string[];
  unknown: string[];
  effortScore: number;
}

function missingEnvVars(surfaces: Surfaces, ground: EnvGround): string[] {
  return [...surfaces.envVars].filter(v => checkEnvVar(v, ground) === 'missing');
}

function classifyScenario(
  scenario: CensusScenario,
  surfaces: Surfaces,
  probes: ProbeCache,
  ground: EnvGround,
  report: Map<string, ReportEntry> | null,
  rawFeatureText: string
): ClassificationResult {
  const httpV = partitionHttp(surfaces, probes);
  const metricV = partitionMetrics(surfaces, probes);
  const fieldV = partitionFields(surfaces, probes);
  const exists = [...httpV.exists, ...metricV.exists, ...fieldV.exists];
  const absent = [...httpV.absent, ...metricV.absent, ...fieldV.absent];
  const unknown = [...httpV.unknown, ...metricV.unknown, ...fieldV.unknown];

  const topology = structuralHit(scenario, rawFeatureText);
  if (topology) {
    return { cls: 'STRUCTURAL', closesIt: topology, exists, absent, unknown, effortScore: 0 };
  }

  const bind = bindClass(scenario);
  const entry = report?.get(reportKey(scenario));
  const failedHere = entry?.status === 'failed';

  if (httpV.absent.length > 0) {
    return {
      cls: 'IMPLEMENT-DESIGN',
      closesIt: `route(s) absent, needs operator design: ${httpV.absent.join(', ')}`,
      exists,
      absent,
      unknown,
      effortScore: httpV.absent.length,
    };
  }
  const nonHttpAbsent = [...metricV.absent, ...fieldV.absent];
  if (nonHttpAbsent.length > 0) {
    return {
      cls: 'IMPLEMENT-BOUNDED',
      closesIt: `add: ${nonHttpAbsent.join(', ')}`,
      exists,
      absent,
      unknown,
      effortScore: nonHttpAbsent.length,
    };
  }
  const missingEnv = missingEnvVars(surfaces, ground);
  if (missingEnv.length > 0) {
    return {
      cls: 'FIXTURE',
      closesIt: `precondition missing: ${missingEnv.join(', ')}`,
      exists,
      absent,
      unknown,
      effortScore: missingEnv.length,
    };
  }
  if (bind !== 'a') {
    const unbound = scenario.steps.filter(s => !s.bound).length;
    return {
      cls: 'WIRE',
      closesIt: `write step glue (${unbound} step(s) unbound, class ${bind})`,
      exists,
      absent,
      unknown,
      effortScore: unbound,
    };
  }
  if (failedHere) {
    return {
      cls: 'DEFECT-STALE',
      closesIt: (entry?.errorMessage ?? 'failed').split('\n')[0] ?? 'failed',
      exists,
      absent,
      unknown,
      effortScore: 1,
    };
  }
  return {
    cls: 'UNCLASSIFIED',
    closesIt: unclassifiedReason(bind, exists.length > 0, entry, report !== null),
    exists,
    absent,
    unknown,
    effortScore: 99,
  };
}

/** Distinguishes the three ways a scenario reaches the fallback: fully wired
 * with every named surface present but the mesh already reports it passing
 * (the `@wip` tag looks stale), fully wired with no report data to confirm
 * pass/fail, or genuinely no HTTP/metric/field surface named anywhere. */
function unclassifiedReason(
  bind: BindClass,
  hasExists: boolean,
  entry: ReportEntry | undefined,
  hasReport: boolean
): string {
  if (bind === 'a' && hasExists && entry?.status === 'passed') {
    return 'fully wired, named surfaces exist, latest mesh run reported "passed" — the @wip tag looks stale; untag it (or capture the real remaining reason)';
  }
  // A report entry that is neither 'passed' nor 'failed' (cucumber-js also
  // emits 'skipped'/'pending', including the mesh lane's own @requires-gate
  // HELDs) is NOT evidence the scenario is fine — it wasn't exercised. Fall
  // through to the same "no usable verdict" message as no report at all.
  if (bind === 'a' && hasExists && (!hasReport || entry === undefined)) {
    return 'fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)';
  }
  if (bind === 'a' && hasExists && entry !== undefined) {
    return (
      `fully wired, named surfaces exist, but the latest mesh run reported "${entry.status}" ` +
      '(not passed or failed — likely a `@requires:` substrate gate HELD it) — not evidence either way; ' +
      're-run once the gated capability is available'
    );
  }
  return 'insufficient surface signal — no HTTP/metric/field surface named in step text or glue';
}

// ---------------------------------------------------------------------------
// step 6 — report
// ---------------------------------------------------------------------------

interface CensusRow {
  scenario: CensusScenario;
  bind: BindClass;
  result: ClassificationResult;
  reason: 'wip' | 'failed';
}

const CLASS_ORDER: CensusClass[] = [
  'WIRE',
  'FIXTURE',
  'IMPLEMENT-BOUNDED',
  'IMPLEMENT-DESIGN',
  'STRUCTURAL',
  'DEFECT-STALE',
  'UNCLASSIFIED',
];

function renderTotals(rows: CensusRow[]): string[] {
  const lines = ['## Totals', '', '| class | count |', '|---|---|'];
  for (const cls of CLASS_ORDER) {
    lines.push(`| ${cls} | ${rows.filter(r => r.result.cls === cls).length} |`);
  }
  lines.push(`| **total classified** | **${rows.length}** |`);
  return lines;
}

function renderFirstTen(rows: CensusRow[]): string[] {
  const lines = ['## First ten cheapest, per class', ''];
  for (const cls of CLASS_ORDER) {
    const inClass = rows
      .filter(r => r.result.cls === cls)
      .sort((a, b) => a.result.effortScore - b.result.effortScore)
      .slice(0, 10);
    if (inClass.length === 0) continue;
    lines.push(`### ${cls} (${rows.filter(r => r.result.cls === cls).length} total)`, '');
    for (const row of inClass) {
      lines.push(
        `- \`${row.scenario.featureUri}\` — ${row.scenario.name} — ${row.result.closesIt}`
      );
    }
    lines.push('');
  }
  return lines;
}

function surfaceList(names: string[]): string {
  return names.length === 0 ? '—' : names.map(n => `\`${n}\``).join(', ');
}

function renderTable(rows: CensusRow[]): string[] {
  const lines = [
    '## Full census — every `@wip` or failed Act I/host scenario',
    '',
    '| feature | scenario | bind | class | exists | absent | closes it |',
    '|---|---|---|---|---|---|---|',
  ];
  const sorted = [...rows].sort(
    (a, b) =>
      a.scenario.featureUri.localeCompare(b.scenario.featureUri) ||
      a.scenario.name.localeCompare(b.scenario.name)
  );
  for (const row of sorted) {
    lines.push(
      `| \`${row.scenario.featureUri}\` | ${row.scenario.name} | ${row.bind} | ${row.result.cls} | ` +
        `${surfaceList(row.result.exists)} | ${surfaceList(row.result.absent)} | ${row.result.closesIt} |`
    );
  }
  return lines;
}

function renderUnclassifiedNote(rows: CensusRow[]): string[] {
  const unknowns = rows.flatMap(r => r.result.unknown);
  if (unknowns.length === 0) return [];
  const distinct = [...new Set(unknowns)].sort((a, b) => a.localeCompare(b));
  return [
    '## Surfaces the probe could not classify',
    '',
    'Fetch failed or returned neither a recognizable ABSENT shape nor a usable sample —',
    'these need a manual look, not a mechanical one:',
    '',
    ...distinct.map(s => `- \`${s}\``),
    '',
  ];
}

function meshHealthLine(health: MeshHealth): string {
  if (health.doorway && health.storage) return 'Mesh health: doorway UP, storage UP — full probe.';
  return (
    `Mesh health: doorway ${health.doorway ? 'UP' : 'DOWN'}, storage ${health.storage ? 'UP' : 'DOWN'} ` +
    '— **PROVISIONAL RUN**. Storage is the authoritative 404 shape for IMPLEMENT-DESIGN/BOUNDED; with ' +
    'it unreachable, EXISTS-vs-ABSENT for surfaces only doorway answered is under-proven — re-run ' +
    '`pnpm census` once storage answers and treat any IMPLEMENT-DESIGN/IMPLEMENT-BOUNDED count from ' +
    'this run as a floor, not a ceiling.'
  );
}

function writeReport(rows: CensusRow[], reportPaths: string[], health: MeshHealth): void {
  const generatedAt = new Date().toISOString();
  const reportLine =
    reportPaths.length > 0
      ? reportPaths.join(', ')
      : '(none found — no DEFECT-STALE class possible this run)';
  const lines = [
    '# Surface census — mechanical WIRE/FIXTURE/IMPLEMENT/STRUCTURAL/DEFECT classification',
    '',
    `_Generated ${generatedAt} by \`pnpm census\` (\`scripts/surface-census.ts\`). Population: every ` +
      '`@wip` or failed Act I/host scenario, joined against a scoped `cucumber-js --dry-run` bind check ' +
      'and live read-only probes of the surfaces its steps name. Re-run with `pnpm census` (or ' +
      '`pnpm census -- --fresh` to bypass the dry-run cache) — the classifier is read-only against the ' +
      'mesh (GET only) and idempotent._',
    '',
    meshHealthLine(health),
    '',
    `Latest mesh report(s) cross-referenced for DEFECT-STALE: ${reportLine}`,
    '',
    ...renderTotals(rows),
    '',
    ...renderFirstTen(rows),
    ...renderUnclassifiedNote(rows),
    ...renderTable(rows),
    '',
  ];
  mkdirSync(join(A2O_ROOT, 'layering'), { recursive: true });
  writeFileSync(OUT_PATH, lines.join('\n'));
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

function candidateFeatureFiles(reportByKey: Map<string, ReportEntry> | null): string[] {
  const all = discoverFeatureFiles(FEATURES_DIR);
  const failedUris = new Set<string>();
  if (reportByKey) {
    for (const [key, entry] of reportByKey) {
      if (entry.status === 'failed') failedUris.add(key.split('::')[0]);
    }
  }
  const candidates = all.filter(f => {
    const uri = relative(A2O_ROOT, f);
    return grepHasWip(f) || failedUris.has(uri);
  });
  return LIMIT ? candidates.slice(0, LIMIT) : candidates;
}

async function main(): Promise<void> {
  ensureEmptyConfig();

  const latest = loadLatestReport();
  if (latest) log(`Latest report(s): ${latest.paths.join(', ')}`);
  else log('No non-empty latest mesh report found — DEFECT-STALE will not fire this run.');

  const files = candidateFeatureFiles(latest?.byKey ?? null);
  log(`Dry-running ${files.length} feature file(s) (concurrency ${CONCURRENCY})...`);

  const rawFeatureLists = await mapWithConcurrency(files, CONCURRENCY, runDryRun);
  const rawFeatures = rawFeatureLists.flat();
  const rawTextByUri = new Map<string, string>();
  for (const f of files) rawTextByUri.set(relative(A2O_ROOT, f), readFileSync(f, 'utf8'));

  const allScenarios = collectScenarios(rawFeatures);
  const scoped = allScenarios.filter(isInScope);
  const population = scoped.filter(s => {
    const wip = s.tags.includes('@wip');
    const failed = latest?.byKey.get(reportKey(s))?.status === 'failed';
    return wip || failed;
  });
  log(
    `Population (Act I/host, @wip or failed): ${population.length} of ${allScenarios.length} scenarios`
  );

  const fileCaches = newFileCaches();
  const surfacesByScenario = new Map<string, Surfaces>();
  const unionSurfaces = emptySurfaces();
  for (const scenario of population) {
    const s = extractSurfaces(scenario, fileCaches);
    surfacesByScenario.set(scenario.scenarioId, s);
    addAll(unionSurfaces.httpPaths, s.httpPaths);
    addAll(unionSurfaces.metrics, s.metrics);
    unionSurfaces.fieldRefs.push(...s.fieldRefs);
    addAll(unionSurfaces.envVars, s.envVars);
    addAll(unionSurfaces.meshActions, s.meshActions);
  }

  log(
    `Distinct surfaces to probe — http:${unionSurfaces.httpPaths.size} metrics:${unionSurfaces.metrics.size} ` +
      `fields:${unionSurfaces.fieldRefs.length} env:${unionSurfaces.envVars.size}`
  );
  const probes = await probeAllSurfaces(unionSurfaces);
  const health = await checkMeshHealth();
  log(
    `Mesh health: doorway=${health.doorway ? 'up' : 'down'} storage=${health.storage ? 'up' : 'down'}`
  );

  const ground: EnvGround = {
    declared: declaredEnvVars(fileCaches),
    fixtureText: JSON.stringify(loadHouseholdFixture(fileCaches) ?? {}),
  };

  const rows: CensusRow[] = population.map(scenario => {
    const surfaces = surfacesByScenario.get(scenario.scenarioId) as Surfaces;
    const rawFeatureText = rawTextByUri.get(scenario.featureUri) ?? '';
    const result = classifyScenario(
      scenario,
      surfaces,
      probes,
      ground,
      latest?.byKey ?? null,
      rawFeatureText
    );
    return {
      scenario,
      bind: bindClass(scenario),
      result,
      reason: scenario.tags.includes('@wip') ? 'wip' : 'failed',
    };
  });

  writeReport(rows, latest?.paths ?? [], health);
  log(`Wrote ${relative(A2O_ROOT, OUT_PATH)} (${rows.length} rows).`);
}

main().catch(e => {
  console.error(e instanceof Error ? (e.stack ?? e.message) : String(e));
  process.exitCode = 1;
});
