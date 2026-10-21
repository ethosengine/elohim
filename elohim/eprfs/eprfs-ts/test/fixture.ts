// Shared by the tests: the vectors file and the fixture-tree builder that mirrors
// eprfs-core/tests/vectors.rs's `build`.
import { readFileSync } from 'node:fs';

import { CID } from 'multiformats/cid';

import { MemoryTier } from '../src/tier.js';
import { cidOf, encodeTree, RAW_CODEC, treeNode, type TreeEntry, type TreeEntryKind } from '../src/tree.js';
import { parsePath, type TreePath } from '../src/walk.js';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const vectors: any = JSON.parse(
  readFileSync(new URL('../../fixtures/tree-vectors.json', import.meta.url), 'utf8'),
);

export function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
}

export function unhex(text: string): Uint8Array {
  const out = new Uint8Array(text.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(text.slice(2 * i, 2 * i + 2), 16);
  return out;
}

const encoder = new TextEncoder();

function nameOf(key: string): Uint8Array {
  return key.startsWith('hex:') ? unhex(key.slice(4)) : encoder.encode(key);
}

/** A vector path: `''` is the root; a `hex:` component is a non-UTF-8 name. */
export function vpath(text: string): TreePath {
  if (text === '') return [];
  return text.split('/').map(nameOf);
}

export function vtext(path: TreePath): string {
  const decoder = new TextDecoder('utf-8', { fatal: true });
  return path
    .map((c) => {
      try {
        return decoder.decode(c);
      } catch {
        return `hex:${hex(c)}`;
      }
    })
    .join('/');
}

export { parsePath };

/** Build a fixture tree into `tier`; returns the root and every directory's CID by path text. */
export async function build(
  tier: MemoryTier,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  fixture: any,
  at = '',
  paths = new Map<string, CID>(),
): Promise<{ root: CID; paths: Map<string, CID> }> {
  const entries: TreeEntry[] = [];
  for (const [key, value] of Object.entries(fixture)) {
    let kind: TreeEntryKind;
    if (typeof value === 'string') {
      const bytes = encoder.encode(value);
      const cid = await cidOf(bytes, RAW_CODEC);
      await tier.put(cid, bytes);
      kind = { file: { blob: cid, executable: false } };
    } else if (value !== null && typeof value === 'object' && '$external' in value) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const e = (value as any).$external;
      kind = { external: { kind: e.kind, reference: unhex(e.referenceHex) } };
    } else if (value !== null && typeof value === 'object' && '$symlink' in value) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const bytes = encoder.encode((value as any).$symlink);
      const cid = await cidOf(bytes, RAW_CODEC);
      await tier.put(cid, bytes);
      kind = { symlink: { target: cid } };
    } else {
      const child = await build(tier, value, at === '' ? key : `${at}/${key}`, paths);
      kind = { directory: { tree: child.root } };
    }
    entries.push({ name: nameOf(key), kind });
  }
  const { cid, bytes } = await encodeTree(treeNode(entries));
  await tier.put(cid, bytes);
  paths.set(at, cid);
  return { root: cid, paths };
}

/** A TreeEntryKind in the JSON form Rust's serde gives it. */
export function kindJson(kind: TreeEntryKind | undefined): unknown {
  if (kind === undefined) return null;
  if ('file' in kind) return { file: { blob: kind.file.blob.toString(), executable: kind.file.executable } };
  if ('directory' in kind) return { directory: { tree: kind.directory.tree.toString() } };
  if ('symlink' in kind) return { symlink: { target: kind.symlink.target.toString() } };
  return { external: { kind: kind.external.kind, reference: Array.from(kind.external.reference) } };
}
