// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

import type { ContentView } from './content-view.js';
import type { ConceptMetadata, AssessmentMetadata, ExperienceStoryMetadata, ExperienceMomentMetadata, PathMetadata } from './metadata-types.js';

export type TypedContentNode =
  | (ContentView & { contentType: 'concept'; metadata: ConceptMetadata })
  | (ContentView & { contentType: 'assessment'; metadata: AssessmentMetadata })
  | (ContentView & { contentType: 'experience-story'; metadata: ExperienceStoryMetadata })
  | (ContentView & { contentType: 'experience-moment'; metadata: ExperienceMomentMetadata })
  | (ContentView & { contentType: 'path'; metadata: PathMetadata })
  | (ContentView & { contentType: string; metadata: Record<string, unknown> });

export function isConceptNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'concept'; metadata: ConceptMetadata } {
  return node.contentType === 'concept';
}

export function isAssessmentNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'assessment'; metadata: AssessmentMetadata } {
  return node.contentType === 'assessment';
}

export function isExperienceStoryNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'experience-story'; metadata: ExperienceStoryMetadata } {
  return node.contentType === 'experience-story';
}

export function isExperienceMomentNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'experience-moment'; metadata: ExperienceMomentMetadata } {
  return node.contentType === 'experience-moment';
}

export function isPathNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'path'; metadata: PathMetadata } {
  return node.contentType === 'path';
}
