/**
 * Seam B in TypeScript (interfaces spec §H.1): the view, the level-batched walk, read and diff,
 * with eprfs-core's semantics, compared with Rust through the shared vectors.
 *
 * Paths are byte components, as Rust's `TreePath`, so a non-UTF-8 name survives everywhere;
 * `pathString` is a display helper that answers `null` rather than decoding lossily (H14).
 * Walk items do not cross the wire: each language walks on its own side over the batch route,
 * and the table in §H.1 maps the two shapes.
 */
import { CID } from 'multiformats/cid';

import type { BatchItem, Tier, TierAnswer } from './tier.js';
import {
  compareBytes,
  DAG_CBOR_CODEC,
  decodeTree,
  limitsOf,
  TreeError,
  validName,
  type TreeEntry,
  type TreeEntryKind,
  type TreeLimits,
  type TreeNode,
} from './tree.js';
import { verifyAddress as verifiesAddress } from '@elohim/epr';
import { absence, type ItemAbsence, type Unanswered } from './wire.js';

export type TreePath = readonly Uint8Array[];

/** The walk's own absences beside the tier ones (§H.1's mapping table onto Rust). */
export type NamedAbsence =
  | { kind: 'absent'; cid: string; path: TreePath; unanswered: Unanswered<ItemAbsence> }
  | { kind: 'not-found'; path: TreePath }
  | { kind: 'through-leaf'; leaf: TreePath }
  | { kind: 'invalid-view'; path: TreePath };

export type Coverage = 'in-view' | 'spine' | 'outside';

export function comparePaths(a: TreePath, b: TreePath): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const c = compareBytes(a[i], b[i]);
    if (c !== 0) return c;
  }
  return a.length - b.length;
}

/** Component-wise prefix: `src` is a prefix of `src/x`, never of `src2`. */
export function startsWith(path: TreePath, prefix: TreePath): boolean {
  return prefix.length <= path.length && prefix.every((c, i) => compareBytes(c, path[i]) === 0);
}

function byteLen(path: TreePath): number {
  return path.reduce((n, c) => n + c.length, 0);
}

const utf8 = new TextDecoder('utf-8', { fatal: true });
const encoder = new TextEncoder();

/** The slash-joined UTF-8 form, `''` for the root; `null` for a non-UTF-8 name. */
export function pathString(path: TreePath): string | null {
  try {
    return path.map((c) => utf8.decode(c)).join('/');
  } catch {
    return null;
  }
}

/** `TreePath::from_slash_bytes` for a string: `''` is the root; empty components refuse. */
export function parsePath(text: string): TreePath {
  if (text === '') return [];
  const components = text.split('/').map((c) => encoder.encode(c));
  if (!components.every(validName)) throw new TreeError('invalid', `invalid path component in ${text}`);
  return components;
}

/** The set of paths a reader intends to act on, normalized to a sorted antichain (as Rust's). */
export class View {
  private constructor(readonly paths: readonly TreePath[]) {}

  static whole(): View {
    return new View([[]]);
  }

  static of(paths: readonly (readonly Uint8Array[])[]): View {
    for (const p of paths) {
      if (!p.every(validName)) throw new TreeError('invalid', 'invalid path component');
    }
    const sorted = [...paths].sort(comparePaths);
    const kept: TreePath[] = [];
    for (const p of sorted) {
      const last = kept[kept.length - 1];
      if (last !== undefined && startsWith(p, last)) continue;
      kept.push(p);
    }
    return new View(kept);
  }

  static ofStrings(paths: readonly string[]): View {
    return View.of(paths.map(parsePath));
  }

  isEmpty(): boolean {
    return this.paths.length === 0;
  }

  coverage(path: readonly Uint8Array[]): Coverage {
    if (this.paths.some((v) => startsWith(path, v))) return 'in-view';
    if (this.paths.some((v) => startsWith(v, path))) return 'spine';
    return 'outside';
  }
}

export type WalkItem =
  | { node: { path: TreePath; cid: CID; node: TreeNode; coverage: 'in-view' | 'spine' } }
  /** `coverage` is additive to the spec's shape: Spine means a view path went through it. */
  | { boundary: { path: TreePath; kind: string; coverage: 'in-view' | 'spine' } }
  | { absent: { viewPath: TreePath; reason: NamedAbsence } };
