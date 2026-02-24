/**
 * Values Hierarchy Discovery Instrument
 *
 * Psychometric instrument definition for exploring personal value priorities.
 * Uses dimensional scoring to produce a profile of all four value dimensions
 * (not winner-take-all).
 *
 * Inspired by the Schwartz Values Survey (adapted for educational purposes).
 *
 * Value Dimensions:
 * - Openness to Change: Creativity, independence, stimulation
 * - Conservation: Security, tradition, conformity
 * - Self-Transcendence: Benevolence, universalism
 * - Self-Enhancement: Achievement, power
 */

import { registerInstrument } from './instrument-registry';

// @coverage: 100.0% (2026-02-24)

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  CreateInstrumentOptions,
  AggregatedReflection,
} from '../../content-io/plugins/sophia/sophia-element-loader';

// ─────────────────────────────────────────────────────────────────────────────
// Instrument ID
// ─────────────────────────────────────────────────────────────────────────────

export const VALUES_HIERARCHY_INSTRUMENT_ID = 'values-hierarchy-discovery';

const VALUES_DIMENSION = 'values';

// ─────────────────────────────────────────────────────────────────────────────
// Subscale Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const VALUES_HIERARCHY_SUBSCALES: SubscaleDefinition[] = [
  {
    id: 'openness',
    name: 'Openness to Change',
    description: 'Valuing creativity, curiosity, independence, and stimulation',
    dimension: VALUES_DIMENSION,
    color: '#F59E0B',
    icon: '🌊',
  },
  {
    id: 'conservation',
    name: 'Conservation',
    description: 'Valuing security, stability, tradition, and social harmony',
    dimension: VALUES_DIMENSION,
    color: '#10B981',
    icon: '🏠',
  },
  {
    id: 'self-transcendence',
    name: 'Self-Transcendence',
    description: 'Valuing benevolence, universalism, and welfare of others',
    dimension: VALUES_DIMENSION,
    color: '#8B5CF6',
    icon: '🌍',
  },
  {
    id: 'self-enhancement',
    name: 'Self-Enhancement',
    description: 'Valuing personal achievement, success, and demonstrating competence',
    dimension: VALUES_DIMENSION,
    color: '#EF4444',
    icon: '🏆',
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Result Type Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const VALUES_HIERARCHY_RESULT_TYPES: ResultTypeDefinition[] = [
  {
    id: 'openness',
    name: 'Openness to Change',
    description:
      'You prioritize creativity, independence, and new experiences. You value the freedom to explore ideas and chart your own course.',
  },
  {
    id: 'conservation',
    name: 'Conservation',
    description:
      'You prioritize security, stability, and tradition. You value the harmony and predictability that established systems provide.',
  },
  {
    id: 'self-transcendence',
    name: 'Self-Transcendence',
    description:
      'You prioritize the welfare of others, benevolence, and universal understanding. You value making a positive difference for people beyond yourself.',
  },
  {
    id: 'self-enhancement',
    name: 'Self-Enhancement',
    description:
      'You prioritize personal achievement and demonstrating competence. You value setting and reaching ambitious goals.',
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Instrument Configuration
// ─────────────────────────────────────────────────────────────────────────────

export const VALUES_HIERARCHY_INSTRUMENT_CONFIG: CreateInstrumentOptions = {
  id: VALUES_HIERARCHY_INSTRUMENT_ID,
  name: 'Personal Values Hierarchy',
  category: 'values',
  description: 'Discover your core values and how they guide your decisions',
  version: '1.0.0',
  subscales: VALUES_HIERARCHY_SUBSCALES,
  resultTypes: VALUES_HIERARCHY_RESULT_TYPES,
  scoringConfig: {
    method: 'dimensional',
    normalize: true,
  },
  interpret: (aggregated: AggregatedReflection) => {
    const scores = aggregated.normalizedScores;
    const entries = Object.entries(scores);
    if (entries.length === 0) return null;

    // For dimensional scoring, return profile of all dimensions
    // Primary is still identified for display purposes
    const sorted = [...entries].sort(([, a]: [string, number], [, b]: [string, number]) => b - a);
    const [primaryId] = sorted[0];
    const resultType = VALUES_HIERARCHY_RESULT_TYPES.find(r => r.id === primaryId);

    return {
      primaryType: {
        typeId: primaryId,
        typeName: resultType?.name ?? primaryId,
        description: resultType?.description,
      },
      profile: {
        dimensions: Object.fromEntries(
          VALUES_HIERARCHY_SUBSCALES.map(s => [s.id, scores[s.id] ?? 0])
        ),
      },
    };
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Self-Registration
// ─────────────────────────────────────────────────────────────────────────────

registerInstrument({
  config: VALUES_HIERARCHY_INSTRUMENT_CONFIG,
  subscales: VALUES_HIERARCHY_SUBSCALES,
  resultTypes: VALUES_HIERARCHY_RESULT_TYPES,
  displayConfig: {
    framework: 'values-hierarchy',
    frameworkDisplayName: 'Values Hierarchy',
    autoFeature: false,
    displayTemplate: '{{primaryType.name}}',
    shortTemplate: '{{primaryType.shortCode}}',
  },
});
