// AUTO-GENERATED from imagodei manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run imagodei:codegen

import type { HumanMetadata, PresenceMetadata } from './metadata-types';
import type { ContentView } from '../../generated/content-view';

export type TypedIdentityNode =
  | (ContentView & { contentType: 'human'; metadata: HumanMetadata })
  | (ContentView & { contentType: 'contributor'; metadata: PresenceMetadata })
  | (ContentView & { contentType: string; metadata: Record<string, unknown> });

export function isHumanNode<T extends { contentType: string }>(
  node: T
): node is T & { contentType: 'human'; metadata: HumanMetadata } {
  return node.contentType === 'human';
}

export function isContributorNode<T extends { contentType: string }>(
  node: T
): node is T & { contentType: 'contributor'; metadata: PresenceMetadata } {
  return node.contentType === 'contributor';
}