export interface WalkStats {
  nodesFetched: number;
  nodesReused: number;
  nodeBytes: number;
  entriesVisited: number;
}

/** Traversal accounting with Rust's labels. */
class Budget {
  private totalEntries = 0;
  private totalBytes = 0;
  private pendingPathBytes = 0;
  private externalPathBytes = 0;
  resultBytes = 0;
  constructor(readonly limits: TreeLimits) {}

  checkDepth(depth: number): void {
    if (depth > this.limits.maxDepth) throw new TreeError('budget', 'tree depth');
  }
  checkChildDepth(depth: number): void {
    if (depth >= this.limits.maxDepth) throw new TreeError('budget', 'tree depth');
  }
  private charge(len: number, label: string): void {
    this.totalBytes += len;
    if (this.totalBytes > this.limits.maxTotalBytes) throw new TreeError('budget', label);
  }
  chargeFetched(len: number): void {
    this.charge(len, 'total bytes');
  }
  chargeReused(len: number): void {
    this.charge(len, 'expanded bytes');
  }
  chargeEntries(n: number): void {
    this.totalEntries += n;
    if (this.totalEntries > this.limits.maxTotalEntries) throw new TreeError('budget', 'total entries');
  }
  childPathBytes(parent: number, nameLen: number): number {
    const child = parent + nameLen;
    if (child > this.limits.maxPathBytes) throw new TreeError('budget', 'path bytes');
    return child;
  }
  pushPendingBytes(bytes: number): void {
    this.pendingPathBytes += bytes;
    if (this.pendingPathBytes > this.limits.maxTotalBytes) throw new TreeError('budget', 'pending path bytes');
  }
  popPending(bytes: number): void {
    this.pendingPathBytes -= bytes;
  }
  chargeExternalPath(bytes: number): void {
    this.externalPathBytes += bytes;
    if (this.externalPathBytes > this.limits.maxTotalBytes) throw new TreeError('budget', 'external path bytes');
  }
  chargeResult(bytes: number): void {
    this.resultBytes += bytes;
    if (this.resultBytes > this.limits.maxTotalBytes) throw new TreeError('budget', 'result path bytes');
  }
}

type Scope = { inView: TreePath } | { spine: TreePath[] };

function scopeCoverage(scope: Scope): 'in-view' | 'spine' {
  return 'inView' in scope ? 'in-view' : 'spine';
}

function rootScope(view: View): Scope | null {
  const first = view.paths[0];
  if (first === undefined) return null;
  return first.length === 0 ? { inView: first } : { spine: [...view.paths] };
}

function groupScope(group: TreePath[], depth: number): Scope | null {
  if (group.length === 0) return null;
  const exact = group.find((t) => t.length === depth + 1);
  return exact !== undefined ? { inView: exact } : { spine: group };
}

/** One node's entries matched against its scope, with the view paths that name no entry. */
function classify(scope: Scope, depth: number, entries: readonly TreeEntry[]): { entries: (Scope | null)[]; unmatched: TreePath[] } {
  if ('inView' in scope) return { entries: entries.map(() => scope), unmatched: [] };
  const tails = scope.spine;
  const out: (Scope | null)[] = [];
  const unmatched: TreePath[] = [];
  let next = 0;
  for (const entry of entries) {
    while (next < tails.length && compareBytes(tails[next][depth], entry.name) < 0) unmatched.push(tails[next++]);
    const start = next;
    while (next < tails.length && compareBytes(tails[next][depth], entry.name) === 0) next++;
    out.push(groupScope(tails.slice(start, next), depth));
  }
  unmatched.push(...tails.slice(next));
  return { entries: out, unmatched };
}

/**
 * A queued directory, as Rust's `Frame`: no ancestor list (content addressing rules out a link
 * cycle and `maxDepth` bounds any chain), and the charge queueing it took from `pending path
 * bytes`, given back when it is visited.
 */
interface Frame {
  cid: CID;
  key: string;
  path: TreePath;
  depth: number;
  pathBytes: number;
  charge: number;
  scope: Scope;
}

/**
 * What a level walk charges per queued directory beyond its path bytes, the same fixed numbers
 * as Rust's `QUEUED_FRAME_BYTES` and `QUEUED_COMPONENT_BYTES`, so both twins refuse alike: a
 * level is held whole between rounds, so its frames are metered by retained size (B13).
 */
