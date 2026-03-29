// AUTO-GENERATED from avodah manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run avodah:codegen

import type { ContentView } from '../../generated/content-view';
import type { WorkStoryMeta, WorkProjectMeta } from './metadata-types';

export type AvodahTypedContentNode =
  | (ContentView & { contentType: 'work-story'; metadata: WorkStoryMeta })
  | (ContentView & { contentType: 'work-project'; metadata: WorkProjectMeta })
  | (ContentView & { contentType: string; metadata: Record<string, unknown> });

export function isWorkStoryNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'work-story'; metadata: WorkStoryMeta } {
  return node.contentType === 'work-story';
}

export function isWorkProjectNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'work-project'; metadata: WorkProjectMeta } {
  return node.contentType === 'work-project';
}
