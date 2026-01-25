/**
 * Feedback Profile Model - Feedback Mechanisms as Gated Privileges
 *
 * Core Insight: "Virality is a privilege, not an entitlement."
 *
 * This model governs WHAT engagement mechanisms are permitted for content,
 * orthogonal to ContentReach (which governs WHERE content can go).
 *
 * Key Principles:
 * 1. NO "LIKES" - The Facebook-style like is fundamentally pernicious
 *    - Replaced with approval voting (up/down) as minimum baseline
 *    - Emotional reactions require context selection
 *    - All engagement mechanisms are Elohim-gated
 *
 * 2. INTELLECTUAL HUMILITY (Micah 6:8 - "walk humbly")
 *    - Profiles can UPGRADE through trust-building (attestations, peer review)
 *    - Profiles can DOWNGRADE through new evidence (research, retractions)
 *    - The system must acknowledge it could be wrong
 *
 * 3. PATH INHERITANCE: MOST RESTRICTIVE WINS
 *    - When content appears in a path, use the more restrictive profile
 *    - Constitutional protection takes precedence
 *
 * Holochain mapping:
 * - Entry type: "feedback_profile"
 * - Linked from content entries
 * - Profile changes create audit trail entries
 */

// ============================================================================
// Feedback Mechanism Types
// ============================================================================

/**
 * FeedbackMechanism - The types of engagement permitted for content.
 *
 * NO "LIKES" - The Facebook-style like is fundamentally pernicious.
 * Instead: approval voting, emotional reactions with context, deliberative mechanisms.
 *
 * Organized by friction level - higher friction = more deliberative, less viral.
 */
export type FeedbackMechanism =
  // Low friction (Elohim-gated for healthy communities)
  | 'approval-vote' // Up/down - minimum baseline, replaces "like"
  | 'emotional-reaction' // "I feel ___ about this" - requires context selection
  | 'affinity-mark' // Personal connection marker (private by default)

  // Medium friction (organic spread)
  | 'graduated-usefulness' // Loomio-style scale with optional reasoning
  | 'graduated-accuracy' // Fact-checking scale, reasoning encouraged
  | 'share-with-context' // Amplification requires meaningful context

  // High friction (deliberative only)
  | 'proposal-vote' // Formal voting with required reasoning
  | 'challenge' // Constitutional challenge
  | 'discussion-only' // No amplification, only deliberation
  | 'citation' // Academic-style reference (high-value, low-volume)
  | 'peer-review' // Formal review with criteria

  // No engagement
  | 'view-only'; // Content visible but no feedback permitted

/**
 * FeedbackFrictionLevel - Categorization of mechanism friction.
 * Higher friction = more deliberative = less viral potential.
 */
export type FeedbackFrictionLevel =
  | 'low' // Quick engagement possible (Elohim-gated)
  | 'medium' // Requires some thought/context
  | 'high' // Deliberative engagement only
  | 'none'; // No engagement permitted

/**
 * Map mechanisms to their friction levels.
 */
export const MECHANISM_FRICTION: Record<FeedbackMechanism, FeedbackFrictionLevel> = {
  'approval-vote': 'low',
  'emotional-reaction': 'low',
  'affinity-mark': 'low',
  'graduated-usefulness': 'medium',
  'graduated-accuracy': 'medium',
  'share-with-context': 'medium',
  'proposal-vote': 'high',
  challenge: 'high',
  'discussion-only': 'high',
  citation: 'high',
  'peer-review': 'high',
  'view-only': 'none',
};

// ============================================================================
// Emotional Reactions (Contextual, NOT low-effort)
// ============================================================================

/**
 * EmotionalReactionType - Contextual emotional responses.
 *
 * These are NOT Facebook-style reactions (thumbs up, heart, etc.)
 * They require selecting a meaningful emotional context.
 *
 * IMPORTANT: Not all reaction types are appropriate for all content.
 * Personal testimony should only allow supportive reactions.
 * See EmotionalReactionConstraints below.
 */
