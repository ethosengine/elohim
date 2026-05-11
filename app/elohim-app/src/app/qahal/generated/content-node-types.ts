// AUTO-GENERATED from qahal manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run qahal:codegen

import type { ContentView } from '../../generated/content-view';
import type { CollectiveMetadata, ProposalMetadata, ChallengeMetadata, StatementMetadata } from './metadata-types';

export type QahalTypedContentNode =
  | (ContentView & { contentType: 'collective'; metadata: CollectiveMetadata })
  | (ContentView & { contentType: 'proposal'; metadata: ProposalMetadata })
  | (ContentView & { contentType: 'challenge'; metadata: ChallengeMetadata })
  | (ContentView & { contentType: 'statement'; metadata: StatementMetadata })
  | (ContentView & { contentType: string; metadata: Record<string, unknown> });

export function isCollectiveNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'collective'; metadata: CollectiveMetadata } {
  return node.contentType === 'collective';
}

export function isProposalNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'proposal'; metadata: ProposalMetadata } {
  return node.contentType === 'proposal';
}

export function isChallengeNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'challenge'; metadata: ChallengeMetadata } {
  return node.contentType === 'challenge';
}

export function isStatementNode<T extends { contentType: string }>(node: T): node is T & { contentType: 'statement'; metadata: StatementMetadata } {
  return node.contentType === 'statement';
}
