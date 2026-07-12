#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readdir, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { basename, dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import {
  HOOK_KIND,
  hookPackageFromSource,
  projectHook,
  verifyHookPackage,
} from './hook-package.mjs';
import {
  AGENT_DOC_KIND,
  agentDocPackageFromSource,
  frontmatterScalar,
  projectAgentDoc,
  runtimeForDoc,
  verifyAgentDocPackage,
} from './agent-doc-packages.mjs';
import {
  COMMAND_KIND,
  commandPackageFromSource,
  projectCommand,
  verifyCommandPackage,
} from './command-packages.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DOMAIN_DIR = resolve(__dirname, '..');
const REPO_ROOT = resolve(DOMAIN_DIR, '../../../..');
const EPR_META_DIR = resolve(REPO_ROOT, '.epr-meta');
const PACKAGE_DIR = resolve(EPR_META_DIR, 'elohim/packages');
const PROJECTION_DIR = resolve(EPR_META_DIR, 'elohim/projections');

// Lodging surface (A2d): the seam where a rogue/non-compliant capability's
// governance drift is recorded. Mirrors the findings-ledger contract used by
// .claude/scripts/ci-harvest.py + runtime-harvest.py (deterministic
// fingerprint, append-only JSONL, small cursor for dedup) — see
// .claude/data/ci-findings.jsonl / .claude/data/runtime-findings.jsonl for
// the sibling shapes this one was copied from.
const GOVERNANCE_LEDGER_REL = '.claude/data/governance-findings.jsonl';
const GOVERNANCE_LEDGER_PATH = resolve(REPO_ROOT, GOVERNANCE_LEDGER_REL);
const GOVERNANCE_CURSOR_PATH = resolve(REPO_ROOT, '.claude/data/governance-cursor.json');

const SKILL_SOURCE_DIR = resolve(REPO_ROOT, '.claude/skills');
const AGENT_SOURCE_DIR = resolve(REPO_ROOT, '.claude/agents');
const HOOK_SOURCE_DIR = resolve(REPO_ROOT, '.claude/hooks');
const COMMAND_SOURCE_DIR = resolve(REPO_ROOT, '.claude/commands');
// Registration surface for hooks. READ-ONLY here: recorded into the package and
// reconciled against, NEVER auto-written (a bad settings.json write can wedge
// the whole PreToolUse gating toolchain).
const SETTINGS_PATH = resolve(REPO_ROOT, '.claude/settings.json');

const args = process.argv.slice(2);
const command = args.find((arg) => !arg.startsWith('-')) ?? 'verify';
const WRITE_FIXTURES = args.includes('--write-fixtures') || args.includes('--write-projections');
const WRITE_RUNTIME = args.includes('--write-runtime');
const LEGACY_WRITE = command === 'verify' && WRITE_FIXTURES;

// `--only <Kind:name|name>` scopes a `project` write to a subset of packages —
// repeatable (`--only a --only b`) and/or comma-separated (`--only a,b`), and a
// value may be a bare id (`librarian`) or a kind-qualified id (`AgentPackage:librarian`).
// This exists because `project --write-runtime` otherwise rewrites EVERY package's
// runtime surface (a whole-tree clobber hazard — it can overwrite an in-flight
// authored file). A plant/edit uses `--only` to regenerate ONLY its target.
// Absent `--only`, behavior is unchanged (all packages).
function parseOnly(argv) {
  const out = [];
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--only') {
      const value = argv[i + 1];
      if (value && !value.startsWith('-')) {
        out.push(...value.split(',').map((v) => v.trim()).filter(Boolean));
        i++;
      }
    } else if (arg.startsWith('--only=')) {
      out.push(...arg.slice('--only='.length).split(',').map((v) => v.trim()).filter(Boolean));
    }
  }
  return out;
}
const ONLY = parseOnly(args);
const DRY_RUN = args.includes('--dry-run');

// Value-bearing flag reader (`--id foo` or `--id=foo`); null when absent.
function flagValue(argv, flag) {
  const idx = argv.indexOf(flag);
  if (idx !== -1 && argv[idx + 1] && !argv[idx + 1].startsWith('-')) return argv[idx + 1];
  const eq = argv.find((a) => a.startsWith(`${flag}=`));
  return eq ? eq.slice(flag.length + 1) : null;
}

// Positional (non-flag) tokens, skipping the values consumed by value-bearing
// flags (`--only`, `--id`) so `adopt-doc path/to/CLAUDE.md --id foo` reads
// `path/to/CLAUDE.md` as the sole argument, not `foo`.
function positionalArgs(argv) {
  const out = [];
  const valueFlags = new Set(['--only', '--id']);
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg.startsWith('-')) {
      if (valueFlags.has(arg) && argv[i + 1] && !argv[i + 1].startsWith('-')) i++;
      continue;
    }
    out.push(arg);
  }
  return out;
}

// Filter a package list to those matching an `--only` selection. A package matches
// a bare id (`pkg.metadata.id`) or a kind-qualified token (`${pkg.kind}:${id}`).
// Empty selection ⇒ identity (all packages), so the default path is untouched.
function selectOnly(packages, only) {
  if (!only || only.length === 0) return packages;
  const set = new Set(only);
  return packages.filter(
    (pkg) => set.has(pkg.metadata.id) || set.has(`${pkg.kind}:${pkg.metadata.id}`),
  );
}

let failures = 0;
let passes = 0;

function fail(message) {
  console.error(`FAIL: ${message}`);
  failures++;
}

function pass(message) {
  console.log(`PASS: ${message}`);
  passes++;
}

function assert(condition, message) {
  if (condition) pass(message);
  else fail(message);
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'));
}

async function writeText(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, value, 'utf8');
}

async function writeJson(path, value) {
  await writeText(path, `${JSON.stringify(value, null, 2)}\n`);
}

async function readIfExists(path) {
  try {
    return await readFile(path, 'utf8');
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}

async function readSettings() {
  const raw = await readIfExists(SETTINGS_PATH);
  if (!raw) return {};
  try {
    return JSON.parse(raw);
  } catch {
    return {};
  }
}

async function listJsonFiles(dir) {
  try {
    return (await readdir(dir)).filter((name) => name.endsWith('.json')).sort();
  } catch (error) {
    if (error?.code === 'ENOENT') return [];
    throw error;
  }
}

function parseMarkdownSurface(path, raw) {
  if (!raw.startsWith('---\n')) {
    throw new Error(`${path} does not start with YAML frontmatter`);
  }
  const close = raw.indexOf('\n---\n', 4);
  if (close === -1) {
    throw new Error(`${path} does not close YAML frontmatter`);
  }
  const frontmatterRaw = raw.slice(4, close);
  const body = raw.slice(close + 5);
  return { frontmatterRaw, frontmatter: parseFrontmatter(frontmatterRaw), body };
}

function parseFrontmatter(raw) {
  const lines = raw.split('\n');
  const root = {};

  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    if (!line.trim()) continue;
    if (line.startsWith(' ')) continue;

    const match = line.match(/^([A-Za-z0-9_-]+):(.*)$/);
    if (!match) {
      throw new Error(`Unsupported frontmatter line: ${line}`);
    }

    const [, key, rest] = match;
    if (rest.trim()) {
      root[key] = parseScalar(rest.trim());
      continue;
    }

    const block = [];
    while (
      index + 1 < lines.length &&
      (lines[index + 1].startsWith(' ') || !lines[index + 1].trim())
    ) {
      index++;
      if (lines[index].trim()) block.push(lines[index]);
    }
    root[key] = parseIndentedBlock(block);
  }

  return root;
}

function parseIndentedBlock(lines) {
  if (lines.length === 0) return {};
  if (lines.every((line) => line.startsWith('  - '))) {
    return parseListBlock(lines);
  }

  const out = {};
  for (const line of lines) {
    const match = line.match(/^  ([A-Za-z0-9_-]+):(.*)$/);
    if (!match) continue;
    out[match[1]] = parseScalar(match[2].trim());
  }
  return out;
}

function parseListBlock(lines) {
  const list = [];
  for (let index = 0; index < lines.length; index++) {
    const item = lines[index].match(/^  - ([A-Za-z0-9_-]+):(.*)$/);
    if (!item) continue;
    const [, name, rest] = item;
    if (rest.trim()) {
      list.push(parseScalar(rest.trim()));
      continue;
    }
    const value = {};
    while (index + 1 < lines.length && lines[index + 1].startsWith('      ')) {
      index++;
      const field = lines[index].match(/^      ([A-Za-z0-9_-]+):(.*)$/);
      if (field) value[field[1]] = parseScalar(field[2].trim());
    }
    list.push({ [name]: value });
  }
  return list;
}

function parseScalar(value) {
  if (value === '') return '';
  if (
    (value.startsWith('"') && value.endsWith('"')) ||
    (value.startsWith("'") && value.endsWith("'"))
  ) {
    return value.slice(1, -1);
  }
  if (value === 'true') return true;
  if (value === 'false') return false;
  return value;
}

function toolRefsFrom(frontmatter) {
  const tools = frontmatter.tools;
  if (!tools || typeof tools !== 'string') return [];
  return tools
    .split(',')
    .map((tool) => tool.trim())
    .filter(Boolean);
}

function mcpServerNames(mcpServers) {
  if (!Array.isArray(mcpServers)) return [];
  return mcpServers.flatMap((entry) => Object.keys(entry));
}

