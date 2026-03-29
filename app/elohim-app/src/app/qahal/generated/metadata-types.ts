// AUTO-GENERATED from qahal manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen

export interface CollectiveMetadata {
  memberCount?: number;
  /** steward-consent | community-vote | constitutional | consensus */
  governanceModel?: string;
  /** individual | family | community | provincial | bioregional | global */
  constitutionalLayer?: string;
  /** H3 cell or place ID for geographic grounding */
  geoBoundary?: string;
  /** Parent collective in governance hierarchy */
  parentCollectiveId?: string;
  description?: string;
  /** private | community | public */
  visibility?: string;
  [key: string]: unknown;
}

export interface ProposalMetadata {
  /** consent | ranked-choice | score | approval | plurality | quadratic */
  mechanism?: string;
  /** Minimum participation ratio (0.0-1.0) */
  quorum?: number;
  /** ISO 8601 voting deadline */
  deadline?: string;
  /** Collective this proposal belongs to */
  collectiveId?: string;
  options?: Record<string, unknown>[];
  /** draft | open | closed | ratified | rejected */
  state?: string;
  constitutionalLayer?: string;
  [key: string]: unknown;
}

export interface ChallengeMetadata {
  /** EPR ID of the challenged artifact */
  targetEprId?: string;
  /** What is being challenged (manifest | content | decision) */
  targetType?: string;
  reason?: string;
  /** Which constitutional layer to escalate to */
  escalationPath?: string;
  /** filed | under-review | upheld | dismissed */
  state?: string;
  filedBy?: string;
  [key: string]: unknown;
}

export interface StatementMetadata {
  /** agree | disagree | pass */
  polarity?: string;
  /** How well this statement bridges opinion clusters (0.0-1.0) */
  bridgingScore?: number;
  /** Which opinion cluster this statement belongs to */
  clusterAffinity?: string;
  /** Parent deliberation/proposal this statement belongs to */
  deliberationId?: string;
  /** Whether this statement divides clusters */
  isDivisive?: boolean;
  [key: string]: unknown;
}
