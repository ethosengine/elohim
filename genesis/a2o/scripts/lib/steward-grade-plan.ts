/**
 * Steward grade — the pure decisions, kept apart from the I/O so they can be tested
 * without a mesh.
 *
 * A genesis peer grades its OWN rows (scripts/steward-grade.ts): a row stored narrower
 * than the reach its repo file authors is widened to exactly that authored reach,
 * through the node's own conductor, and only when that conductor proves the node's own
 * agent authored the row. Four rules live here:
 *
 *   1. Commons fence. Only an authored reach of `commons` or `public` is ever graded
 *      toward. Any other authored word (intimate, community, an unknown word, none) is
 *      refused — this step opens content, it never decides how open intimate content is.
 *   2. Never narrow. A row already at or above the authored openness is `current`.
 *   3. Exactly the authored reach. A widen targets the authored reach, never more.
 *   4. Own authorship, proven by the conductor (see `ownAuthorshipRefusal`). Storage
 *      headers (`X-Agent-Cid`) are never consulted: they are forgeable
 *      (backlog security-storage-direct-caller-unauthenticated).
 */
import { isReach, REACH_OPENNESS, type Reach } from '../../../seeder/src/generated/schema-enums.js';

/** Authored reaches a grade may widen toward. */
export const GRADEABLE_REACHES: readonly Reach[] = ['commons', 'public'];

/**
 * The reach an item's repo file authors, resolved the way the seeder's builders read
 * it: a content atom authors `reach` only (`resolveContentReach`); a path authors
 * `reach`, else its legacy `visibility` key (`buildPathInput`).
 */
export function authoredReachFor(
  kind: 'content' | 'path',
  json: { reach?: unknown; visibility?: unknown }
): string | undefined {
  const r = kind === 'path' ? (json.reach ?? json.visibility) : json.reach;
  return typeof r === 'string' && r.trim() !== '' ? r : undefined;
}

/** What an ANONYMOUS read of `GET /db/content/{id}` observed. */
export type ObservedRow =
  /** 200 — the row serves anonymously at this reach. */
  | { kind: 'served'; reach: string | null }
  /** 403 — the reach gate refused an anonymous caller; `requiredReach` from the body. */
  | { kind: 'gated'; requiredReach: string | null }
  /** 404 — this peer holds no row for the id. */
  | { kind: 'absent' };

export type RowVerdict =
  | { action: 'current'; from: string; to: Reach }
  /** Narrower than authored: widening needs the conductor's authorship proof. */
  | { action: 'needs-authorship'; from: Reach; to: Reach }
  | { action: 'refused'; from: string; to: string; reason: string };

/** Rules 1–3: what the row alone says, before any conductor is asked. */
export function classifyRow(authored: string | undefined, row: ObservedRow): RowVerdict {
  const to = authored ?? 'none';
  if (authored === undefined) {
    return { action: 'refused', from: '?', to, reason: 'no authored reach in the repo file' };
  }
  if (!isReach(authored)) {
    return { action: 'refused', from: '?', to, reason: `unknown authored reach "${authored}"` };
  }
  if (!GRADEABLE_REACHES.includes(authored)) {
    return {
      action: 'refused',
      from: '?',
      to,
      reason: `authored reach "${authored}" is outside the commons fence (commons/public)`,
    };
  }
  if (row.kind === 'absent') {
    return { action: 'refused', from: 'absent', to, reason: 'no row on this peer to grade' };
  }
  const from = row.kind === 'served' ? row.reach : row.requiredReach;
  if (from === null || from === '') {
    return {
      action: 'refused',
      from: '?',
      to,
      reason:
        row.kind === 'served'
          ? 'the row states no reach'
          : 'the reach gate refused without a requiredReach',
    };
  }
  if (!isReach(from)) {
    return { action: 'refused', from, to, reason: `unknown stored reach "${from}"` };
  }
  if (REACH_OPENNESS[from] >= REACH_OPENNESS[authored]) {
    // Never narrow: at or above the authored openness is left alone.
    return { action: 'current', from, to: authored };
  }
  return { action: 'needs-authorship', from, to: authored };
}

