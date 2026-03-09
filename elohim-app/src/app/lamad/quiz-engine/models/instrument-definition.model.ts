/**
 * Instrument Definition Model - Unified schema for psychometric instruments.
 *
 * This is the serializable definition that lives in a ContentNode's body
 * (contentType: 'instrument', contentFormat: 'instrument-json').
 *
 * It merges:
 * - CreateInstrumentOptions (psychometric structure)
 * - InstrumentDisplayConfig (display rules)
 * - AssessmentInstrument metadata (validation, contributors, licensing)
 *
 * Instruments flow through the content pipeline:
 * genesis JSON -> seeder -> elohim-storage -> doorway -> InstrumentLoaderService -> registry
 */

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  ScoringConfig,
} from '../../content-io/plugins/sophia/sophia-element-loader';
import type { InstrumentDisplayConfig } from '../instruments/instrument-registry';

// ─────────────────────────────────────────────────────────────────────────────
// Instrument Definition
// ─────────────────────────────────────────────────────────────────────────────

export interface InstrumentDefinition {
  // Core identity
  id: string;
  name: string;
  version: string;
  category: string;
  framework: string;
  description: string;

  // Psychometric structure
  subscales: SubscaleDefinition[];
  resultTypes: ResultTypeDefinition[];
  scoringConfig: ScoringConfig;

  // Display rules (from Step 1)
  display: InstrumentDisplayConfig;

  // Metadata (optional, from AssessmentInstrument)
  estimatedMinutes?: number;
  recommendedInterval?: string; // ISO 8601 duration (e.g., "P6M")
  contentWarnings?: string[];
  license?: string;
  reach?: string;
}