// Structured, LOSSLESS capture of a `.claude` agent's `mcpServers:` frontmatter
// block. The generic frontmatter parser is lossy for this shape (it drops the
// nested `args:` list), so mcp-bearing agents could not be flipped without
// losing their MCP wiring — the v1 STOP rule. This dedicated parse preserves the
// full shape: an ordered list of `{ name, config }` where each config value is a
// scalar (e.g. `command`, `type`, `url`) or a list (e.g. `args`), key order kept.
// Its inverse is `stringifyMcpServers`; parse(emit(x)) === x is asserted at verify.
// The two real corpus shapes are mempalace (`command` + `args:[…]`) and the
// http/sse servers (`type` + `url`), both round-tripped by this pair.
function parseMcpServersBlock(frontmatterRaw) {
  if (typeof frontmatterRaw !== 'string') return [];
  const lines = frontmatterRaw.split('\n');
  const start = lines.findIndex((line) => line === 'mcpServers:');
  if (start === -1) return [];
  const servers = [];
  let current = null;
  let listField = null;
  for (let i = start + 1; i < lines.length; i++) {
    const line = lines[i];
    if (!line.startsWith(' ')) break; // a column-0 line closes the block
    const server = line.match(/^ {2}- ([A-Za-z0-9_./-]+):\s*$/);
    if (server) {
      current = { name: server[1], config: {} };
      servers.push(current);
      listField = null;
      continue;
    }
    if (!current) continue;
    const item = line.match(/^ {8}- (.*)$/);
    if (item && listField) {
      current.config[listField].push(item[1]);
      continue;
    }
    const field = line.match(/^ {6}([A-Za-z0-9_./-]+):(.*)$/);
    if (field) {
      const rest = field[2].trim();
      if (rest === '') {
        current.config[field[1]] = [];
        listField = field[1];
      } else {
        current.config[field[1]] = rest;
        listField = null;
      }
    }
  }
  return servers;
}

// Emit the canonical `.claude` `mcpServers:` block BODY (the indented lines under
// the `mcpServers:` header, which the caller prepends) from the structured form,
// byte-for-byte reproducing the authored shape: `  - name:` / `      key: scalar`
// / `      key:` + `        - item`. Fixed 2/6/8-space indentation matches the
// corpus convention (the block is always top-level frontmatter).
function stringifyMcpServers(servers) {
  let out = '';
  for (const { name, config } of servers) {
    out += `  - ${name}:\n`;
    for (const [key, value] of Object.entries(config)) {
      if (Array.isArray(value)) {
        out += `      ${key}:\n`;
        for (const item of value) out += `        - ${item}\n`;
      } else {
        out += `      ${key}: ${value}\n`;
      }
    }
  }
  return out;
}

function governanceFor(kind, id) {
  return {
    eprRef: `epr:elohim-agent/${kind}/${id}`, // offline floor anchor; resolves to earned trust when the substrate is reachable
    policy: 'capability-governance@1',
    gates: ['epr-meta-resolver', 'elohim-agent:packages:verify'],
    ledger: GOVERNANCE_LEDGER_REL,
  };
}

// ── Lodging: dedup-guarded append to the governance-findings ledger ──

function governanceFingerprint(kind, id, assertionClass) {
  // Stable/deterministic: same drift (same package, same failing assertion
  // class) always yields the same fingerprint, so it never re-fires once
  // lodged — the whole point of the cursor dedup.
  const norm = `${kind}:${id}:${assertionClass}`.toLowerCase();
  return createHash('sha256').update(norm).digest('hex').slice(0, 12);
}

async function loadGovernanceCursor() {
  const raw = await readIfExists(GOVERNANCE_CURSOR_PATH);
  if (!raw) return { run: 0, known: {} };
  try {
    const parsed = JSON.parse(raw);
    return { run: typeof parsed.run === 'number' ? parsed.run : 0, known: parsed.known ?? {} };
  } catch {
    return { run: 0, known: {} };
  }
}

async function writeGovernanceCursor(cursor) {
  await writeJson(GOVERNANCE_CURSOR_PATH, cursor);
}

async function appendGovernanceLedgerLine(entry) {
  await mkdir(dirname(GOVERNANCE_LEDGER_PATH), { recursive: true });
  const existing = (await readIfExists(GOVERNANCE_LEDGER_PATH)) ?? '';
  await writeFile(GOVERNANCE_LEDGER_PATH, `${existing}${JSON.stringify(entry)}\n`, 'utf8');
}

async function lodgeGovernanceFinding({ fingerprint, kind, id, detail }) {
  const cursor = await loadGovernanceCursor();
  cursor.run += 1;
  const known = cursor.known[fingerprint];
  if (known && known.status === 'open') {
    // Already lodged and still open: bump recurrence bookkeeping only —
    // never append a duplicate line to the ledger.
    known.last_run = cursor.run;
    known.seen = (known.seen ?? 1) + 1;
    await writeGovernanceCursor(cursor);
    return { lodged: false, fingerprint };
  }

  const ts = new Date().toISOString();
  await appendGovernanceLedgerLine({
    ts,
    fp: fingerprint,
    class: 'governance-non-compliance',
    kind,
    id,
    detail,
    status: 'open',
    first_seen: ts,
    seen: 1,
  });
  cursor.known[fingerprint] = { status: 'open', first_run: cursor.run, last_run: cursor.run, seen: 1 };
  await writeGovernanceCursor(cursor);
  return { lodged: true, fingerprint };
}

function skillPackageFromClaude(path, parsed) {
  const name = parsed.frontmatter.name;
  const version = parsed.frontmatter.metadata?.version ?? '1.0.0';
  const description = parsed.frontmatter.description;
  const governance = governanceFor('skills', name);

  return {
    apiVersion: 'elohim-agent/v1alpha1',
    kind: 'SkillPackage',
    metadata: {
      id: name,
      name,
      version,
      description,
      triggerDescription: description,
      runtimeTargets: ['claude', 'codex'],
      sourceRuntime: 'claude',
      assetRefs: [],
      author: parsed.frontmatter.metadata?.author,
      governance,
    },
    instructions: {
      format: 'markdown',
      body: parsed.body,
    },
    projections: {
      claude: {
        path: relative(REPO_ROOT, path),
        frontmatter: parsed.frontmatter,
        frontmatterRaw: parsed.frontmatterRaw,
      },
      codex: {
        path: `.codex/skills/${name}/SKILL.md`,
        frontmatter: codexFrontmatter({
          name,
          description,
          packageKind: 'SkillPackage',
          sourcePath: relative(REPO_ROOT, path),
          sourceRuntime: 'claude',
          governance,
        }),
      },
    },
  };
}

function agentPackageFromClaude(path, parsed) {
  const name = parsed.frontmatter.name;
  const description = parsed.frontmatter.description;
  const model = parsed.frontmatter.model;
  const color = parsed.frontmatter.color;
  const toolRefs = toolRefsFrom(parsed.frontmatter);
  const mcpServersStructured = parseMcpServersBlock(parsed.frontmatterRaw);
  const governance = governanceFor('agents', name);

  return {
    apiVersion: 'elohim-agent/v1alpha1',
    kind: 'AgentPackage',
    metadata: {
      id: name,
      name,
      version: '1.0.0',
      description,
      role: name,
      modelHints: {
        claudeModel: model,
        claudeColor: color,
      },
      capabilityRefs: [],
      toolRefs,
      sourceRuntime: 'claude',
      mcpServerRefs: mcpServerNames(parsed.frontmatter.mcpServers),
      // Structured, lossless MCP wiring (server config, incl. the `args:` list the
      // generic parser drops), captured from the raw frontmatter so an mcp-bearing
      // agent can be flipped without losing its MCP block. Absent when the agent
      // carries no `mcpServers:` block.
      ...(mcpServersStructured.length ? { mcpServers: mcpServersStructured } : {}),
      governance,
    },
    instructions: {
      format: 'markdown',
      body: parsed.body,
    },
    projections: {
      claude: {
        path: relative(REPO_ROOT, path),
        frontmatter: parsed.frontmatter,
        frontmatterRaw: parsed.frontmatterRaw,
      },
      codex: {
        path: `.codex/agents/${name}.md`,
        frontmatter: codexFrontmatter({
          name,
          description,
          packageKind: 'AgentPackage',
          sourcePath: relative(REPO_ROOT, path),
          sourceRuntime: 'claude',
          model,
          tools: toolRefs,
          governance,
        }),
      },
    },
  };
}

function codexFrontmatter({
  name,
  description,
  packageKind,
  sourcePath,
  sourceRuntime,
  model,
  tools,
  governance,
}) {
  const metadata = {
    runtime: 'codex',
    sourceRuntime,
    sourcePath,
    packageKind,
  };
  const frontmatter = { name, description, metadata };
  if (model) frontmatter.model = model;
  if (tools?.length) frontmatter.tools = tools.join(', ');
  if (governance) frontmatter.governance = governance.eprRef;
  return frontmatter;
}

function projectClaude(pkg) {
  return projectMarkdownSurface(pkg, 'claude');
}

function projectCodex(pkg) {
  return projectMarkdownSurface(pkg, 'codex');
}

// Generated Claude frontmatter for a package-master (FLIPPED) skill/agent.
// Origin is preserved (metadata.sourceRuntime stays 'claude' — "born from
// Claude"); authority is flipped (metadata.master: 'package'). The governance
// eprRef rides along as a backref line so the generated `.claude` surface still
// points home to its package governance anchor. Mirrors codexFrontmatter's
// lowering of package metadata into a runtime frontmatter dialect.
function claudeFrontmatterFromPackage(pkg) {
  const { name, description, sourceRuntime, master, governance } = pkg.metadata;
  const metadata = { sourceRuntime, master };
  if (governance?.eprRef) metadata.governance = governance.eprRef;
  const frontmatter = { name, description };
  // Agents carry a LOAD-BEARING execution contract that skills do not: Claude
  // Code reads top-level `tools:` to scope the subagent's tool access, `model:`
  // to pick its model, and `color:` for its UI tint. Reconstruct that contract
  // from package metadata on the flip — dropping it would silently promote a
  // locked-down agent to default tools + default model (capability escalation).
  // Kept flat (matching the un-flipped agent frontmatter) and placed before the
  // nested `metadata:` block so the flip diff is additive: the original
  // tools/model/color lines stay put, `metadata:` is appended. Skills have none
  // of this metadata, so the three fields are naturally omitted for them.
  if (pkg.kind === 'AgentPackage') {
    const toolRefs = pkg.metadata.toolRefs ?? [];
    const modelHints = pkg.metadata.modelHints ?? {};
    if (toolRefs.length) frontmatter.tools = toolRefs.join(', ');
    // Reconstruct the MCP wiring block from structured metadata, placed between
    // `tools:` and `model:` to match the authored corpus order (so the flip diff
    // stays additive). `stringifyYaml` renders the `mcpServers` key via
    // `stringifyMcpServers` (byte-identical to the source block). Absent for
    // mcp-less agents. This is what lifts the v1 mcp-less STOP rule.
    const mcpServers = pkg.metadata.mcpServers ?? [];
    if (mcpServers.length) frontmatter.mcpServers = mcpServers;
    if (modelHints.claudeModel) frontmatter.model = modelHints.claudeModel;
    if (modelHints.claudeColor) frontmatter.color = modelHints.claudeColor;
  }
  frontmatter.metadata = metadata;
  return frontmatter;
}