const QUEUED_FRAME_BYTES = 192;
const QUEUED_COMPONENT_BYTES = 24;

const utf8Encoder = new TextEncoder();

function strBytes(text: string): number {
  return utf8Encoder.encode(text).length;
}

/** A tier id's bytes without its form prefix, as Rust's `TierId::as_str`. */
function idBytes(id: string): number {
  return strBytes(id.slice(id.indexOf(':') + 1));
}

function absenceBytes(why: ItemAbsence): number {
  switch (why.kind) {
    case 'not-held':
      return why.tried.reduce((n, id) => n + idBytes(id), 0);
    case 'refused':
      return idBytes(why.by) + (why.refusal.needsStanding === null ? 0 : strBytes(why.refusal.needsStanding));
    case 'over-budget':
      return idBytes(why.by) + strBytes(why.rule);
    case 'unreachable':
      return idBytes(why.tier);
    case 'membership-unknown':
      return idBytes(why.by);
    case 'integrity':
      return idBytes(why.from);
  }
}

/**
 * What a gathered walk keeps for one absence, as Rust's `WalkItem::retained_bytes`: its paths and
 * every string its unanswered parts carry (Rust keeps the reason beside the unanswered parts, so
 * the reason's ids count twice there and here).
 */
function retainedBytes(a: { viewPath: TreePath; reason: NamedAbsence }): number {
  const r = a.reason;
  if (r.kind === 'not-found') return byteLen(a.viewPath) + byteLen(r.path);
  if (r.kind === 'through-leaf') return byteLen(a.viewPath) + byteLen(r.leaf);
  if (r.kind !== 'absent') return byteLen(a.viewPath);
  const u = r.unanswered;
  const comeBack = u.comeBack.when === 'on-event' ? strBytes(u.comeBack.event) : 0;
  const meanwhile =
    u.meanwhile.local === 'use-held' ? strBytes(u.meanwhile.what) : u.meanwhile.local === 'nothing' ? strBytes(u.meanwhile.because) : 0;
  const escalation =
    u.escalation === null ? 0 : u.escalation.whoCanBeAsked.reduce((n, w) => n + strBytes(w), strBytes(u.escalation.changed));
  return byteLen(a.viewPath) + byteLen(r.path) + 2 * absenceBytes(u.why) + comeBack + meanwhile + escalation;
}

function checkRootCodec(root: CID): void {
  if (root.code !== DAG_CBOR_CODEC) throw new TreeError('invalid', 'root must use DAG-CBOR CID codec');
}

function checkViewBytes(view: View, limits: TreeLimits): void {
  if (view.paths.some((p) => byteLen(p) > limits.maxPathBytes)) throw new TreeError('budget', 'path bytes');
}

function join(path: TreePath, name: Uint8Array): TreePath {
  return [...path, name];
}

function absentReason(frame: Frame, unanswered: Unanswered<ItemAbsence>): NamedAbsence {
  return { kind: 'absent', cid: frame.key, path: frame.path, unanswered };
}

/** A gathered walk, as Rust's `WalkedView`. */
export interface WalkedView {
  nodes: { path: TreePath; cid: CID; node: TreeNode; coverage: 'in-view' | 'spine'; entryCoverage: Coverage[] }[];
  boundaries: { path: TreePath; kind: string; reference: Uint8Array; coverage: 'in-view' | 'spine' }[];
  absent: { viewPath: TreePath; reason: NamedAbsence }[];
  stats: WalkStats;
}

/**
 * The level-batched walk, gathered: one `fetchBlobs` per level chunk of at most
 * `maxPendingEntries` frames, unseen items re-sent once, absences named and the walk continued;
 * integrity and an own-tier failure are errors (B4); an item never answered is withheld as
 * unreachable, never not-held (B3). The result equals Rust's `walk_batched` on the shared vectors.
 */
