#!/usr/bin/env node
import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const DOMAIN_DIR = resolve(__dirname, '..');
const REPO_ROOT = resolve(DOMAIN_DIR, '../../../..');
const EPR_META_DIR = resolve(REPO_ROOT, '.epr-meta');
const PACKAGE_DIR = resolve(EPR_META_DIR, 'elohim/packages');
const PROJECTION_DIR = resolve(EPR_META_DIR, 'elohim/projections');

const SKILL_SOURCE_DIR = resolve(REPO_ROOT, '.claude/skills');
const AGENT_SOURCE_DIR = resolve(REPO_ROOT, '.claude/agents');

const args = process.argv.slice(2);
const command = args.find((arg) => !arg.startsWith('-')) ?? 'verify';
const WRITE_FIXTURES = args.includes('--write-fixtures') || args.includes('--write-projections');
const WRITE_RUNTIME = args.includes('--write-runtime');
const LEGACY_WRITE = command === 'verify' && WRITE_FIXTURES;

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

function skillPackageFromClaude(path, parsed) {
  const name = parsed.frontmatter.name;
  const version = parsed.frontmatter.metadata?.version ?? '1.0.0';
  const description = parsed.frontmatter.description;

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
        }),
      },
    },
  };
}

function codexFrontmatter({ name, description, packageKind, sourcePath, sourceRuntime, model, tools }) {
  const metadata = {
    runtime: 'codex',
    sourceRuntime,
    sourcePath,
    packageKind,
  };
  const frontmatter = { name, description, metadata };
  if (model) frontmatter.model = model;
  if (tools?.length) frontmatter.tools = tools.join(', ');
  return frontmatter;
}

function projectClaude(pkg) {
  return projectMarkdownSurface(pkg, 'claude');
}

function projectCodex(pkg) {
  return projectMarkdownSurface(pkg, 'codex');
}

function projectMarkdownSurface(pkg, runtime) {
  const projection = pkg.projections[runtime];
  const frontmatter = projection.frontmatterRaw
    ? `${projection.frontmatterRaw}\n`
    : stringifyYaml(projection.frontmatter);
  return `---\n${frontmatter}---\n${pkg.instructions.body}`;
}

