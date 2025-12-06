/**
 * Resource Transformer
 *
 * Transforms resource files (books, videos, organizations, tools) into ContentNodes.
 * Resources are external references that support learning paths.
 *
 * Source: data/content/elohim-protocol/EPIC/resources/TYPE/*.md
 * Output: ContentNode with contentType based on resource type
 */

import { ContentNode, ContentType } from '../models/content-node.model';
import { ParsedContent } from '../models/import-context.model';
import {
  extractTags,
  extractDescription
} from '../parsers/markdown-parser';

/**
 * Resource type mapping to content types
 */
const RESOURCE_TYPE_MAP: Record<string, ContentType> = {
  book: 'reference',
  books: 'reference',
  video: 'reference',
  videos: 'reference',
  article: 'reference',
  articles: 'reference',
  organization: 'reference',
  organizations: 'reference',
  tool: 'reference',
  tools: 'reference',
  template: 'reference',
  templates: 'reference',
  example: 'example',
  examples: 'example'
};

/**
 * Transform resource content into a ContentNode
 */
export function transformResource(
  parsed: ParsedContent,
  sourceNodeId?: string
): ContentNode {
  const now = new Date().toISOString();

  // Generate resource node ID
  const id = generateResourceId(parsed);

  // Extract tags
  const tags = extractTags(parsed);
  tags.push('resource');

  // Add resource type tag
  const resourceType = parsed.pathMeta.resourceType || 'other';
  tags.push(resourceType);

  // Extract description
  const description = extractDescription(parsed);

  // Determine content type from resource type
  const contentType: ContentType = RESOURCE_TYPE_MAP[resourceType] || 'reference';

  // Build metadata
  const metadata: Record<string, unknown> = {
    category: 'resource',
    resourceType,
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

  // Extract resource-specific frontmatter
  if (parsed.frontmatter.author) {
    metadata.author = parsed.frontmatter.author;
  }
  if (parsed.frontmatter.authors) {
    metadata.authors = parsed.frontmatter.authors;
  }
  if (parsed.frontmatter.url) {
    metadata.url = parsed.frontmatter.url;
  }
  if (parsed.frontmatter.isbn) {
    metadata.isbn = parsed.frontmatter.isbn;
  }
  if (parsed.frontmatter.year) {
    metadata.year = parsed.frontmatter.year;
  }
  if (parsed.frontmatter.publisher) {
    metadata.publisher = parsed.frontmatter.publisher;
  }
  if (parsed.frontmatter.duration) {
    metadata.duration = parsed.frontmatter.duration;
  }
  if (parsed.frontmatter.platform) {
    metadata.platform = parsed.frontmatter.platform;
  }
  if (parsed.frontmatter.website) {
    metadata.website = parsed.frontmatter.website;
  }
  if (parsed.frontmatter.organization) {
    metadata.organization = parsed.frontmatter.organization;
  }

  // Build related node IDs
  const relatedNodeIds: string[] = [];

  // Add source node
  if (sourceNodeId) {
    relatedNodeIds.push(sourceNodeId);
  }

  // Add epic node
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    relatedNodeIds.push(`epic-${parsed.pathMeta.epic}`);
  }

  // Add archetype node if resource is user-specific
  if (parsed.pathMeta.userType) {
    const archetypeNodeId = `role-${parsed.pathMeta.epic}-${parsed.pathMeta.userType}`;
    relatedNodeIds.push(archetypeNodeId);
  }

  // Extract related concepts from frontmatter
  if (Array.isArray(parsed.frontmatter.related_concepts)) {
    for (const concept of parsed.frontmatter.related_concepts) {
      if (typeof concept === 'string') {
        relatedNodeIds.push(`concept-${concept.toLowerCase().replace(/\s+/g, '-')}`);
      }
    }
  }

  return {
    id,
    contentType,
    title: extractResourceTitle(parsed),
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
 * Generate resource node ID
 */
function generateResourceId(parsed: ParsedContent): string {
  const parts = ['resource'];

  // Add resource type
  const resourceType = parsed.pathMeta.resourceType || 'other';
  parts.push(resourceType);

  // Add epic if present
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    parts.push(parsed.pathMeta.epic);
  }

  // Add base name
  const baseName = parsed.pathMeta.baseName.toLowerCase();
  parts.push(baseName);

  // Normalize and join
  return parts
    .map(p => p.toLowerCase().replace(/[^a-z0-9]/g, '-'))
    .join('-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Extract resource title
 */
function extractResourceTitle(parsed: ParsedContent): string {
  // Priority 1: title from frontmatter
  if (parsed.frontmatter.title && typeof parsed.frontmatter.title === 'string') {
    return parsed.frontmatter.title;
  }

  // Priority 2: name from frontmatter (for organizations)
  if (parsed.frontmatter.name && typeof parsed.frontmatter.name === 'string') {
    return parsed.frontmatter.name;
  }

  // Priority 3: parsed title
  if (parsed.title && parsed.title !== 'Untitled') {
    return parsed.title;
  }

  // Priority 4: base name formatted
  return formatBaseName(parsed.pathMeta.baseName);
}

/**
 * Format base name for display
 */
function formatBaseName(baseName: string): string {
  return baseName
    .replace(/[-_]/g, ' ')
    .split(' ')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Check if content should be transformed as resource
 */
export function isResourceContent(parsed: ParsedContent): boolean {
  return parsed.pathMeta.isResource;
}

/**
 * Get resource type for categorization
 */
export function getResourceType(parsed: ParsedContent): string {
  return parsed.pathMeta.resourceType || 'other';
}
