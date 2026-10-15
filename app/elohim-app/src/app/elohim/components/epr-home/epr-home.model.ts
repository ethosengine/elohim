/**
 * View model for the EPR atom home. Projected from the RAW ContentView wire
 * shape (GET /db/content/{id}) — read as delivered, no case conversion. The
 * frame computes display words only; identity, reach and validation are
 * backend-owned and shown verbatim.
 */

import type { ResilienceSnapshotView } from '@app/generated/resilience-snapshot-view';
import type {
  ChallengeView,
  ContentEngagementStatsView,
  VerificationResultView,
} from '@elohim/storage-client/generated';
import type { ResilienceView } from '@elohim/storage-client/generated/ResilienceView';

export type FocalShape = 'immersive' | 'reading';

const IMMERSIVE_FORMATS = new Set(['html5-app', 'video', 'audio', 'external', 'sophia-quiz-json']);

export function focalShape(contentFormat: string): FocalShape {
  return IMMERSIVE_FORMATS.has(contentFormat) ? 'immersive' : 'reading';
}

const REACH_SUBTITLES: Record<string, string> = {
  commons: 'anyone can reach this',
  collective: 'members of the collective',
  invited: 'people who were invited',
  familiar: 'households that know each other',
  trusted: 'trusted households',
  intimate: 'the household',
  private: 'only its steward',
  self: 'only its steward',
};

export function reachSubtitle(reach: string): string {
  return REACH_SUBTITLES[reach] ?? '';
}

export function anchorWords(trust: string | null, dhtAnchorState: string | null): string {
  if (trust !== 'notarized') return 'Not yet notarized';
  return dhtAnchorState === 'verified' ? 'anchor verified here' : 'anchor not yet verified here';
}

export interface EprHomeAtom {
  id: string;
  title: string;
  description: string;
  contentType: string;
  contentFormat: string;
  shape: FocalShape;
  reach: string;
  trust: string | null;
  dhtAnchorHash: string | null;
  dhtAnchorState: string | null;
  validationStatus: string | null;
  blobHash: string | null;
  blobCid: string | null;
  contentSizeBytes: number | null;
  createdAt: string;
  updatedAt: string;
  author: string | null;
  license: string | null;
  sourceUrl: string | null;
  canonicalUrl: string | null;
  estimatedTime: string | null;
  category: string | null;
  relatedIds: string[];
  tags: string[];
  version: string | null;
}

function str(v: unknown): string | null {
  return typeof v === 'string' && v.length > 0 ? v : null;
}

function versionOf(v: unknown): string | null {
  if (typeof v === 'number' && Number.isFinite(v)) return String(v);
  return str(v);
}

export function toAtom(raw: Record<string, unknown>): EprHomeAtom {
  const metadata = (raw['metadata'] as Record<string, unknown> | null) ?? {};
  const related = metadata['relatedNodeIds'];
  const contentFormat = String(raw['contentFormat'] ?? '');
  const authors = metadata['authors'];
  return {
    id: String(raw['id'] ?? ''),
    title: String(raw['title'] ?? raw['id'] ?? ''),
    description: String(raw['description'] ?? ''),
    contentType: String(raw['contentType'] ?? ''),
    contentFormat,
    shape: focalShape(contentFormat),
    reach: String(raw['reach'] ?? 'commons'),
    trust: str(raw['trust']),
    dhtAnchorHash: str(raw['dhtAnchorHash']),
    dhtAnchorState: str(raw['dhtAnchorState']),
    validationStatus: str(raw['validationStatus']),
    blobHash: str(raw['blobHash']),
    blobCid: str(raw['blobCid']),
    contentSizeBytes: typeof raw['contentSizeBytes'] === 'number' ? raw['contentSizeBytes'] : null,
    createdAt: String(raw['createdAt'] ?? ''),
    updatedAt: String(raw['updatedAt'] ?? ''),
    author: str(metadata['author']) ?? (Array.isArray(authors) ? authors.join(', ') : null),
    license: str(metadata['license']),
    sourceUrl: str(metadata['sourceUrl']),
    canonicalUrl: str(metadata['canonicalUrl']),
    estimatedTime: str(metadata['estimatedTime']),
    category: str(metadata['category']),
    relatedIds: Array.isArray(related) ? (related as string[]) : [],
    tags: Array.isArray(raw['tags'])
      ? (raw['tags'] as unknown[]).filter((t): t is string => typeof t === 'string' && !!t)
      : [],
    version: versionOf(metadata['version']),
  };
}

