/**
 * Discovery Assessment Model - Self-discovery instruments and results.
 *
 * Discovery assessments are NOT knowledge tests - they're instruments for
 * self-understanding that become part of your ImagoDei (identity profile).
 *
 * Examples:
 * - CliftonStrengths: Belief, Restorative, Learner, Analytical, Responsibility
 * - Enneagram: 1w2 (Reformer with Helper wing)
 * - MBTI: ISTP (Virtuoso)
 * - Learning Style: Visual, Auditory, Kinesthetic
 * - Values Assessment: Growth, Connection, Autonomy
 * - Attachment Style: Secure, Anxious, Avoidant
 *
 * These results:
 * - Are stored as attestations in the user's profile
 * - Can be used for path personalization
 * - Enable reflection and self-understanding
 * - Are private by default (user controls visibility)
 */

import { type ReachLevel } from '@app/elohim/models/protocol-core.model';
import { type ResearchConsentScope } from '@app/lamad/models/knowledge-map.model';

import { getDisplayConfigByFramework } from '../instruments/instrument-registry';

// @coverage: 100.0% (2026-02-24)

// ─────────────────────────────────────────────────────────────────────────────
// Discovery Assessment Types
// ─────────────────────────────────────────────────────────────────────────────

/** Well-known categories for type-safe references in existing code. */
export const KNOWN_CATEGORIES = [
  'personality', // Enneagram, MBTI, Big Five
  'strengths', // CliftonStrengths, VIA Character Strengths
  'values', // Personal values, political compass
  'learning', // Learning styles, cognitive preferences
  'emotional', // Attachment style, emotional intelligence
  'relational', // Love languages, communication styles
  'vocational', // Career interests, work preferences
  'spiritual', // Spiritual gifts, faith journey
] as const;
export type KnownCategory = (typeof KNOWN_CATEGORIES)[number];

/**
 * Categories of discovery assessments.
 * Open string - new categories can be added via content pipeline.
 */
// eslint-disable-next-line sonarjs/redundant-type-aliases -- semantic alias: marks string as category identifier
export type DiscoveryCategory = string;

/** Well-known frameworks for type-safe references in existing code. */
export const KNOWN_FRAMEWORKS = [
  'enneagram',
  'mbti',
  'big-five',
  'clifton-strengths', // eslint-disable-line sonarjs/no-duplicate-string -- canonical source for framework literals
  'via-strengths',
  'disc',
  'love-languages',
  'attachment-style',
  'learning-style',
  'political-compass',
  'values-hierarchy',
] as const;
export type KnownFramework = (typeof KNOWN_FRAMEWORKS)[number];

/**
 * Discovery assessment frameworks.
 * Open string - new frameworks can be added via content pipeline.
 */
// eslint-disable-next-line sonarjs/redundant-type-aliases -- semantic alias: marks string as framework identifier
export type DiscoveryFramework = string;

// ─────────────────────────────────────────────────────────────────────────────
// Discovery Assessment Definition
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A discovery assessment instrument definition.
 */
export interface DiscoveryAssessment {
  /** Unique ID */
  id: string;

  /** Display title */
  title: string;

  /** Description of what this assessment reveals */
  description: string;

  /** Category of self-discovery */
  category: DiscoveryCategory;

  /** Framework (e.g., 'enneagram', 'mbti') */
  framework: DiscoveryFramework;

  /** Content node ID for the assessment content */
  contentNodeId: string;

  /** Estimated time to complete */
  estimatedMinutes: number;

  /** Questions in the assessment */
  questions: DiscoveryQuestion[];

  /** How results are calculated */
  scoring: DiscoveryScoringConfig;

  /** Possible result types for this assessment */
  resultTypes: DiscoveryResultType[];

  /** Whether results can be retaken */
  allowRetake: boolean;

  /** Minimum time between retakes (ms) */
  retakeCooldownMs?: number;
}

/**
 * A question in a discovery assessment.
 */
export interface DiscoveryQuestion {
  /** Unique question ID */
  id: string;

  /** Question text */
  text: string;

  /** Question type */
  type: 'likert' | 'slider' | 'ranking' | 'multiple-choice' | 'forced-choice';

  /** Options for choice-based questions */
  options?: DiscoveryOption[];

  /** Scale configuration for likert/slider */
  scale?: {
    min: number;
    max: number;
    minLabel: string;
    maxLabel: string;
    step?: number;
  };

  /** Which subscales/dimensions this question measures */
  measures: string[];

  /** Weight for scoring */
  weight?: number;

  /** Reverse-scored? */
  reversed?: boolean;
}

/**
 * An option in a discovery question.
 */
export interface DiscoveryOption {
  /** Option value */
  value: string | number;

  /** Display label */
  label: string;

  /** Which subscales this option contributes to */
  contributes?: Record<string, number>;
}

/**
 * Configuration for how results are calculated.
 */
