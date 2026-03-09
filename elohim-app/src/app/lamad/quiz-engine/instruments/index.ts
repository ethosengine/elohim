/**
 * Instrument barrel - re-exports all instruments and the registry.
 *
 * Importing this module triggers self-registration of all instruments.
 */

// Registry (import first so it's available for instrument self-registration)
export {
  registerInstrument,
  getInstrument,
  getRegisteredInstrumentIds,
  getDisplayConfigByFramework,
  registerFromDefinition,
} from './instrument-registry';
export type { InstrumentRegistryEntry, InstrumentDisplayConfig } from './instrument-registry';

// Instruments (each self-registers on import)
export {
  EPIC_DOMAIN_INSTRUMENT_ID,
  EPIC_DOMAIN_SUBSCALES,
  EPIC_DOMAIN_RESULT_TYPES,
  EPIC_DOMAIN_INSTRUMENT_CONFIG,
  getEpicSubscale,
  getEpicSubscaleIds,
  getEpicResultType,
  findPrimaryEpicDomain,
  sortEpicDomainsByScore,
} from './epic-domain.instrument';

export {
  STRENGTHS_FINDER_INSTRUMENT_ID,
  STRENGTHS_FINDER_SUBSCALES,
  STRENGTHS_FINDER_RESULT_TYPES,
  STRENGTHS_FINDER_INSTRUMENT_CONFIG,
} from './strengths-finder.instrument';

export {
  ATTACHMENT_STYLE_INSTRUMENT_ID,
  ATTACHMENT_STYLE_SUBSCALES,
  ATTACHMENT_STYLE_RESULT_TYPES,
  ATTACHMENT_STYLE_INSTRUMENT_CONFIG,
} from './attachment-style.instrument';

export {
  VALUES_HIERARCHY_INSTRUMENT_ID,
  VALUES_HIERARCHY_SUBSCALES,
  VALUES_HIERARCHY_RESULT_TYPES,
  VALUES_HIERARCHY_INSTRUMENT_CONFIG,
} from './values-hierarchy.instrument';
