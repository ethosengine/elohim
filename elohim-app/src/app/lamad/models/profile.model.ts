/**
 * Profile Models - Human-Centered Identity
 *
 * Aligned with Imago Dei Framework:
 * - imagodei-core: Stable identity center
 * - imagodei-experience: Learning and transformation
 * - imagodei-gifts: Developed capabilities
 * - imagodei-synthesis: Growth and meaning-making
 *
 * These models support viewing a human's journey through Lamad
 * not as consumption metrics, but as a narrative of growth.
 */

import { ContentType } from './content-node.model';
import { MasteryLevel } from './agent.model';

/**
 * Human Profile Summary
 *
 * The stable center of identity (imagodei-core) combined with
 * growth indicators (imagodei-synthesis).
 */
export interface HumanProfile {
  /** Unique identifier (session ID or Holochain agent hash) */
  id: string;

  /** Display name chosen by the human */
  displayName: string;

  /** Whether this is a session-based identity */
  isSessionBased: boolean;

  /** When this human first began their journey */
  journeyStartedAt: string;

  /** Most recent activity */
  lastActiveAt: string;

  /** High-level journey statistics */
  journeyStats: JourneyStats;

  /** Current focus areas (what they're actively learning) */
  currentFocus: CurrentFocus[];

  /** Gifts developed through learning (imagodei-gifts) */
  developedCapabilities: DevelopedCapability[];
}

/**
 * Journey Statistics
 *
 * Quantitative measures of the learning journey,
 * presented as growth indicators rather than consumption metrics.
 */
export interface JourneyStats {
  /** Total content explored (breadth of exploration) */
  territoryExplored: number;

  /** Paths embarked upon */
  journeysStarted: number;

  /** Paths completed (milestones achieved) */
  journeysCompleted: number;

  /** Individual steps completed across all paths */
  stepsCompleted: number;

  /** Content marked as meaningful (affinity > 0.5) */
  meaningfulEncounters: number;

  /** Total time invested in learning (milliseconds) */
  timeInvested: number;

  /** Number of sessions (return visits indicate commitment) */
  sessionsCount: number;
}

/**
 * Current Focus
 *
 * What the human is actively working on (imagodei-experience).
 * Supports the "continue where you left off" experience.
 */
export interface CurrentFocus {
  /** Path being traversed */
  pathId: string;

  /** Human-readable path title */
  pathTitle: string;

  /** Current step in the journey */
  currentStepIndex: number;

  /** Total steps in this path */
  totalSteps: number;

  /** Progress as percentage (0-100) */
  progressPercent: number;

  /** When last active on this path */
  lastActiveAt: string;

  /** The next step's title for quick resumption */
  nextStepTitle?: string;

  /** The next step's narrative (why it matters) */
  nextStepNarrative?: string;
}

/**
 * Developed Capability
 *
 * Skills and understanding gained through completed paths (imagodei-gifts).
 * This is derived from attestations earned.
 */
export interface DevelopedCapability {
  /** Attestation ID */
  id: string;

  /** Human-readable name */
  name: string;

  /** Description of what this capability represents */
  description: string;

  /** When this capability was developed */
  earnedAt: string;

  /** Path that granted this capability */
  sourcePath?: string;

  /** Mastery level achieved */
  level: MasteryLevel;

  /** Icon for UI display */
  icon?: string;
}

/**
 * Learning Timeline Event
 *
 * A chronological record of significant moments (imagodei-experience).
 * These are transformation points, not just activity logs.
 */
export interface TimelineEvent {
  /** Unique event ID */
  id: string;

  /** Event type determines display and significance */
  type: TimelineEventType;

  /** When this occurred */
  timestamp: string;

  /** Human-readable title */
  title: string;

  /** Additional context about the event */
  description?: string;

  /** Related resource (path, content, etc.) */
  resourceId?: string;

  /** Resource type for navigation */
  resourceType?: 'path' | 'content' | 'attestation';

  /** Significance level (for filtering/highlighting) */
  significance: 'milestone' | 'progress' | 'activity';
}

