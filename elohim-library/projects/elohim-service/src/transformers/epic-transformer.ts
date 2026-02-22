/**
 * Epic Transformer
 *
 * Transforms epic.md files into 'epic' ContentNodes.
 * Epics are high-level narrative containers (governance, economic, cultural, etc.)
 *
 * Source: data/content/elohim-protocol/governance/epic.md
 * Output: ContentNode with contentType: 'epic'
 */

import { ContentNode } from '../models/content-node.model';
import { ParsedContent } from '../models/import-context.model';
import { extractTags, extractDescription, extractRelatedUsers } from '../parsers/markdown-parser';
import {
  normalizeId,
  buildBaseMetadata,
  addProvenanceMetadata,
  addGovernanceScopeMetadata,
  buildContentNode,
  titleCaseWithSuffix,
} from '../utils';

/**
 * Transform epic content into an epic ContentNode
 */
export function transformEpic(parsed: ParsedContent, sourceNodeId?: string): ContentNode {
  const now = new Date().toISOString();

  // Generate epic node ID
  const id = generateEpicId(parsed);

  // Extract tags
  const tags = extractTags(parsed);
  tags.push('epic');

  // Extract description
  const description = extractDescription(parsed);

  // Build metadata
  const metadata: Record<string, unknown> = {
    category: 'epic',
    epicName: parsed.pathMeta.epic,
    domain: parsed.pathMeta.domain,
    ...buildBaseMetadata(),
  };

  // Add provenance link
  addProvenanceMetadata(metadata, sourceNodeId);

  // Add frontmatter fields
  if (parsed.frontmatter.epic_domain) {
    metadata.epicDomain = parsed.frontmatter.epic_domain;
  }
  if (parsed.frontmatter.vision) {
    metadata.vision = parsed.frontmatter.vision;
  }
  if (parsed.frontmatter.scope) {
    metadata.scope = parsed.frontmatter.scope;
  }

  // Add governance scope if present
  addGovernanceScopeMetadata(metadata, parsed.frontmatter);

  // Extract related users for graph connections
  const relatedNodeIds = extractRelatedUsers(parsed.frontmatter);

  // Add source node to related
  if (sourceNodeId) {
    relatedNodeIds.push(sourceNodeId);
  }

  return buildContentNode({
    id,
    contentType: 'epic',
    title: extractEpicTitle(parsed),
    description,
    content: parsed.rawContent,
    contentFormat: 'markdown',
    tags,
    sourcePath: parsed.pathMeta.fullPath,
    relatedNodeIds,
    metadata,
    createdAt: now,
  });
}

/**
 * Generate epic node ID
 */
function generateEpicId(parsed: ParsedContent): string {
  const parts = ['epic'];

  // Add epic name
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    parts.push(parsed.pathMeta.epic);
  } else if (parsed.frontmatter.epic && typeof parsed.frontmatter.epic === 'string') {
    parts.push(parsed.frontmatter.epic);
  }

  // Normalize and join
  return normalizeId(parts);
}

/**
 * Extract epic title
 */
function extractEpicTitle(parsed: ParsedContent): string {
  // Priority 1: title from frontmatter
  if (parsed.frontmatter.title && typeof parsed.frontmatter.title === 'string') {
    return parsed.frontmatter.title;
  }

  // Priority 2: epic_domain from frontmatter
  if (parsed.frontmatter.epic_domain && typeof parsed.frontmatter.epic_domain === 'string') {
    return parsed.frontmatter.epic_domain;
  }

  // Priority 3: parsed title
  if (parsed.title && parsed.title !== 'Untitled') {
    return parsed.title;
  }

  // Priority 4: epic name formatted
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    return formatEpicName(parsed.pathMeta.epic);
  }

  return 'Unknown Epic';
}

/**
 * Format epic name for display
 */
function formatEpicName(epic: string): string {
  return titleCaseWithSuffix(epic, 'Epic');
}

/**
 * Check if content should be transformed as epic
 */
export function isEpicContent(parsed: ParsedContent): boolean {
  return parsed.pathMeta.isEpicNarrative;
}