export async function collectWalk(
  tier: Tier,
  root: CID,
  view: View,
  limits?: Partial<TreeLimits>,
): Promise<WalkedView> {
  const l = limitsOf(limits);
  checkRootCodec(root);
  checkViewBytes(view, l);
  const budget = new Budget(l);
  // What the result keeps, charged as each node or absence is kept, as Rust's `LevelWalk`.
  const retained = new Budget(l);
  const stats: WalkStats = { nodesFetched: 0, nodesReused: 0, nodeBytes: 0, entriesVisited: 0 };
  const cache = new Map<string, { node: TreeNode; len: number }>();
  const nodes: WalkedView['nodes'] = [];
  const trailing: { key: TreePath; seq: number; item: WalkItem; reference?: Uint8Array }[] = [];
  let seq = 0;
  const push = (key: TreePath, item: WalkItem, reference?: Uint8Array) => trailing.push({ key, seq: seq++, item, reference });
  const pushAbsent = (key: TreePath, a: { viewPath: TreePath; reason: NamedAbsence }) => {
    retained.chargeResult(retainedBytes(a));
    push(key, { absent: a });
  };

  const scope = rootScope(view);
  let level: Frame[] =
    scope === null ? [] : [{ cid: root, key: root.toString(), path: [], depth: 0, pathBytes: 0, charge: 0, scope }];
  while (level.length > 0) {
    const next: Frame[] = [];
    while (level.length > 0) {
      const chunk = level.splice(0, Math.max(1, l.maxPendingEntries));
      const wantedKeys = new Set<string>();
      const wanted: BatchItem[] = [];
      for (const f of chunk) {
        if (!cache.has(f.key) && !wantedKeys.has(f.key)) {
          wantedKeys.add(f.key);
          wanted.push({ cid: f.cid, path: f.path });
        }
      }
      const offered = new Map<string, { node: TreeNode; len: number }>();
      const absentBy = new Map<string, Unanswered<ItemAbsence>>();
      let unseen = wanted;
      for (let attempt = 0; attempt < 2 && unseen.length > 0; attempt++) {
        const answers: [BatchItem, TierAnswer][] = [];
        try {
          await tier.fetchBlobs(root, unseen, [], (item, r) => answers.push([item, r]));
        } catch {
          continue;
        }
        const seen = new Set<string>();
        for (const [item, r] of answers) {
          const key = item.cid.toString();
          if (!wantedKeys.has(key) || offered.has(key) || absentBy.has(key)) continue;
          seen.add(key);
          if (r instanceof Uint8Array) {
            if (!(await verifiesAddress(item.cid, r))) throw new TreeError('integrity', key);
            if (r.length > l.maxNodeBytes) throw new TreeError('budget', 'encoded node bytes');
            budget.chargeFetched(r.length);
            const node = decodeTree(r, l);
            stats.nodesFetched += 1;
            stats.nodeBytes += r.length;
            offered.set(key, { node, len: r.length });
          } else if (r.kind === 'absent') {
            rejectWalkError(r.unanswered, key);
            absentBy.set(key, r.unanswered);
          }
        }
        unseen = unseen.filter((i) => !seen.has(i.cid.toString()));
      }
      // What the tier never answered is its absence, named by the tier asked: withheld from a
      // remote tier, an error from the own tier (B3, B4). Nothing here invents a tier id.
      for (const item of unseen) {
        const why = absence.unreachable(tier.record.id);
        rejectWalkError(why, item.cid.toString());
        absentBy.set(item.cid.toString(), why);
      }

      for (const frame of chunk) {
        budget.popPending(frame.charge);
        budget.checkDepth(frame.depth);
        let node: TreeNode;
        const fresh = offered.get(frame.key);
        const held = cache.get(frame.key);
        if (fresh !== undefined) {
          offered.delete(frame.key);
          cache.set(frame.key, fresh);
          node = fresh.node;
        } else if (held !== undefined) {
          budget.chargeReused(held.len);
          stats.nodesReused += 1;
          node = held.node;
        } else {
          const why = absentBy.get(frame.key);
          if (why === undefined) throw new TreeError('invalid', `${frame.key} was neither offered nor named`);
          const viewPaths = 'inView' in frame.scope ? [frame.scope.inView] : frame.scope.spine;
          for (const viewPath of viewPaths) pushAbsent(frame.path, { viewPath, reason: absentReason(frame, why) });
          continue;
        }
        stats.entriesVisited += node.entries.length;
        budget.chargeEntries(node.entries.length);
        const classified = classify(frame.scope, frame.depth, node.entries);
        node.entries.forEach((entry, i) => {
          const s = classified.entries[i];
          if (s === null) return;
          const childBytes = budget.childPathBytes(frame.pathBytes, entry.name.length);
          const childPath = join(frame.path, entry.name);
          const kind = entry.kind;
          if ('directory' in kind) {
            budget.checkChildDepth(frame.depth);
            const charge = childBytes + (frame.depth + 1) * QUEUED_COMPONENT_BYTES + QUEUED_FRAME_BYTES;
            budget.pushPendingBytes(charge);
            next.push({
              cid: kind.directory.tree,
              key: kind.directory.tree.toString(),
              path: childPath,
              depth: frame.depth + 1,
              pathBytes: childBytes,
              charge,
              scope: s,
            });
          } else if ('external' in kind) {
            budget.chargeExternalPath(childBytes);
            push(frame.path, { boundary: { path: childPath, kind: kind.external.kind, coverage: scopeCoverage(s) } }, kind.external.reference);
          } else if ('spine' in s) {
            for (const viewPath of s.spine) pushAbsent(frame.path, { viewPath, reason: { kind: 'through-leaf', leaf: childPath } });
          }
        });
        for (const viewPath of classified.unmatched) {
          pushAbsent(frame.path, { viewPath, reason: { kind: 'not-found', path: join(frame.path, viewPath[frame.depth]) } });
        }
        retained.chargeResult(byteLen(frame.path));
        nodes.push({
          path: frame.path,
          cid: frame.cid,
          node,
          coverage: scopeCoverage(frame.scope),
          entryCoverage: classified.entries.map((e) => (e === null ? 'outside' : scopeCoverage(e))),
        });
      }
    }
    level = next;
  }

  nodes.sort((a, b) => comparePaths(a.path, b.path));
  trailing.sort((a, b) => comparePaths(a.key, b.key) || a.seq - b.seq);
  const walked: WalkedView = { nodes, boundaries: [], absent: [], stats };
  for (const t of trailing) {
    if ('boundary' in t.item) walked.boundaries.push({ ...t.item.boundary, reference: t.reference ?? new Uint8Array() });
    else if ('absent' in t.item) walked.absent.push(t.item.absent);
  }
  return walked;
}

