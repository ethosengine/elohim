// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

import type { ContentView } from '../../generated/content-view';
import type { ConceptMetadata, AssessmentMetadata, PathMetadata } from './metadata-types';

export type TypedContentNode =
  | (ContentView & { contentType: 'concept'; metadata: ConceptMetadata })
  | (ContentView & { contentType: 'assessment'; metadata: AssessmentMetadata })
  | (ContentView & { contentType: 'path'; metadata: PathMetadata })
  | (ContentView & { contentType: string; metadata: Record<string, unknown> });

export function isConceptNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'concept'; metadata: ConceptMetadata } {
  return node.contentType === 'concept';
}

export function isAssessmentNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'assessment'; metadata: AssessmentMetadata } {
  return node.contentType === 'assessment';
}

export function isPathNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'path'; metadata: PathMetadata } {
  return node.contentType === 'path';
}
