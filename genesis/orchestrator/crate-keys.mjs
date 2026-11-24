import { createHash } from 'node:crypto';
import { dirname } from 'node:path';

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
 * recursively, so a changed crate moves exactly itself and its reverse dependencies.
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

  const held = new Map();
  const visiting = new Set();
  const keyOf = id => {
    if (held.has(id)) return held.get(id);
    if (visiting.has(id)) throw new Error(`workspace dependency cycle at ${id}`);
    const pkg = packages.get(id);
    if (!pkg) throw new Error(`workspace member metadata absent for ${id}`);
    visiting.add(id);
    const node = nodes.get(id) || { deps: [], features: [] };
    const dependencies = (node.deps || [])
      .map(dep => dep.pkg)
      .filter(dep => members.has(dep))
      .map(keyOf)
      .sort();
    const tree = treeOf(dirname(pkg.manifest_path));
    if (!tree) throw new Error(`tree absent for ${pkg.name}`);
    const key = digest({
      step,
      toolchain,
      cargoEnv: cargoEnv || {},
      tree,
      lockDigest,
      outsideReadsDigest,
      features: [...(node.features || [])].sort(),
      dependencies,
    });
    visiting.delete(id);
    held.set(id, key);
    return key;
  };

  return Object.fromEntries([...packages.values()]
    .sort((a, b) => a.name.localeCompare(b.name))
    .map(pkg => [pkg.name, keyOf(pkg.id)]));
}
