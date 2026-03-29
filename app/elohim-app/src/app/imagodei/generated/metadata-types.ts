// AUTO-GENERATED from imagodei manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run imagodei:codegen

export interface HumanMetadata {
  /** Human-readable display name */
  displayName?: string;
  /** Self-authored biography or description */
  bio?: string;
  /** Self-declared location (free text, not verified) */
  location?: string;
  /** Current agency stage: visitor | observer | participant | steward | elder */
  agencyStage?: string;
  /** How visible this profile is (maps to protocol reach levels) */
  profileReach?: string;
  /** Identity category: person | organization | collective */
  category?: string;
  /** Content domains this human has demonstrated affinity with */
  affinities?: string[];
  /** External identity links (email, DID, social handles). Key is provider, value is identifier. */
  externalIdentifiers?: Record<string, unknown>;
  [key: string]: unknown;
}

export interface PresenceMetadata {
  /** Presence lifecycle: unclaimed | claimed | active | dormant */
  presenceState?: string;
  /** Content IDs that established this contributor presence (first contributions) */
  establishingContentIds?: string[];
  /** Accumulated affinity score from curation acts (not attention) */
  affinityTotal?: number;
  /** Count of unique humans who have engaged with this contributor's content */
  uniqueEngagers?: number;
  /** Number of times this contributor's content has been cited or referenced */
  citationCount?: number;
  /** Aggregate recognition from economic events flowing to this contributor */
  recognitionScore?: number;
  [key: string]: unknown;
}
