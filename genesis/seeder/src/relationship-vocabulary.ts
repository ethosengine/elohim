/**
 * Relationship-type canonicalization for the seeder's extractor.
 *
 * The lamad manifest (`elohim/sdk/domains/lamad/manifest/relationships.json`,
 * projected here as `LAMAD_RELATIONSHIPS`) is the source of truth for the
 * relationship vocabulary, and elohim-storage's `relationship_service` accepts
 * exactly that set (plus legacy ids). Content authors, however, write
 * lower-case, prose-shaped types (`extends`, `derived_from`, `prereq`) — until
 * 2026-08-28 those went to storage upper-cased and verbatim, the bulk create
 * answered `HTTP 400 relationship_type 'EXTENDS' is not valid`, and the WHOLE
 * local relationship graph was dropped on the first bad item.
 *
 * `canonicalRelationshipType` maps authored types onto the manifest: exact
 * manifest ids pass through; known aliases map to the closest manifest id;
 * anything else lands on `RELATES_TO` ("used when a more specific relationship
 * type does not apply") and is COUNTED, never silently dropped.
 */
import { LAMAD_RELATIONSHIPS } from './generated/manifest-types.js';

const MANIFEST = new Set<string>(LAMAD_RELATIONSHIPS as readonly string[]);

/** Authored-alias → manifest id. Keys are compared lower-case, `-`/space → `_`. */
const ALIASES: Record<string, string> = {
  // "builds on / extends X" — the source needs X for completeness
  extends: 'DEPENDS_ON',
  extend: 'DEPENDS_ON',
  derived_from: 'DEPENDS_ON',
  derives_from: 'DEPENDS_ON',
  // learner prerequisite
  prereq: 'REQUIRES',
  prerequisite: 'REQUIRES',
  requires: 'REQUIRES',
  // legacy upper-case ids the older validator accepted
  followup: 'FOLLOWS',
  follow_up: 'FOLLOWS',
  parent: 'BELONGS_TO',
  child: 'CONTAINS',
  relates: 'RELATES_TO',
  references: 'REFERENCES',
  contains: 'CONTAINS',
  implements: 'IMPLEMENTS',
  validates: 'VALIDATES',
  describes: 'DESCRIBES',
  attached_to: 'ATTACHED_TO',
  belongs_to: 'BELONGS_TO',
};

export interface CanonicalRelationship {
  /** A manifest relationship id, always. */
  type: string;
  /** How it got there — `manifest` (already canonical), `alias`, or `fallback`. */
  via: 'manifest' | 'alias' | 'fallback';
  /** The authored value, normalized, for the remap summary. */
  authored: string;
}

export function canonicalRelationshipType(raw: unknown): CanonicalRelationship {
  const authored = String(raw ?? 'RELATES_TO').trim();
  const upper = authored.toUpperCase().replace(/[-\s]+/g, '_');
  if (MANIFEST.has(upper)) return { type: upper, via: 'manifest', authored };
  const alias = ALIASES[authored.toLowerCase().replace(/[-\s]+/g, '_')];
  if (alias) return { type: alias, via: 'alias', authored };
  return { type: 'RELATES_TO', via: 'fallback', authored };
}

/** Accumulates remaps so the seeder can print ONE summary line instead of silence. */
export class RelationshipRemapLedger {
  private readonly counts = new Map<string, number>();
  note(c: CanonicalRelationship): void {
    if (c.via === 'manifest') return;
    const key = `${c.authored} → ${c.type} (${c.via})`;
    this.counts.set(key, (this.counts.get(key) ?? 0) + 1);
  }
  summary(): string | null {
    if (this.counts.size === 0) return null;
    return [...this.counts.entries()]
      .sort((a, b) => b[1] - a[1])
      .map(([k, n]) => `${n}× ${k}`)
      .join(', ');
  }
}