/** One fetchBlobs per tree level chunk, as walk_batched; `stats` settles when the iteration ends. */
export function walkBatched(
  tier: Tier,
  root: CID,
  view: View,
  limits?: Partial<TreeLimits>,
): { items: AsyncIterable<WalkItem>; stats: Promise<WalkStats> } {
  const result = collectWalk(tier, root, view, limits);
  const stats = result.then((w) => w.stats);
  stats.catch(() => undefined);
  async function* items(): AsyncIterable<WalkItem> {
    const walked = await result;
    // Nodes in pre-order, each followed by the boundaries among its entries; then the absences,
    // in the depth-first walk's order.
    const keyOf = (p: TreePath) => p.map((c) => Array.from(c).join(',')).join('/');
    const after = new Map<string, WalkItem[]>();
    for (const b of walked.boundaries) {
      const k = keyOf(b.path.slice(0, -1));
      const list = after.get(k) ?? [];
      list.push({ boundary: { path: b.path, kind: b.kind, coverage: b.coverage } });
      after.set(k, list);
    }
    for (const n of walked.nodes) {
      yield { node: { path: n.path, cid: n.cid, node: n.node, coverage: n.coverage } };
      for (const item of after.get(keyOf(n.path)) ?? []) yield item;
    }
    for (const a of walked.absent) yield { absent: a };
  }
  return { items: items(), stats };
}

/**
 * The one rule every walk applies to a tier's named absence (B4), as Rust's `absence_error`:
 * bytes that failed their address are an integrity error, and the own tier failing (the only
 * party that names `own`) is a storage error. Every other absence stays an absence.
 */
function rejectWalkError(u: Unanswered<ItemAbsence>, key: string): void {
  if (u.why.kind === 'integrity') throw new TreeError('integrity', key);
  if (u.why.kind === 'unreachable' && u.why.tier === 'own') throw new TreeError('storage', key);
}

async function fetchNode(
  tier: Tier,
  root: CID,
  cid: CID,
  path: TreePath,
  budget: Budget,
): Promise<TreeNode | NamedAbsence> {
  const r = await tier.fetchAt(root, { cid, path }, []);
  if (!(r instanceof Uint8Array)) {
    if (r.kind === 'absent') rejectWalkError(r.unanswered, cid.toString());
    return r.kind === 'absent' ? { ...r, path } : r;
  }
  if (!(await verifiesAddress(cid, r))) throw new TreeError('integrity', cid.toString());
  if (r.length > budget.limits.maxNodeBytes) throw new TreeError('budget', 'encoded node bytes');
  budget.chargeFetched(r.length);
  const node = decodeTree(r, budget.limits);
  budget.chargeEntries(node.entries.length);
  return node;
}

