/**
 * Instrument Registry - Central registry for all psychometric instruments.
 *
 * Instruments self-register at import time. The discovery-quiz component
 * looks up instruments by ID to get subscales, result types, and config.
 *
 * Display configs are indexed by framework for format-function lookups.
 */

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  CreateInstrumentOptions,
  AggregatedReflection,
} from '../../content-io/plugins/sophia/sophia-element-loader';
import type { InstrumentDefinition } from '../models/instrument-definition.model';

// @coverage: 100.0% (2026-02-24)

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Display configuration for an instrument.
 * Carries the data needed by format functions so new instruments
 * don't require code changes (switch statements, Record lookups).
 */
export interface InstrumentDisplayConfig {
  /** Framework key for lookups (e.g., 'attachment-style') */
  framework: string;
  /** Human-readable framework name (e.g., "Attachment Style") */
  frameworkDisplayName: string;
  /** Whether to auto-feature on profile */
  autoFeature: boolean;
  /** Template for full display string using {{path}} interpolation */
  displayTemplate?: string;
  /** Template for badge/short display string */
  shortTemplate?: string;
}

export interface InstrumentRegistryEntry {
  config: CreateInstrumentOptions;
  subscales: SubscaleDefinition[];
  resultTypes: ResultTypeDefinition[];
  displayConfig?: InstrumentDisplayConfig;
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry
// ─────────────────────────────────────────────────────────────────────────────

const registry = new Map<string, InstrumentRegistryEntry>();
const frameworkIndex = new Map<string, InstrumentDisplayConfig>();

/**
 * Register an instrument in the global registry.
 * Called at module import time by each instrument file.
 */
export function registerInstrument(entry: InstrumentRegistryEntry): void {
  registry.set(entry.config.id, entry);
  if (entry.displayConfig) {
    frameworkIndex.set(entry.displayConfig.framework, entry.displayConfig);
  }
}

/**
 * Look up an instrument by ID.
 */
export function getInstrument(id: string): InstrumentRegistryEntry | undefined {
  return registry.get(id);
}

/**
 * Get all registered instrument IDs.
 */
export function getRegisteredInstrumentIds(): string[] {
  return Array.from(registry.keys());
}

/**
 * Look up display config by framework key.
 * Used by format functions to get data-driven display rules.
 */
export function getDisplayConfigByFramework(
  framework: string
): InstrumentDisplayConfig | undefined {
  return frameworkIndex.get(framework);
}

/**
 * Register an instrument from a content-pipeline InstrumentDefinition.
 * Converts the serializable definition into a registry entry with an
 * interpret closure built from the scoring config.
 */
export function registerFromDefinition(def: InstrumentDefinition): InstrumentRegistryEntry {
  const config: CreateInstrumentOptions = {
    id: def.id,
    name: def.name,
    category: def.category,
    description: def.description,
    version: def.version,
    subscales: def.subscales,
    resultTypes: def.resultTypes,
    scoringConfig: def.scoringConfig,
    interpret: (aggregated: AggregatedReflection) => {
      return interpretFromScoringConfig(def.scoringConfig, aggregated, def.resultTypes);
    },
  };

  const entry: InstrumentRegistryEntry = {
    config,
    subscales: def.subscales,
    resultTypes: def.resultTypes,
    displayConfig: def.display,
  };

  registerInstrument(entry);
  return entry;
}

// ─────────────────────────────────────────────────────────────────────────────
// Scoring Interpretation (basic engine, expanded in Step 4)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Interpret aggregated reflections using the scoring config.
 * Returns a primary type result based on the scoring method.
 */
function findHighestSubscale(
  entries: [string, number][],
  resultTypes: ResultTypeDefinition[]
): { primaryType: { typeId: string; typeName: string; description?: string } } {
  const sorted = [...entries].sort(([, a], [, b]) => b - a);
  const [primaryId] = sorted[0];
  const resultType = resultTypes.find(r => r.id === primaryId);
  return {
    primaryType: {
      typeId: primaryId,
      typeName: resultType?.name ?? primaryId,
      description: resultType?.description,
    },
  };
}

function interpretFromScoringConfig(
  scoringConfig: CreateInstrumentOptions['scoringConfig'],
  aggregated: AggregatedReflection,
  resultTypes: ResultTypeDefinition[]
): { primaryType: { typeId: string; typeName: string; description?: string } } | null {
  const scores = scoringConfig.normalize ? aggregated.normalizedScores : aggregated.subscaleTotals;
  const entries = Object.entries(scores);
  if (entries.length === 0) return null;

  switch (scoringConfig.method) {
    case 'highest-subscale':
    case 'dimensional':
      return findHighestSubscale(entries, resultTypes);

    case 'threshold-based': {
      if (!scoringConfig.thresholds) return null;
      const profile: Record<string, number> = {};
      for (const [subscale, threshold] of Object.entries(scoringConfig.thresholds)) {
        profile[subscale] = (scores[subscale] ?? 0) >= threshold ? 1 : 0;
      }
      const match = resultTypes.find(
        rt =>
          rt.subscaleProfile &&
          Object.entries(rt.subscaleProfile).every(([key, val]) => profile[key] === val)
      );
      return {
        primaryType: {
          typeId: match?.id ?? 'unknown',
          typeName: match?.name ?? 'Unknown',
          description: match?.description,
        },
      };
    }

    default:
      return null;
  }
}
