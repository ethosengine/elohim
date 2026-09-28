/**
 * Steward grade — the pure decisions, kept apart from the I/O so they can be tested
 * without a mesh.
 *
 * A genesis peer grades its OWN rows (scripts/steward-grade.ts): a row stored narrower
 * than the reach its repo file authors is widened to exactly that authored reach,
 * through the node's own conductor, and only when the conductors prove the row is the
 * genesis stewards' own. Four rules live here:
 *
 *   1. Commons fence. Only an authored reach of `commons` or `public` is ever graded
 *      toward. Any other authored word (intimate, community, an unknown word, none) is
 *      refused — this step opens content, it never decides how open intimate content is.
 *   2. Never narrow. A row already at or above the authored openness is `current`.
 *   3. Exactly the authored reach. A widen targets the authored reach, never more.
 *   4. Steward authorship, proven by conductors (see `stewardAuthorshipRefusal` and
 *      `decideWiden`). The genesis corpus is co-authored by the operator's genesis
 *      peers, each seeding the same ids through its own agent onto the shared lamad
 *      DHT. So the steward set is this node's agent plus each co-steward's agent, and
 *      every key in it is read from that peer's OWN conductor admin interface. Storage
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
 * The agents whose authorship a grade admits: this node's own agent, and each
 * co-steward's agent (another genesis peer), every key read from that peer's own
 * conductor. A co-steward whose conductor did not answer is simply absent: the ids its
 * agent touched stay refused, which is safe.
 */
export interface StewardSet {
  me: string;
  /** Co-steward agent key → where it was read (its conductor admin WS). */
  coStewards: ReadonlyMap<string, string>;
}

export function stewardSet(me: string, coStewards: Iterable<[string, string]> = []): StewardSet {
  const m = new Map<string, string>();
  for (const [k, src] of coStewards) if (k !== me) m.set(k, src);
  return { me, coStewards: m };
}

function isSteward(s: StewardSet, agent: string | null): boolean {
  return agent !== null && (agent === s.me || s.coStewards.has(agent));
}

function outsideSet(s: StewardSet): string {
  const n = s.coStewards.size;
  return n === 0 ? 'not this node' : `not this node or one of its ${n} co-steward(s)`;
}

/**
 * Rule 4 over an existing version chain. Returns undefined when every version the
 * widen could build on is proven a steward's, else the reason it is not.
 *
 * The widen is a storage PATCH that re-signs through the conductor's `update_content`,
 * which supersedes the NEWEST `IdToContent` link for the id, whatever root it sits in.
 * So the proof covers every version that PATCH could build on, not just the elected
 * head. Every one of these must hold:
 *
 *   a. `resolve_content_head(id)` (Network) answered with a head;
 *   b. that head's action author is in the steward set;
 *   c. `get_content_lineage({ action_hash: head })` (Network) names a steward as the
 *      author of the head's exact root Create;
 *   d. the lineage saw the id's whole anchor: no other root claims the id, no link
 *      target was unfetchable or invalid, and the walk was not truncated;
 *   e. EVERY candidate on the id anchor was fetched, lies in that root, and was
 *      authored by a steward — no delegate's or foreign agent's update is in the chain.
 *
 * Anything unproven refuses; a refusal is re-tried on the next run, never forced.
 */
export function stewardAuthorshipRefusal(
  stewards: StewardSet,
  head: HeadB64,
  lineage: LineageB64 | null
): string | undefined {
  const not = outsideSet(stewards);
  if (!isSteward(stewards, head.author))
    return `head ${short(head.headActionHash)} authored by ${short(head.author)}, ${not}`;
  if (!lineage) return 'lineage of the head is not retrievable';
  if (!isSteward(stewards, lineage.rootAuthor))
    return `root authored by ${short(lineage.rootAuthor)}, ${not}`;
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
    if (!isSteward(stewards, c.author)) {
      return `version ${short(c.actionHash)} authored by ${short(c.author ?? '?')}, ${not}`;
    }
  }
  return undefined;
}

/** What this node's conductor answered about the id's version chain. */
export type ChainAnswer =
  /** The zome said "not yet" (PENDING / not retrievable): retry on a later run. */
  | { kind: 'pending'; detail: string }
  /** `resolve_content_head` (Network) answered: no version chain for this id. */
  | { kind: 'no-chain' }
  /** A head, and the lineage of its root (null when it could not be read). */
  | { kind: 'chain'; head: HeadB64; lineage: LineageB64 | null };