/** One candidate of `get_content_lineage` with hashes already base64-encoded. */
export interface LineageCandidateB64 {
  actionHash: string;
  author: string | null;
  fetchOutcome: string;
  inRoot: boolean;
}

/** `get_content_lineage` with hashes base64-encoded (only the fields read here). */
export interface LineageB64 {
  rootAuthor: string;
  contentId: string;
  candidates: LineageCandidateB64[];
  otherRootCandidates: number;
  unfetchableCandidates: number;
  invalidLinkTargets: number;
  truncated: boolean;
}

/** `resolve_content_head` with hashes base64-encoded (only the fields read here). */
export interface HeadB64 {
  headActionHash: string;
  author: string;
}

/**
 * Rule 4 — the own-authorship proof. Returns undefined when the node's own agent is
 * proven the author, else the reason it is not.
 *
 * The widen is a storage PATCH that re-signs through the conductor's `update_content`,
 * which supersedes the NEWEST `IdToContent` link for the id, whatever root it sits in.
 * So the proof covers every version that PATCH could build on, not just the elected
 * head. Every one of these must hold:
 *
 *   a. `resolve_content_head(id)` (Network) answers — the id has a version chain here;
 *   b. that head's action author is `me`;
 *   c. `get_content_lineage({ action_hash: head })` (Network) names `me` as the author
 *      of the head's exact root Create;
 *   d. the lineage saw the id's whole anchor: no other root claims the id, no link
 *      target was unfetchable or invalid, and the walk was not truncated;
 *   e. EVERY candidate on the id anchor was fetched, lies in that root, and was
 *      authored by `me` — no delegate's or foreign agent's update is in the chain.
 *
 * Anything unproven refuses; a refusal is re-tried on the next run, never forced.
 */
export function ownAuthorshipRefusal(
  me: string,
  head: HeadB64 | null,
  lineage: LineageB64 | null
): string | undefined {
  if (!head) return 'no version chain for this id on this conductor';
  if (head.author !== me)
    return `head ${short(head.headActionHash)} authored by ${short(head.author)}, not this node`;
  if (!lineage) return 'lineage of the head is not retrievable';
  if (lineage.rootAuthor !== me)
    return `root authored by ${short(lineage.rootAuthor)}, not this node`;
  if (lineage.otherRootCandidates > 0) {
    return `${lineage.otherRootCandidates} other root(s) claim this id`;
  }
  if (lineage.unfetchableCandidates > 0 || lineage.invalidLinkTargets > 0) {
    return `lineage incomplete (${lineage.unfetchableCandidates} unfetchable, ${lineage.invalidLinkTargets} invalid)`;
  }
  if (lineage.truncated) return 'lineage truncated';
  if (lineage.candidates.length === 0) return 'lineage has no candidates';
  for (const c of lineage.candidates) {
    if (c.fetchOutcome !== 'fetched' || !c.inRoot) {
      return `candidate ${short(c.actionHash)} is ${c.fetchOutcome}`;
    }
    if (c.author !== me) {
      return `version ${short(c.actionHash)} authored by ${short(c.author ?? '?')}, not this node`;
    }
  }
  return undefined;
}

/** The PATCH body a widen sends: the authored reach and nothing else. */
export function widenPatch(to: Reach): { reach: Reach } {
  return { reach: to };
}

/** A re-read after the PATCH confirms the widen only when it serves at exactly `to`. */
export function widenLanded(to: Reach, reread: ObservedRow): boolean {
  return reread.kind === 'served' && reread.reach === to;
}

function short(h: string): string {
  return h.length > 20 ? `${h.slice(0, 16)}…` : h;
}
