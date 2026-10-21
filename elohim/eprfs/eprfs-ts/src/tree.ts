/**
 * The version-1 canonical tree, byte for byte as `eprfs-core/src/tree.rs` encodes it.
 *
 * A node is the DAG-CBOR map `{ entries, version }`; an entry is `{ kind, name }` with the name a
 * byte string (names are bytes, and may be non-UTF-8); a kind is externally tagged
 * (`{ file: { blob, executable } }`, `{ directory: { tree } }`, `{ symlink: { target } }`,
 * `{ external: { kind, reference } }`) with every link a tag-42 CID. Decoding refuses unknown
 * fields and noncanonical bytes by re-encoding and comparing, as `decode_tree` does.
 */
import { decodeCanonical, encodeCanonical } from '@elohim/epr';
import { CID } from 'multiformats/cid';
import { sha256 } from 'multiformats/hashes/sha2';

export const TREE_VERSION = 1;
export const RAW_CODEC = 0x55;
export const DAG_CBOR_CODEC = 0x71;

export type TreeEntryKind =
  | { file: { blob: CID; executable: boolean } }
  | { directory: { tree: CID } }
  | { symlink: { target: CID } }
  | { external: { kind: string; reference: Uint8Array } };
export interface TreeEntry {
  name: Uint8Array;
  kind: TreeEntryKind;
}
export interface TreeNode {
  version: 1;
  entries: readonly TreeEntry[];
}
export interface TreeLimits {
  maxNodeBytes: number;
  maxEntriesPerNode: number;
  maxDepth: number;
  maxTotalEntries: number;
  maxTotalBytes: number;
  maxPathBytes: number;
  maxPendingEntries: number;
}

/** `TreeLimits::default()` in Rust. */
export const DEFAULT_LIMITS: Readonly<TreeLimits> = Object.freeze({
  maxNodeBytes: 4 * 1024 * 1024,
  maxEntriesPerNode: 100_000,
  maxDepth: 128,
  maxTotalEntries: 1_000_000,
  maxTotalBytes: 1024 * 1024 * 1024,
  maxPathBytes: 4096,
  maxPendingEntries: 4096,
});

export function limitsOf(limits?: Partial<TreeLimits>): TreeLimits {
  return { ...DEFAULT_LIMITS, ...limits };
}

/**
 * A refusal about the tree itself or the request, never an absence: tiers name absences as
 * values (decision 11), and these are the errors Rust raises as `TreeError`.
 */
export type TreeErrorKind =
  | 'invalid'
  | 'budget'
  | 'integrity'
  | 'storage'
  | 'not-a-leaf'
  | 'external-boundary'
  | 'unanswered';
export class TreeError extends Error {
  constructor(
    readonly kind: TreeErrorKind,
    message: string,
    readonly detail?: unknown,
  ) {
    super(`${kind}: ${message}`);
    this.name = 'TreeError';
  }
}

/** Byte order of two names: the order tree entries are sorted in. */
export function compareBytes(a: Uint8Array, b: Uint8Array): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    if (a[i] !== b[i]) return a[i] - b[i];
  }
  return a.length - b.length;
}

export function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && compareBytes(a, b) === 0;
}

/** `valid_name` in Rust: non-empty, not `.` or `..`, no NUL and no `/`. */
export function validName(name: Uint8Array): boolean {
  if (name.length === 0) return false;
  if (name.length === 1 && name[0] === 0x2e) return false;
  if (name.length === 2 && name[0] === 0x2e && name[1] === 0x2e) return false;
  return !name.includes(0) && !name.includes(0x2f);
}

function linkCodecIs(cid: CID, codec: number): boolean {
  return cid.version === 1 && cid.code === codec;
}

function validateNode(node: TreeNode, limits: TreeLimits): void {
  if (node.version !== TREE_VERSION) {
    throw new TreeError('invalid', `unsupported tree version ${String(node.version)}`);
  }
  if (node.entries.length > limits.maxEntriesPerNode) {
    throw new TreeError('budget', 'entries per node');
  }
  let last: Uint8Array | null = null;
  for (const entry of node.entries) {
    if (!validName(entry.name)) throw new TreeError('invalid', 'invalid single-component byte name');
    if (last !== null && compareBytes(last, entry.name) >= 0) {
      throw new TreeError('invalid', 'names must be unique and byte-sorted');
    }
    last = entry.name;
    const kind = entry.kind;
    if ('file' in kind || 'symlink' in kind) {
      const link = 'file' in kind ? kind.file.blob : kind.symlink.target;
      if (!linkCodecIs(link, RAW_CODEC)) throw new TreeError('invalid', 'leaf must use raw CID codec');
    } else if ('directory' in kind) {
      if (!linkCodecIs(kind.directory.tree, DAG_CBOR_CODEC)) {
        throw new TreeError('invalid', 'directory must use DAG-CBOR CID codec');
      }
    } else if (kind.external.kind.length === 0 || kind.external.reference.length === 0) {
      throw new TreeError('invalid', 'external boundary needs kind and reference');
    }
  }
}

/** Sort by raw name bytes and validate, as `TreeNode::new` does. */
export function treeNode(entries: readonly TreeEntry[]): TreeNode {
  const sorted = [...entries].sort((a, b) => compareBytes(a.name, b.name));
  const node: TreeNode = { version: 1, entries: sorted };
  validateNode(node, DEFAULT_LIMITS);
  return node;
}

