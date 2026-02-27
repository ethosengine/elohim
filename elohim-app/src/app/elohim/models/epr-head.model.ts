/**
 * EPR Head — IPLD-compatible content metadata.
 *
 * An EPR Head is the mutable, gossip-friendly metadata envelope for
 * content in the Elohim Protocol. It carries three-pillar context
 * (lamad/shefa/qahal) alongside standard IPLD links to immutable
 * content bytes.
 *
 * This type mirrors the IPLD DAG-CBOR schema defined in
 * protocol-specification.md Appendix F. When serialized as DAG-CBOR:
 * - `content` becomes an IPLD CID link: { "/": "bafkrei..." }
 * - `relationships[].target` become IPLD CID links
 * - All other fields serialize as standard CBOR
 *
 * See: protocol-specification.md Appendix E (EPR URI) and Appendix F (IPLD Alignment)
 */

// ── IPLD Content Address ─────────────────────────────────────────────────────

// Content addresses are plain strings: CIDv1 (bafkrei...) or legacy sha256-{hex}.
// In IPLD DAG-CBOR, these serialize as CID links: { "/": "bafkrei..." }.
// In our current JSON wire format, they're plain strings.

// ── Three-Pillar Context ─────────────────────────────────────────────────────

/** Lamad pillar — knowledge context. What this content is and how it relates. */
export interface EprLamadContext {
  /** Human-readable title */
  title: string;
  /** Content type: concept, epic, path, assessment, simulation, etc. */
  contentType: string;
  /** Short description */
  description?: string;
  /** Content format: markdown, sophia, html5-app, etc. */
  contentFormat?: string;
  /** Discovery tags */
  tags?: string[];
}

/** Shefa pillar — value context. Who stewards this content and what recognition flows. */
export interface EprShefaContext {
  /** DIDs of stewards responsible for this content */
  stewards?: string[];
  /** Stewardship allocation percentages (parallel to stewards array) */
  allocations?: number[];
}

/** Qahal pillar — governance context. What authority ratified this and what reach applies. */
export interface EprQahalContext {
  /** Access level: personal, trusted, community, commons */
  reach?: string;
  /** Constitutional layer: personal, communal, global */
  layer?: string;
}

// ── Relationships ────────────────────────────────────────────────────────────

/** Typed relationship to another EPR Head. */
export interface EprRelationship {
  /** Relationship type: PREREQUISITE, TEACHES, CONTAINS, REFERENCES, etc. */
  type: string;
  /** Target content ID (resolves to another EPR Head) */
  target: string;
  /** Optional target content address (CID link in IPLD) */
  targetCid?: string;
}

// ── EPR Head ─────────────────────────────────────────────────────────────────

/**
 * The EPR Head — the ~500B gossip-friendly metadata envelope.
 *
 * In the IPLD ecosystem, this is a DAG-CBOR document with CID links.
 * The three-pillar fields (lamad, shefa, qahal) are EPR's extension
 * to standard IPLD — any IPLD tool can traverse the links, but only
 * EPR-aware tools understand the pillar semantics.
 */
export interface EprHead {
  /** Schema version for forward compatibility */
  version: number;

  /** Stable content ID (slug) */
  id: string;

  /** CID of the current content bytes (IPLD link to Tier 3) */
  content: string;

  /** Knowledge pillar context */
  lamad: EprLamadContext;

  /** Value pillar context */
  shefa: EprShefaContext;

  /** Governance pillar context */
  qahal: EprQahalContext;

  /** Typed relationships to other content */
  relationships: EprRelationship[];

  /** Author DID */
  author?: string;

  /** ISO 8601 timestamp of last update */
  updated?: string;
}
