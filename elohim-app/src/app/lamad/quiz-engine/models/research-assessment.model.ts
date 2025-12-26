/**
 * Research Assessment Model - Individual Research Participation
 *
 * This module handles individual-level research participation:
 * - Personal consent preferences for data sharing
 * - Research instrument frameworks (psychometric, sociological)
 * - Individual response storage
 *
 * For collective/relational research (dyads, groups, networks):
 * @see @app/qahal/models/collective-research.model.ts
 *
 * For personal discovery attestations:
 * @see ./discovery-assessment.model.ts
 */

import type { DiscoveryCategory, DiscoveryFramework } from './discovery-assessment.model';

// =============================================================================
// Research Categories & Frameworks (Individual Instruments)
// =============================================================================

/**
 * Categories of research assessments.
 * Extends personal discovery with research-oriented categories.
 */
export type ResearchCategory =
  | DiscoveryCategory          // Personal discovery categories
  | 'moral-political'          // Moral foundations, political compass
  | 'social-attitudes'         // Trust, civic engagement attitudes
  | 'well-being'               // Life satisfaction, affect, flourishing
  | 'cognitive-style'          // Thinking patterns, decision-making
  | 'behavioral-tendencies';   // Habit patterns, self-regulation

/**
 * Well-known research assessment frameworks.
 * These are individual-administered instruments.
 */
export type ResearchFramework =
  | DiscoveryFramework         // Personal discovery frameworks
  | 'moral-foundations-mfq2'   // Moral Foundations Questionnaire 2
  | 'political-compass'        // Political/economic spectrum
  | 'trust-scale'              // Generalized trust measures
  | 'civic-engagement'         // Civic engagement scales
  | 'wellbeing-wemwbs'         // Warwick-Edinburgh Mental Wellbeing
  | 'flourishing-perma'        // Seligman's PERMA model
  | 'attachment-ecr-r'         // Experiences in Close Relationships - Revised
  | 'big-five-ocean'           // OCEAN personality (research version)
  | 'need-for-cognition'       // Thinking enjoyment scale
  | 'self-efficacy'            // Bandura's self-efficacy
  | 'custom-research';         // Custom research instrument

// =============================================================================
// Individual Research Consent
// =============================================================================

/**
 * Personal consent level for research data sharing.
 * Stored in ImagoDei as part of identity preferences.
 */
export type PersonalResearchConsent =
  | 'none'                    // No research participation
  | 'aggregate-only'          // Only anonymized aggregates
  | 'pseudonymous'            // Pseudonymized individual data
  | 'identifiable'            // Identifiable with explicit per-study consent
  | 'longitudinal'            // Allow re-contact for follow-up
  | 'open-science';           // Contribute to open datasets

/**
 * Personal research participation preferences.
 * Stored in ImagoDei identity settings.
 */
export interface ResearchPreferences {
  /** Default consent level */
  defaultConsent: PersonalResearchConsent;

  /** Categories willing to participate in */
  participationCategories: ResearchCategory[];

  /** Allow experience sampling prompts? */
  allowESMPrompts: boolean;

  /** Maximum ESM prompts per day (if allowed) */
  maxESMPromptsPerDay?: number;

  /** Preferred prompt times (if ESM allowed) */
  preferredPromptTimes?: {
    start: string;  // HH:mm format
    end: string;
  };

  /** Allow matching with others (dyads, groups)? */
  allowMatching: boolean;

  /** Matching preferences (if allowed) */
  matchingPreferences?: {
    dyadTypes: DyadMatchingPreference[];
    groupSizeRange?: [number, number];
  };

  /** Topics/studies to exclude */
  excludedTopics?: string[];

  /** Allow commercial research use? */
  allowCommercialUse: boolean;

  /** Receive notifications about research opportunities? */
  notifyOpportunities: boolean;
}

/**
 * Dyad matching preferences for relational research.
 */
export interface DyadMatchingPreference {
  /** Type of dyad willing to form */
  type: 'romantic-partner' | 'friend' | 'family' | 'mentor-mentee' | 'any';

  /** How to match */
  matchingMethod: 'invite-specific' | 'platform-match' | 'open-enrollment';

  /** Role preference (for distinguishable dyads) */
  rolePreference?: 'either' | 'role1' | 'role2';
}

// =============================================================================
// Individual Research Response
// =============================================================================

/**
 * An individual's response to a research instrument.
 * Can be aggregated with others in Qahal research pools.
 */
export interface ResearchResponse {
  /** Unique response ID */
  id: string;

  /** Assessment/instrument ID */
  assessmentId: string;

  /** Framework used */
  framework: ResearchFramework;

  /** Human ID (may be pseudonymized in exports) */
  humanId: string;

  /** Response timestamp */
  completedAt: string;

  /** Item-level responses */
  items: Record<string, unknown>;

  /** Computed subscale scores */
  subscaleScores: Record<string, number>;

  /** Context at time of response (optional) */
  context?: ResponseContext;

  /** Consent level for this specific response */
  consentLevel: PersonalResearchConsent;

  /** Study ID (if part of a coordinated study) */
  studyId?: string;

  /** Wave number (for longitudinal studies) */
  wave?: number;
}