export type WidenDecision =
  /** Widen over the existing chain: `update_content` re-signs as this node. */
  | { action: 'widen'; mode: 'chain'; from: Reach; to: Reach }
  /**
   * The row is in storage but no chain exists on the DHT: the PATCH publishes this
   * node's OWN root (`create_content`), carrying the authored reach from the start.
   */
  | { action: 'widen'; mode: 'new-root'; from: Reach; to: Reach }
  | { action: 'current'; from: string; to: Reach }
  | { action: 'refused'; from: string; to: string; reason: string };

/**
 * Rules 1–4 together: the row's verdict (rules 1–3, `classifyRow`) and, for a row that
 * needs it, the conductor's answer about its chain (rule 4).
 *
 *   - `current` / `refused` verdicts pass through unchanged: a row is never narrowed,
 *     and nothing outside the commons fence is ever graded, whatever the chain says.
 *   - `pending` refuses (retry later) — a "not yet" is never read as "no chain".
 *   - `no-chain` widens as a NEW ROOT authored by this node. The anonymous read already
 *     showed the row exists here at a narrower reach, so the row is real content this
 *     node holds; no other agent's chain exists to be superseded, and the zome's
 *     `create_content` refuses if one appears in the meantime.
 *   - `chain` widens only when `stewardAuthorshipRefusal` admits it.
 */
export function decideWiden(
  verdict: RowVerdict,
  stewards: StewardSet,
  answer: ChainAnswer
): WidenDecision {
  if (verdict.action !== 'needs-authorship') return verdict;
  const { from, to } = verdict;
  switch (answer.kind) {
    case 'pending':
      return {
        action: 'refused',
        from,
        to,
        reason: `authorship pending on the DHT: ${answer.detail}`,
      };
    case 'no-chain':
      return { action: 'widen', mode: 'new-root', from, to };
    case 'chain': {
      const why = stewardAuthorshipRefusal(stewards, answer.head, answer.lineage);
      return why
        ? { action: 'refused', from, to, reason: why }
        : { action: 'widen', mode: 'chain', from, to };
    }
  }
}

/** The PATCH body a widen sends: the authored reach and nothing else. */
export function widenPatch(to: Reach): { reach: Reach } {
  return { reach: to };
}

/** A re-read after the PATCH confirms the widen only when it serves at exactly `to`. */
export function widenLanded(to: Reach, reread: ObservedRow): boolean {
  return reread.kind === 'served' && reread.reach === to;
}

// ─── PATCH retry ladder ───────────────────────────────────────────────────────

/** Waits a shed PATCH may spend before it is reported `failed`. */
export const SHED_MAX_WAITS = 12;

/**
 * A PATCH the peer shed rather than refused: 503 with the projector `catching-up`, or a
 * zome call that timed out on the conductor websocket. Both are waited out on a bounded
 * ladder (the seeder's pattern). Anything else is final.
 */
export function isShedPatch(status: number, bodyText: string): boolean {
  if (status !== 503) return false;
  try {
    const body = JSON.parse(bodyText) as { status?: unknown; error?: unknown };
    if (body?.status === 'catching-up') return true;
    return typeof body?.error === 'string' && /websocket error: timeout/i.test(body.error);
  } catch {
    return false;
  }
}

/** Honour Retry-After (header seconds, else the body's `retryAfter`), default 5 s, 2–15 s. */
export function shedDelayMs(retryAfterHeader: string | null, bodyText: string): number {
  let secs = retryAfterHeader ? Number.parseInt(retryAfterHeader, 10) : Number.NaN;
  if (!Number.isFinite(secs)) {
    try {
      const b = JSON.parse(bodyText) as { retryAfter?: unknown };
      if (typeof b?.retryAfter === 'number') secs = b.retryAfter;
    } catch {
      /* no body hint */
    }
  }
  return Math.min(Number.isFinite(secs) ? Math.max(secs, 2) * 1000 : 5000, 15_000);
}

function short(h: string): string {
  return h.length > 20 ? `${h.slice(0, 16)}…` : h;
}
