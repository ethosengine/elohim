/**
 * Epic Domain Discovery Instrument
 *
 * Psychometric instrument definition for discovering which Elohim Protocol
 * epic domain resonates most with a learner. Uses highest-subscale scoring
 * to determine the primary domain recommendation.
 *
 * This file only contains the instrument definition. Scoring logic is handled
 * by psyche-core via the Sophia plugin's Psyche API.
 *
 * Epic Domains:
 * - Governance (AI Constitutional): Shaping AI governance and constitutional frameworks
 * - Care (Value Scanner): Recognizing and valuing care work, supporting caregivers
 * - Economic (Economic Coordination): Transforming workplace dynamics, worker ownership
 * - Public (Public Observer): Democratic participation, transparency, civic engagement
 * - Social (Social Medium): Building healthier digital spaces, online communication
 */

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  CreateInstrumentOptions,
  AggregatedReflection,
} from '../../content-io/plugins/sophia/sophia-element-loader';

// ─────────────────────────────────────────────────────────────────────────────
// Instrument ID
// ─────────────────────────────────────────────────────────────────────────────

export const EPIC_DOMAIN_INSTRUMENT_ID = 'epic-domain-discovery';

/** Dimension identifier for all Epic Domain subscales */
const EPIC_DOMAIN_DIMENSION = 'epic-domain';

// ─────────────────────────────────────────────────────────────────────────────
// Subscale Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const EPIC_DOMAIN_SUBSCALES: SubscaleDefinition[] = [
  {
    id: 'governance',
    name: 'AI Constitutional',
    description:
      'Interest in AI governance, constitutional frameworks, and ensuring AI serves humanity',
    dimension: EPIC_DOMAIN_DIMENSION,
    color: '#8B5CF6', // Purple
    icon: '🏛️',
  },
  {
    id: 'care',
    name: 'Value Scanner',
    description:
      'Passion for recognizing care work, supporting caregivers, and making invisible contributions visible',
    dimension: EPIC_DOMAIN_DIMENSION,
    color: '#EC4899', // Pink
    icon: '💝',
  },
  {
    id: 'economic',
    name: 'Economic Coordination',
    description:
      'Interest in transforming workplace dynamics, worker ownership, and equitable economic systems',
    dimension: EPIC_DOMAIN_DIMENSION,
    color: '#10B981', // Green
    icon: '📊',
  },
  {
    id: 'public',
    name: 'Public Observer',
    description: 'Commitment to democratic participation, transparency, and civic engagement',
    dimension: EPIC_DOMAIN_DIMENSION,
    color: '#3B82F6', // Blue
    icon: '🔍',
  },
  {
    id: 'social',
    name: 'Social Medium',
    description:
      'Focus on building healthier digital spaces, fostering connection, and improving online communication',
    dimension: EPIC_DOMAIN_DIMENSION,
    color: '#F59E0B', // Amber
    icon: '💬',
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Result Type Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const EPIC_DOMAIN_RESULT_TYPES: ResultTypeDefinition[] = [
  {
    id: 'governance',
    name: 'AI Constitutional',
    description:
      "You're drawn to shaping how AI systems are governed and ensuring they serve humanity through proper constitutional frameworks.",
  },
  {
    id: 'care',
    name: 'Value Scanner',
    description:
      "You're passionate about recognizing and valuing care work, supporting caregivers, and making invisible contributions visible.",
  },
  {
    id: 'economic',
    name: 'Economic Coordination',
    description:
      "You're interested in transforming workplace dynamics, promoting worker ownership, and creating more equitable economic systems.",
  },
  {
    id: 'public',
    name: 'Public Observer',
    description:
      "You're committed to strengthening democratic participation, increasing transparency, and empowering civic engagement.",
  },
  {
    id: 'social',
    name: 'Social Medium',
    description:
      "You're focused on building healthier digital spaces, fostering genuine connection, and improving online communication.",
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Instrument Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Epic Domain Discovery Instrument configuration.
 * Used with psyche-core's createInstrument() function.
 */
export const EPIC_DOMAIN_INSTRUMENT_CONFIG: CreateInstrumentOptions = {
  id: EPIC_DOMAIN_INSTRUMENT_ID,
  name: 'Epic Domain Discovery',
  category: 'vocational',
  description:
    'Discover which Elohim Protocol epic domain resonates most with your interests and values',
  version: '1.0.0',
  subscales: EPIC_DOMAIN_SUBSCALES,
  resultTypes: EPIC_DOMAIN_RESULT_TYPES,
  scoringConfig: {
    method: 'highest-subscale',
    normalize: true,
  },
  // Default interpret function - psyche-core handles this based on scoringConfig
  interpret: (aggregated: AggregatedReflection) => {
    // Find highest normalized score
    const entries = Object.entries(aggregated.normalizedScores);
    if (entries.length === 0) return null;

    const [primaryId] = entries.toSorted(([, a], [, b]) => b - a)[0];
    const resultType = EPIC_DOMAIN_RESULT_TYPES.find(r => r.id === primaryId);

    return {
      primaryType: {
        typeId: primaryId,
        typeName: resultType?.name ?? primaryId,
        description: resultType?.description,
      },
    };
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Helper Accessors (for backward compatibility with existing code)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Get subscale metadata by ID.
 */
export function getEpicSubscale(id: string): SubscaleDefinition | undefined {
  return EPIC_DOMAIN_SUBSCALES.find(s => s.id === id);
}

/**
 * Get all epic domain subscale IDs.
 */
export function getEpicSubscaleIds(): string[] {
  return EPIC_DOMAIN_SUBSCALES.map(s => s.id);
}

/**
 * Get result type by ID.
 */
export function getEpicResultType(id: string): ResultTypeDefinition | undefined {
  return EPIC_DOMAIN_RESULT_TYPES.find(r => r.id === id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Backward Compatibility Exports
// These functions are deprecated - use psyche-core API via getPsycheAPI() instead
// ─────────────────────────────────────────────────────────────────────────────

/**
 * @deprecated Use psyche-core's getPrimarySubscale() instead
 */
export function findPrimaryEpicDomain(subscaleTotals: Record<string, number>): {
  id: string;
  name: string;
  score: number;
  icon: string;
  color: string;
} | null {
  const entries = Object.entries(subscaleTotals);
  if (entries.length === 0) return null;

  let maxId = entries[0][0];
  let maxScore = entries[0][1];

  for (const [id, score] of entries) {
    if (score > maxScore) {
      maxScore = score;
      maxId = id;
    }
  }

  const subscale = getEpicSubscale(maxId);
  if (!subscale) return null;

  return {
    id: maxId,
    name: subscale.name,
    score: maxScore,
    icon: subscale.icon ?? '📌',
    color: subscale.color ?? '#888',
  };
}

/**
 * @deprecated Use psyche-core's getTopSubscales() instead
 */
export function sortEpicDomainsByScore(subscaleTotals: Record<string, number>): {
  id: string;
  name: string;
  icon: string;
  color: string;
  score: number;
  percent: number;
}[] {
  const total = Object.values(subscaleTotals).reduce((sum, v) => sum + v, 0) || 1;

  return EPIC_DOMAIN_SUBSCALES.map(subscale => ({
    id: subscale.id,
    name: subscale.name,
    icon: subscale.icon ?? '📌',
    color: subscale.color ?? '#888',
    score: subscaleTotals[subscale.id] || 0,
    percent: ((subscaleTotals[subscale.id] || 0) / total) * 100,
  })).sort((a, b) => b.score - a.score);
}