export interface DiscoveryScoringConfig {
  /** Subscales/dimensions measured */
  subscales: string[];

  /** How to determine the result type */
  method: 'highest-subscale' | 'threshold' | 'combination' | 'custom';

  /** Thresholds for threshold-based scoring */
  thresholds?: Record<string, { low: number; high: number }>;

  /** Custom scoring function name (if method is 'custom') */
  customScorer?: string;
}

/**
 * A possible result type for a discovery assessment.
 */
export interface DiscoveryResultType {
  /** Result type ID (e.g., 'type-1', 'istp', 'achiever') */
  id: string;

  /** Display name */
  name: string;

  /** Short code/abbreviation */
  shortCode?: string;

  /** Description of this type */
  description: string;

  /** Strengths associated with this type */
  strengths?: string[];

  /** Growth areas for this type */
  growthAreas?: string[];

  /** Related types (e.g., wing types, variants) */
  relatedTypes?: string[];

  /** Icon or emoji for display */
  icon?: string;

  /** Color for UI display */
  color?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Discovery Result (what gets stored as attestation)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * The result of completing a discovery assessment.
 * This becomes an attestation in the user's ImagoDei.
 */
export interface DiscoveryResult {
  /** Unique result ID */
  id: string;

  /** Assessment ID */
  assessmentId: string;

  /** Assessment title (denormalized for display) */
  assessmentTitle: string;

  /** Framework (e.g., 'enneagram') */
  framework: DiscoveryFramework;

  /** Category */
  category: DiscoveryCategory;

  /** Human who completed the assessment */
  humanId: string;

  /** When the assessment was completed */
  completedAt: string;

  /** Primary result type */
  primaryType: DiscoveryResultSummary;

  /** Secondary/additional results (e.g., wing, variant) */
  secondaryTypes?: DiscoveryResultSummary[];

  /** Raw subscale scores */
  subscaleScores: Record<string, number>;

  /** Formatted display string (e.g., "1w2", "ISTP", "Belief • Learner • Analytical") */
  displayString: string;

  /** Short display (for badges/chips) */
  shortDisplay: string;

  /** Per-result research consent override (when set, overrides user's default consent) */
  researchConsent?: ResearchConsentScope;

  /** User's reflection on this result */
  reflection?: string;

  /** Content node ID for viewing full results */
  resultContentId?: string;
}

/**
 * Summary of a single result type.
 */
export interface DiscoveryResultSummary {
  /** Type ID */
  typeId: string;

  /** Display name */
  name: string;

  /** Short code */
  shortCode?: string;

  /** Score/strength for this type (0-1) */
  score: number;

  /** Icon */
  icon?: string;

  /** Color for UI display */
  color?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Discovery Attestation (stored in ImagoDei)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A discovery attestation - wraps DiscoveryResult for the attestation system.
 */
export interface DiscoveryAttestation {
  /** Attestation ID */
  id: string;

  /** Human ID */
  humanId: string;

  /** Attestation type is always 'discovery' */
  type: 'discovery';

  /** The discovery result */
  result: DiscoveryResult;

  /** When earned */
  earnedAt: string;

  /** Visibility setting */
  visibility: 'private' | 'trusted' | 'community' | 'public';

  /** Machine-readable reach scope for filtering (derived from visibility) */
  reach: ReachLevel;

  /** Whether this attestation is featured on profile */
  featured: boolean;

