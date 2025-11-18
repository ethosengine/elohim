/**
 * Attestations Model - Earned achievements and credentials
 *
 * Part of the Lamad learning platform's fog-of-war and earned access system.
 * Attestations represent proven capacity through completing learning journeys.
 *
 * Examples:
 * - "4th grade math mastery" (educational)
 * - "Ham Radio Technician/General/Extra" (skill certification)
 * - "AS, BS, MS, Dr. Degree Equivalent" (academic credentials)
 * - "Closest Intimate Partner" (relationship trust)
 * - "Civic Organizer Level 2" (proven community contribution)
 * - "Trauma Support Capacity" (emotional maturity)
 *
 * Future implementation for progressive revelation and earned content access.
 */

/**
 * Types of attestations across different domains
 */
export type AttestationType =
  | 'educational'      // Academic achievements (math mastery, grade levels, etc.)
  | 'skill'            // Technical or professional skills (ham radio, coding, etc.)
  | 'relational'       // Trust and relationship depth (intimate partner, friend, etc.)
  | 'civic'            // Community engagement and organizing capacity
  | 'professional'     // Work credentials and expertise
  | 'emotional'        // Emotional maturity and capacity (trauma support, etc.)
  | 'time-based'       // Sustained engagement over time (consistency, dedication)
  | 'social-proof';    // Community endorsements

/**
 * An earned attestation/achievement/badge
 */
export interface Attestation {
  id: string;
  name: string;
  description: string;
  type: AttestationType;

  /** The unique journey taken to earn this attestation */
  journey?: AttestationJourney;

  /** When this attestation was earned */
  earnedAt: Date;

  /** Optional expiration (some attestations may need renewal) */
  expiresAt?: Date;

  /** Who/what issued this attestation */
  issuedBy?: string;  // 'system', 'community', or specific steward ID

  /** Can this attestation be revoked? */
  revocable: boolean;

  /** Metadata for domain-specific extensions */
  metadata?: {
    [key: string]: any;
  };
}

/**
 * The journey taken to earn an attestation - this IS the proof of capacity
 */
export interface AttestationJourney {
  /** Content nodes visited/engaged with along the way */
  nodesVisited: string[];

  /** Starting and ending affinity levels */
  startingAffinity: { [nodeId: string]: number };
  endingAffinity: { [nodeId: string]: number };

  /** Practice exercises completed */
  exercisesCompleted?: string[];

  /** Real-world applications demonstrated */
  applicationsCompleted?: string[];

  /** Community endorsements received */
  endorsements?: Endorsement[];

  /** Time invested in the journey */
  timeInvested?: number;  // milliseconds

  /** Duration of the journey */
  startDate: Date;
  endDate: Date;
}

/**
 * Community endorsement as proof of capacity
 */
export interface Endorsement {
  endorserId: string;
  endorserName?: string;
  endorsedAt: Date;
  reason?: string;
  weight?: number;  // Some endorsers may have more weight based on their own attestations
}

/**
 * Requirements to earn an attestation
 */
export interface AttestationRequirement {
  /** Minimum affinity level with specific content nodes */
  requiredAffinity?: { [nodeId: string]: number };

  /** Other attestations that must already be earned (prerequisites) */
  prerequisiteAttestations?: string[];

  /** Number of practice exercises that must be completed */
  minimumExercises?: number;

  /** Number of real-world applications required */
  minimumApplications?: number;

  /** Number of community endorsements required */
  minimumEndorsements?: number;

  /** Minimum time investment required */
  minimumTimeInvested?: number;  // milliseconds

  /** Minimum sustained engagement period */
  minimumDuration?: number;  // milliseconds
}

/**
 * Content access requirements - what attestations unlock which content
 * (Smart contracts for human flourishing, negotiated by agents, expressed in plain text)
 */
export interface ContentAccessRequirement {
  contentNodeId: string;

  /** Attestations required to view this content (OR logic) */
  requiredAttestations?: string[];

  /** All of these attestations required (AND logic) */
  requiredAllAttestations?: string[];

  /** Minimum affinity with prerequisite content */
  prerequisiteAffinity?: { [nodeId: string]: number };

  /** Community endorsements as alternative access */
  alternativeEndorsements?: {
    count: number;
    fromAttestationHolders?: string[];  // Must be from people with these attestations
  };

  /** Who manages this access requirement (steward) */
  steward: string;  // Node ID or special identifier

  /** Can this requirement be revoked? */
  revocable: boolean;

  /** Why this content requires earned access */
  reason?: string;

  /** Plain text explanation for users */
  explanation?: string;
}

/**
 * User's collection of earned attestations
 */
export interface UserAttestations {
  userId: string;
  attestations: Attestation[];
  lastUpdated: Date;
}

/**
 * Progress toward earning an attestation
 */
export interface AttestationProgress {
  attestationId: string;
  requirement: AttestationRequirement;
  currentProgress: {
    affinityProgress: { [nodeId: string]: number };
    exercisesCompleted: number;
    applicationsCompleted: number;
    endorsementsReceived: number;
    timeInvested: number;
    durationSoFar: number;
  };
  percentComplete: number;
  estimatedCompletion?: Date;
}
