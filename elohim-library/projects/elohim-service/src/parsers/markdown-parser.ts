/**
 * Markdown Parser
 *
 * Parses markdown files with YAML frontmatter into structured content.
 * Extracts sections, tags, and metadata for transformation into ContentNodes.
 */

import * as yaml from 'js-yaml';
import * as crypto from 'crypto';
import { ParsedContent, ParsedSection } from '../models/import-context.model';
import { PathMetadata } from '../models/path-metadata.model';

/**
 * Parse a markdown file
 */
export function parseMarkdown(
  content: string,
  pathMeta: PathMetadata
): ParsedContent {
  const lines = content.split('\n');

  // Extract frontmatter
  const { frontmatter, contentStartIndex } = extractFrontmatter(lines);

  // Get content after frontmatter
  const bodyLines = lines.slice(contentStartIndex);
  const bodyContent = bodyLines.join('\n');

  // Extract title
  const title = extractTitle(bodyLines, frontmatter, pathMeta);

  // Extract sections
  const sections = extractSections(bodyLines);

  // Calculate content hash
  const contentHash = crypto.createHash('sha256').update(content).digest('hex');

  return {
    pathMeta,
    frontmatter,
    rawContent: content,
    sections,
    title,
    contentHash
  };
}

/**
 * Extract YAML frontmatter from markdown
 */
function extractFrontmatter(
  lines: string[]
): { frontmatter: Record<string, unknown>; contentStartIndex: number } {
  if (lines[0]?.trim() !== '---') {
    return { frontmatter: {}, contentStartIndex: 0 };
  }

  // Find closing ---
  let endIndex = -1;
  for (let i = 1; i < lines.length; i++) {
    if (lines[i].trim() === '---') {
      endIndex = i;
      break;
    }
  }

  if (endIndex === -1) {
    return { frontmatter: {}, contentStartIndex: 0 };
  }

  // Parse YAML
  const yamlContent = lines.slice(1, endIndex).join('\n');
  let frontmatter: Record<string, unknown> = {};

  try {
    const parsed = yaml.load(yamlContent);
    if (parsed && typeof parsed === 'object') {
      frontmatter = parsed as Record<string, unknown>;
    }
  } catch (e) {
    // Invalid YAML, continue with empty frontmatter
    console.warn(`Failed to parse YAML frontmatter: ${e}`);
  }

  return {
    frontmatter,
    contentStartIndex: endIndex + 1
  };
}

/**
 * Extract title from markdown content
 */
function extractTitle(
  lines: string[],
  frontmatter: Record<string, unknown>,
  pathMeta: PathMetadata
): string {
  // Priority 1: frontmatter title
  if (frontmatter.title && typeof frontmatter.title === 'string') {
    return frontmatter.title;
  }

  // Priority 2: frontmatter archetype_name
  if (frontmatter.archetype_name && typeof frontmatter.archetype_name === 'string') {
    return frontmatter.archetype_name;
  }

  // Priority 3: First H1 heading
  for (const line of lines) {
    const h1Match = /^#\s+(.+)$/.exec(line);
    if (h1Match) {
      return h1Match[1].trim().replace(/\*\*/g, '');
    }
  }

  // Priority 4: First H2 heading
  for (const line of lines) {
    const h2Match = /^##\s+(.+)$/.exec(line);
    if (h2Match) {
      return h2Match[1].trim().replace(/\*\*/g, '');
    }
  }

  // Priority 5: Generate from path metadata
  return generateTitleFromPath(pathMeta);
}

/**
 * Generate a title from path metadata
 */