function find(node: TreeNode, name: Uint8Array): TreeEntry | undefined {
  let lo = 0;
  let hi = node.entries.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const c = compareBytes(node.entries[mid].name, name);
    if (c === 0) return node.entries[mid];
    if (c < 0) lo = mid + 1;
    else hi = mid - 1;
  }
  return undefined;
}

/**
 * Read one file or symlink: the spine nodes to it plus its one verified leaf. An absence is a
 * value (`not-found`, `through-leaf`, or a tier's `absent` naming the path where it fell short);
 * reading a directory or through an external boundary is a refusal of the request (thrown).
 */
export async function readPath(
  tier: Tier,
  root: CID,
  path: readonly Uint8Array[],
  limits?: Partial<TreeLimits>,
): Promise<Uint8Array | NamedAbsence> {
  checkRootCodec(root);
  const budget = new Budget(limitsOf(limits));
  if (byteLen(path) > budget.limits.maxPathBytes) throw new TreeError('budget', 'path bytes');
  if (path.length === 0) throw new TreeError('not-a-leaf', 'the root is a directory');
  let cid = root;
  let here: TreePath = [];
  for (let depth = 0; depth < path.length; depth++) {
    budget.checkDepth(depth);
    const node = await fetchNode(tier, root, cid, here, budget);
    if ('kind' in node) return node;
    const entry = find(node, path[depth]);
    if (entry === undefined) return { kind: 'not-found', path };
    const last = depth === path.length - 1;
    here = join(here, path[depth]);
    const kind = entry.kind;
    if ('external' in kind) throw new TreeError('external-boundary', kind.external.kind, here);
    if ('directory' in kind) {
      if (last) throw new TreeError('not-a-leaf', 'a directory is not a leaf', here);
      budget.checkChildDepth(depth);
      cid = kind.directory.tree;
      continue;
    }
    if (!last) return { kind: 'through-leaf', leaf: here };
    const leaf = 'file' in kind ? kind.file.blob : kind.symlink.target;
    const bytes = await tier.fetchAt(root, { cid: leaf, path }, []);
    if (!(bytes instanceof Uint8Array)) {
      if (bytes.kind === 'absent') rejectWalkError(bytes.unanswered, leaf.toString());
      return bytes.kind === 'absent' ? { ...bytes, path } : bytes;
    }
    if (!(await verifiesAddress(leaf, bytes))) throw new TreeError('integrity', leaf.toString());
    budget.chargeFetched(bytes.length);
    return bytes;
  }
  throw new TreeError('not-a-leaf', 'unreachable');
}

function kindEqual(a: TreeEntryKind | undefined, b: TreeEntryKind | undefined): boolean {
  if (a === undefined || b === undefined) return a === b;
  if ('file' in a && 'file' in b) return a.file.executable === b.file.executable && a.file.blob.equals(b.file.blob);
  if ('directory' in a && 'directory' in b) return a.directory.tree.equals(b.directory.tree);
  if ('symlink' in a && 'symlink' in b) return a.symlink.target.equals(b.symlink.target);
  if ('external' in a && 'external' in b) {
    return a.external.kind === b.external.kind && compareBytes(a.external.reference, b.external.reference) === 0;
  }
  return false;
}

function dirOf(kind: TreeEntryKind | undefined): CID | undefined {
  return kind !== undefined && 'directory' in kind ? kind.directory.tree : undefined;
}

function isExternal(kind: TreeEntryKind | undefined): boolean {
  return kind !== undefined && 'external' in kind;
}

function externalBytes(kind: TreeEntryKind | undefined): number {
  return kind !== undefined && 'external' in kind ? encoder.encode(kind.external.kind).length + kind.external.reference.length : 0;
}

function scopeFor(scope: Scope, depth: number, name: Uint8Array): Scope | null {
  if ('inView' in scope) return scope;
  return groupScope(
    scope.spine.filter((t) => compareBytes(t[depth], name) === 0),
    depth,
  );
}