export type TimelineEventType =
  | 'journey_started'      // Began a new learning path
  | 'journey_completed'    // Finished a learning path
  | 'step_completed'       // Completed a step
  | 'capability_earned'    // Earned an attestation
  | 'meaningful_encounter' // Marked content with high affinity
  | 'note_created'         // Added a personal reflection
  | 'return_visit'         // Returned after absence (commitment)
  | 'first_exploration';   // First content viewed (journey begins)

/**
 * Content Engagement
 *
 * Deep engagement with specific content (imagodei-synthesis).
 * High affinity indicates resonance and meaning-making.
 */
export interface ContentEngagement {
  /** Content node ID */
  nodeId: string;

  /** Content title */
  title: string;

  /** Content type */
  contentType: ContentType;

  /** Affinity score (0.0 to 1.0) */
  affinity: number;

  /** Times viewed */
  viewCount: number;

  /** Last interaction */
  lastViewedAt: string;

  /** Personal notes attached */
  hasNotes: boolean;

  /** Paths containing this content */
  containingPaths: string[];
}

/**
 * Note with Context
 *
 * Personal reflections attached to learning (imagodei-synthesis).
 * Notes are meaning-making artifacts, not just annotations.
 */
export interface NoteWithContext {
  /** Note ID */
  id: string;

  /** The reflection/note content */
  content: string;

  /** When created */
  createdAt: string;

  /** When last modified */
  updatedAt: string;

  /** What this note is attached to */
  context: NoteContext;
}

export interface NoteContext {
  /** Context type */
  type: 'path_step' | 'content' | 'general';

  /** Path ID (if path_step) */
  pathId?: string;

  /** Path title */
  pathTitle?: string;

  /** Step index (if path_step) */
  stepIndex?: number;

  /** Step title */
  stepTitle?: string;

  /** Content ID (if content) */
  contentId?: string;

  /** Content title */
  contentTitle?: string;
}

/**
 * Resume Point
 *
 * Smart suggestion for where to continue (imagodei-experience).
 * Honors the human's ongoing journey rather than starting fresh.
 */
export interface ResumePoint {
  /** Type of resumption */
  type: 'continue_path' | 'revisit_content' | 'explore_new';

  /** Primary action title */
  title: string;

  /** Why this is suggested */
  reason: string;

  /** Path ID if continuing a path */
  pathId?: string;

  /** Step index if continuing a path */
  stepIndex?: number;

  /** Content ID if revisiting or exploring */
  contentId?: string;

  /** Time since last activity (for context) */
  daysSinceActive: number;
}

/**
 * Path with Progress
 *
 * A learning path enriched with the human's progress.
 */
export interface PathWithProgress {
  /** Path ID */
  pathId: string;

  /** Path title */
  title: string;

  /** Path description */
  description: string;

  /** Difficulty level */
  difficulty: 'beginner' | 'intermediate' | 'advanced';

  /** Total steps */
  totalSteps: number;

  /** Steps completed */
  completedSteps: number;

  /** Current step index */
  currentStepIndex: number;

  /** Progress percentage */
  progressPercent: number;

  /** When started */
  startedAt?: string;

  /** When completed (if finished) */
  completedAt?: string;

  /** Estimated time remaining */
  estimatedTimeRemaining?: string;

  /** Attestations that will be earned */
  attestationsGranted?: string[];
}

/**
 * Paths Overview
 *
 * Organized view of all paths relevant to this human.
 */
export interface PathsOverview {
  /** Paths currently in progress */
  inProgress: PathWithProgress[];

  /** Paths completed */
  completed: PathWithProgress[];

  /** Suggested paths based on interests */
  suggested: PathWithProgress[];
}

/**
 * Profile Summary for Quick Display
 *
 * Lightweight version for headers, cards, etc.
 */
export interface ProfileSummaryCompact {
  displayName: string;
  isSessionBased: boolean;
  journeysCompleted: number;
  capabilitiesEarned: number;
  currentFocusTitle?: string;
  currentFocusProgress?: number;
}