  /** Sort order for display */
  displayOrder: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Framework-Specific Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Enneagram-specific result.
 */
export interface EnneagramResult extends DiscoveryResult {
  framework: 'enneagram';
  /** Core type (1-9) */
  coreType: number;
  /** Wing (e.g., 'w1' or 'w2' for type 9) */
  wing?: string;
  /** Tritype (e.g., '147') */
  tritype?: string;
  /** Instinctual variant (sp, so, sx) */
  instinct?: 'sp' | 'so' | 'sx';
  /** Integration/disintegration directions */
  integration?: { stress: number; growth: number };
}

/**
 * MBTI-specific result.
 */
export interface MBTIResult extends DiscoveryResult {
  framework: 'mbti';
  /** Four-letter type code */
  typeCode: string;
  /** Individual preferences with strength */
  preferences: {
    EI: { letter: 'E' | 'I'; strength: number };
    SN: { letter: 'S' | 'N'; strength: number };
    TF: { letter: 'T' | 'F'; strength: number };
    JP: { letter: 'J' | 'P'; strength: number };
  };
  /** Cognitive functions stack */
  cognitiveStack?: string[];
}

/**
 * CliftonStrengths-specific result.
 */
export interface CliftonStrengthsResult extends DiscoveryResult {
  framework: 'clifton-strengths';
  /** Top 5 signature themes */
  signatureThemes: CliftonTheme[];
  /** All 34 themes ranked (if full assessment) */
  allThemes?: CliftonTheme[];
  /** Domain breakdown */
  domains: {
    executing: number;
    influencing: number;
    relationship: number;
    strategic: number;
  };
}

/**
 * A CliftonStrengths theme.
 */
export interface CliftonTheme {
  /** Theme name (e.g., 'Belief', 'Learner') */
  name: string;
  /** Domain this theme belongs to */
  domain: 'executing' | 'influencing' | 'relationship' | 'strategic';
  /** Rank (1-34) */
  rank: number;
  /** Score if available */
  score?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Template Interpolation
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Resolve a dot-path (e.g., "primaryType.name") against a data object.
 */
function getNestedValue(obj: Record<string, unknown>, path: string): unknown {
  let current: unknown = obj;
  for (const key of path.split('.')) {
    if (current === null || current === undefined || typeof current !== 'object') return undefined;
    current = (current as Record<string, unknown>)[key];
  }
  return current;
}

/**
 * Interpolate {{path}} placeholders in a template against a data object.
 * Undefined values become empty strings. Result is trimmed.
 */
export function interpolateTemplate(template: string, data: Record<string, unknown>): string {
  return template
    .replace(/\{\{(\w[\w.]*)\}\}/g, (_, path: string) => {
      const value = getNestedValue(data, path.trim());
      return value !== null && value !== undefined ? String(value) : '';
    })
    .trim();
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create a display string for a discovery result.
 * Tries registry-backed display template first, falls back to framework-specific logic.
 */
export function formatDiscoveryResult(result: DiscoveryResult): string {
  const displayConfig = getDisplayConfigByFramework(result.framework);
  if (displayConfig?.displayTemplate) {
    const rendered = interpolateTemplate(
      displayConfig.displayTemplate,
      result as unknown as Record<string, unknown>
    );
    if (rendered) return rendered;
  }

  switch (result.framework) {
    case 'enneagram': {
      const enneagram = result as EnneagramResult;
      let display = `Type ${enneagram.coreType}`;
      if (enneagram.wing) display += enneagram.wing;
      if (enneagram.instinct) display += ` (${enneagram.instinct})`;
      return display;
    }

    case 'mbti': {
      const mbti = result as MBTIResult;
      return mbti.typeCode;
    }

    case 'clifton-strengths': {
      const clifton = result as CliftonStrengthsResult;
      return clifton.signatureThemes
        .slice(0, 5)
        .map(t => t.name)
        .join(' • ');
    }

    default:
      return result.primaryType.name;
  }
}

/**
 * Create a short display string (for badges).
 * Tries registry-backed short template first, falls back to framework-specific logic.
 */
export function formatDiscoveryShort(result: DiscoveryResult): string {
  const displayConfig = getDisplayConfigByFramework(result.framework);
  if (displayConfig?.shortTemplate) {
    const rendered = interpolateTemplate(
      displayConfig.shortTemplate,
      result as unknown as Record<string, unknown>
    );
    if (rendered) return rendered;
  }

  switch (result.framework) {
    case 'enneagram': {
      const enneagram = result as EnneagramResult;
      return `${enneagram.coreType}${enneagram.wing ?? ''}`;
    }

    case 'mbti': {
      const mbti = result as MBTIResult;
      return mbti.typeCode;
    }

    case 'clifton-strengths': {
      const clifton = result as CliftonStrengthsResult;
      return clifton.signatureThemes
        .slice(0, 3)
        .map(t => t.name.substring(0, 3))
        .join('·');
    }

    default:
      return result.primaryType.shortCode ?? result.primaryType.name.substring(0, 4);
  }
}

/**
 * Get framework display name.
 * Tries registry first, falls back to known names.
 */
export function getFrameworkDisplayName(framework: DiscoveryFramework): string {
  const displayConfig = getDisplayConfigByFramework(framework);
  if (displayConfig) return displayConfig.frameworkDisplayName;

  const names: Record<DiscoveryFramework, string> = {
    enneagram: 'Enneagram',
    mbti: 'MBTI',
    'big-five': 'Big Five',
    'clifton-strengths': 'CliftonStrengths',
    'via-strengths': 'VIA Character Strengths',
    disc: 'DISC',
    'love-languages': 'Love Languages',
    'attachment-style': 'Attachment Style',
    'learning-style': 'Learning Style',
    'political-compass': 'Political Compass',
    'values-hierarchy': 'Values Hierarchy',
    custom: 'Assessment',
  };
  return names[framework] || framework;
}

/**
 * Get category icon.
 */
export function getCategoryIcon(category: DiscoveryCategory): string {
  const icons: Record<DiscoveryCategory, string> = {
    personality: '🎭',
    strengths: '💪',
    values: '💎',
    learning: '📚',
    emotional: '💝',
    relational: '🤝',
    vocational: '💼',
    spiritual: '✨',
  };
  return icons[category] || '🔍';
}
