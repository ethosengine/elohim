// Cargo's resolved graph owns package reachability, including path dependencies
// outside the workspace. An unknown compile input keeps the full workspace gate.

import { spawnSync } from 'child_process';
import { basename, dirname, relative, resolve } from 'path';

/** Metadata resolves dependencies but never builds; failed discovery cannot narrow a gate. */
export function loadMetadata({ root, workspaceDir, spawn = spawnSync }) {
  const env = { ...process.env };
  delete env.CARGO_TARGET_DIR;
  const options = { cwd: root, encoding: 'utf8', env, timeout: 60_000, maxBuffer: 32 * 1024 * 1024 };
  try {
    const rustc = spawn('rustc', ['-vV'], options);
    if (rustc.status !== 0) return null;
    const host = rustc.stdout.match(/^host:\s*(\S+)\s*$/m)?.[1];
    if (!host) return null;
    const result = spawn('cargo', [
      'metadata', '--format-version', '1', '--locked', '--filter-platform', host,
      '--manifest-path', resolve(root, workspaceDir, 'Cargo.toml'),
    ], options);
    if (result.status !== 0) return null;
    return JSON.parse(result.stdout);
  } catch {
    return null;
  }
}

function covers(dir, file) {
  return dir === '.' || file === dir || file.startsWith(`${dir}/`);
}

/** readsOf takes a repo-relative package directory and returns repo-relative read paths. */
export function affectedPackages({ root, workspaceDir, files, metadata, readsOf = () => [] }) {
  if (!metadata) return { packages: null, why: 'metadata unavailable' };
  if (!files?.length) return { packages: null, why: 'no changed files' };
  if (!metadata.resolve?.nodes || !metadata.workspace_members?.length || !metadata.packages) {
    return { packages: null, why: 'metadata graph unavailable' };
  }

  const repoPath = path => relative(resolve(root), resolve(root, path)).split('\\').join('/') || '.';
  const workspace = repoPath(workspaceDir);
  const packages = metadata.packages.map(pkg => ({ ...pkg, dir: repoPath(dirname(pkg.manifest_path)) }))
    .filter(pkg => pkg.dir !== '..' && !pkg.dir.startsWith('../'))
    .sort((a, b) => b.dir.length - a.dir.length);
  const members = new Set(metadata.workspace_members);
  const manifests = new Set(packages.map(pkg => repoPath(pkg.manifest_path)));
  if (packages.filter(pkg => members.has(pkg.id)).length !== members.size) {
    return { packages: null, why: 'workspace member metadata unavailable' };
  }
  const changed = files.map(repoPath);
  const affected = new Set();
  for (const file of changed) {
    const owner = packages.find(pkg => covers(pkg.dir, file));
    // These inputs can alter the graph or compiler even inside a package directory.
    if (basename(file) === 'Cargo.lock') return { packages: null, why: `Cargo.lock changed: ${file}` };
    if (basename(file) === 'Cargo.toml' && !manifests.has(file)) {
      return { packages: null, why: `non-package Cargo.toml changed: ${file}` };
    }
    if (file === '.cargo/config.toml' || file.endsWith('/.cargo/config.toml') ||
        basename(file).startsWith('rust-toolchain') || covers('genesis/orchestrator/rust-style', file)) {
      return { packages: null, why: `shared compile input changed: ${file}` };
    }
    if (owner) affected.add(owner.id);
    else if (covers(workspace, file)) return { packages: null, why: `unmapped workspace file: ${file}` };
    // Unmapped files outside the workspace (such as docs) cannot change a compile.
    // Test reads of those files are handled by readsOf below, before reverse reachability.
  }
  for (const pkg of packages.filter(pkg => members.has(pkg.id))) {
    if (readsOf(pkg.dir).some(read => changed.some(file => covers(repoPath(read), file)))) {
      affected.add(pkg.id);
    }
  }

  const dependents = new Map();
  for (const node of metadata.resolve.nodes) {
    for (const dep of node.deps) {
      if (!dependents.has(dep.pkg)) dependents.set(dep.pkg, []);
      dependents.get(dep.pkg).push(node.id);
    }
  }
  const queue = [...affected];
  for (let i = 0; i < queue.length; i++) {
    for (const id of dependents.get(queue[i]) || []) {
      if (affected.has(id)) continue;
      affected.add(id);
      queue.push(id);
    }
  }
  const selected = packages.filter(pkg => members.has(pkg.id) && affected.has(pkg.id));
  if (selected.length === members.size) return { packages: null, why: 'every member affected' };
  return {
    packages: selected.map(pkg => pkg.name).sort(),
    why: selected.length ? 'changed packages, test reads and reverse dependencies' : 'no workspace members affected',
  };
}