export type EmotionalReactionType =
  // Supportive reactions (generally safe for personal content)
  | 'moved' // This moved me emotionally
  | 'grateful' // I'm grateful for this
  | 'inspired' // This inspires me
  | 'hopeful' // This gives me hope
  | 'grieving' // This connects to grief/loss (solidarity)

  // Critical/Challenging reactions (require accountability, may not be appropriate for personal content)
  | 'challenged' // This challenged my thinking
  | 'concerned' // This concerns me
  | 'uncomfortable'; // This makes me uncomfortable (important signal, but can be weaponized)

/**
 * EmotionalReactionCategory - Categorization for permission purposes.
 */
export type EmotionalReactionCategory = 'supportive' | 'critical';

/**
 * Map reaction types to categories.
 * This determines which reactions are appropriate for different content types.
 */
export const REACTION_CATEGORIES: Record<EmotionalReactionType, EmotionalReactionCategory> = {
  moved: 'supportive',
  grateful: 'supportive',
  inspired: 'supportive',
  hopeful: 'supportive',
  grieving: 'supportive',
  challenged: 'critical',
  concerned: 'critical',
  uncomfortable: 'critical',
};

/**
 * EmotionalReactionConstraints - What reaction types are permitted.
 *
 * Personal content should restrict critical reactions to prevent harassment.
 * Example: Mother-in-law putting a "laugh" (or "uncomfortable") on someone's
 * personal grief post is a form of abuse.
 *
 * MEDIATED REACTIONS: Rather than hard-blocking, Elohim can mediate:
 * - Allow the action but intercept it
 * - Explain why it's considered harmful in this context
 * - Give user choice: proceed (author won't see) or reconsider
 * - Track patterns for social health monitoring
 */
export interface EmotionalReactionConstraints {
  /** Which reaction types are permitted without mediation */
  permittedTypes: EmotionalReactionType[];

  /** Which categories are permitted (alternative to listing types) */
  permittedCategories?: EmotionalReactionCategory[];

  /** Which reactions are mediated (not blocked, but intercepted with explanation) */
  mediatedTypes?: MediatedReaction[];

  /** Must reactions be attributed (non-anonymous)? */
  requireAttribution: boolean;

  /** Can the content author hide reactions they find harmful? */
  authorCanHide: boolean;

  /** Must critical reactions include reasoning? */
  criticalRequiresReasoning: boolean;
}

/**
 * MediatedReaction - A reaction type that is intercepted rather than blocked.
 *
 * Elohim mediation teaches rather than just prevents:
 * - User can still express the reaction
 * - But gets constitutional reasoning about why it may be harmful
 * - Can proceed (won't be visible to author) or reconsider
 * - Pattern is tracked for social health monitoring
 */
export interface MediatedReaction {
  /** The reaction type being mediated */
  type: EmotionalReactionType;

  /** Why this reaction is being mediated in this context */
  constitutionalReasoning: string;

  /** What happens if user proceeds anyway */
  proceedBehavior: MediatedReactionBehavior;

  /** Prompt shown to user when they try this reaction */
  mediationPrompt: string;

  /** Alternative reaction suggestions */
  suggestedAlternatives?: EmotionalReactionType[];
}

/**
 * MediatedReactionBehavior - What happens if user proceeds with mediated reaction.
 */
export interface MediatedReactionBehavior {
  /** Is the reaction visible to the content author? */
  visibleToAuthor: boolean;

  /** Is the reaction visible to others? */
  visibleToOthers: boolean;

  /** Is it logged for social health monitoring? */
  loggedForMonitoring: boolean;

  /** Does it affect the user's social trust score? */
  affectsTrustScore: boolean;

  /** Explanation shown to user about consequences */
  consequenceExplanation: string;
}

/**
 * MediationLog - Record of a user proceeding through mediation.
 *
 * This is behavioral telemetry - a strong signal about social orientation.
 * Patterns of ignoring constitutional reasoning may indicate:
 * - Narcissistic tendencies (repeated dismissal of others' experiences)
 * - Social health concerns
 * - Need for Elohim intervention
 */
export interface MediationLog {
  /** Who proceeded through mediation */
  userId: string;

  /** What content was the target */
  contentId: string;

  /** What type of content (personal testimony, etc.) */
  contentType: string;

  /** What reaction type was attempted */
  reactionType: EmotionalReactionType;

  /** The constitutional reasoning they were shown */
  reasoningShown: string;

