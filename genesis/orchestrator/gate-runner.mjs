#!/usr/bin/env node
// One local gate path for humans and pre-push. Detection and execution both
// come from build-manifest.json; no project-name command switch lives here.

import { readFileSync } from 'fs';
import { spawnSync } from 'child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { loadManifests } from './manifest-utils.mjs';
import { loadGateRegistry } from './pipeline-registry.mjs';
import { walkGraph } from './graph-walker.mjs';
import { filterChanged } from './ci-ignore.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

export function selectGateProjects(registry, target) {
  if (registry.has(target)) return [registry.get(target)];

  const normalized = target.replace(/^\.\//, '').replace(/\/$/, '');
  const matches = [...registry.values()]
    .filter(project => normalized === project.dir || normalized.startsWith(`${project.dir}/`))
    .sort((a, b) => b.dir.length - a.dir.length);
  if (matches.length === 0) throw new Error(`Unknown gate project or path: ${target}`);
  const longest = matches[0].dir.length;
  return matches.filter(project => project.dir.length === longest);
}

export function projectsForChanges(root, changedFiles) {
  const manifests = loadManifests(root);
  const registry = loadGateRegistry(root);
  const result = walkGraph(manifests, filterChanged(changedFiles));
  return result.projects.map(project => {
    const registered = registry.get(project.name);
    if (!registered) throw new Error(`Detected unregistered gate project: ${project.name}`);
    return { ...registered, reasons: project.reasons };
  });
}

function runProject(project, printOnly, namesOnly) {
  const cargo = project.run.cargo || {};
  const args = [
    ROOT,
    project.name,
    project.dir,
    project.run.kind,
    project.run.recipe,
    cargo.workspace || '',
    cargo.targetDir || '',
    cargo.profile || 'dev',
    Object.hasOwn(cargo, 'rustflags') ? cargo.rustflags : '__inherit__',
  ];

  if (namesOnly) {
    process.stdout.write(`${project.name}\n`);
    return 0;
  }
  if (printOnly) {
    process.stdout.write(`${JSON.stringify({ name: project.name, dir: project.dir, run: project.run, reasons: project.reasons || [] })}\n`);
    return 0;
  }

  process.stdout.write(`\n[gate] ${project.name} (${project.dir})\n`);
  const result = spawnSync('bash', [resolve(ROOT, 'genesis/orchestrator/run-local-gate.sh'), ...args], {
    cwd: ROOT,
    stdio: 'inherit',
  });
  return result.status ?? 1;
}

function usage() {
  console.error('usage: gate-runner.mjs (--target <project-or-path> | --changed-file-list | --list) [--print]');
}

const isMain = import.meta.url === `file://${process.argv[1]}` ||
  import.meta.url === `file://${resolve(process.argv[1])}`;

if (isMain) {
  const args = process.argv.slice(2);
  const printOnly = args.includes('--print');
  const namesOnly = args.includes('--names');
  const registry = loadGateRegistry(ROOT);
  let projects;

  if (args.includes('--list')) {
    for (const project of registry.values()) {
      process.stdout.write(`${project.name}\t${project.dir}\t${project.run.kind}:${project.run.recipe}\n`);
    }
    process.exit(0);
  } else if (args.includes('--changed-file-list')) {
    const files = readFileSync(0, 'utf8').split('\n').map(value => value.trim()).filter(Boolean);
    projects = projectsForChanges(ROOT, files);
  } else {
    const index = args.indexOf('--target');
    if (index < 0 || !args[index + 1]) {
      usage();
      process.exit(2);
    }
    projects = selectGateProjects(registry, args[index + 1]);
  }

  if (projects.length === 0) {
    process.stdout.write('[gate] no manifest-declared projects selected\n');
    process.exit(0);
  }

  let failed = 0;
  for (const project of projects) failed |= runProject(project, printOnly, namesOnly);
  process.exit(failed ? 1 : 0);
}