/**
 * Context captured at response time.
 */
export interface ResponseContext {
  /** Time of day */
  timeOfDay?: 'morning' | 'afternoon' | 'evening' | 'night';

  /** Day of week */
  dayOfWeek?: number;

  /** Response latency (seconds) */
  responseLatency?: number;

  /** Device type */
  deviceType?: 'mobile' | 'tablet' | 'desktop';

  /** Location type (if consented) */
  locationType?: 'home' | 'work' | 'transit' | 'other';
}

// =============================================================================
// ESM/EMA Individual Configuration
// =============================================================================

/**
 * Experience Sampling prompt delivered to an individual.
 */
export interface ESMPromptDelivery {
  /** Prompt ID */
  promptId: string;

  /** Study ID */
  studyId: string;

  /** Human receiving the prompt */
  humanId: string;

  /** When the prompt was delivered */
  deliveredAt: string;

  /** When the prompt expires */
  expiresAt: string;

  /** Current status */
  status: 'pending' | 'completed' | 'expired' | 'dismissed';

  /** Response if completed */
  response?: ESMPromptResponse;
}

/**
 * Response to an ESM prompt.
 */
export interface ESMPromptResponse {
  /** Response timestamp */
  respondedAt: string;

  /** Latency from delivery to response (ms) */
  latencyMs: number;

  /** Response data */
  data: Record<string, unknown>;

  /** Context captured */
  context?: ResponseContext;
}

// =============================================================================
// Research Instrument Display
// =============================================================================

/**
 * Metadata for displaying a research instrument in the UI.
 */
export interface ResearchInstrumentInfo {
  /** Framework ID */
  framework: ResearchFramework;

  /** Display name */
  displayName: string;

  /** Short description */
  description: string;

  /** Academic citation */
  citation?: string;

  /** Original authors */
  authors?: string[];

  /** Publication year */
  year?: number;

  /** Estimated completion time (minutes) */
  estimatedMinutes: number;

  /** Number of items/questions */
  itemCount: number;

  /** What the instrument measures */
  measures: string[];

  /** Category */
  category: ResearchCategory;

  /** Whether results are sharable in ImagoDei */
  sharableAsAttestation: boolean;
}

/**
 * Well-known research instrument metadata.
 */
export const RESEARCH_INSTRUMENTS: Partial<Record<ResearchFramework, ResearchInstrumentInfo>> = {
  'moral-foundations-mfq2': {
    framework: 'moral-foundations-mfq2',
    displayName: 'Moral Foundations Questionnaire 2',
    description: 'Measures six moral foundations: Care, Equality, Proportionality, Loyalty, Authority, Purity',
    citation: 'Atari, M., Graham, J., & Haidt, J. (2023)',
    authors: ['Mohammad Atari', 'Jesse Graham', 'Jonathan Haidt'],
    year: 2023,
    estimatedMinutes: 10,
    itemCount: 36,
    measures: ['care', 'equality', 'proportionality', 'loyalty', 'authority', 'purity'],
    category: 'moral-political',
    sharableAsAttestation: true,
  },
  'wellbeing-wemwbs': {
    framework: 'wellbeing-wemwbs',
    displayName: 'Warwick-Edinburgh Mental Wellbeing Scale',
    description: 'Measures positive mental wellbeing',
    citation: 'Tennant et al. (2007)',
    authors: ['Ruth Tennant', 'et al.'],
    year: 2007,
    estimatedMinutes: 5,
    itemCount: 14,
    measures: ['mental-wellbeing'],
    category: 'well-being',
    sharableAsAttestation: true,
  },
  'flourishing-perma': {
    framework: 'flourishing-perma',
    displayName: 'PERMA Profiler',
    description: 'Measures five pillars of wellbeing: Positive emotion, Engagement, Relationships, Meaning, Accomplishment',
    citation: 'Butler, J., & Kern, M.L. (2016)',
    authors: ['Judy Butler', 'Margaret L. Kern'],
    year: 2016,
    estimatedMinutes: 8,
    itemCount: 23,
    measures: ['positive-emotion', 'engagement', 'relationships', 'meaning', 'accomplishment'],
    category: 'well-being',
    sharableAsAttestation: true,
  },
  'attachment-ecr-r': {
    framework: 'attachment-ecr-r',
    displayName: 'Experiences in Close Relationships - Revised',
    description: 'Measures attachment-related anxiety and avoidance in close relationships',
    citation: 'Fraley, Waller, & Brennan (2000)',
    authors: ['R. Chris Fraley', 'Niels G. Waller', 'Kelly A. Brennan'],
    year: 2000,
    estimatedMinutes: 10,
    itemCount: 36,
    measures: ['attachment-anxiety', 'attachment-avoidance'],
    category: 'emotional',
    sharableAsAttestation: true,
  },
  'political-compass': {
    framework: 'political-compass',
    displayName: 'Political Compass',
    description: 'Maps political orientation on economic and social dimensions',
    estimatedMinutes: 15,
    itemCount: 62,
    measures: ['economic-left-right', 'social-libertarian-authoritarian'],
    category: 'moral-political',
    sharableAsAttestation: true,
  },
};