function generateTitleFromPath(pathMeta: PathMetadata): string {
  const parts: string[] = [];

  if (pathMeta.userType) {
    parts.push(formatUserType(pathMeta.userType));
  }

  if (pathMeta.epic && pathMeta.epic !== 'other') {
    parts.push(formatEpic(pathMeta.epic));
  }

  if (pathMeta.baseName && !['readme', 'epic', 'index'].includes(pathMeta.baseName.toLowerCase())) {
    parts.push(formatBaseName(pathMeta.baseName));
  }

  if (parts.length === 0) {
    return pathMeta.baseName || 'Untitled';
  }

  return parts.join(' - ');
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
 * Format epic name for display
 */
function formatEpic(epic: string): string {
  return epic
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
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
 * Extract sections with headings
 */
function extractSections(lines: string[]): ParsedSection[] {
  const sections: ParsedSection[] = [];
  let currentSection: ParsedSection | null = null;
  const sectionStack: ParsedSection[] = [];

  for (const line of lines) {
    const headingMatch = /^(#{1,6})\s+(.+)$/.exec(line);

    if (headingMatch) {
      const level = headingMatch[1].length;
      const title = headingMatch[2].trim().replace(/\*\*/g, '');
      const anchor = generateAnchor(title);

      const newSection: ParsedSection = {
        level,
        title,
        anchor,
        content: '',
        children: []
      };

      // Find parent section
      while (sectionStack.length > 0 && sectionStack[sectionStack.length - 1].level >= level) {
        sectionStack.pop();
      }

      if (sectionStack.length > 0) {
        // Add as child of current parent
        sectionStack[sectionStack.length - 1].children.push(newSection);
      } else {
        // Top-level section
        sections.push(newSection);
      }

      sectionStack.push(newSection);
      currentSection = newSection;
    } else if (currentSection) {
      // Add content to current section
      currentSection.content += line + '\n';
    }
  }

  return sections;
}

/**
 * Generate anchor ID from heading text
 */
function generateAnchor(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, '')
    .replace(/\s+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Extract description from parsed content
 */
export function extractDescription(parsed: ParsedContent, maxLength = 300): string {
  // Priority 1: frontmatter description
  if (parsed.frontmatter.description && typeof parsed.frontmatter.description === 'string') {
    return truncate(parsed.frontmatter.description, maxLength);
  }

  // Priority 2: frontmatter epic_domain
  if (parsed.frontmatter.epic_domain && typeof parsed.frontmatter.epic_domain === 'string') {
    return truncate(parsed.frontmatter.epic_domain, maxLength);
  }

  // Priority 3: First paragraph of first section
  if (parsed.sections && parsed.sections.length > 0) {
    const firstContent = parsed.sections[0].content.trim();
    const firstParagraph = firstContent.split(/\n\n/)[0];
    if (firstParagraph) {
      return truncate(firstParagraph.replace(/\n/g, ' '), maxLength);
    }
  }

  // Priority 4: First non-empty line of raw content
  const lines = parsed.rawContent.split('\n');
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed && !trimmed.startsWith('#') && !trimmed.startsWith('---')) {
      return truncate(trimmed, maxLength);
    }
  }

  return '';
}

/**
 * Truncate text to max length
 */
function truncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength - 3) + '...';
}

/**
 * Extract tags from parsed content
 */
export function extractTags(parsed: ParsedContent): string[] {
  const tags = new Set<string>();

  // From frontmatter tags
  if (Array.isArray(parsed.frontmatter.tags)) {
    for (const tag of parsed.frontmatter.tags) {
      if (typeof tag === 'string') {
        tags.add(tag.toLowerCase());
      }
    }
  }

  // From frontmatter epic
  if (parsed.frontmatter.epic && typeof parsed.frontmatter.epic === 'string') {
    tags.add(parsed.frontmatter.epic.toLowerCase());
  }

  // From frontmatter user_type
  if (parsed.frontmatter.user_type && typeof parsed.frontmatter.user_type === 'string') {
    tags.add(parsed.frontmatter.user_type.toLowerCase().replace(/_/g, '-'));
  }

  // From path metadata
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    tags.add(parsed.pathMeta.epic.toLowerCase());
  }

  if (parsed.pathMeta.userType) {
    tags.add(parsed.pathMeta.userType.toLowerCase().replace(/_/g, '-'));
  }

  if (parsed.pathMeta.contentCategory) {
    tags.add(parsed.pathMeta.contentCategory.toLowerCase());
  }

  // From @tag patterns in content
  const tagMatches = parsed.rawContent.matchAll(/@([\w-]+)/g);
  for (const match of tagMatches) {
    tags.add(match[1].toLowerCase());
  }

  return Array.from(tags);
}

/**
 * Extract related user IDs from frontmatter
 */
export function extractRelatedUsers(frontmatter: Record<string, unknown>): string[] {
  const relatedUsers: string[] = [];

  if (Array.isArray(frontmatter.related_users)) {
    for (const user of frontmatter.related_users) {
      if (typeof user === 'string') {
        // Convert user type to node ID format
        relatedUsers.push(`archetype-${user.toLowerCase().replace(/_/g, '-')}`);
      }
    }
  }

  return relatedUsers;
}

/**
 * Extract governance scope from frontmatter
 */
export function extractGovernanceScope(frontmatter: Record<string, unknown>): string[] {
  if (Array.isArray(frontmatter.governance_scope)) {
    return frontmatter.governance_scope.filter(s => typeof s === 'string') as string[];
  }
  return [];
}
