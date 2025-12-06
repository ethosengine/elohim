/**
 * Source Node Transformer
 *
 * Creates ContentNodes of type 'source' that preserve the raw imported content.
 * These form the provenance layer - every derived concept links back to its source.
 *
 * Source nodes:
 * - Store raw content exactly as imported
 * - Are not typically included in learning paths
 * - Enable "view source" for curious learners
 * - Provide audit trail for content derivation
 */

import { ContentNode } from '../models/content-node.model';
import { ParsedContent } from '../models/import-context.model';
import {
  extractTags,
  extractDescription,
  extractRelatedUsers,
  extractGovernanceScope
} from '../parsers/markdown-parser';
import { extractGherkinTags, extractGherkinDescription } from '../parsers/gherkin-parser';

/**
 * Transform parsed content into a source ContentNode
 */
export function transformToSourceNode(parsed: ParsedContent): ContentNode {
  const now = new Date().toISOString();
  const isGherkin = parsed.pathMeta.extension === '.feature';

  // Generate source node ID
  const id = generateSourceId(parsed);

  // Extract tags based on content type
  const tags = isGherkin
    ? extractGherkinTags(parsed)
    : extractTags(parsed);

  // Always add 'source' tag
  tags.push('source');

  // Extract description
  const description = isGherkin
    ? extractGherkinDescription(parsed)
    : extractDescription(parsed);

  // Determine source type from path metadata
  const sourceType = determineSourceType(parsed);

  // Build metadata
  const metadata: Record<string, unknown> = {
    sourceType,
    sourcePath: parsed.pathMeta.relativePath,
    importedAt: now,
    importVersion: '1.0.0',
    contentHash: parsed.contentHash,
    category: parsed.pathMeta.contentCategory
  };

  // Add frontmatter fields
  if (parsed.frontmatter.epic) {
    metadata.epic = parsed.frontmatter.epic;
  }
  if (parsed.frontmatter.user_type) {
    metadata.userType = parsed.frontmatter.user_type;
  }

  // Add governance scope if present
  const governanceScope = extractGovernanceScope(parsed.frontmatter);
  if (governanceScope.length > 0) {
    metadata.governanceScope = governanceScope;
  }

  // Extract related users for graph connections
  const relatedNodeIds = extractRelatedUsers(parsed.frontmatter);

  return {
    id,
    contentType: 'source',
    title: `${parsed.title} (Source)`,
    description,
    content: parsed.rawContent,
    contentFormat: isGherkin ? 'gherkin' : 'markdown',
    tags,
    sourcePath: parsed.pathMeta.fullPath,
    relatedNodeIds,
    metadata,
    reach: 'commons',
    createdAt: now,
    updatedAt: now
  };
}

/**
 * Generate source node ID
 */
function generateSourceId(parsed: ParsedContent): string {
  const parts = ['source'];

  // Add epic if present
  const epic = parsed.pathMeta.epic;
  if (epic && epic !== 'other') {
    parts.push(epic);
  }

  // Add user type if present
  if (parsed.pathMeta.userType) {
    parts.push(parsed.pathMeta.userType);
  }

  // Add base name (unless generic)
  const baseName = parsed.pathMeta.baseName.toLowerCase();
  if (!['readme', 'epic', 'index'].includes(baseName)) {
    parts.push(baseName);
  } else {
    // For generic names, add content category
    parts.push(parsed.pathMeta.contentCategory);
  }

  // Normalize and join
  return parts
    .map(p => p.toLowerCase().replace(/[^a-z0-9]/g, '-'))
    .join('-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Determine source type from parsed content
 */
function determineSourceType(parsed: ParsedContent): string {
  if (parsed.pathMeta.isEpicNarrative) {
    return 'epic-narrative';
  }

  if (parsed.pathMeta.isArchetypeDefinition) {
    return 'archetype-definition';
  }

  if (parsed.pathMeta.isScenario) {
    return 'behavioral-scenario';
  }

  if (parsed.pathMeta.isResource) {
    return `resource-${parsed.pathMeta.resourceType || 'other'}`;
  }

  return 'documentation';
}

/**
 * Check if a source node should be created for this content
 */
export function shouldCreateSourceNode(parsed: ParsedContent): boolean {
  // Create source nodes for all content from data/content directory
  return parsed.pathMeta.relativePath.includes('data/content') ||
         parsed.pathMeta.relativePath.includes('data\\content') ||
         parsed.pathMeta.domain !== undefined;
}