/**
 * Diff two roots under a view as one co-walk, with Rust's `diff_trees` rules: equal CIDs are
 * never fetched; a directory added or removed in view is reported once; an ancestor of a view
 * path is never itself a change unless an External sits there; output is pre-order. A node no
 * tier gave is a refusal (`unanswered`), since one answer cannot be partial.
 */
export async function diffTrees(
  tier: Tier,
  oldRoot: CID,
  newRoot: CID,
  view: View,
  limits?: Partial<TreeLimits>,
): Promise<{ path: TreePath; old?: TreeEntryKind; new?: TreeEntryKind }[]> {
  checkRootCodec(oldRoot);
  checkRootCodec(newRoot);
  const l = limitsOf(limits);
  checkViewBytes(view, l);
  const scope = rootScope(view);
  if (scope === null || oldRoot.equals(newRoot)) return [];
  const budget = new Budget(l);
  const changes: { path: TreePath; old?: TreeEntryKind; new?: TreeEntryKind }[] = [];
  type DiffFrame = { old?: CID; new?: CID; path: TreePath; depth: number; ancestors: string[]; pathBytes: number; scope: Scope };
  const stack: DiffFrame[] = [{ old: oldRoot, new: newRoot, path: [], depth: 0, ancestors: [], pathBytes: 0, scope }];
  const side = async (sideRoot: CID, cid: CID | undefined, path: TreePath, depth: number) => {
    if (cid === undefined) {
      budget.checkDepth(depth);
      return [] as readonly TreeEntry[];
    }
    budget.checkDepth(depth);
    const node = await fetchNode(tier, sideRoot, cid, path, budget);
    if ('kind' in node) throw new TreeError('unanswered', `no tier gave the node at ${pathString(path) ?? '<bytes>'}`, node);
    return node.entries;
  };
  while (stack.length > 0) {
    const frame = stack.pop()!;
    budget.popPending(frame.pathBytes);
    const pair = `${frame.old?.toString() ?? '-'}|${frame.new?.toString() ?? '-'}`;
    if (frame.ancestors.includes(pair)) throw new TreeError('invalid', 'directory link cycle');
    const olds = await side(oldRoot, frame.old, frame.path, frame.depth);
    const news = await side(newRoot, frame.new, frame.path, frame.depth);
    const lineage = [...frame.ancestors, pair];
    let i = 0;
    let j = 0;
    while (i < olds.length || j < news.length) {
      let name: Uint8Array;
      let o: TreeEntryKind | undefined;
      let n: TreeEntryKind | undefined;
      const c = i < olds.length && j < news.length ? compareBytes(olds[i].name, news[j].name) : i < olds.length ? -1 : 1;
      if (c === 0) {
        name = olds[i].name;
        o = olds[i++].kind;
        n = news[j++].kind;
      } else if (c < 0) {
        name = olds[i].name;
        o = olds[i++].kind;
      } else {
        name = news[j].name;
        n = news[j++].kind;
      }
      const s = scopeFor(frame.scope, frame.depth, name);
      if (s === null || kindEqual(o, n)) continue;
      const childBytes = budget.childPathBytes(frame.pathBytes, name.length);
      const childPath = join(frame.path, name);
      const [oldDir, newDir] = [dirOf(o), dirOf(n)];
      const boundary = isExternal(o) || isExternal(n);
      let descend: boolean;
      let report: boolean;
      if ('spine' in s && boundary) [descend, report] = [false, true];
      else if ('spine' in s) [descend, report] = [oldDir !== undefined || newDir !== undefined, false];
      else {
        const both = oldDir !== undefined && newDir !== undefined;
        [descend, report] = [both, !both];
      }
      if (descend) {
        budget.checkChildDepth(frame.depth);
        if (stack.length >= l.maxPendingEntries) throw new TreeError('budget', 'pending entries');
        budget.pushPendingBytes(childBytes);
        stack.push({ old: oldDir, new: newDir, path: childPath, depth: frame.depth + 1, ancestors: lineage, pathBytes: childBytes, scope: s });
      } else if (report) {
        budget.chargeResult(childBytes + externalBytes(o) + externalBytes(n));
        changes.push({ path: childPath, ...(o !== undefined ? { old: o } : {}), ...(n !== undefined ? { new: n } : {}) });
      }
    }
  }
  return changes.sort((a, b) => comparePaths(a.path, b.path));
}
