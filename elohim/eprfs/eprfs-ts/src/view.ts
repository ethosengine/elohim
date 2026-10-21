/**
 * The serializable carrier of a root and a view (interfaces spec §B.3), the TypeScript twin of
 * eprfs-core's `TreeView`: the same canonical DAG-CBOR form (`{ paths, root }`, byte-string path
 * components, a tag-42 root), the same decode refusals (an empty, unsorted or covered path list),
 * and therefore the same CID, pinned by the shared vectors.
 */
import { decodeCanonical, encodeCanonical } from '@elohim/epr';
import { CID } from 'multiformats/cid';

import { cidOf, DAG_CBOR_CODEC, TreeError } from './tree.js';
import { comparePaths, pathString, startsWith, View, type TreePath } from './walk.js';

function checkCanonical(paths: readonly TreePath[]): void {
  if (paths.length === 0) throw new TreeError('invalid', 'a tree view names no path');
  for (let i = 1; i < paths.length; i++) {
    if (comparePaths(paths[i - 1], paths[i]) >= 0) {
      throw new TreeError('invalid', 'tree-view paths are not sorted component-wise');
    }
    if (startsWith(paths[i], paths[i - 1])) throw new TreeError('invalid', 'a tree-view path is covered by another');
  }
}

/** The canonical bytes of a root and a view (the view is normalized already). */
export function encodeTreeView(root: CID, view: View): Uint8Array {
  checkCanonical(view.paths);
  return encodeCanonical({ paths: view.paths.map((p) => [...p]), root });
}

/** `TreeView::cid`: the need key of R1. */
export async function treeViewCid(root: CID, view: View): Promise<CID> {
  return cidOf(encodeTreeView(root, view), DAG_CBOR_CODEC);
}

/** Decodes the canonical form, refusing what Rust's `Deserialize` refuses. */
export function decodeTreeView(bytes: Uint8Array): { root: CID; view: View } {
  const raw = decodeCanonical<Record<string, unknown>>(bytes);
  const keys = Object.keys(raw ?? {}).sort();
  if (keys.length !== 2 || keys[0] !== 'paths' || keys[1] !== 'root') {
    throw new TreeError('invalid', 'a tree view is exactly { paths, root }');
  }
  const root = CID.asCID(raw.root);
  if (root === null || root.code !== DAG_CBOR_CODEC) throw new TreeError('invalid', 'tree-view root must be a DAG-CBOR link');
  if (!Array.isArray(raw.paths)) throw new TreeError('invalid', 'tree-view paths');
  const paths = raw.paths.map((p: unknown) => {
    if (!Array.isArray(p) || !p.every((c) => c instanceof Uint8Array)) throw new TreeError('invalid', 'tree-view path');
    return p as Uint8Array[];
  });
  checkCanonical(paths);
  return { root, view: View.of(paths) };
}

/** The human-readable form `{ paths, root }`; a non-UTF-8 name refuses, never decodes lossily. */
export function treeViewJson(root: CID, view: View): { paths: string[]; root: string } {
  checkCanonical(view.paths);
  const paths = view.paths.map((p) => {
    const text = pathString(p);
    if (text === null) throw new TreeError('invalid', 'a tree-view path is not UTF-8');
    return text;
  });
  return { paths, root: root.toString() };
}

/** Decodes the human-readable form with the same refusals. */
export function parseTreeViewJson(value: { paths: readonly string[]; root: string }): { root: CID; view: View } {
  const encoder = new TextEncoder();
  const paths: TreePath[] = value.paths.map((p) => (p === '' ? [] : p.split('/').map((c) => encoder.encode(c))));
  checkCanonical(paths);
  const root = CID.parse(value.root);
  if (root.code !== DAG_CBOR_CODEC) throw new TreeError('invalid', 'tree-view root must be a DAG-CBOR link');
  return { root, view: View.of(paths) };
}