/** Short anchor for display: first 12 + last 8 characters. */
export function shortAnchor(hash: string): string {
  return hash.length <= 24 ? hash : `${hash.slice(0, 12)}…${hash.slice(-8)}`;
}

/** "May 27, 2026" from "2026-05-27 20:46:37" or ISO. */
export function dayWords(stamp: string): string {
  const d = new Date(stamp.replace(' ', 'T'));
  if (Number.isNaN(d.getTime())) return stamp;
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
}

export interface StewardRow {
  stewardPresenceId: string;
  contributionType: string;
  effectiveFrom: string;
  /** The steward presence's display name, when the allocation carries it. */
  displayName?: string | null;
  /** Share of stewardship, 0..1, as the allocation states it. */
  allocationRatio?: number;
  recognitionAccumulated?: number;
  /** The dispute's reason, when the allocation is under dispute. */
  disputeReason?: string | null;
  disputed?: boolean;
}

export interface HoldingWords {
  /** 'unknown' = not yet asked (no snapshot request has resolved). 'known' =
   * asked, whether that turned up a felt status or came back empty. */
  state: 'unknown' | 'known';
  headline: string;
  has: number;
  wants: number;
  warm: boolean;
  households: string[];
  action: string;
}

/** The ONE holding verdict on the page, in household words (spec §2.2). */
export function holdingWords(snapshot: ResilienceSnapshotView | null | undefined): HoldingWords {
  if (snapshot === undefined) {
    return {
      state: 'unknown',
      headline: 'Checking who holds this…',
      has: 0,
      wants: 3,
      warm: false,
      households: [],
      action: '',
    };
  }
  const felt = snapshot?.feltStatus;
  if (!felt) {
    return {
      state: 'known',
      headline: "We can't confirm this is backed up anywhere yet.",
      has: 0,
      wants: 3,
      warm: true,
      households: [],
      action: 'Invite a household to help hold this',
    };
  }
  return {
    state: 'known',
    headline: felt.headline,
    has: felt.floor.hasHouseholds,
    wants: felt.floor.wantsHouseholds,
    warm: felt.reassurance === 'needs-help' || felt.reassurance === 'not-yet-seen',
    households: felt.heldBy.map(h =>
      h.intraHubPeers ? `${h.label ?? h.id} · ${h.intraHubPeers} peers` : (h.label ?? h.id)
    ),
    action: felt.suggestedAction ?? '',
  };
}

export function heldChip(words: HoldingWords): string {
  if (words.state === 'unknown') return 'Checking who holds this…';
  return words.has === 0
    ? 'Not yet held by any household'
    : `Held by ${words.has} of ${words.wants} households`;
}

// ── Folds (spec §11.1) ──────────────────────────────────────────────────────

/** The legs that carry a fold; "here" is addressable but fold-less. */
export type FoldLeg = 'holds' | 'lives' | 'governed' | 'from';
export type LegAnchor = FoldLeg | 'here';

const LEG_ANCHORS: ReadonlySet<string> = new Set(['holds', 'lives', 'governed', 'from', 'here']);

/** The retired viewer's tab fragments, mapped onto the leg that now carries each concern. */
const LEGACY_TAB_FRAGMENTS: ReadonlyMap<string, LegAnchor | null> = new Map([
  ['network', 'holds'],
  ['governance', 'governed'],
  ['trust', 'from'],
  ['content', null],
]);

/** A route fragment as the leg it opens; null when it names no leg. */
export function legFromFragment(fragment: string | null | undefined): LegAnchor | null {
  if (!fragment) return null;
  if (LEGACY_TAB_FRAGMENTS.has(fragment)) return LEGACY_TAB_FRAGMENTS.get(fragment) ?? null;
  return LEG_ANCHORS.has(fragment) ? (fragment as LegAnchor) : null;
}

// ── Network detail (Who holds it → fold) ────────────────────────────────────

/** bigint-typed wire fields arrive as JSON numbers. */
function num(v: bigint | number | null | undefined): number {
  const n = Number(v ?? 0);
  return Number.isFinite(n) ? n : 0;
}

export interface HealthWords {
  line: string;
  warm: boolean;
}

const HEALTH_HEADS: Readonly<Record<string, string>> = {
  healthy: 'Healthy',
  degraded: 'Thinning',
  at_risk: 'At risk',
};

