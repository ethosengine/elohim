/**
 * Resource-Exhaustion Shape Report
 *
 * The fourth step of the tune→document→report loop (spec:
 * genesis/docs/superpowers/specs/2026-06-15-node-resource-tunables-and-exhaustion-shape-design.md).
 *
 * Reads the self-heal / resilience backlog entries (genesis/data/timeline/backlog/*.md)
 * and the deterministic CI ledger (.claude/data/ci-findings.jsonl), then aggregates
 * them into a stack-shape view: WHERE on the stack (which node) runaway-resource
 * issues occur, of what CLASS (cpu/memory/db-pool/arc-dht/sessions/disk-pvc/runtime-sched),
 * at what self-heal LIFECYCLE, and whether a tunable limit is DOCUMENTED for that class.
 *
 * It builds net-new infra on nothing: it aggregates existing artifacts. The
 * markdown leads with a node×class matrix (the "shape"). Read-only.
 *
 * Usage:  tsx scripts/build-resource-shape-report.ts [--backlog-dir D] [--ci-findings F] [--out-json J] [--out-md M]
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import * as AjvNs from 'ajv/dist/2020.js';

const AjvCtor: new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default =
  (
    AjvNs as unknown as {
      default: new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default;
    }
  ).default ??
  (AjvNs as unknown as new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default);

// ---------------------------------------------------------------------------
// Resource-class vocabulary + classifier (pure — unit-tested)
// ---------------------------------------------------------------------------

export const RESOURCE_CLASSES = [
  'cpu',
  'memory',
  'db-pool',
  'arc-dht',
  'sessions',
  'disk-pvc',
  'runtime-sched',
] as const;
export type ResourceClass = (typeof RESOURCE_CLASSES)[number];

/** Keyword → class. Matched against the entry's tags + title + "What is exhausted" text. */
const CLASS_KEYWORDS: Record<ResourceClass, string[]> = {
  cpu: ['cpu', 'throttle', 'cfs', 'cpu starvation', 'starve'],
  // Resource-context only — bare "memory" false-matches MemPalace/"memory kit".
  memory: [
    'oom',
    'oomkill',
    'memcg',
    'anon-rss',
    'rss ',
    'memory limit',
    'memory pressure',
    'out of memory',
    'working set',
  ],
  'db-pool': [
    'db-pool',
    'db pool',
    'sqlite',
    'read pool',
    'read-pool',
    'r2d2',
    'connection pool',
    'busy_timeout',
    'pool size',
  ],
  'arc-dht': [
    'arc-factor',
    'target_arc_factor',
    'authority arc',
    'dht',
    'arc shrink',
    'arc-shrink',
  ],
  // Resource-context only — bare "session" false-matches "dev session".
  sessions: [
    'app-ws',
    'app_ws',
    'reconnect',
    'websocket',
    'admin-ws',
    'ws session',
    'session pressure',
  ],
  'disk-pvc': ['pvc', 'openebs', 'jiva', 'no space', 'disk pressure', 'watermark', 'fingerprint'],
  'runtime-sched': [
    'liveness',
    'crash-loop',
    'crashloop',
    'watchdog',
    'startup',
    'serializ',
    'scheduling',
    'admission-shed',
    'shed',
    'backpressure',
    'wedge',
    'firehose',
    'sigkill',
    'self-heal',
  ],
};

/** Documented tunable knobs by class (the "limit" half of the loop). Empty ⇒ no documented lever yet. */
const DOCUMENTED_TUNABLES: Record<ResourceClass, string[]> = {
  cpu: ['deployments.json edgenodeCpuLimit'],
  memory: ['deployments.json edgenodeMemoryLimit'],
  'db-pool': ['STORAGE_DB_POOL_SIZE'],
  'arc-dht': ['ELOHIM_TARGET_ARC_FACTOR (target_arc_factor)'],
  sessions: ['DOORWAY_WORKER_THREADS', 'WORKER_COUNT', 'DOORWAY_MAX_INFLIGHT'],
  'disk-pvc': ['cargo-pool / pool-policy.json (operator)'],
  'runtime-sched': ['WARMUP_PACE_MS', 'DOORWAY_MONGO_OP_TIMEOUT_MS', 'DOORWAY_HEALTH_STALE_MS'],
};