  /** Did they proceed anyway? */
  proceededAnyway: boolean;

  /** If they reconsidered, did they choose an alternative? */
  alternativeChosen?: EmotionalReactionType;

  /** Timestamp */
  loggedAt: string;
}

/**
 * UserMediationPattern - Aggregate pattern for a user's mediation history.
 *
 * Elohim use this to:
 * - Detect concerning patterns (repeated ignoring of mediation)
 * - Calibrate trust for other interactions
 * - Trigger care protocols for potential victims
 * - Adjust intervention thresholds
 */
export interface UserMediationPattern {
  /** The user being analyzed */
  userId: string;

  /** Total mediation prompts shown */
  totalMediationsShown: number;

  /** How many times they proceeded anyway */
  proceedThroughCount: number;

  /** How many times they reconsidered */
  reconsideredCount: number;

  /** Proceed-through rate (high = concerning) */
  proceedThroughRate: number;

  /** Which reaction types they most often proceed through on */
  mostCommonProceedTypes: EmotionalReactionType[];

  /** Pattern assessment */
  patternAssessment: MediationPatternAssessment;

  /** When was this pattern last computed */
  computedAt: string;
}

/**
 * MediationPatternAssessment - Elohim assessment of mediation pattern.
 */
export interface MediationPatternAssessment {
  /** Overall pattern classification */
  classification: 'healthy' | 'educational-needed' | 'concerning' | 'intervention-required';

  /** Specific concerns identified */
  concerns: MediationConcern[];

  /** Recommended Elohim actions */
  recommendedActions: string[];

  /** Should this affect the user's trust score? */
  trustScoreImpact: 'none' | 'minor-decrease' | 'moderate-decrease' | 'significant-decrease';

  /** Constitutional principles this pattern may violate */
  principlesConcerned: string[];
}

/**
 * MediationConcern - A specific concern from mediation pattern analysis.
 */
export interface MediationConcern {
  /** Type of concern */
  type:
    | 'dismissiveness'
    | 'potential-harassment'
    | 'empathy-deficit'
    | 'boundary-violation'
    | 'pattern-escalation';

  /** Description */
  description: string;

  /** Evidence from mediation logs */
  evidenceCount: number;

  /** Severity */
  severity: 'low' | 'moderate' | 'high';
}

/**
 * Default emotional reaction constraints by content sensitivity.
 */
export const DEFAULT_REACTION_CONSTRAINTS: Record<string, EmotionalReactionConstraints> = {
  // Personal testimony - supportive reactions permitted, critical reactions mediated
  'personal-testimony': {
    permittedTypes: ['moved', 'grateful', 'inspired', 'hopeful', 'grieving'],
    permittedCategories: ['supportive'],
    // Critical reactions are mediated, not blocked - teaches rather than prevents
    mediatedTypes: [
      {
        type: 'challenged',
        constitutionalReasoning:
          'This is personal testimony. Expressing that you feel "challenged" by someone\'s personal experience may come across as dismissive of their lived reality.',
        mediationPrompt:
          'This person is sharing a personal experience. Are you sure you want to express that you feel "challenged" by it?',
        proceedBehavior: {
          visibleToAuthor: false,
          visibleToOthers: false,
          loggedForMonitoring: true,
          affectsTrustScore: false, // First time is educational
          consequenceExplanation:
            "Your reaction will be recorded for your own reflection, but won't be visible to the author or others.",
        },
        suggestedAlternatives: ['moved', 'grateful'],
      },
      {
        type: 'uncomfortable',
        constitutionalReasoning:
          'Expressing discomfort with someone\'s personal testimony can feel like ridicule or dismissal - similar to the "tyranny of the laughing emoji" phenomenon.',
        mediationPrompt:
          'This person is sharing something personal. Expressing "uncomfortable" on personal testimony can feel hurtful. Would you like to engage differently?',
        proceedBehavior: {
          visibleToAuthor: false,
          visibleToOthers: false,
          loggedForMonitoring: true,
          affectsTrustScore: false,
          consequenceExplanation:
            'Your reaction will be recorded but not shown to anyone. If you have genuine concerns, consider using discussion instead.',
        },
        suggestedAlternatives: ['concerned'], // Concern is more constructive
      },
    ],
    requireAttribution: true,
    authorCanHide: true,
    criticalRequiresReasoning: true,
  },

  // Learning content - all reactions permitted, must be attributed
  'learning-content': {
    permittedTypes: [
      'moved',
      'grateful',
      'inspired',
      'hopeful',
      'grieving',
      'challenged',
      'concerned',
      'uncomfortable',
    ],
    requireAttribution: true,
    authorCanHide: false, // Learning content should accept critique
    criticalRequiresReasoning: true,
  },

  // Research content - all reactions, critical must have reasoning
  'research-content': {
    permittedTypes: [
      'moved',
      'grateful',
      'inspired',
      'hopeful',
      'grieving',
      'challenged',
      'concerned',
      'uncomfortable',
    ],
    requireAttribution: true,
    authorCanHide: false,
    criticalRequiresReasoning: true,
  },

  // Community announcements - all reactions
  'community-announcement': {
    permittedTypes: [
      'moved',
      'grateful',
      'inspired',
      'hopeful',
      'grieving',
      'challenged',
      'concerned',
      'uncomfortable',
    ],
    requireAttribution: true,
    authorCanHide: false,
    criticalRequiresReasoning: false,
  },
};