// Generated Codex frontmatter for a package-master (FLIPPED) skill/agent — the
// codex backend at parity with the claude one. Carries `master: 'package'` and
// re-roots provenance: `sourcePath` points at the PACKAGE (the authoritative
// root), NOT the stale `.claude` source it was born from. Identity (name) never
// forks per runtime.
function codexFrontmatterFromPackage(pkg) {
  const { name, description, sourceRuntime, master, governance } = pkg.metadata;
  const dir = pkg.kind === 'SkillPackage' ? 'skills' : 'agents';
  const metadata = {
    runtime: 'codex',
    sourceRuntime,
    master,
    sourcePath: `.epr-meta/elohim/packages/${dir}/${pkg.metadata.id}.json`,
    packageKind: pkg.kind,
  };
  const frontmatter = { name, description, metadata };
  // Re-emit the agent execution contract for codex too, at parity with the
  // un-flipped `codexFrontmatter` path (which already emits model + tools). The
  // flip path had regressed relative to it and dropped both. Codex carries no
  // `color` on either path — keep it claude-only.
  if (pkg.kind === 'AgentPackage') {
    const toolRefs = pkg.metadata.toolRefs ?? [];
    const claudeModel = pkg.metadata.modelHints?.claudeModel;
    if (claudeModel) frontmatter.model = claudeModel;
    if (toolRefs.length) frontmatter.tools = toolRefs.join(', ');
  }
  if (governance?.eprRef) frontmatter.governance = governance.eprRef;
  return frontmatter;
}

function projectMarkdownSurface(pkg, runtime) {
  const projection = pkg.projections[runtime];
  // Per-runtime "compiler backend" seam: each runtime lowers package metadata
  // into its own frontmatter dialect. The DEFAULT backend is identity /
  // passthrough — a Claude-sourced package keeps its human-authored frontmatter
  // verbatim (frontmatterRaw) so import(source) round-trips byte-identically and
  // the fidelity gate stays green. A package that has FLIPPED to package-master
  // (metadata.master === 'package') no longer has an authoritative `.claude`
  // source, so its Claude frontmatter is GENERATED from package metadata instead
  // of read from stale frontmatterRaw. The codex backend does the same via
  // codexFrontmatterFromPackage, so BOTH projections reflect the package root.
  let frontmatter;
  if (runtime === 'claude' && pkg.metadata.master === 'package') {
    frontmatter = stringifyYaml(claudeFrontmatterFromPackage(pkg));
  } else if (runtime === 'codex' && pkg.metadata.master === 'package') {
    frontmatter = stringifyYaml(codexFrontmatterFromPackage(pkg));
  } else if (projection.frontmatterRaw) {
    frontmatter = `${projection.frontmatterRaw}\n`;
  } else {
    frontmatter = stringifyYaml(projection.frontmatter);
  }
  return `---\n${frontmatter}---\n${pkg.instructions.body}`;
}

function stringifyYaml(value, indent = '') {
  let out = '';
  for (const [key, field] of Object.entries(value)) {
    // The `mcpServers` block is a list-of-single-key-maps shape the generic
    // scalar/object serializer cannot render; delegate to the dedicated emitter.
    // Always top-level frontmatter, so its fixed 2/6/8-space indentation is correct.
    if (key === 'mcpServers' && Array.isArray(field)) {
      out += `${indent}${key}:\n${stringifyMcpServers(field)}`;
      continue;
    }
    if (field && typeof field === 'object' && !Array.isArray(field)) {
      out += `${indent}${key}:\n${stringifyYaml(field, `${indent}  `)}`;
    } else {
      out += `${indent}${key}: ${formatYamlScalar(field)}\n`;
    }
  }
  return out;
}

