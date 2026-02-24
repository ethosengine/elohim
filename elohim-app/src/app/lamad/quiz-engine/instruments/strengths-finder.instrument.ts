/**
 * Strengths Finder Discovery Instrument
 *
 * Psychometric instrument definition for discovering character strengths.
 * Uses highest-subscale scoring to determine the primary strength.
 *
 * Inspired by the VIA Character Strengths Survey (adapted for educational purposes).
 *
 * Strengths:
 * - Creativity: Finding new ways to look at things
 * - Curiosity: Being curious about the world
 * - Fairness: Letting others share first
 * - Kindness: Helping everyone you meet
 * - Perseverance: Finishing what you start
 * - Hope: Finding the positive in the negative
 */

import { registerInstrument } from './instrument-registry';

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  CreateInstrumentOptions,
  AggregatedReflection,
} from '../../content-io/plugins/sophia/sophia-element-loader';

// ─────────────────────────────────────────────────────────────────────────────
// Instrument ID
// ─────────────────────────────────────────────────────────────────────────────

export const STRENGTHS_FINDER_INSTRUMENT_ID = 'strengths-finder-discovery';

const STRENGTHS_DIMENSION = 'character-strengths';

// ─────────────────────────────────────────────────────────────────────────────
// Subscale Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const STRENGTHS_FINDER_SUBSCALES: SubscaleDefinition[] = [
  {
    id: 'creativity',
    name: 'Creativity',
    description: 'Finding new ways to look at things, imagination and independent thinking',
    dimension: STRENGTHS_DIMENSION,
    color: '#F59E0B',
    icon: '🎨',
  },
  {
    id: 'curiosity',
    name: 'Curiosity',
    description: 'Being curious about the world, seeking new experiences and knowledge',
    dimension: STRENGTHS_DIMENSION,
    color: '#3B82F6',
    icon: '🔍',
  },
  {
    id: 'fairness',
    name: 'Fairness',
    description: 'Treating all people fairly, giving everyone a chance',
    dimension: STRENGTHS_DIMENSION,
    color: '#8B5CF6',
    icon: '⚖️',
  },
  {
    id: 'kindness',
    name: 'Kindness',
    description: 'Being generous and helpful to others, showing compassion',
    dimension: STRENGTHS_DIMENSION,
    color: '#EC4899',
    icon: '💝',
  },
  {
    id: 'perseverance',
    name: 'Perseverance',
    description: 'Finishing what you start, persistence through challenges',
    dimension: STRENGTHS_DIMENSION,
    color: '#10B981',
    icon: '🏔️',
  },
  {
    id: 'hope',
    name: 'Hope',
    description: 'Finding the positive in difficult situations, optimism about the future',
    dimension: STRENGTHS_DIMENSION,
    color: '#06B6D4',
    icon: '🌟',
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Result Type Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const STRENGTHS_FINDER_RESULT_TYPES: ResultTypeDefinition[] = [
  {
    id: 'creativity',
    name: 'Creativity',
    description:
      'Your signature strength is creativity. You naturally find new ways to look at things and think outside the box.',
  },
  {
    id: 'curiosity',
    name: 'Curiosity',
    description:
      'Your signature strength is curiosity. You have a natural drive to explore, learn, and understand the world around you.',
  },
  {
    id: 'fairness',
    name: 'Fairness',
    description:
      'Your signature strength is fairness. You believe in treating everyone equitably and giving all people a fair chance.',
  },
  {
    id: 'kindness',
    name: 'Kindness',
    description:
      'Your signature strength is kindness. You naturally look for ways to help others and show compassion.',
  },
  {
    id: 'perseverance',
    name: 'Perseverance',
    description:
      'Your signature strength is perseverance. You finish what you start and persist through challenges.',
  },
  {
    id: 'hope',
    name: 'Hope',
    description:
      'Your signature strength is hope. You can always find the positive in what seems negative and stay optimistic about the future.',
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Instrument Configuration
// ─────────────────────────────────────────────────────────────────────────────

export const STRENGTHS_FINDER_INSTRUMENT_CONFIG: CreateInstrumentOptions = {
  id: STRENGTHS_FINDER_INSTRUMENT_ID,
  name: 'Character Strengths Discovery',
  category: 'personality',
  description: 'Discover your signature character strengths',
  version: '1.0.0',
  subscales: STRENGTHS_FINDER_SUBSCALES,
  resultTypes: STRENGTHS_FINDER_RESULT_TYPES,
  scoringConfig: {
    method: 'highest-subscale',
    normalize: true,
  },
  interpret: (aggregated: AggregatedReflection) => {
    const entries = Object.entries(aggregated.normalizedScores);
    if (entries.length === 0) return null;

    const sorted = [...entries].sort(([, a]: [string, number], [, b]: [string, number]) => b - a);
    const [primaryId] = sorted[0];
    const resultType = STRENGTHS_FINDER_RESULT_TYPES.find(r => r.id === primaryId);

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
// Self-Registration
// ─────────────────────────────────────────────────────────────────────────────

registerInstrument({
  config: STRENGTHS_FINDER_INSTRUMENT_CONFIG,
  subscales: STRENGTHS_FINDER_SUBSCALES,
  resultTypes: STRENGTHS_FINDER_RESULT_TYPES,
});