/**
 * EmotionalReaction - A user's emotional response to content.
 */
export interface EmotionalReaction {
  /** The type of emotional response */
  type: EmotionalReactionType;

  /** Optional elaboration on why (required for critical reactions on some content) */
  context?: string;

  /** Is this visible to others or personal only? */
  private: boolean;

  /** Was this reaction hidden by the author? (only possible if authorCanHide) */
  hiddenByAuthor?: boolean;

  /** When was this response recorded */
  respondedAt: string;

  /** Who responded (required if requireAttribution) */
  responderId: string;
}

/**
 * Descriptions for emotional reactions (for UI).
 */
export const EMOTIONAL_REACTION_DESCRIPTIONS: Record<EmotionalReactionType, string> = {
  moved: 'This moved me emotionally',
  challenged: 'This challenged my thinking',
  grateful: 'I am grateful for this',
  concerned: 'This concerns me',
  inspired: 'This inspires me',
  uncomfortable: 'This makes me uncomfortable',
  hopeful: 'This gives me hope',
  grieving: 'This connects to grief or loss',
};

// ============================================================================
// Feedback Profile Core
// ============================================================================

/**
 * FeedbackProfile - Governs what engagement mechanisms are permitted for content.
 *
 * Profiles are DYNAMIC - they can be upgraded OR downgraded.
 * This flexibility embodies intellectual humility (Micah 6:8 - "walk humbly"):
 * - New research may invalidate previously trusted content
 * - Authors may retract based on new information
 * - The system must acknowledge it could be wrong
 */
export interface FeedbackProfile {
  /** Unique identifier */
  id: string;

  /** Which mechanisms are currently permitted */
  permittedMechanisms: FeedbackMechanism[];

  /** Which mechanisms are explicitly prohibited (and why) */
  prohibitedMechanisms: ProhibitedMechanism[];

  /** Default mechanism when multiple are available */
  defaultMechanism?: FeedbackMechanism;

  /** Friction requirements for amplification */
  amplificationRequirements?: AmplificationRequirement;

  /**
   * Constraints on emotional reactions if 'emotional-reaction' is permitted.
   * Guards against "tyranny of the laughing emoji" - using reactions to ridicule/shame.
   * Personal content should restrict critical reactions to prevent harassment.
   */
  emotionalReactionConstraints?: EmotionalReactionConstraints;

  /** How was this profile determined? */
  determination: FeedbackProfileDetermination;

  /** Profile evolution - upgrades AND downgrades (intellectual humility) */
  evolution: ProfileEvolution;

  /** Timestamps */
  createdAt: string;
  updatedAt: string;
}

/**
 * ProhibitedMechanism - A mechanism explicitly denied with justification.
 */
export interface ProhibitedMechanism {
  /** The mechanism that is prohibited */
  mechanism: FeedbackMechanism;

  /** Constitutional justification for prohibition */
  reason: string;

  /** Who prohibited this (Elohim ID or 'constitutional') */
  prohibitedBy: string;