function formatYamlScalar(value) {
  if (typeof value === 'string') {
    if (/[:#\n]|^\s|\s$/.test(value)) return JSON.stringify(value);
    return value;
  }
  return JSON.stringify(value);
}

// Authority marker: a `.claude` source is NOT re-imported as a Claude-sourced
// package when the PACKAGE is already the authoritative master for it. Two
// distinct cases, one predicate:
//  - native packages (metadata.sourceRuntime === 'elohim-agent'): born in the
//    package; `.claude` is a pure projection with no human-authored master.
//  - FLIPPED packages (metadata.master === 'package'): born FROM Claude
//    (metadata.sourceRuntime stays 'claude' — origin preserved) but authority
//    has moved to the package. Re-importing from the generated `.claude` would
//    overwrite the master, so it must be skipped. The generated `.claude`
//    frontmatter of a flipped skill carries `master: package`, making this
//    detectable at the source surface itself.
function isPackageAuthoritative(frontmatter) {
  const meta = frontmatter?.metadata ?? {};
  return meta.sourceRuntime === 'elohim-agent' || meta.master === 'package';
}

async function loadSourcePackages({ skillDir = SKILL_SOURCE_DIR, agentDir = AGENT_SOURCE_DIR } = {}) {
  const sourcePackages = [];
  let skillDirs = [];
  try {
    skillDirs = (await readdir(skillDir, { withFileTypes: true }))
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort();
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  for (const dir of skillDirs) {
    const path = resolve(skillDir, dir, 'SKILL.md');
    const raw = await readIfExists(path);
    if (raw) {
      const parsed = parseMarkdownSurface(path, raw);
      if (!isPackageAuthoritative(parsed.frontmatter)) {
        sourcePackages.push(skillPackageFromClaude(path, parsed));
      }
    }
  }

  let agentFiles = [];
  try {
    agentFiles = (await readdir(agentDir)).filter((name) => name.endsWith('.md')).sort();
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  for (const file of agentFiles) {
    const path = resolve(agentDir, file);
    const raw = await readFile(path, 'utf8');
    const parsed = parseMarkdownSurface(path, raw);
    if (!isPackageAuthoritative(parsed.frontmatter)) {
      sourcePackages.push(agentPackageFromClaude(path, parsed));
    }
  }

  for (const pkg of await loadSourceHooks()) {
    sourcePackages.push(pkg);
  }

  for (const pkg of await loadSourceDocs()) {
    sourcePackages.push(pkg);
  }

  for (const pkg of await loadSourceCommands()) {
    sourcePackages.push(pkg);
  }

  return sourcePackages;
}

// Command source loading is readdir-driven (like skills/agents, NOT the opt-in
// package-driven hook/doc path) so first adoption works with no packages present.
// Commands mostly have no frontmatter, so an inline authority marker is
// unreliable; instead we read the existing command packages and SKIP any that is
// package-first (native or FLIPPED master: package) — re-importing its source
// would clobber the package authority. Un-planted commands (the default here) are
// imported source-fidelity and guarded by verifySourceFidelity.
async function loadSourceCommands() {
  let files = [];
  try {
    files = (await readdir(COMMAND_SOURCE_DIR)).filter((name) => name.endsWith('.md')).sort();
  } catch (error) {
    if (error?.code === 'ENOENT') return [];
    throw error;
  }
  if (files.length === 0) return [];

  // Ids whose existing package has flipped/native authority — never re-import.
  const cmdPkgDir = resolve(PACKAGE_DIR, 'commands');
  const planted = new Set();
  for (const file of await listJsonFiles(cmdPkgDir)) {
    const pkg = await readJson(resolve(cmdPkgDir, file));
    if (pkg.kind === COMMAND_KIND && (pkg.metadata.sourceRuntime !== 'claude' || pkg.metadata.master === 'package')) {
      planted.add(pkg.metadata.id);
    }
  }

  const out = [];
  for (const file of files) {
    const srcPath = resolve(COMMAND_SOURCE_DIR, file);
    const raw = await readIfExists(srcPath);
    if (raw === null) continue;
    const id = basename(file, '.md');
    if (planted.has(id)) continue;
    out.push(
      commandPackageFromSource(srcPath, raw, {
        repoRoot: REPO_ROOT,
        id,
        governance: governanceFor('commands', id),
      }),
    );
  }
  return out;
}

// Hook source loading is OPT-IN and package-driven — the divergence from the
// markdown path. There are ~28 hook `.py` files but only ADOPTED ones (those
// with a package under .epr-meta/elohim/packages/hooks/) are treated as source
// packages. This keeps planting scope-contained (plant ONE, don't blow scope to
// all hooks) AND keeps the imported-source-coverage gate honest: it never
// demands a package for an un-adopted hook.
//
// A `.py` cannot carry an inline authority marker (rule 3), so authority is
// read from the PACKAGE: a native (sourceRuntime elohim-agent) or FLIPPED
// (master: package) hook is package-first and is NOT re-imported from source —
// its freshness is proved package-first via project(package) === `.claude/hooks`
// (verifyRuntimeProjectionIfPresent), not the human-source fidelity gate.
async function loadSourceHooks() {
  const hookPkgDir = resolve(PACKAGE_DIR, 'hooks');
  const files = await listJsonFiles(hookPkgDir);
  if (files.length === 0) return [];
  const settings = await readSettings();
  const out = [];
  for (const file of files) {
    const pkg = await readJson(resolve(hookPkgDir, file));
    if (pkg.kind !== HOOK_KIND) continue;
    if (pkg.metadata.sourceRuntime !== 'claude' || pkg.metadata.master === 'package') continue;
    const srcPath = resolve(REPO_ROOT, pkg.projections.claude.path);
    const raw = await readIfExists(srcPath);
    if (raw === null) continue;
    out.push(
      hookPackageFromSource(srcPath, raw, settings, {
        repoRoot: REPO_ROOT,
        governance: governanceFor('hooks', pkg.metadata.id),
      }),
    );
  }
  return out;
}

// Agent-doc source loading is OPT-IN and package-driven — mirroring the hook
// path, and deliberately NOT a bulk readdir of all 142 CLAUDE.mds. Only ADOPTED
// docs (those with a package under .epr-meta/elohim/packages/agentdocs/) are
// treated as source packages. This keeps planting scope-contained (plant ONE
// gospel doc, prove it byte-identical, then the next) AND keeps the
// imported-source-coverage gate honest — it never demands a package for an
// un-adopted doc.
//
// Authority is read from the PACKAGE (a doc's frontmatter is owned by cite-gen
// and must never carry an authority marker): a native (sourceRuntime
// elohim-agent) or FLIPPED (master: package) agent-doc is package-first and is
// NOT re-imported from source — its freshness is proved package-first via
// project(package) === the doc file (verifyRuntimeProjectionIfPresent), byte-for-
// byte, NOT the human-source fidelity gate. An un-flipped adopted doc IS
// re-imported (source stays master) and guarded by verifySourceFidelity.
async function loadSourceDocs() {
  const docPkgDir = resolve(PACKAGE_DIR, 'agentdocs');
  const files = await listJsonFiles(docPkgDir);
  if (files.length === 0) return [];
  const out = [];
  for (const file of files) {
    const pkg = await readJson(resolve(docPkgDir, file));
    if (pkg.kind !== AGENT_DOC_KIND) continue;
    if (pkg.metadata.sourceRuntime === 'elohim-agent' || pkg.metadata.master === 'package') continue;
    const srcPath = resolve(REPO_ROOT, pkg.source.path);
    const raw = await readIfExists(srcPath);
    if (raw === null) continue;
    out.push(
      agentDocPackageFromSource(srcPath, raw, {
        repoRoot: REPO_ROOT,
        id: pkg.metadata.id,
        governance: governanceFor('agentdocs', pkg.metadata.id),
        composition: pkg.composition,
        composedBy: pkg.metadata.composedBy,
      }),
    );
  }
  return out;
}

function packagePathFor(pkg) {
  if (pkg.kind === HOOK_KIND) return resolve(PACKAGE_DIR, 'hooks', `${pkg.metadata.id}.json`);
  if (pkg.kind === AGENT_DOC_KIND) return resolve(PACKAGE_DIR, 'agentdocs', `${pkg.metadata.id}.json`);
  if (pkg.kind === COMMAND_KIND) return resolve(PACKAGE_DIR, 'commands', `${pkg.metadata.id}.json`);
  return pkg.kind === 'SkillPackage'
    ? resolve(PACKAGE_DIR, 'skills', `${pkg.metadata.id}.json`)
    : resolve(PACKAGE_DIR, 'agents', `${pkg.metadata.id}.json`);
}

function projectionFixturePathsFor(pkg) {
  // A hook has a SINGLE projected artifact (the code) and no codex target — the
  // returned object has only a `claude` key, and the write/verify loops iterate
  // Object.keys(...) so they naturally do the right thing for one or two runtimes.
  if (pkg.kind === HOOK_KIND) {
    return { claude: resolve(PROJECTION_DIR, 'claude/hooks', `${pkg.metadata.id}.py`) };
  }
  if (pkg.kind === AGENT_DOC_KIND) {
    // An agent-doc has ONE projection for the plant case (its native runtime), but
    // MAY carry a derived-runtime projection too (a claude gospel that also emits a
    // codex AGENTS.md). Iterate every declared projection so both fixtures are
    // written/verified; a single-projection doc naturally yields one entry.
    const out = {};
    for (const runtime of Object.keys(pkg.projections)) {
      const base = basename(pkg.projections[runtime].path);
      out[runtime] = resolve(PROJECTION_DIR, runtime, 'agentdocs', pkg.metadata.id, base);
    }
    return out;
  }
  if (pkg.kind === COMMAND_KIND) {
    return {
      claude: resolve(PROJECTION_DIR, 'claude/commands', `${pkg.metadata.id}.md`),
      codex: resolve(PROJECTION_DIR, 'codex/commands', `${pkg.metadata.id}.md`),
    };
  }
  return pkg.kind === 'SkillPackage'
    ? {
        claude: resolve(PROJECTION_DIR, 'claude/skills', pkg.metadata.id, 'SKILL.md'),
        codex: resolve(PROJECTION_DIR, 'codex/skills', pkg.metadata.id, 'SKILL.md'),
      }
    : {
        claude: resolve(PROJECTION_DIR, 'claude/agents', `${pkg.metadata.id}.md`),
        codex: resolve(PROJECTION_DIR, 'codex/agents', `${pkg.metadata.id}.md`),
      };
}

function runtimePathsFor(pkg) {
  if (pkg.kind === HOOK_KIND) {
    return { claude: resolve(REPO_ROOT, pkg.projections.claude.path) };
  }
  if (pkg.kind === AGENT_DOC_KIND) {
    const out = {};
    for (const runtime of Object.keys(pkg.projections)) {
      out[runtime] = resolve(REPO_ROOT, pkg.projections[runtime].path);
    }
    return out;
  }
  // CommandPackage and the markdown skill/agent packages both carry claude+codex
  // projection paths; the generic two-runtime return covers them.
  return {
    claude: resolve(REPO_ROOT, pkg.projections.claude.path),
    codex: resolve(REPO_ROOT, pkg.projections.codex.path),
  };
}

function projectedTextFor(pkg, runtime) {
  // Hooks project VERBATIM (byte-for-byte passthrough) and are runtime-agnostic.
  if (pkg.kind === HOOK_KIND) return projectHook(pkg);
  // Agent-docs project VERBATIM for their NATIVE runtime — the ENTIRE raw file
  // (frontmatter incl. cite envelopes + body) emitted unchanged, so the flip never
  // rewrites a gospel doc. A DERIVED runtime (codex AGENTS.md from a claude gospel)
  // gets the generated governance preamble + verbatim body; `runtime` selects.
  if (pkg.kind === AGENT_DOC_KIND) return projectAgentDoc(pkg, runtime);
  // Commands project VERBATIM (byte-for-byte), the same body to both the claude
  // source home and the codex mirror.
  if (pkg.kind === COMMAND_KIND) return projectCommand(pkg);
  return runtime === 'claude' ? projectClaude(pkg) : projectCodex(pkg);
}

async function writePackages(packages) {
  for (const pkg of packages) {
    await writeJson(packagePathFor(pkg), pkg);
  }
}

async function writeProjectionFixtures(packages) {
  for (const pkg of packages) {
    const paths = projectionFixturePathsFor(pkg);
    for (const runtime of Object.keys(paths)) {
      await writeText(paths[runtime], projectedTextFor(pkg, runtime));
    }
  }
}

async function writeRuntimeProjections(packages) {
  for (const pkg of packages) {
    const paths = runtimePathsFor(pkg);
    for (const runtime of Object.keys(paths)) {
      await writeText(paths[runtime], projectedTextFor(pkg, runtime));
    }
  }
}

async function initLayout() {
  const manifest = resolve(EPR_META_DIR, 'manifest.md');
  await mkdir(resolve(PACKAGE_DIR, 'skills'), { recursive: true });
  await mkdir(resolve(PACKAGE_DIR, 'agents'), { recursive: true });
  await mkdir(resolve(PACKAGE_DIR, 'hooks'), { recursive: true });
  await mkdir(resolve(PACKAGE_DIR, 'agentdocs'), { recursive: true });
  await mkdir(resolve(PACKAGE_DIR, 'commands'), { recursive: true });
  await mkdir(resolve(PROJECTION_DIR, 'claude'), { recursive: true });
  await mkdir(resolve(PROJECTION_DIR, 'codex'), { recursive: true });
  try {
    await readFile(manifest, 'utf8');
  } catch {
    await writeText(
      manifest,
      `---\nepr-meta-version: 1\nid: repo-root-governance\nroot: true\n---\n\n# repo root governance\n`,
    );
  }
  pass(`initialized elohim package layout under ${relative(REPO_ROOT, EPR_META_DIR)}`);
}

async function loadPackageFixtures() {
  const skillsDir = resolve(PACKAGE_DIR, 'skills');
  const agentsDir = resolve(PACKAGE_DIR, 'agents');
  const hooksDir = resolve(PACKAGE_DIR, 'hooks');
  const agentDocsDir = resolve(PACKAGE_DIR, 'agentdocs');
  const commandsDir = resolve(PACKAGE_DIR, 'commands');
  const skillFiles = await listJsonFiles(skillsDir);
  const agentFiles = await listJsonFiles(agentsDir);
  const hookFiles = await listJsonFiles(hooksDir);
  const agentDocFiles = await listJsonFiles(agentDocsDir);
  const commandFiles = await listJsonFiles(commandsDir);
  return [
    ...(await Promise.all(skillFiles.map((file) => readJson(resolve(skillsDir, file))))),
    ...(await Promise.all(agentFiles.map((file) => readJson(resolve(agentsDir, file))))),
    ...(await Promise.all(hookFiles.map((file) => readJson(resolve(hooksDir, file))))),
    ...(await Promise.all(agentDocFiles.map((file) => readJson(resolve(agentDocsDir, file))))),
    ...(await Promise.all(commandFiles.map((file) => readJson(resolve(commandsDir, file))))),
  ];
}

async function loadValidators() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  const skillSchema = await readJson(resolve(DOMAIN_DIR, 'schemas/skill-package.schema.json'));
  const agentSchema = await readJson(resolve(DOMAIN_DIR, 'schemas/agent-package.schema.json'));
  const hookSchema = await readJson(resolve(DOMAIN_DIR, 'schemas/hook-package.schema.json'));
  const agentDocSchema = await readJson(
    resolve(DOMAIN_DIR, 'schemas/agent-doc-package.schema.json'),
  );
  const commandSchema = await readJson(
    resolve(DOMAIN_DIR, 'schemas/command-package.schema.json'),
  );
  return {
    skill: ajv.compile(skillSchema),
    agent: ajv.compile(agentSchema),
    hook: ajv.compile(hookSchema),
    agentdoc: ajv.compile(agentDocSchema),
    command: ajv.compile(commandSchema),
  };
}

async function verifyPackage(pkg, validators, settings) {
  const validate =
    pkg.kind === 'SkillPackage'
      ? validators.skill
      : pkg.kind === HOOK_KIND
        ? validators.hook
        : pkg.kind === AGENT_DOC_KIND
          ? validators.agentdoc
          : pkg.kind === COMMAND_KIND
            ? validators.command
            : validators.agent;
  assert(
    validate(pkg),
    `${pkg.kind} package ${pkg.metadata.id} validates: ${JSON.stringify(validate.errors)}`,
  );

  assert(pkg.metadata.id === pkg.metadata.name, `${pkg.metadata.id} name/id round-trip`);

  if (pkg.kind === HOOK_KIND) {
    // Hooks are code + registration, not markdown + frontmatter: skip every
    // markdown-only assertion (frontmatter description, agent tool round-trip,
    // codex governance backref). The hook-specific gates live in the module.
    await verifyHookPackage(pkg, {
      assert,
      settings,
      lodge: ({ assertionClass, detail }) =>
        lodgeGovernanceFinding({
          fingerprint: governanceFingerprint(pkg.kind, pkg.metadata.id, assertionClass),
          kind: pkg.kind,
          id: pkg.metadata.id,
          detail,
        }),
    });
    assert(
      Boolean(pkg.metadata.governance?.eprRef),
      `${pkg.metadata.id} has metadata.governance.eprRef`,
    );
    // Byte-identity: the projected fixture and the runtime `.py` must equal
    // source.body exactly (strict `===`, transform-free). Claude-only for hooks.
    await verifyProjectionFixture(pkg, 'claude');
    await verifyRuntimeProjectionIfPresent(pkg, 'claude');
    return;
  }

  if (pkg.kind === AGENT_DOC_KIND) {
    // Agent-docs are GOSPEL markdown, not markdown+frontmatter-package: skip
    // every markdown-package-only assertion (frontmatter description round-trip,
    // agent tool round-trip, codex governance backref). The agent-doc-specific
    // gates live in the module. The doc-plant floor is BYTE-IDENTITY — the
    // projected fixture and the runtime doc must equal source.body EXACTLY
    // (strict `===`, transform-free, verbatim frontmatter incl. cite envelopes),
    // proving the flip never rewrote a single byte of the gospel doc.
    verifyAgentDocPackage(pkg, { assert });
    assert(
      Boolean(pkg.metadata.governance?.eprRef),
      `${pkg.metadata.id} has metadata.governance.eprRef`,
    );
    // Byte-identity for EVERY declared projection: the native-runtime projection is
    // verbatim source.body; a derived codex projection is preamble + source.body.
    for (const runtime of Object.keys(pkg.projections)) {
      await verifyProjectionFixture(pkg, runtime);
      await verifyRuntimeProjectionIfPresent(pkg, runtime);
    }
    return;
  }

  if (pkg.kind === COMMAND_KIND) {
    // Commands are VERBATIM markdown, not markdown+frontmatter-package: skip the
    // markdown-package-only assertions. The byte-identity floor is that both the
    // claude source home and the codex mirror equal source.body exactly (proved by
    // verifyProjectionFixture / verifyRuntimeProjectionIfPresent), and the claude
    // source round-trip by verifySourceFidelity.
    verifyCommandPackage(pkg, { assert });
    assert(
      Boolean(pkg.metadata.governance?.eprRef),
      `${pkg.metadata.id} has metadata.governance.eprRef`,
    );
    await verifyProjectionFixture(pkg, 'claude');
    await verifyProjectionFixture(pkg, 'codex');
    await verifyRuntimeProjectionIfPresent(pkg, 'claude');
    await verifyRuntimeProjectionIfPresent(pkg, 'codex');
    return;
  }

  assert(pkg.instructions.body.length > 0, `${pkg.metadata.id} has canonical instruction body`);
  assert(
    pkg.projections.claude.frontmatter.description === pkg.metadata.description,
    `${pkg.metadata.id} Claude projection description matches package metadata`,
  );
  assert(
    pkg.projections.codex.frontmatter.description === pkg.metadata.description,
    `${pkg.metadata.id} Codex projection description matches package metadata`,
  );

  if (pkg.kind === 'AgentPackage' && pkg.metadata.sourceRuntime === 'claude') {
    const claudeTools = toolRefsFrom(pkg.projections.claude.frontmatter);
    assert(
      JSON.stringify(claudeTools) === JSON.stringify(pkg.metadata.toolRefs),
      `${pkg.metadata.id} tool metadata round-trip`,
    );
  }

  if (pkg.kind === 'AgentPackage' && pkg.metadata.master === 'package') {
    // MCP-fidelity guard (supersedes the v1 mcp-less STOP rule). Structured
    // `metadata.mcpServers` now preserves the full server config, so a flipped
    // agent MAY carry an mcpServers block — provided the flip loses NOTHING. If
    // the agent has MCP wiring, prove: (1) the structured capture is present, and
    // (2) it round-trips through the emitter byte-for-byte (parse(emit(x)) === x),
    // and (3) the GENERATED flip frontmatter actually carries the reconstructed
    // block. A regression that drops the emitter fails (2)/(3) here.
    const rawFm = pkg.projections?.claude?.frontmatterRaw ?? '';
    const mcpServers = pkg.metadata.mcpServers ?? [];
    const rawHasMcp = /(^|\n)mcpServers:/.test(rawFm);
    if (rawHasMcp || mcpServers.length) {
      assert(
        mcpServers.length > 0,
        `${pkg.metadata.id} flipped agent with MCP wiring has structured metadata.mcpServers (not dropped)`,
      );
      const emitted = stringifyMcpServers(mcpServers);
      const reparsed = parseMcpServersBlock(`mcpServers:\n${emitted}model: x\n`);
      assert(
        JSON.stringify(reparsed) === JSON.stringify(mcpServers),
        `${pkg.metadata.id} mcpServers structured round-trip (parse(emit(metadata.mcpServers)) === metadata.mcpServers)`,
      );
      assert(
        stringifyYaml(claudeFrontmatterFromPackage(pkg)).includes(`mcpServers:\n${emitted}`),
        `${pkg.metadata.id} generated .claude frontmatter carries the reconstructed mcpServers block (flip preserves MCP wiring)`,
      );
    }

    // Contract round-trip: freshness (project(package) === file) is tautological
    // because the file was regenerated from the package. Prove capability
    // preservation instead — assert the GENERATED flip frontmatter reconstructs
    // the execution contract (tools/model/color) from package metadata, not the
    // stored import-time JSON.
    const genClaude = parseFrontmatter(stringifyYaml(claudeFrontmatterFromPackage(pkg)));
    assert(
      JSON.stringify(toolRefsFrom(genClaude)) === JSON.stringify(pkg.metadata.toolRefs ?? []),
      `${pkg.metadata.id} generated .claude tools contract round-trip (flip preserves toolRefs)`,
    );
    assert(
      genClaude.model === pkg.metadata.modelHints?.claudeModel,
      `${pkg.metadata.id} generated .claude model contract round-trip (flip preserves claudeModel)`,
    );
    assert(
      genClaude.color === pkg.metadata.modelHints?.claudeColor,
      `${pkg.metadata.id} generated .claude color contract round-trip (flip preserves claudeColor)`,
    );
    const genCodex = parseFrontmatter(stringifyYaml(codexFrontmatterFromPackage(pkg)));
    assert(
      JSON.stringify(toolRefsFrom(genCodex)) === JSON.stringify(pkg.metadata.toolRefs ?? []),
      `${pkg.metadata.id} generated .codex tools contract round-trip (flip preserves toolRefs)`,
    );
    assert(
      genCodex.model === pkg.metadata.modelHints?.claudeModel,
      `${pkg.metadata.id} generated .codex model contract round-trip (flip preserves claudeModel)`,
    );
  }

  assert(
    Boolean(pkg.metadata.governance?.eprRef),
    `${pkg.metadata.id} has metadata.governance.eprRef`,
  );

  await verifyProjectionFixture(pkg, 'claude');
  await verifyProjectionFixture(pkg, 'codex');
  await verifyRuntimeProjectionIfPresent(pkg, 'claude');
  await verifyRuntimeProjectionIfPresent(pkg, 'codex');
  await verifyGovernanceBackref(pkg);
}

async function verifyGovernanceBackref(pkg) {
  const eprRef = pkg.metadata.governance?.eprRef;
  if (!eprRef) return;
  const paths = projectionFixturePathsFor(pkg);
  const codexFixture = await readIfExists(paths.codex);
  const ok = typeof codexFixture === 'string' && codexFixture.includes(eprRef);
  assert(
    ok,
    `${pkg.metadata.id} governance backref matches package: codex projection fixture contains ${eprRef}`,
  );
  if (!ok) {
    await lodgeGovernanceFinding({
      fingerprint: governanceFingerprint(pkg.kind, pkg.metadata.id, 'governance-backref'),
      kind: pkg.kind,
      id: pkg.metadata.id,
      detail:
        `governance backref missing/stale: codex projection fixture ` +
        `${relative(REPO_ROOT, paths.codex)} does not contain expected eprRef ${eprRef}`,
    });
  }
}

async function verifyProjectionFixture(pkg, runtime) {
  const paths = projectionFixturePathsFor(pkg);
  await verifyProjection(
    paths[runtime],
    projectedTextFor(pkg, runtime),
    `projection fixture is fresh: ${relative(REPO_ROOT, paths[runtime])}`,
    `stale projection fixture: ${relative(REPO_ROOT, paths[runtime])} (run pnpm run elohim-agent:packages:write)`,
    `missing expected projection fixture: ${relative(REPO_ROOT, paths[runtime])} (run pnpm run elohim-agent:packages:write)`,
  );
}

async function verifyRuntimeProjectionIfPresent(pkg, runtime) {
  const paths = runtimePathsFor(pkg);
  const actual = await readIfExists(paths[runtime]);
  if (actual === null) {
    pass(`runtime projection absent; skipped: ${relative(REPO_ROOT, paths[runtime])}`);
    return;
  }
  const expected = projectedTextFor(pkg, runtime);
  assert(actual === expected, `runtime projection is fresh: ${relative(REPO_ROOT, paths[runtime])}`);
  if (actual !== expected) {
    fail(
      `stale runtime projection: ${relative(REPO_ROOT, paths[runtime])} ` +
        `(run node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-runtime)`,
    );
  }
}

async function verifyProjection(path, expected, passMessage, staleMessage, missingMessage) {
  const actual = await readIfExists(path);
  if (actual === null) {
    fail(missingMessage);
    return;
  }
  assert(actual === expected, passMessage);
  if (actual !== expected) {
    fail(staleMessage);
  }
}

async function verifySourceFidelity(sourcePackages) {
  // Round-trip fidelity floor: project(import(source)) === source, byte-for-byte.
  for (const pkg of sourcePackages) {
    if (pkg.metadata.sourceRuntime !== 'claude') continue;
    const sourcePath = resolve(REPO_ROOT, pkg.projections.claude.path);
    const original = await readFile(sourcePath, 'utf8');
    // projectedTextFor routes a HookPackage to the verbatim passthrough
    // projector (strict byte-identity), and skill/agent to the markdown surface.
    const ok = projectedTextFor(pkg, 'claude') === original;
    assert(ok, `fidelity: project(import(${pkg.kind}:${pkg.metadata.id})) === source`);
    if (!ok) {
      await lodgeGovernanceFinding({
        fingerprint: governanceFingerprint(pkg.kind, pkg.metadata.id, 'fidelity'),
        kind: pkg.kind,
        id: pkg.metadata.id,
        detail:
          `source fidelity drift: project(import(...)) !== source ` +
          `(${relative(REPO_ROOT, sourcePath)})`,
      });
    }
  }
}

function verifyImportedSourceCoverage(sourcePackages, packageFixtures) {
  const sourceIds = new Set(sourcePackages.map((pkg) => `${pkg.kind}:${pkg.metadata.id}`));
  const fixtureIds = new Set(packageFixtures.map((pkg) => `${pkg.kind}:${pkg.metadata.id}`));

  for (const id of sourceIds) {
    assert(fixtureIds.has(id), `imported package exists for ${id}`);
  }
  for (const pkg of packageFixtures) {
    // Package-first packages do not require a re-importable Claude source:
    //  - native (sourceRuntime !== 'claude'), and
    //  - FLIPPED (master === 'package') — origin stays 'claude' but authority
    //    is the package, and loadSourcePackages deliberately skips re-importing
    //    its generated `.claude`, so it is absent from sourceIds by design.
    // Its freshness is proved package-first via project(package) === `.claude`
    // (verifyRuntimeProjectionIfPresent), NOT the human-source fidelity gate.
    if (pkg.metadata.sourceRuntime !== 'claude' || pkg.metadata.master === 'package') {
      pass(`package-first; does not require re-importable Claude source: ${pkg.kind}:${pkg.metadata.id}`);
      continue;
    }
    assert(sourceIds.has(`${pkg.kind}:${pkg.metadata.id}`), `Claude-sourced package still has Claude source ${pkg.kind}:${pkg.metadata.id}`);
  }
}

async function runImport({ writeProjections }) {
  const sourcePackages = await loadSourcePackages();
  assert(sourcePackages.length > 0, 'found Claude source packages to import');
  await writePackages(sourcePackages);
  if (writeProjections) {
    await writeProjectionFixtures(sourcePackages);
  }
  pass(
    writeProjections
      ? 'imported Claude packages and wrote projection fixtures'
      : 'imported Claude packages',
  );
}

async function runProject({ writeFixtures, writeRuntime, only = [] }) {
  const packages = selectOnly(await loadPackageFixtures(), only);
  assert(
    packages.length > 0,
    only.length ? `--only matched at least one package: ${only.join(', ')}` : 'found elohim packages to project',
  );
  if (writeFixtures) {
    await writeProjectionFixtures(packages);
    pass('wrote package-derived projection fixtures');
  }
  if (writeRuntime) {
    await writeRuntimeProjections(packages);
    pass('wrote package-derived runtime projections');
  }
  if (!writeFixtures && !writeRuntime) {
    for (const pkg of packages) {
      pass(`projection is derivable from package ${pkg.kind}:${pkg.metadata.id}`);
    }
  }
}

// The authority class of a package, for the verify accounting line:
//  - native         : born in the package (sourceRuntime elohim-agent), no runtime source.
//  - package-first  : born from a runtime but FLIPPED (master: package) — authority is the package.
//  - source-fidelity: un-flipped runtime-sourced — proved by verifySourceFidelity
//                     (project(import(source)) === source).
function packageClass(pkg) {
  if (pkg.metadata.sourceRuntime === 'elohim-agent') return 'native';
  return pkg.metadata.master === 'package' ? 'package-first' : 'source-fidelity';
}

async function runVerify() {
  const sourcePackages = await loadSourcePackages();
  const packageFixtures = await loadPackageFixtures();
  assert(packageFixtures.length > 0, 'loads elohim package fixtures');
  verifyImportedSourceCoverage(sourcePackages, packageFixtures);
  await verifySourceFidelity(sourcePackages);

  const validators = await loadValidators();
  const settings = await readSettings();
  for (const pkg of packageFixtures) {
    await verifyPackage(pkg, validators, settings);
  }

  // Per-class accounting so a shifting check total self-explains (a plant moves a
  // package from source-fidelity → package-first; an adopt adds one; etc.).
  const classes = { 'package-first': 0, 'source-fidelity': 0, native: 0 };
  for (const pkg of packageFixtures) classes[packageClass(pkg)] += 1;
  console.log(
    `\n${packageFixtures.length} packages: ${classes['package-first']} package-first, ` +
      `${classes['source-fidelity']} source-fidelity, ${classes.native} native`,
  );
}

// ── Synthetic self-test for the package-master flip machinery ──
// Proves the three flip behaviours against SYNTHETIC fixtures only. It never
// reads or writes the real `.claude` / `.codex` / `.epr-meta` tree: the
// filesystem leg builds a throwaway `.claude/skills` tree under the OS temp
// dir; the projector/verify legs run purely in memory. Uses the same
// assert/pass/fail harness, so a broken invariant fails this command too.
async function runSelfTest() {
  const sandbox = await mkdtemp(resolve(tmpdir(), 'epr-flip-selftest-'));
  try {
    const body = '# Synthetic Flip\n\nSynthetic skill body for the flip self-test.\n';
    const eprRef = 'epr:elohim-agent/skills/synthetic-flip';
    const description = 'Synthetic flipped skill for the package-master self-test.';

    // (a) loadSourcePackages skips a flipped `.claude` source, still imports a
    //     normal one. Build a throwaway `.claude/skills` tree in the sandbox.
    const tmpSkills = resolve(sandbox, '.claude/skills');
    const tmpAgents = resolve(sandbox, '.claude/agents');
    await mkdir(tmpAgents, { recursive: true });
    await mkdir(resolve(tmpSkills, 'synthetic-flip'), { recursive: true });
    await mkdir(resolve(tmpSkills, 'synthetic-normal'), { recursive: true });
    await writeFile(
      resolve(tmpSkills, 'synthetic-flip/SKILL.md'),
      `---\nname: synthetic-flip\ndescription: ${description}\nmetadata:\n  sourceRuntime: claude\n  master: package\n---\n${body}`,
      'utf8',
    );
    await writeFile(
      resolve(tmpSkills, 'synthetic-normal/SKILL.md'),
      `---\nname: synthetic-normal\ndescription: A normal claude skill.\n---\n${body}`,
      'utf8',
    );
    const imported = await loadSourcePackages({ skillDir: tmpSkills, agentDir: tmpAgents });
    const importedIds = new Set(imported.map((pkg) => `${pkg.kind}:${pkg.metadata.id}`));
    assert(
      importedIds.has('SkillPackage:synthetic-normal'),
      'selftest(a): loadSourcePackages imports a normal claude skill',
    );
    assert(
      !importedIds.has('SkillPackage:synthetic-flip'),
      'selftest(a): loadSourcePackages SKIPS a flipped (master: package) claude skill',
    );

    // (b) The projector GENERATES the flipped `.claude` frontmatter from package
    //     metadata (not stale frontmatterRaw), and non-flipped stays passthrough.
    const staleMarker = 'STALE-HUMAN-AUTHORED-DO-NOT-EMIT';
    const flippedPkg = {
      apiVersion: 'elohim-agent/v1alpha1',
      kind: 'SkillPackage',
      metadata: {
        id: 'synthetic-flip',
        name: 'synthetic-flip',
        version: '1.0.0',
        description,
        triggerDescription: description,
        runtimeTargets: ['claude', 'codex'],
        sourceRuntime: 'claude', // origin preserved — born from Claude
        master: 'package', // authority flipped to package-first
        assetRefs: [],
        governance: governanceFor('skills', 'synthetic-flip'),
      },
      instructions: { format: 'markdown', body },
      projections: {
        claude: {
          path: '.claude/skills/synthetic-flip/SKILL.md',
          frontmatter: { name: 'synthetic-flip', description },
          // A deliberately-stale human-authored frontmatter that MUST be ignored
          // now that the package is master.
          frontmatterRaw: `name: synthetic-flip\ndescription: ${staleMarker}`,
        },
        codex: {
          path: '.codex/skills/synthetic-flip/SKILL.md',
          frontmatter: codexFrontmatter({
            name: 'synthetic-flip',
            description,
            packageKind: 'SkillPackage',
            sourcePath: '.claude/skills/synthetic-flip/SKILL.md',
            sourceRuntime: 'claude',
            governance: governanceFor('skills', 'synthetic-flip'),
          }),
        },
      },
    };
    const flippedClaude = projectClaude(flippedPkg);
    assert(
      flippedClaude.includes('master: package'),
      'selftest(b): generated flipped .claude carries metadata.master: package',
    );
    assert(
      flippedClaude.includes('sourceRuntime: claude'),
      'selftest(b): generated flipped .claude preserves metadata.sourceRuntime: claude',
    );
    assert(
      flippedClaude.includes(eprRef),
      'selftest(b): generated flipped .claude carries the governance eprRef backref',
    );
    assert(
      flippedClaude.includes(body),
      'selftest(b): generated flipped .claude preserves the instruction body',
    );
    assert(
      !flippedClaude.includes(staleMarker),
      'selftest(b): flipped .claude IGNORES stale frontmatterRaw (generated, not passthrough)',
    );

    const normalBody = '# Normal\n\nnormal body\n';
    const normalRawFm = 'name: synthetic-normal\ndescription: A normal claude skill.';
    const normalPkg = {
      kind: 'SkillPackage',
      metadata: { id: 'synthetic-normal', sourceRuntime: 'claude' },
      instructions: { format: 'markdown', body: normalBody },
      projections: {
        claude: { path: '.claude/skills/synthetic-normal/SKILL.md', frontmatterRaw: normalRawFm },
        codex: { path: '.codex/skills/synthetic-normal/SKILL.md', frontmatter: {} },
      },
    };
    assert(
      projectClaude(normalPkg) === `---\n${normalRawFm}\n---\n${normalBody}`,
      'selftest(b): non-flipped .claude uses verbatim frontmatterRaw passthrough (fidelity preserved)',
    );

    // (c) verifyImportedSourceCoverage treats a flipped package as package-first:
    //     absent from imported sources (skipped) must NOT raise a coverage
    //     failure. Failure-count delta of 0 is the proof.
    const normalCoverage = {
      kind: 'SkillPackage',
      metadata: { id: 'synthetic-normal', sourceRuntime: 'claude' },
    };
    const flippedCoverage = {
      kind: 'SkillPackage',
      metadata: { id: 'synthetic-flip', sourceRuntime: 'claude', master: 'package' },
    };
    const failuresBefore = failures;
    verifyImportedSourceCoverage([normalCoverage], [normalCoverage, flippedCoverage]);
    assert(
      failures === failuresBefore,
      'selftest(c): verifyImportedSourceCoverage takes package-first path for a flipped package (no source-coverage failure)',
    );

    // (d) The flip generators reconstruct an AGENT's execution contract
    //     (tools/model/color) from package metadata — capability preservation.
    //     Skills carry no such metadata; agents do, and dropping it is a
    //     capability-escalation bug. Proven in memory on a synthetic AgentPackage.
    const agentBody = '# Synthetic Agent\n\nSynthetic agent body for the flip self-test.\n';
    const flippedAgent = {
      apiVersion: 'elohim-agent/v1alpha1',
      kind: 'AgentPackage',
      metadata: {
        id: 'synthetic-agent-flip',
        name: 'synthetic-agent-flip',
        version: '1.0.0',
        description: 'Synthetic flipped agent for the contract self-test.',
        role: 'synthetic-agent-flip',
        modelHints: { claudeModel: 'haiku', claudeColor: 'pink' },
        capabilityRefs: [],
        toolRefs: ['Read', 'Edit', 'Bash'],
        sourceRuntime: 'claude', // origin preserved — born from Claude
        master: 'package', // authority flipped to package-first
        mcpServerRefs: [],
        governance: governanceFor('agents', 'synthetic-agent-flip'),
      },
      instructions: { format: 'markdown', body: agentBody },
      projections: {
        claude: { path: '.claude/agents/synthetic-agent-flip.md', frontmatter: {}, frontmatterRaw: '' },
        codex: { path: '.codex/agents/synthetic-agent-flip.md', frontmatter: {} },
      },
    };
    const flippedAgentClaude = projectClaude(flippedAgent);
    assert(
      flippedAgentClaude.includes('tools: Read, Edit, Bash'),
      'selftest(d): flipped agent .claude reconstructs tools from metadata.toolRefs',
    );
    assert(
      flippedAgentClaude.includes('model: haiku'),
      'selftest(d): flipped agent .claude reconstructs model from metadata.modelHints.claudeModel',
    );
    assert(
      flippedAgentClaude.includes('color: pink'),
      'selftest(d): flipped agent .claude reconstructs color from metadata.modelHints.claudeColor',
    );
    assert(
      flippedAgentClaude.includes('master: package'),
      'selftest(d): flipped agent .claude still carries metadata.master: package',
    );
    const flippedAgentCodex = projectCodex(flippedAgent);
    assert(
      flippedAgentCodex.includes('model: haiku'),
      'selftest(d): flipped agent .codex reconstructs model (parity with un-flipped codex path)',
    );
    assert(
      flippedAgentCodex.includes('tools: Read, Edit, Bash'),
      'selftest(d): flipped agent .codex reconstructs tools (parity with un-flipped codex path)',
    );
    assert(
      !/\ncolor:/.test(flippedAgentCodex),
      'selftest(d): flipped agent .codex omits color (claude-only field)',
    );

    // (e) Agent-doc plant is BYTE-IDENTITY under the FLIP: import copies the
    //     ENTIRE raw file (frontmatter incl. a cite envelope + body) verbatim and
    //     project returns it unchanged, so a gospel CLAUDE.md and its cite
    //     fingerprint survive byte-for-byte. Proven in memory on a synthetic doc
    //     carrying a real-shaped `cites:` envelope with a sha256 fingerprint.
    const docRaw =
      '---\n' +
      'id: synthetic-doc-gospel\n' +
      'cites:\n' +
      '  - some-target | why it is cited | sha256:deadbeefcafe0001 | path: some/target.md\n' +
      '---\n\n' +
      '# Synthetic Doc\n\nGospel body for the agent-doc flip self-test.\n';
    const docPath = resolve(sandbox, 'synthetic/CLAUDE.md');
    const importedDoc = agentDocPackageFromSource(docPath, docRaw, {
      repoRoot: sandbox,
      id: 'synthetic-doc-gospel',
      governance: governanceFor('agentdocs', 'synthetic-doc-gospel'),
      composition: 'composes as a leaf gospel doc; managed verbatim by its package.',
      composedBy: 'selftest',
      master: 'package',
    });
    assert(
      projectAgentDoc(importedDoc) === docRaw,
      'selftest(e): project(import(agent-doc)) === source, byte-for-byte (verbatim frontmatter + cites + body)',
    );
    assert(
      projectAgentDoc(importedDoc).includes('sha256:deadbeefcafe0001'),
      'selftest(e): the cite envelope fingerprint survives the flip byte-identical',
    );
    assert(
      importedDoc.metadata.gospelId === 'synthetic-doc-gospel' &&
        importedDoc.metadata.master === 'package',
      'selftest(e): agent-doc records gospelId + master:package in the PACKAGE (not the doc surface)',
    );
    assert(
      !/(^|\n)\s*master:\s*package\b/.test(docRaw),
      'selftest(e): the doc surface itself carries NO authority marker (cite-gen owns the frontmatter)',
    );
    assert(
      runtimeForDoc(docPath) === 'claude' &&
        runtimeForDoc(resolve(sandbox, 'x/AGENTS.md')) === 'codex',
      'selftest(e): runtime is fixed by basename (CLAUDE.md→claude, AGENTS.md→codex)',
    );

    // (f) `--only` scopes a project write to a subset (plant/edit regenerates ONLY
    //     its target, avoiding the whole-tree runtime clobber). Empty selection is
    //     identity (default unchanged); a token matches a bare id or `Kind:id`; a
    //     kind-mismatched token matches nothing. Pure filter — no filesystem.
    const onlyPkgs = [
      { kind: 'SkillPackage', metadata: { id: 'alpha' } },
      { kind: 'AgentPackage', metadata: { id: 'beta' } },
      { kind: 'SkillPackage', metadata: { id: 'gamma' } },
    ];
    assert(
      selectOnly(onlyPkgs, []).length === 3,
      'selftest(f): empty --only selects all (default behavior unchanged)',
    );
    assert(
      JSON.stringify(selectOnly(onlyPkgs, ['beta']).map((p) => p.metadata.id)) === '["beta"]',
      'selftest(f): --only bare id selects exactly that package',
    );
    assert(
      JSON.stringify(
        selectOnly(onlyPkgs, ['SkillPackage:alpha', 'gamma']).map((p) => p.metadata.id),
      ) === '["alpha","gamma"]',
      'selftest(f): --only is repeatable/comma-separated and accepts Kind:id + bare id',
    );
    assert(
      selectOnly(onlyPkgs, ['SkillPackage:beta']).length === 0,
      'selftest(f): a kind-qualified token that mismatches the kind selects nothing (beta is an AgentPackage)',
    );
    assert(
      JSON.stringify(parseOnly(['project', '--only', 'a,b', '--only', 'c'])) ===
        JSON.stringify(['a', 'b', 'c']),
      'selftest(f): parseOnly merges comma-separated + repeated --only flags',
    );

    // (g) mcpServers structured capture + emitter reproduces the REAL corpus
    //     block shapes byte-for-byte, and a flipped mcp-bearing agent carries the
    //     reconstructed block in-position (tools → mcpServers → model). This is
    //     the fidelity proof that lifts the v1 mcp-less STOP rule. Two shapes:
    //     mempalace (command + args:[…]) and the http/sse servers (type + url,
    //     multiple servers) — the exact bytes from `.claude/agents/{librarian,
    //     after-action}.md`.
    const mempalaceBlock =
      'mcpServers:\n' +
      '  - mempalace:\n' +
      '      command: mempalace-mcp\n' +
      '      args:\n' +
      '        - --palace\n' +
      '        - /projects/elohim/.mempalace/palace\n';
    const mempalaceParsed = parseMcpServersBlock(`${mempalaceBlock}model: opus\n`);
    assert(
      `mcpServers:\n${stringifyMcpServers(mempalaceParsed)}` === mempalaceBlock,
      'selftest(g): mempalace block (command + args list) round-trips byte-for-byte',
    );
    assert(
      mempalaceParsed.length === 1 &&
        mempalaceParsed[0].name === 'mempalace' &&
        JSON.stringify(mempalaceParsed[0].config.args) ===
          JSON.stringify(['--palace', '/projects/elohim/.mempalace/palace']),
      'selftest(g): mempalace args list is captured (the field the generic parser drops)',
    );
    const multiBlock =
      'mcpServers:\n' +
      '  - jenkins:\n' +
      '      type: http\n' +
      '      url: https://jenkins.ethosengine.com/mcp-server/mcp\n' +
      '  - observability:\n' +
      '      type: sse\n' +
      '      url: http://observability-mcp.observability.svc.cluster.local:8000/sse\n';
    const multiParsed = parseMcpServersBlock(`${multiBlock}model: sonnet\n`);
    assert(
      `mcpServers:\n${stringifyMcpServers(multiParsed)}` === multiBlock,
      'selftest(g): multi-server type/url block round-trips byte-for-byte',
    );
    // A flipped mcp-bearing agent: the generated .claude carries the block between
    // tools and model, byte-identical to the source, with nothing dropped.
    const mcpAgent = {
      apiVersion: 'elohim-agent/v1alpha1',
      kind: 'AgentPackage',
      metadata: {
        id: 'synthetic-mcp-flip',
        name: 'synthetic-mcp-flip',
        version: '1.0.0',
        description: 'Synthetic flipped mcp-bearing agent.',
        role: 'synthetic-mcp-flip',
        modelHints: { claudeModel: 'opus', claudeColor: 'blue' },
        capabilityRefs: [],
        toolRefs: ['Task', 'Bash'],
        sourceRuntime: 'claude',
        master: 'package',
        mcpServerRefs: ['mempalace'],
        mcpServers: mempalaceParsed,
        governance: governanceFor('agents', 'synthetic-mcp-flip'),
      },
      instructions: { format: 'markdown', body: '# a\n\nbody\n' },
      projections: {
        claude: { path: '.claude/agents/synthetic-mcp-flip.md', frontmatter: {}, frontmatterRaw: '' },
        codex: { path: '.codex/agents/synthetic-mcp-flip.md', frontmatter: {} },
      },
    };
    const mcpFlipClaude = projectClaude(mcpAgent);
    assert(
      mcpFlipClaude.includes(`tools: Task, Bash\n${mempalaceBlock}model: opus\n`),
      'selftest(g): flipped mcp agent .claude carries the mcpServers block in-position (tools → mcpServers → model), byte-identical',
    );
    assert(
      !projectCodex(mcpAgent).includes('mcpServers:'),
      'selftest(g): flipped mcp agent .codex omits mcpServers (claude-only wiring, parity with un-flipped codex path)',
    );

    // (h) import-hook / adopt-doc argument parsing: positionalArgs skips the
    //     value consumed by a value-bearing flag (--id/--only), and flagValue
    //     reads both `--id foo` and `--id=foo`.
    assert(
      JSON.stringify(positionalArgs(['adopt-doc', 'a/b/CLAUDE.md', '--id', 'foo', '--dry-run'])) ===
        JSON.stringify(['adopt-doc', 'a/b/CLAUDE.md']),
      'selftest(h): positionalArgs skips the --id value (doc path is the sole argument)',
    );
    assert(
      JSON.stringify(positionalArgs(['import-hook', 'my-hook', '--dry-run'])) ===
        JSON.stringify(['import-hook', 'my-hook']),
      'selftest(h): positionalArgs keeps the hook name past a boolean flag',
    );
    assert(
      flagValue(['adopt-doc', 'x', '--id', 'foo'], '--id') === 'foo' &&
        flagValue(['adopt-doc', 'x', '--id=bar'], '--id') === 'bar' &&
        flagValue(['adopt-doc', 'x'], '--id') === null,
      'selftest(h): flagValue reads --id both spaced and =, null when absent',
    );
  } finally {
    await rm(sandbox, { recursive: true, force: true });
  }
}

// Refuse to overwrite an already-planted (package-master) package from its
// generated source — that would clobber the authority the flip established.
async function refusesPlantedClobber(pkgPath, id) {
  const existing = await readIfExists(pkgPath);
  if (!existing) return false;
  try {
    if (JSON.parse(existing)?.metadata?.master === 'package') {
      fail(
        `${id} is already planted (master: package) — re-importing from its generated source would ` +
          `clobber the package authority; edit the package JSON directly instead`,
      );
      return true;
    }
  } catch {
    // unparseable existing package — let the scaffold overwrite it
  }
  return false;
}

// `import-hook <name>` — scaffold a HookPackage from `.claude/hooks/<name>.py`
// plus its settings.json registration (read READ-ONLY; settings.json is NEVER
// written). Writes the package JSON + its verbatim projection fixture, ready to
// be planted later. `--dry-run` prints the package without writing.
async function runImportHook(name, { dryRun }) {
  assert(Boolean(name), 'import-hook requires a hook name: import-hook <name> [--dry-run]');
  if (!name) return;
  const srcPath = resolve(HOOK_SOURCE_DIR, `${name}.py`);
  const raw = await readIfExists(srcPath);
  assert(raw !== null, `import-hook: source exists ${relative(REPO_ROOT, srcPath)}`);
  if (raw === null) return;
  const pkgPath = resolve(PACKAGE_DIR, 'hooks', `${name}.json`);
  if (await refusesPlantedClobber(pkgPath, name)) return;
  const settings = await readSettings();
  const pkg = hookPackageFromSource(srcPath, raw, settings, {
    repoRoot: REPO_ROOT,
    governance: governanceFor('hooks', name),
  });
  if (dryRun) {
    console.log(JSON.stringify(pkg, null, 2));
    pass(`import-hook dry-run: scaffolded HookPackage ${name} (registration ${pkg.registration ? 'recorded' : 'null — unregistered'}; not written)`);
    return;
  }
  await writeJson(pkgPath, pkg);
  await writeProjectionFixtures([pkg]);
  pass(
    `import-hook: scaffolded HookPackage ${name} → ${relative(REPO_ROOT, pkgPath)} ` +
      `(settings.json registration recorded read-only, never written)`,
  );
}

// `adopt-doc <path> [--id <id>]` — scaffold an AgentDocPackage from a CLAUDE.md /
// AGENTS.md path. The doc bytes are copied VERBATIM (frontmatter incl. cite
// envelopes + body). The id defaults to the doc's frontmatter `id:` (its gospel
// slug); pass `--id` when the doc declares none. Writes the package + its
// verbatim projection fixture. `--dry-run` prints without writing.
async function runAdoptDoc(docPath, { id, dryRun }) {
  assert(Boolean(docPath), 'adopt-doc requires a doc path: adopt-doc <path> [--id <id>] [--dry-run]');
  if (!docPath) return;
  const srcPath = resolve(REPO_ROOT, docPath);
  const raw = await readIfExists(srcPath);
  assert(raw !== null, `adopt-doc: source exists ${docPath}`);
  if (raw === null) return;
  const gospelId = frontmatterScalar(raw, 'id');
  const docId = id ?? gospelId;
  assert(
    Boolean(docId),
    `adopt-doc: derived an id for ${docPath} (from frontmatter id:, else pass --id <id>)`,
  );
  if (!docId) return;
  const pkgPath = resolve(PACKAGE_DIR, 'agentdocs', `${docId}.json`);
  if (await refusesPlantedClobber(pkgPath, docId)) return;
  const pkg = agentDocPackageFromSource(srcPath, raw, {
    repoRoot: REPO_ROOT,
    id: docId,
    governance: governanceFor('agentdocs', docId),
  });
  if (dryRun) {
    console.log(JSON.stringify(pkg, null, 2));
    pass(`adopt-doc dry-run: scaffolded AgentDocPackage ${docId} (not written)`);
    return;
  }
  await writeJson(pkgPath, pkg);
  await writeProjectionFixtures([pkg]);
  pass(`adopt-doc: scaffolded AgentDocPackage ${docId} → ${relative(REPO_ROOT, pkgPath)}`);
}

async function main() {
  const positionals = positionalArgs(args);
  switch (command) {
    case 'init':
      await initLayout();
      break;
    case 'selftest':
      await runSelfTest();
      break;
    case 'import':
      await runImport({ writeProjections: WRITE_FIXTURES });
      break;
    case 'import-hook':
      await runImportHook(positionals[1], { dryRun: DRY_RUN });
      break;
    case 'adopt-doc':
      await runAdoptDoc(positionals[1], { id: flagValue(args, '--id'), dryRun: DRY_RUN });
      break;
    case 'project':
      await runProject({ writeFixtures: WRITE_FIXTURES, writeRuntime: WRITE_RUNTIME, only: ONLY });
      break;
    case 'verify':
      if (LEGACY_WRITE) {
        await runImport({ writeProjections: false });
        await runProject({ writeFixtures: true, writeRuntime: false });
      }
      await runVerify();
      break;
    default:
      fail(
        `unknown command: ${command} (expected init, import, import-hook, adopt-doc, project, verify, selftest)`,
      );
  }

  if (failures > 0) {
    console.error(`\nelohim-agent package checks failed: ${failures} failed, ${passes} passed`);
    process.exit(1);
  }
  console.log(`\nelohim-agent package checks passed: ${passes} passed`);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