/** Classify a backlog entry into resource classes from its tags + free text. */
export function classifyResource(text: string, tags: string[]): ResourceClass[] {
  const hay = (text + ' ' + tags.join(' ')).toLowerCase();
  const out: ResourceClass[] = [];
  for (const cls of RESOURCE_CLASSES) {
    if (CLASS_KEYWORDS[cls].some(kw => hay.includes(kw))) out.push(cls);
  }
  return out;
}

export function documentedTunablesFor(classes: ResourceClass[]): string[] {
  const out = new Set<string>();
  for (const c of classes) for (const t of DOCUMENTED_TUNABLES[c]) out.add(t);
  return [...out];
}

// ---------------------------------------------------------------------------
// Frontmatter parsing (no YAML dep in a2o — parse the simple `---` block)
// ---------------------------------------------------------------------------

export interface BacklogFrontmatter {
  slug?: string;
  title?: string;
  severity?: string;
  selfHealStatus?: string;
  nodes: string[];
  tags: string[];
  fingerprints: string[];
}

function stripQuotes(s: string): string {
  return s
    .trim()
    .replace(/^["']|["']$/g, '')
    .trim();
}

/** Parse an inline `[a, b, "c"]` array; returns [] for `[]` or absent. */
function parseInlineArray(v: string): string[] {
  const m = /^\[(.*)\]$/s.exec(v);
  if (!m) return [];
  return m[1]
    .split(',')
    .map(x => stripQuotes(x))
    .filter(x => x.length > 0);
}

export function parseFrontmatter(md: string): BacklogFrontmatter | null {
  const m = /^---\n([\s\S]*?)\n---/.exec(md);
  if (!m) return null;
  const fm: BacklogFrontmatter = { nodes: [], tags: [], fingerprints: [] };
  for (const line of m[1].split('\n')) {
    const kv = /^([a-zA-Z_]+):[ \t]*(.*)/.exec(line);
    if (!kv) continue;
    const [, key, rawVal] = kv;
    const val = rawVal.trim();
    switch (key) {
      case 'slug':
        fm.slug = stripQuotes(val);
        break;
      case 'title':
        fm.title = stripQuotes(val);
        break;
      case 'severity':
        fm.severity = stripQuotes(val);
        break;
      case 'self_heal_status':
        fm.selfHealStatus = stripQuotes(val);
        break;
      case 'nodes':
        fm.nodes = parseInlineArray(val);
        break;
      case 'tags':
        fm.tags = parseInlineArray(val);
        break;
      case 'fingerprints':
        fm.fingerprints = parseInlineArray(val);
        break;
      default:
        break;
    }
  }
  return fm;
}

/** Extract the body text under a "## What is exhausted" heading (first paragraph), if present. */
export function exhaustedText(md: string): string | null {
  const m = /^##\s+What is exhausted\s*\n([\s\S]*?)(?:\n##\s|\n*$)/im.exec(md);
  return m ? m[1].trim() : null;
}

// ---------------------------------------------------------------------------
// Aggregation (pure — unit-tested)
// ---------------------------------------------------------------------------

export interface ShapeEntry {
  slug: string;
  title: string;
  severity: string | null;
  selfHealStatus: string | null;
  nodes: string[];
  classes: ResourceClass[];
  fingerprints: string[];
  ciOccurrences: number;
  hasExhaustedSection: boolean;
  documentedTunables: string[];
}

/** Accumulate one node's contribution for a single backlog entry into the running shape maps. */
function accumulateNodeEntry(
  node: string,
  life: string,
  classes: ResourceClass[],
  byNode: Record<
    string,
    { total: number; classes: Record<string, number>; lifecycle: Record<string, number> }
  >,
  cells: Record<string, Record<string, number>>,
  nodeSet: Set<string>
): void {
  nodeSet.add(node);
  byNode[node] ??= { total: 0, classes: {}, lifecycle: {} };
  byNode[node].total += 1;
  byNode[node].lifecycle[life] = (byNode[node].lifecycle[life] ?? 0) + 1;
  cells[node] ??= {};
  for (const c of classes) {
    byNode[node].classes[c] = (byNode[node].classes[c] ?? 0) + 1;
    cells[node][c] = (cells[node][c] ?? 0) + 1;
  }
}

export function buildShape(entries: ShapeEntry[]) {
  const byNode: Record<
    string,
    { total: number; classes: Record<string, number>; lifecycle: Record<string, number> }
  > = {};
  const byClass: Record<
    string,
    {
      total: number;
      nodes: string[];
      documentedTunables: string[];
      lifecycle: Record<string, number>;
    }
  > = {};
  const byLifecycle: Record<string, number> = {};
  const cells: Record<string, Record<string, number>> = {};

  const nodeSet = new Set<string>();
  const classSet = new Set<string>();

  for (const e of entries) {
    const life = e.selfHealStatus ?? 'unstated';
    byLifecycle[life] = (byLifecycle[life] ?? 0) + 1;

    const nodes = e.nodes.length > 0 ? e.nodes : ['(unattributed)'];
    for (const node of nodes) {
      accumulateNodeEntry(node, life, e.classes, byNode, cells, nodeSet);
    }
    for (const c of e.classes) {
      classSet.add(c);
      byClass[c] ??= {
        total: 0,
        nodes: [],
        documentedTunables: DOCUMENTED_TUNABLES[c] ?? [],
        lifecycle: {},
      };
      byClass[c].total += 1;
      byClass[c].lifecycle[life] = (byClass[c].lifecycle[life] ?? 0) + 1;
      for (const node of nodes) if (!byClass[c].nodes.includes(node)) byClass[c].nodes.push(node);
    }
  }

  return {
    byNode,
    byClass,
    byLifecycle,
    matrix: {
      nodes: [...nodeSet].sort((a, b) => a.localeCompare(b)),
      classes: RESOURCE_CLASSES.filter(c => classSet.has(c)),
      cells,
    },
  };
}

// ---------------------------------------------------------------------------
// CI ledger
// ---------------------------------------------------------------------------

interface CiFinding {
  fp: string;
  seen?: number;
  backlog?: string;
}

function loadCiFindings(path: string): CiFinding[] {
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf-8')
    .split('\n')
    .map(l => l.trim())
    .filter(Boolean)
    .map(l => {
      try {
        return JSON.parse(l) as CiFinding;
      } catch {
        return null;
      }
    })
    .filter((x): x is CiFinding => x !== null);
}

// ---------------------------------------------------------------------------
// Build
// ---------------------------------------------------------------------------

export function collectEntries(backlogDir: string, ci: CiFinding[]): ShapeEntry[] {
  if (!existsSync(backlogDir)) return [];
  const entries: ShapeEntry[] = [];
  for (const file of readdirSync(backlogDir).filter(f => f.endsWith('.md'))) {
    const md = readFileSync(join(backlogDir, file), 'utf-8');
    const fm = parseFrontmatter(md);
    if (!fm) continue;
    const slug = fm.slug ?? file.replace(/\.md$/, '');
    const exhausted = exhaustedText(md);
    const hasExhaustedSection = exhausted !== null;
    const classes = classifyResource(`${fm.title ?? ''} ${exhausted ?? ''}`, fm.tags);

    // Only resource/resilience/self-heal entries belong in the shape report.
    if (classes.length === 0 && !fm.selfHealStatus && !hasExhaustedSection) continue;

    const ciForEntry = ci.filter(f => f.backlog === slug || fm.fingerprints.includes(f.fp));
    const ciOccurrences = ciForEntry.reduce((sum, f) => sum + (f.seen ?? 1), 0);

    entries.push({
      slug,
      title: fm.title ?? slug,
      severity: fm.severity ?? null,
      selfHealStatus: fm.selfHealStatus ?? null,
      nodes: fm.nodes,
      classes,
      fingerprints: fm.fingerprints,
      ciOccurrences,
      hasExhaustedSection,
      documentedTunables: documentedTunablesFor(classes),
    });
  }
  return entries.sort((a, b) => a.slug.localeCompare(b.slug));
}

function renderMarkdown(report: ReturnType<typeof assembleReport>): string {
  const { shape, entries, entryCount } = report;
  const L: string[] = [];
  L.push('# Resource-Exhaustion Shape Report', '');
  L.push(
    `Aggregates **${entryCount}** self-heal/resilience backlog entries × the CI ledger into the shape of where runaway-resource issues occur on the stack.`,
    ''
  );

  // Node × class matrix (the "shape").
  L.push('## Shape — node × resource-class', '');
  const cls = shape.matrix.classes;
  if (cls.length === 0 || shape.matrix.nodes.length === 0) {
    L.push('_No node×class incidents recorded._', '');
  } else {
    L.push(`| node | ${cls.join(' | ')} |`);
    L.push(`|------|${cls.map(() => '------').join('|')}|`);
    for (const node of shape.matrix.nodes) {
      const row = cls.map(c =>
        shape.matrix.cells[node]?.[c] ? String(shape.matrix.cells[node][c]) : '·'
      );
      L.push(`| ${node} | ${row.join(' | ')} |`);
    }
    L.push('');
  }

  // Per-class, with documented-tunable annotation (the loop-closing axis).
  L.push('## By resource class — documented tunable vs gap', '');
  for (const c of cls) {
    const info = shape.byClass[c];
    const knob =
      info.documentedTunables.length > 0
        ? info.documentedTunables.join(', ')
        : '⚠ NO DOCUMENTED TUNABLE';
    L.push(`- **${c}** — ${info.total} incident(s) on ${info.nodes.join(', ')}; tunable: ${knob}`);
  }
  L.push('');

  // Lifecycle.
  L.push('## By self-heal lifecycle', '');
  for (const [k, v] of Object.entries(shape.byLifecycle)) L.push(`- ${k}: ${v}`);
  L.push('');

  // Entry detail.
  L.push('## Entries', '');
  for (const e of entries) {
    L.push(
      `- **${e.slug}** [${e.classes.join(', ') || 'unclassified'}] — ${e.severity ?? '?'} / ${e.selfHealStatus ?? 'unstated'}; nodes: ${e.nodes.join(', ') || '(unattributed)'}; CI occurrences: ${e.ciOccurrences}`
    );
  }
  L.push('');
  return L.join('\n');
}

export function assembleReport(backlogDir: string, ciFindingsPath: string) {
  const ci = loadCiFindings(ciFindingsPath);
  const entries = collectEntries(backlogDir, ci);
  return {
    generatedFrom: { backlogDir, ciFindings: ciFindingsPath },
    entryCount: entries.length,
    resourceClasses: [...RESOURCE_CLASSES],
    entries,
    shape: buildShape(entries),
  };
}

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const isCli =
  process.argv[1]?.endsWith('build-resource-shape-report.ts') ||
  process.argv[1]?.endsWith('build-resource-shape-report.js');

if (isCli) {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  const repoRoot = join(scriptDir, '..', '..', '..');
  const opts = new Map<string, string>();
  for (let i = 2; i < process.argv.length - 1; i += 2)
    opts.set(process.argv[i], process.argv[i + 1]);

  const backlogDir = opts.get('--backlog-dir') ?? join(repoRoot, 'genesis/data/timeline/backlog');
  const ciFindings = opts.get('--ci-findings') ?? join(repoRoot, '.claude/data/ci-findings.jsonl');
  const reportsDir = opts.get('--reports-dir') ?? join(scriptDir, '..', 'reports');
  const outJson = opts.get('--out-json') ?? join(reportsDir, 'resource-shape.json');
  const outMd = opts.get('--out-md') ?? join(reportsDir, 'resource-shape.md');
  const schemaPath = join(scriptDir, '..', 'schemas', 'resource-shape-report.schema.json');

  const report = assembleReport(backlogDir, ciFindings);

  // Schema-validate before writing (parity with build-sprint-report).
  const ajv = new AjvCtor({ strict: false, allErrors: true });
  const validate = ajv.compile(JSON.parse(readFileSync(schemaPath, 'utf-8')));
  if (!validate(report)) {
    console.error('resource-shape report failed schema validation:');
    console.error(JSON.stringify(validate.errors, null, 2));
    process.exit(1);
  }

  if (!existsSync(reportsDir)) mkdirSync(reportsDir, { recursive: true });
  writeFileSync(outJson, JSON.stringify(report, null, 2));
  writeFileSync(outMd, renderMarkdown(report));

  console.log(`Resource-shape report: ${report.entryCount} entries → ${outJson} + ${outMd}`);
  const undocumented = report.shape.matrix.classes.filter(
    c => (report.shape.byClass[c]?.documentedTunables.length ?? 0) === 0
  );
  if (undocumented.length > 0) {
    console.log(`⚠ resource classes WITHOUT a documented tunable: ${undocumented.join(', ')}`);
  }
}
