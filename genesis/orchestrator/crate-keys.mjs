import { createHash } from 'node:crypto';
import { dirname } from 'node:path';

// Mirrors the root justfile's `_gate-eprfs` recipe. Only Cargo workspace-wide steps can
// honestly attribute their closure key to every member; the other steps cover different tools.
export const EPRFS_RECIPE_STEP_COVERAGE = Object.freeze({
  fmt: 'workspace',
  precheck: null,
  lint: 'workspace',
  bins: null,
  ark: null,
  test: 'workspace',
});

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]));
  }
  return value;
}

function digest(value) {
  return createHash('sha256').update(JSON.stringify(canonical(value))).digest('hex');
}

/**
 * One shadow verdict key per Cargo workspace member. Workspace dependency keys compose
 * recursively, so a changed crate moves exactly itself and its reverse dependencies, including
 * the members that reach it only as a dev-dependency.
 */
export function crateKeys({
  metadata,
  treeOf,
  toolchain,
  lockDigest,
  outsideReadsDigest,
  cargoEnv,
  step,
}) {
  if (!metadata?.workspace_members || !metadata?.packages || !metadata?.resolve?.nodes) return {};
  const members = new Set(metadata.workspace_members);
  const packages = new Map(metadata.packages.filter(pkg => members.has(pkg.id)).map(pkg => [pkg.id, pkg]));
  const nodes = new Map(metadata.resolve.nodes.map(node => [node.id, node]));
  if (packages.size !== members.size) return {};

  // Cargo forbids cycles among normal and build dependencies, but a dev-dependency may close one
  // (eprfs-serve dev-depends on itself with its `testing` feature). So each member has a library
  // key over its own tree, its features and its normal/build workspace dependencies' library keys
  // (acyclic, recursive), and its step key adds the library keys of its dev-only workspace
  // dependencies, minus itself, whose tree the library key already holds.
  const devOnly = dep => Array.isArray(dep.dep_kinds) && dep.dep_kinds.length > 0
    && dep.dep_kinds.every(kind => kind?.kind === 'dev');
  const held = new Map();
  const visiting = new Set();
  const libraryKeyOf = id => {
    if (held.has(id)) return held.get(id);
    if (visiting.has(id)) throw new Error(`workspace dependency cycle at ${id}`);
    const pkg = packages.get(id);
    if (!pkg) throw new Error(`workspace member metadata absent for ${id}`);
    visiting.add(id);
    const node = nodes.get(id) || { deps: [], features: [] };
    const dependencies = (node.deps || [])
      .filter(dep => members.has(dep.pkg) && !devOnly(dep))
      .map(dep => libraryKeyOf(dep.pkg))
      .sort();
    const tree = treeOf(dirname(pkg.manifest_path));
    if (!tree) throw new Error(`tree absent for ${pkg.name}`);
    const key = digest({
      tree,
      features: [...(node.features || [])].sort(),
      dependencies,
    });
    visiting.delete(id);
    held.set(id, key);
    return key;
  };
  const keyOf = id => {
    const node = nodes.get(id) || { deps: [] };
    const devDependencies = (node.deps || [])
      .filter(dep => members.has(dep.pkg) && dep.pkg !== id && devOnly(dep))
      .map(dep => libraryKeyOf(dep.pkg))
      .sort();
    return digest({
      step,
      toolchain,
      cargoEnv: cargoEnv || {},
      lockDigest,
      outsideReadsDigest,
      library: libraryKeyOf(id),
      devDependencies,
    });
  };

  return Object.fromEntries([...packages.values()]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map(pkg => [pkg.name, keyOf(pkg.id)]));
}
