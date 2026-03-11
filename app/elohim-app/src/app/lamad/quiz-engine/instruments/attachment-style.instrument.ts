/**
 * Attachment Style Discovery Instrument
 *
 * Psychometric instrument definition for exploring attachment patterns.
 * Uses threshold-based scoring on two dimensions (anxiety, avoidance)
 * to determine attachment style via quadrant logic.
 *
 * Inspired by Experiences in Close Relationships (ECR-R) (adapted for
 * educational purposes).
 *
 * Attachment Styles (quadrant model):
 * - Secure: Low anxiety, low avoidance
 * - Anxious-Preoccupied: High anxiety, low avoidance
 * - Dismissive-Avoidant: Low anxiety, high avoidance
 * - Fearful-Avoidant: High anxiety, high avoidance
 */

import { registerInstrument } from './instrument-registry';

// @coverage: 38.1% (2026-02-24)

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  CreateInstrumentOptions,
  AggregatedReflection,
} from '../../content-io/plugins/sophia/sophia-element-loader';

// ─────────────────────────────────────────────────────────────────────────────
// Instrument ID
// ─────────────────────────────────────────────────────────────────────────────

export const ATTACHMENT_STYLE_INSTRUMENT_ID = 'attachment-style-discovery';

const ATTACHMENT_DIMENSION = 'attachment';

// ─────────────────────────────────────────────────────────────────────────────
// Subscale Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const ATTACHMENT_STYLE_SUBSCALES: SubscaleDefinition[] = [
  {
    id: 'anxiety',
    name: 'Anxiety',
    description: 'Worry about abandonment and need for reassurance in relationships',
    dimension: ATTACHMENT_DIMENSION,
    color: '#EF4444',
    icon: '💭',
  },
  {
    id: 'avoidance',
    name: 'Avoidance',
    description: 'Discomfort with closeness and reluctance to depend on others',
    dimension: ATTACHMENT_DIMENSION,
    color: '#6366F1',
    icon: '🛡️',
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Result Type Definitions
// ─────────────────────────────────────────────────────────────────────────────

export const ATTACHMENT_STYLE_RESULT_TYPES: ResultTypeDefinition[] = [
  {
    id: 'secure',
    name: 'Secure',
    description:
      'You tend to feel comfortable with closeness and are confident in your relationships. You find it relatively easy to depend on others and have others depend on you.',
    subscaleProfile: { anxiety: 0, avoidance: 0 },
  },
  {
    id: 'anxious-preoccupied',
    name: 'Anxious-Preoccupied',
    description:
      'You tend to seek high levels of closeness and approval from others. You may worry about whether your relationships are as strong as you want them to be.',
    subscaleProfile: { anxiety: 1, avoidance: 0 },
  },
  {
    id: 'dismissive-avoidant',
    name: 'Dismissive-Avoidant',
    description:
      'You tend to value independence and self-sufficiency. You may prefer not to depend on others or have others depend on you.',
    subscaleProfile: { anxiety: 0, avoidance: 1 },
  },
  {
    id: 'fearful-avoidant',
    name: 'Fearful-Avoidant',
    description:
      'You may have mixed feelings about closeness. You desire close relationships but may find it difficult to trust or depend on others fully.',
    subscaleProfile: { anxiety: 1, avoidance: 1 },
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// Thresholds
// ─────────────────────────────────────────────────────────────────────────────

// Midpoint of a 7-point Likert scale = 4.0
// anxiety: 3 questions (max raw = 21), midpoint = 12
// avoidance: 2 questions reverse-scored (max raw = 14), midpoint = 8
const ANXIETY_MIDPOINT = 12;
const AVOIDANCE_MIDPOINT = 8;

// ─────────────────────────────────────────────────────────────────────────────
// Instrument Configuration
// ─────────────────────────────────────────────────────────────────────────────

export const ATTACHMENT_STYLE_INSTRUMENT_CONFIG: CreateInstrumentOptions = {
  id: ATTACHMENT_STYLE_INSTRUMENT_ID,
  name: 'Attachment Style Exploration',
  category: 'personality',
  description: 'Understand your patterns in close relationships',
  version: '1.0.0',
  subscales: ATTACHMENT_STYLE_SUBSCALES,
  resultTypes: ATTACHMENT_STYLE_RESULT_TYPES,
  scoringConfig: {
    method: 'threshold-based',
    normalize: false,
    thresholds: {
      anxiety: ANXIETY_MIDPOINT,
      avoidance: AVOIDANCE_MIDPOINT,
    },
  },
  interpret: (aggregated: AggregatedReflection) => {
    const anxietyScore = aggregated.subscaleTotals['anxiety'] ?? 0;
    const avoidanceScore = aggregated.subscaleTotals['avoidance'] ?? 0;

    const highAnxiety = anxietyScore >= ANXIETY_MIDPOINT;
    const highAvoidance = avoidanceScore >= AVOIDANCE_MIDPOINT;

    let resultId: string;
    if (!highAnxiety && !highAvoidance) {
      resultId = 'secure';
    } else if (highAnxiety && !highAvoidance) {
      resultId = 'anxious-preoccupied';
    } else if (!highAnxiety && highAvoidance) {
      resultId = 'dismissive-avoidant';
    } else {
      resultId = 'fearful-avoidant';
    }

    const resultType = ATTACHMENT_STYLE_RESULT_TYPES.find(r => r.id === resultId);

    return {
      primaryType: {
        typeId: resultId,
        typeName: resultType?.name ?? resultId,
        description: resultType?.description,
      },
      profile: {
        dimensions: {
          anxiety: anxietyScore,
          avoidance: avoidanceScore,
        },
      },
    };
  },
};

// ─────────────────────────────────────────────────────────────────────────────
// Self-Registration
// ─────────────────────────────────────────────────────────────────────────────

registerInstrument({
  config: ATTACHMENT_STYLE_INSTRUMENT_CONFIG,
  subscales: ATTACHMENT_STYLE_SUBSCALES,
  resultTypes: ATTACHMENT_STYLE_RESULT_TYPES,
  displayConfig: {
    framework: 'attachment-style',
    frameworkDisplayName: 'Attachment Style',
    autoFeature: false,
    displayTemplate: '{{primaryType.name}}',
    shortTemplate: '{{primaryType.shortCode}}',
  },
});
