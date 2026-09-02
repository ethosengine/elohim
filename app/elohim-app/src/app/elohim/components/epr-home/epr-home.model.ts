/**
 * View model for the EPR atom home. Projected from the RAW ContentView wire
 * shape (GET /db/content/{id}) — read as delivered, no case conversion. The
 * frame computes display words only; identity, reach and validation are
 * backend-owned and shown verbatim.
 */

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
}

function str(v: unknown): string | null {
  return typeof v === 'string' && v.length > 0 ? v : null;
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
