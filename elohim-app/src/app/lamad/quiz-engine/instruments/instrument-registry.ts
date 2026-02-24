/**
 * Instrument Registry - Central registry for all psychometric instruments.
 *
 * Instruments self-register at import time. The discovery-quiz component
 * looks up instruments by ID to get subscales, result types, and config.
 */

import type {
  SubscaleDefinition,
  ResultTypeDefinition,
  CreateInstrumentOptions,
} from '../../content-io/plugins/sophia/sophia-element-loader';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export interface InstrumentRegistryEntry {
  config: CreateInstrumentOptions;
  subscales: SubscaleDefinition[];
  resultTypes: ResultTypeDefinition[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Registry
// ─────────────────────────────────────────────────────────────────────────────

const registry = new Map<string, InstrumentRegistryEntry>();

/**
 * Register an instrument in the global registry.
 * Called at module import time by each instrument file.
 */
export function registerInstrument(entry: InstrumentRegistryEntry): void {
  registry.set(entry.config.id, entry);
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