  /** Can this prohibition ever be lifted? */
  permanent: boolean;
}

/**
 * AmplificationRequirement - Friction requirements for content to spread.
 */
export interface AmplificationRequirement {
  /** Minimum friction level for any amplification */
  minimumFriction: 'none' | 'comment-required' | 'reasoning-required' | 'deliberation-required';

  /** Must recipient accept before amplification completes? */
  recipientConsent: boolean;

  /** Cool-down period between amplifications */
  cooldownPeriod?: string; // ISO 8601 duration
}

// ============================================================================
// Profile Determination (How profiles are assigned)
// ============================================================================

/**
 * FeedbackProfileDetermination - How a profile was assigned.
 */
export interface FeedbackProfileDetermination {
  /** How was this profile assigned? */
  method: ProfileDeterminationMethod;

  /** Which Elohim participated in the determination */
  participatingElohim: string[];

  /** Constitutional principles cited */
  principlesCited: string[];

  /** Reasoning for this profile */
  reasoning: string;

  /** When was this determined */
  determinedAt: string;

  /** Is this determination challengeable? */
  challengeable: boolean;
}

export type ProfileDeterminationMethod =
  | 'content-type-default' // Auto-assigned based on content type
  | 'elohim-negotiation' // Elohim negotiated this profile
  | 'governance-decision' // Formal governance decision
  | 'creator-request'; // Content creator requested this profile

// ============================================================================
// Profile Evolution (Upgrade AND Downgrade - Intellectual Humility)
// ============================================================================

/**
 * ProfileEvolution - Tracks how a profile can and has changed.
 *
 * Intellectual humility requires the system to acknowledge:
 * - Content can earn MORE trust through peer review, attestation
 * - Content can LOSE trust through new research, retractions, governance
 */
export interface ProfileEvolution {
  /** Can this profile be upgraded? */
  upgradeEligibility?: UpgradeEligibility;

  /** Can this profile be downgraded? What would trigger it? */
  downgradeVulnerabilities?: DowngradeVulnerability[];

  /** History of profile changes */
  history: ProfileChange[];
}

/**
 * ProfileChange - A record of a profile upgrade or downgrade.
 */
export interface ProfileChange {
  /** Type of change */
  changeType: 'upgrade' | 'downgrade' | 'initial';

  /** Mechanisms before the change */
  previousMechanisms: FeedbackMechanism[];

  /** Mechanisms after the change */
  newMechanisms: FeedbackMechanism[];

  /** Why this change happened */
  reason: string;

  /** What triggered this change */
  triggeredBy: ProfileChangeTrigger;

  /** When did this change occur */
  changedAt: string;

  /** Who/what made this change (Elohim ID or governance decision ID) */
  changedBy: string;
}

/**
 * ProfileChangeTrigger - What can cause a profile to change.
 *
 * Upgrades require trust-building.
 * Downgrades embody intellectual humility - acknowledging we could be wrong.
 */
export type ProfileChangeTrigger =
  // Upgrade triggers
  | 'attestation-earned' // Earned new attestation
  | 'peer-review-passed' // Passed peer review
  | 'community-endorsement' // Community vouched for content
  | 'time-trust-earned' // Stood the test of time

  // Downgrade triggers (intellectual humility)
  | 'new-research' // New research contradicts content
  | 'author-retraction' // Author walked it back (courage!)
  | 'challenge-upheld' // Constitutional challenge succeeded
  | 'accuracy-dispute' // Accuracy questioned and validated

  // Either direction
  | 'governance-decision'; // Formal governance ruling

/**
 * UpgradeEligibility - What a profile could upgrade to and how.
 */
export interface UpgradeEligibility {
  /** What mechanisms could be added */
  potentialMechanisms: FeedbackMechanism[];

  /** What would unlock them */
  requirements: UpgradeRequirement[];
}

/**
 * UpgradeRequirement - What's needed to add a mechanism.
 */
export interface UpgradeRequirement {
  /** The mechanism this requirement unlocks */
  mechanism: FeedbackMechanism;

  /** Attestations required (any of these) */
  attestationsRequired?: string[];

  /** Minimum trust score required */
  minimumTrustScore?: number;

  /** How long at current profile before eligible */
  timeAtCurrentProfile?: string; // ISO 8601 duration

