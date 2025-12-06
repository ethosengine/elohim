/**
 * Transformers convert parsed content into ContentNodes
 *
 * Each transformer handles a specific content type:
 * - source: Preserves raw content with provenance metadata
 * - archetype: Transforms README.md files into role definitions
 * - epic: Transforms epic.md files into epic narratives
 * - scenario: Transforms .feature files into behavioral scenarios
 * - resource: Transforms books/videos/tools into reference nodes
 */

export * from './source-transformer';
export * from './archetype-transformer';
export * from './epic-transformer';
export * from './scenario-transformer';
export * from './resource-transformer';

// Re-export key functions for convenience
export {
  transformToSourceNode,
  shouldCreateSourceNode
} from './source-transformer';

export {
  transformArchetype,
  isArchetypeContent
} from './archetype-transformer';

export {
  transformEpic,
  isEpicContent
} from './epic-transformer';

export {
  transformScenarios,
  transformFeatureFile,
  isScenarioContent
} from './scenario-transformer';

export {
  transformResource,
  isResourceContent,
  getResourceType
} from './resource-transformer';
