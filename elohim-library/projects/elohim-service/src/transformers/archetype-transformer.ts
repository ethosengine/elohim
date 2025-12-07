/**
 * Archetype Transformer
 *
 * Transforms README.md files in role directories into 'role' ContentNodes.
 * Archetypes are persona/role definitions like "Policy Maker", "Worker", etc.
 *
 * Source: data/content/elohim-protocol/governance/policy_maker/README.md
 * Output: ContentNode with contentType: 'role'
 */

import { ContentNode } from '../models/content-node.model';
import { ParsedContent } from '../models/import-context.model';
import {
  extractTags,
  extractDescription,
  extractRelatedUsers,
  extractGovernanceScope
} from '../parsers/markdown-parser';

/**
 * Transform archetype content into a role ContentNode
 */
export function transformArchetype(
  parsed: ParsedContent,
  sourceNodeId?: string
): ContentNode {
  const now = new Date().toISOString();

  // Generate role node ID
  const id = generateArchetypeId(parsed);

  // Extract tags
  const tags = extractTags(parsed);
  tags.push('role', 'archetype');

  // Extract description
  const description = extractDescription(parsed);

  // Build metadata
  const metadata: Record<string, unknown> = {
    category: 'archetype',
    epic: parsed.pathMeta.epic,
    userType: parsed.pathMeta.userType,
    source: 'elohim-import',
    sourceVersion: '1.0.0'
  };

  // Add provenance link
  if (sourceNodeId) {
    metadata.derivedFrom = sourceNodeId;
    metadata.extractionMethod = 'direct-import';
  }

  // Add frontmatter fields
  if (parsed.frontmatter.archetype_name) {
    metadata.archetypeName = parsed.frontmatter.archetype_name;
  }
  if (parsed.frontmatter.epic_domain) {
    metadata.epicDomain = parsed.frontmatter.epic_domain;
  }

  // Add governance scope if present
  const governanceScope = extractGovernanceScope(parsed.frontmatter);
  if (governanceScope.length > 0) {
    metadata.governanceScope = governanceScope;
  }

  // Extract related users for graph connections
  const relatedNodeIds = extractRelatedUsers(parsed.frontmatter);

  // Add source node to related
  if (sourceNodeId) {
    relatedNodeIds.push(sourceNodeId);
  }

  // Add epic node to related
  const epicNodeId = `epic-${parsed.pathMeta.epic}`;
  if (!relatedNodeIds.includes(epicNodeId)) {
    relatedNodeIds.push(epicNodeId);
  }

  return {
    id,
    contentType: 'role',
    title: extractArchetypeTitle(parsed),
    description,
    content: parsed.rawContent,
    contentFormat: 'markdown',
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
 * Generate archetype node ID
 */
function generateArchetypeId(parsed: ParsedContent): string {
  const parts = ['role'];

  // Add epic
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    parts.push(parsed.pathMeta.epic);
  }

  // Add user type
  if (parsed.pathMeta.userType) {
    parts.push(parsed.pathMeta.userType);
  }

  // Normalize and join
  return parts
    .map(p => p.toLowerCase().replace(/[^a-z0-9]/g, '-'))
    .join('-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Extract archetype title
 */
function extractArchetypeTitle(parsed: ParsedContent): string {
  // Priority 1: archetype_name from frontmatter
  if (parsed.frontmatter.archetype_name && typeof parsed.frontmatter.archetype_name === 'string') {
    return parsed.frontmatter.archetype_name;
  }

  // Priority 2: title from frontmatter
  if (parsed.frontmatter.title && typeof parsed.frontmatter.title === 'string') {
    return parsed.frontmatter.title;
  }

  // Priority 3: parsed title
  if (parsed.title && parsed.title !== 'Untitled') {
    return parsed.title;
  }

  // Priority 4: user_type formatted
  if (parsed.pathMeta.userType) {
    return formatUserType(parsed.pathMeta.userType);
  }

  return 'Unknown Role';
}

/**
 * Format user type for display
 */
function formatUserType(userType: string): string {
  return userType
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Check if content should be transformed as archetype
 */
export function isArchetypeContent(parsed: ParsedContent): boolean {
  return parsed.pathMeta.isArchetypeDefinition;
}