  /** Does this require explicit governance approval? */
  governanceApproval?: boolean;
}

/**
 * DowngradeVulnerability - What could cause a downgrade.
 */
export interface DowngradeVulnerability {
  /** What could trigger a downgrade */
  trigger: ProfileChangeTrigger;

  /** Which mechanisms would be affected */
  affectedMechanisms: FeedbackMechanism[];

  /** What threshold/criteria would trigger this */
  threshold?: string;
}

// ============================================================================
// Default Profiles by Content Type
// ============================================================================

/**
 * FeedbackProfileTemplate - Template for creating profiles.
 */
export type FeedbackProfileTemplate = Omit<
  FeedbackProfile,
  'id' | 'createdAt' | 'updatedAt' | 'evolution'
> & {
  evolution?: Partial<ProfileEvolution>;
};

/**
 * Default feedback profiles by content type.
 *
 * Note: NO "LIKES" ANYWHERE. Approval-vote is the minimum low-friction mechanism.
 * Elohim can further restrict based on community health.
 */
export const DEFAULT_FEEDBACK_PROFILES: Record<string, FeedbackProfileTemplate> = {
  // Learning content - thoughtful engagement, organic spread
  'learning-content': {
    permittedMechanisms: [
      'affinity-mark',
      'graduated-usefulness',
      'emotional-reaction',
      'share-with-context',
      'citation',
    ],
    prohibitedMechanisms: [],
    defaultMechanism: 'graduated-usefulness',
    determination: {
      method: 'content-type-default',
      participatingElohim: [],
      principlesCited: ['learning-benefits-thoughtful-engagement'],
      reasoning: 'Learning content benefits from thoughtful engagement over quick reactions',
      determinedAt: '',
      challengeable: true,
    },
  },

  // Community announcements - approval voting ok, emotional reactions appropriate
  'community-announcement': {
    permittedMechanisms: [
      'approval-vote',
      'emotional-reaction',
      'affinity-mark',
      'share-with-context',
    ],
    prohibitedMechanisms: [],
    defaultMechanism: 'approval-vote',
    determination: {
      method: 'content-type-default',
      participatingElohim: [],
      principlesCited: ['community-coordination'],
      reasoning: 'Community announcements serve coordination; approval voting helps gauge interest',
      determinedAt: '',
      challengeable: true,
    },
  },

  // Personal testimony/story - protection from viral exploitation and "tyranny of the laughing emoji"
  'personal-testimony': {
    permittedMechanisms: ['emotional-reaction', 'affinity-mark', 'discussion-only'],
    prohibitedMechanisms: [],
    amplificationRequirements: {
      minimumFriction: 'reasoning-required',
      recipientConsent: true,
    },
    emotionalReactionConstraints: {
      // Only supportive reactions - prevent ridicule/shame
      permittedTypes: ['moved', 'grateful', 'inspired', 'hopeful', 'grieving'],
      permittedCategories: ['supportive'],
      requireAttribution: true,
      authorCanHide: true, // Authors can protect themselves from harmful reactions
      criticalRequiresReasoning: true,
    },
    determination: {
      method: 'content-type-default',
      participatingElohim: [],
      principlesCited: ['human-dignity', 'imago-dei', 'protection-from-ridicule'],
      reasoning:
        'Personal stories are not commodities; protect from viral exploitation and "tyranny of the laughing emoji"',
      determinedAt: '',
      challengeable: true,
    },
  },

  // Research/Academic - peer review and citation primary
  'research-content': {
    permittedMechanisms: [
      'peer-review',
      'citation',
      'graduated-accuracy',
      'discussion-only',
      'challenge',
    ],
    prohibitedMechanisms: [],
    defaultMechanism: 'peer-review',
    determination: {
      method: 'content-type-default',
      participatingElohim: [],
      principlesCited: ['epistemic-integrity', 'intellectual-humility'],
      reasoning: 'Research requires rigorous engagement; peer review is primary mechanism',
      determinedAt: '',
      challengeable: true,
    },
  },

  // Potentially sensitive content - deliberation only, no amplification
  'sensitive-content': {
    permittedMechanisms: ['discussion-only', 'challenge'],
    prohibitedMechanisms: [
      {
        mechanism: 'approval-vote',
        reason: 'Content requires deliberation not voting',
        prohibitedBy: 'constitutional',
        permanent: false,
      },
      {
        mechanism: 'share-with-context',
        reason: 'Requires governance review before amplification',
        prohibitedBy: 'constitutional',
        permanent: false,
      },
    ],
    determination: {
      method: 'content-type-default',
      participatingElohim: [],
      principlesCited: ['constitutional-protection', 'deliberative-democracy'],
      reasoning: 'Sensitive content requires deliberation, not quick reactions or amplification',
      determinedAt: '',
      challengeable: true,
    },
  },

  // Governance proposals - formal engagement only
  'governance-proposal': {
    permittedMechanisms: ['proposal-vote', 'discussion-only', 'challenge'],
    prohibitedMechanisms: [],
    defaultMechanism: 'proposal-vote',
    determination: {
      method: 'content-type-default',
      participatingElohim: [],
      principlesCited: ['deliberative-democracy', 'constitutional-process'],
      reasoning: 'Governance proposals require formal, structured engagement',
      determinedAt: '',
      challengeable: false, // Governance profiles are not challengeable
    },
  },

  // View-only (e.g., content under review, disputed, etc.)
  restricted: {
    permittedMechanisms: ['view-only'],
    prohibitedMechanisms: [
      {
        mechanism: 'approval-vote',
        reason: 'Content status under review',
        prohibitedBy: 'governance',
        permanent: false,
      },
    ],
    determination: {
      method: 'governance-decision',
      participatingElohim: [],
      principlesCited: ['due-process'],
      reasoning: 'Content under review or disputed; engagement restricted pending resolution',
      determinedAt: '',
      challengeable: true,
    },
  },
};

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get the most restrictive profile between two profiles.
 * Used when content appears in a path context.
 */
