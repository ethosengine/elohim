/**
 * Utility functions for building ContentNode objects
 */

import { ContentNode, ContentType, ContentReach } from '../models/content-node.model';

/**
 * Build a ContentNode with standard fields
 * @param config - Configuration object for the ContentNode
 * @returns A complete ContentNode object
 */
export function buildContentNode(config: {
  id: string;
  contentType: ContentType;
  title: string;
  description: string;
  content: string;
  contentFormat: 'markdown' | 'gherkin';
  tags: string[];
  sourcePath: string;
  relatedNodeIds: string[];
  metadata: Record<string, unknown>;
  reach?: ContentReach;
  createdAt?: string;
  updatedAt?: string;
}): ContentNode {
  const now = config.createdAt || new Date().toISOString();
  return {
    id: config.id,
    contentType: config.contentType,
    title: config.title,
    description: config.description,
    content: config.content,
    contentFormat: config.contentFormat,
    tags: config.tags,
    sourcePath: config.sourcePath,
    relatedNodeIds: config.relatedNodeIds,
    metadata: config.metadata,
    reach: config.reach ?? 'commons',
    createdAt: now,
    updatedAt: config.updatedAt ?? now
  };
}
