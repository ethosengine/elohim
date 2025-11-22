/**
 * Barrel export for all documentation models
 */

export * from './document-node.model';
export * from './epic-node.model';
export * from './document-graph.model';
export * from './lamad-node-types';

// Export from node-relationship.model (keep the primary RelationshipType)
export * from './node-relationship.model';

// Export from feature-node.model (excluding ScenarioExamples to avoid conflict)
export type { FeatureNode, FeatureCategory } from './feature-node.model';

// Export from scenario-node.model (includes ScenarioExamples)
export * from './scenario-node.model';

// Export from content-node.model (excluding RelationshipType alias to avoid conflict)
export type {
  ContentNode,
  ContentMetadata,
  ContentFormat,
  ContentRelationship
} from './content-node.model';
export { ContentRelationshipType, ContentFormatType } from './content-node.model';