export function getMostRestrictiveProfile(
  profileA: FeedbackProfile,
  profileB: FeedbackProfile
): FeedbackProfile {
  // Compare by number of permitted mechanisms (fewer = more restrictive)
  const aCount = profileA.permittedMechanisms.length;
  const bCount = profileB.permittedMechanisms.length;

  if (aCount <= bCount) {
    return profileA;
  }
  return profileB;
}

/**
 * Check if a mechanism is permitted by a profile.
 */
export function isMechanismPermitted(
  profile: FeedbackProfile,
  mechanism: FeedbackMechanism
): boolean {
  // Check if explicitly prohibited
  if (profile.prohibitedMechanisms.some(p => p.mechanism === mechanism)) {
    return false;
  }

  // Check if in permitted list
  return profile.permittedMechanisms.includes(mechanism);
}

/**
 * Get the friction level of a profile based on its most permissive mechanism.
 */
export function getProfileFrictionLevel(profile: FeedbackProfile): FeedbackFrictionLevel {
  const frictionOrder: FeedbackFrictionLevel[] = ['low', 'medium', 'high', 'none'];

  let lowestFriction: FeedbackFrictionLevel = 'none';

  for (const mechanism of profile.permittedMechanisms) {
    const friction = MECHANISM_FRICTION[mechanism];
    if (frictionOrder.indexOf(friction) < frictionOrder.indexOf(lowestFriction)) {
      lowestFriction = friction;
    }
  }

  return lowestFriction;
}

/**
 * Create a new FeedbackProfile from a template.
 */
export function createProfileFromTemplate(
  template: FeedbackProfileTemplate,
  id: string
): FeedbackProfile {
  const now = new Date().toISOString();

  return {
    ...template,
    id,
    createdAt: now,
    updatedAt: now,
    determination: {
      ...template.determination,
      determinedAt: template.determination.determinedAt || now,
    },
    evolution: {
      upgradeEligibility: template.evolution?.upgradeEligibility,
      downgradeVulnerabilities: template.evolution?.downgradeVulnerabilities ?? [],
      history: [
        {
          changeType: 'initial',
          previousMechanisms: [],
          newMechanisms: template.permittedMechanisms,
          reason: template.determination.reasoning,
          triggeredBy: 'governance-decision',
          changedAt: now,
          changedBy: 'system',
        },
      ],
    },
  };
}
