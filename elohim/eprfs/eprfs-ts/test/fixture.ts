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

/**
 * The chunker fixtures' frozen generator (design gate CDC 2), read from the `chunker` section's
 * header: xorshift64* (shifts 12/25/27) from the seed, each step's 64-bit product emitted
 * little-endian, the stream truncated to `len`. BigInt, masked to 64 bits.
 */
export function xorshiftBytes(seedHex: string, multiplierHex: string, len: number): Uint8Array {
  const m64 = (1n << 64n) - 1n;
  const multiplier = BigInt(multiplierHex);
  let state = BigInt(seedHex);
  const out = new Uint8Array(Math.ceil(len / 8) * 8);
  const view = new DataView(out.buffer);
  for (let at = 0; at < len; at += 8) {
    state ^= state >> 12n;
    state = (state ^ (state << 25n)) & m64;
    state ^= state >> 27n;
    view.setBigUint64(at, (state * multiplier) & m64, true);
  }
  return out.subarray(0, len);
}

/** The input bytes of one `chunker.vectors` row. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function chunkerInput(section: any, vector: any): Uint8Array {
  const len: number = vector.length;
  switch (vector.source) {
    case 'xorshift64*':
      return xorshiftBytes(section.generator.seed, section.generator.multiplier, len);
    case 'zeros':
      return new Uint8Array(len);
    case 'pattern': {
      const pattern = encoder.encode(vector.pattern as string);
      const out = new Uint8Array(len);
      for (let i = 0; i < len; i++) out[i] = pattern[i % pattern.length];
      return out;
    }
    default:
      throw new Error(`unknown chunker fixture source ${vector.source}`);
  }
}