function stringifyYaml(value, indent = '') {
  let out = '';
  for (const [key, field] of Object.entries(value)) {
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

async function loadSourcePackages() {
  const sourcePackages = [];
  let skillDirs = [];
  try {
    skillDirs = (await readdir(SKILL_SOURCE_DIR, { withFileTypes: true }))
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .sort();
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  for (const dir of skillDirs) {
    const path = resolve(SKILL_SOURCE_DIR, dir, 'SKILL.md');
    const raw = await readIfExists(path);
    if (raw) {
      const parsed = parseMarkdownSurface(path, raw);
      if (parsed.frontmatter.metadata?.sourceRuntime !== 'elohim-agent') {
        sourcePackages.push(skillPackageFromClaude(path, parsed));
      }
    }
  }

  let agentFiles = [];
  try {
    agentFiles = (await readdir(AGENT_SOURCE_DIR)).filter((name) => name.endsWith('.md')).sort();
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  for (const file of agentFiles) {
    const path = resolve(AGENT_SOURCE_DIR, file);
    const raw = await readFile(path, 'utf8');
    sourcePackages.push(agentPackageFromClaude(path, parseMarkdownSurface(path, raw)));
  }

  return sourcePackages;
}

function packagePathFor(pkg) {
  return pkg.kind === 'SkillPackage'
    ? resolve(PACKAGE_DIR, 'skills', `${pkg.metadata.id}.json`)
    : resolve(PACKAGE_DIR, 'agents', `${pkg.metadata.id}.json`);
}

function projectionFixturePathsFor(pkg) {
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
  return {
    claude: resolve(REPO_ROOT, pkg.projections.claude.path),
    codex: resolve(REPO_ROOT, pkg.projections.codex.path),
  };
}

function projectedTextFor(pkg, runtime) {
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
    await writeText(paths.claude, projectClaude(pkg));
    await writeText(paths.codex, projectCodex(pkg));
  }
}

async function writeRuntimeProjections(packages) {
  for (const pkg of packages) {
    const paths = runtimePathsFor(pkg);
    await writeText(paths.claude, projectClaude(pkg));
    await writeText(paths.codex, projectCodex(pkg));
  }
}

async function initLayout() {
  const manifest = resolve(EPR_META_DIR, 'manifest.md');
  await mkdir(resolve(PACKAGE_DIR, 'skills'), { recursive: true });
  await mkdir(resolve(PACKAGE_DIR, 'agents'), { recursive: true });
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
  const skillFiles = await listJsonFiles(skillsDir);
  const agentFiles = await listJsonFiles(agentsDir);
  return [
    ...(await Promise.all(skillFiles.map((file) => readJson(resolve(skillsDir, file))))),
    ...(await Promise.all(agentFiles.map((file) => readJson(resolve(agentsDir, file))))),
  ];
}

async function loadValidators() {
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  const skillSchema = await readJson(resolve(DOMAIN_DIR, 'schemas/skill-package.schema.json'));
  const agentSchema = await readJson(resolve(DOMAIN_DIR, 'schemas/agent-package.schema.json'));
  return {
    skill: ajv.compile(skillSchema),
    agent: ajv.compile(agentSchema),
  };
}

async function verifyPackage(pkg, validators) {
  const validate = pkg.kind === 'SkillPackage' ? validators.skill : validators.agent;
  assert(
    validate(pkg),
    `${pkg.kind} package ${pkg.metadata.id} validates: ${JSON.stringify(validate.errors)}`,
  );

  assert(pkg.metadata.id === pkg.metadata.name, `${pkg.metadata.id} name/id round-trip`);
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

  await verifyProjectionFixture(pkg, 'claude');
  await verifyProjectionFixture(pkg, 'codex');
  await verifyRuntimeProjectionIfPresent(pkg, 'claude');
  await verifyRuntimeProjectionIfPresent(pkg, 'codex');
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
    assert(
      projectClaude(pkg) === original,
      `fidelity: project(import(${pkg.kind}:${pkg.metadata.id})) === source`,
    );
  }
}

function verifyImportedSourceCoverage(sourcePackages, packageFixtures) {
  const sourceIds = new Set(sourcePackages.map((pkg) => `${pkg.kind}:${pkg.metadata.id}`));
  const fixtureIds = new Set(packageFixtures.map((pkg) => `${pkg.kind}:${pkg.metadata.id}`));

  for (const id of sourceIds) {
    assert(fixtureIds.has(id), `imported package exists for ${id}`);
  }
  for (const pkg of packageFixtures) {
    if (pkg.metadata.sourceRuntime !== 'claude') {
      pass(`native package does not require Claude source: ${pkg.kind}:${pkg.metadata.id}`);
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

async function runProject({ writeFixtures, writeRuntime }) {
  const packages = await loadPackageFixtures();
  assert(packages.length > 0, 'found elohim packages to project');
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

async function runVerify() {
  const sourcePackages = await loadSourcePackages();
  const packageFixtures = await loadPackageFixtures();
  assert(packageFixtures.length > 0, 'loads elohim package fixtures');
  verifyImportedSourceCoverage(sourcePackages, packageFixtures);
  await verifySourceFidelity(sourcePackages);

  const validators = await loadValidators();
  for (const pkg of packageFixtures) {
    await verifyPackage(pkg, validators);
  }
}

async function main() {
  switch (command) {
    case 'init':
      await initLayout();
      break;
    case 'import':
      await runImport({ writeProjections: WRITE_FIXTURES });
      break;
    case 'project':
      await runProject({ writeFixtures: WRITE_FIXTURES, writeRuntime: WRITE_RUNTIME });
      break;
    case 'verify':
      if (LEGACY_WRITE) {
        await runImport({ writeProjections: false });
        await runProject({ writeFixtures: true, writeRuntime: false });
      }
      await runVerify();
      break;
    default:
      fail(`unknown command: ${command} (expected init, import, project, verify)`);
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