/** CIDv1 over sha2-256 with the given codec. */
export async function cidOf(bytes: Uint8Array, codec: number): Promise<CID> {
  return CID.create(1, codec, await sha256.digest(bytes));
}

/** `encode_tree`: the canonical bytes and their DAG-CBOR CID. */
export async function encodeTree(
  node: TreeNode,
  limits?: Partial<TreeLimits>,
): Promise<{ cid: CID; bytes: Uint8Array }> {
  const l = limitsOf(limits);
  validateNode(node, l);
  const bytes = encodeCanonical({ entries: node.entries, version: node.version });
  if (bytes.length > l.maxNodeBytes || bytes.length > l.maxTotalBytes) {
    throw new TreeError('budget', 'encoded node bytes');
  }
  return { cid: await cidOf(bytes, DAG_CBOR_CODEC), bytes };
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value) && !(value instanceof Uint8Array);
}

function exactKeys(value: Record<string, unknown>, keys: readonly string[], what: string): void {
  const got = Object.keys(value).sort();
  const want = [...keys].sort();
  if (got.length !== want.length || got.some((k, i) => k !== want[i])) {
    throw new TreeError('invalid', `DAG-CBOR decoding: unexpected fields in ${what}`);
  }
}

function asCid(value: unknown, what: string): CID {
  const cid = CID.asCID(value);
  if (cid === null) throw new TreeError('invalid', `DAG-CBOR decoding: ${what} is not a link`);
  return cid;
}

function asBytes(value: unknown, what: string): Uint8Array {
  if (!(value instanceof Uint8Array)) throw new TreeError('invalid', `DAG-CBOR decoding: ${what} is not bytes`);
  return value;
}

function decodeKind(value: unknown): TreeEntryKind {
  if (!isPlainObject(value) || Object.keys(value).length !== 1) {
    throw new TreeError('invalid', 'DAG-CBOR decoding: an entry kind is one tagged variant');
  }
  const [tag] = Object.keys(value);
  const inner = value[tag];
  if (!isPlainObject(inner)) throw new TreeError('invalid', 'DAG-CBOR decoding: malformed entry kind');
  switch (tag) {
    case 'file':
      exactKeys(inner, ['blob', 'executable'], 'file');
      if (typeof inner.executable !== 'boolean') throw new TreeError('invalid', 'DAG-CBOR decoding: executable');
      return { file: { blob: asCid(inner.blob, 'blob'), executable: inner.executable } };
    case 'directory':
      exactKeys(inner, ['tree'], 'directory');
      return { directory: { tree: asCid(inner.tree, 'tree') } };
    case 'symlink':
      exactKeys(inner, ['target'], 'symlink');
      return { symlink: { target: asCid(inner.target, 'target') } };
    case 'external':
      exactKeys(inner, ['kind', 'reference'], 'external');
      if (typeof inner.kind !== 'string') throw new TreeError('invalid', 'DAG-CBOR decoding: external kind');
      return { external: { kind: inner.kind, reference: asBytes(inner.reference, 'reference') } };
    default:
      throw new TreeError('invalid', `DAG-CBOR decoding: unknown entry kind ${tag}`);
  }
}

/** Decodes tree.rs's canonical bytes and re-encodes them, comparing byte for byte, as decode_tree does. */
export function decodeTree(bytes: Uint8Array, limits?: Partial<TreeLimits>): TreeNode {
  const l = limitsOf(limits);
  if (bytes.length > l.maxNodeBytes || bytes.length > l.maxTotalBytes) {
    throw new TreeError('budget', 'encoded node bytes');
  }
  let raw: unknown;
  try {
    raw = decodeCanonical(bytes);
  } catch (error) {
    throw new TreeError('invalid', `DAG-CBOR decoding: ${String(error)}`);
  }
  if (!isPlainObject(raw)) throw new TreeError('invalid', 'DAG-CBOR decoding: a node is a map');
  exactKeys(raw, ['entries', 'version'], 'node');
  if (!Array.isArray(raw.entries) || typeof raw.version !== 'number') {
    throw new TreeError('invalid', 'DAG-CBOR decoding: malformed node');
  }
  if (raw.version !== TREE_VERSION) {
    throw new TreeError('invalid', `unsupported tree version ${raw.version}`);
  }
  const entries = raw.entries.map((entry: unknown): TreeEntry => {
    if (!isPlainObject(entry)) throw new TreeError('invalid', 'DAG-CBOR decoding: an entry is a map');
    exactKeys(entry, ['kind', 'name'], 'entry');
    return { name: asBytes(entry.name, 'name'), kind: decodeKind(entry.kind) };
  });
  const node: TreeNode = { version: 1, entries };
  validateNode(node, l);
  const canonical = encodeCanonical({ entries: node.entries, version: node.version });
  if (!bytesEqual(canonical, bytes)) throw new TreeError('invalid', 'noncanonical DAG-CBOR tree');
  return node;
}

/** The link an entry names, or null for an external boundary. */
export function entryLink(kind: TreeEntryKind): CID | null {
  if ('file' in kind) return kind.file.blob;
  if ('directory' in kind) return kind.directory.tree;
  if ('symlink' in kind) return kind.symlink.target;
  return null;
}