/** The storage peer's own health verdict, in words first (spec §11.2). */
export function healthWords(r: ResilienceView): HealthWords {
  const head = HEALTH_HEADS[r.health.status] ?? 'Not yet known';
  const n = r.health.canSurviveFailures;
  const peers = n === 1 ? 'peer' : 'peers';
  const tail =
    n > 0
      ? `can lose ${n} ${peers} and still be whole`
      : 'losing one more peer could leave it incomplete';
  return { line: `${head} · ${tail}`, warm: r.health.status !== 'healthy' };
}

/** "12 GB" — storage committed, as a size, never a percentage. */
export function sizeWords(bytes: bigint | number): string {
  const b = num(bytes);
  if (b >= 1_073_741_824) return `${Math.round(b / 1_073_741_824)} GB`;
  if (b >= 1_048_576) return `${Math.round(b / 1_048_576)} MB`;
  if (b >= 1024) return `${Math.round(b / 1024)} KB`;
  return `${b} bytes`;
}

function pieces(n: number): string {
  return n === 1 ? 'piece' : 'pieces';
}

/** The "is it still whole" check, in one line; carries the peer's error when there is one. */
export function verifyLine(v: VerificationResultView): string {
  const base = v.verified
    ? `Whole · ${v.shardsLocated} of ${v.shardsNeeded} ${pieces(v.shardsNeeded)} found · ${num(v.reconstructionTimeMs)} ms`
    : `Not whole · ${v.shardsMissing} ${pieces(v.shardsMissing)} missing`;
  return v.error ? `${base} · ${v.error}` : base;
}

// ── Who's here (attention) ──────────────────────────────────────────────────

/** "about 1 in 3 who viewed it finished" — the completion rate in words, never a percentage. */
export function completionWords(rate: number): string | null {
  // A non-finite rate (NaN) is "not counted", like zero.
  if (!Number.isFinite(rate) || rate <= 0) return null;
  if (rate >= 0.95) return 'nearly everyone who viewed it finished';
  const oneIn = Math.round(1 / rate);
  return oneIn <= 1 ? 'most who viewed it finished' : `about 1 in ${oneIn} who viewed it finished`;
}

/** The attention line, or the one-line empty state. */
export function attentionLine(stats: ContentEngagementStatsView | null | undefined): string {
  const views = num(stats?.views);
  if (!stats || views === 0) return 'No one has been counted here yet.';
  const people = num(stats.uniqueViewers);
  const completions = num(stats.completions);
  const parts = [
    `Viewed ${views} ${views === 1 ? 'time' : 'times'} by ${people} ${people === 1 ? 'person' : 'people'}`,
    `${completions} ${completions === 1 ? 'completion' : 'completions'}`,
  ];
  const finished = completionWords(stats.completionRate);
  if (finished) parts.push(finished);
  return parts.join(' · ');
}

// ── How it's governed ───────────────────────────────────────────────────────

/** Labels from the governance state's JSON, as plain words. */
export function labelWords(labels: unknown): string[] {
  if (!Array.isArray(labels)) return [];
  return labels
    .map((l: unknown) => {
      if (typeof l === 'string') return l;
      if (l && typeof l === 'object') {
        const o = l as Record<string, unknown>;
        return str(o['labelType']) ?? str(o['label']) ?? str(o['name']);
      }
      return null;
    })
    .filter((x): x is string => !!x)
    .map(w => w.replace(/[_-]/g, ' '));
}

/** Days left on a challenge's response deadline, in words; null without a deadline. */
export function daysRemainingWords(c: ChallengeView, now: number = Date.now()): string | null {
  const deadline = c.responseDeadline ?? c.slaDeadline;
  if (!deadline) return null;
  const at = new Date(deadline.replace(' ', 'T')).getTime();
  if (Number.isNaN(at)) return null;
  const days = Math.ceil((at - now) / 86_400_000);
  if (days < 0) return 'past its response time';
  if (days === 0) return 'response due today';
  return days === 1 ? '1 day left to respond' : `${days} days left to respond`;
}

/** A conversation's row title: its title, else the first line of its first message. */
export function discussionTitle(d: { title: string | null; body: string }): string {
  if (d.title) return d.title;
  const first =
    d.body
      .split('\n')
      .find(l => l.trim().length > 0)
      ?.trim() ?? '';
  if (!first) return 'Untitled conversation';
  return first.length > 120 ? `${first.slice(0, 117)}…` : first;
}
