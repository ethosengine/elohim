/**
 * Markdown Parser (Refactored)
 *
 * Parses markdown files with YAML frontmatter into structured content.
 * ONLY handles parsing structure - extraction of semantic meaning (tags, descriptions)
 * is handled by transformers.
 */

import * as yaml from 'js-yaml';

import { PathMetadata } from '../models/path-metadata.model';

import { buildParserResult, splitLines, matchLine } from './base-parser';
import { MarkdownParserResult, ParsedSection, ParserError } from './parser-result';

/**
 * Parse a markdown file
 */
export function parseMarkdown(content: string, pathMeta: PathMetadata): MarkdownParserResult {
  try {
    const lines = splitLines(content);

    // Extract frontmatter
    const { frontmatter, contentStartIndex } = extractFrontmatter(lines, pathMeta.fullPath);

    // Get content after frontmatter
    const bodyLines = lines.slice(contentStartIndex);

    // Extract title
    const title = extractTitle(bodyLines, frontmatter, pathMeta);

    // Extract sections
    const sections = extractSections(bodyLines);

    // Build base result
    const baseResult = buildParserResult(content, pathMeta, frontmatter, title);

    return {
      ...baseResult,
      sections,
    };
  } catch (error) {
    if (error instanceof ParserError) {
      throw error;
    }
    throw new ParserError(
      `Failed to parse markdown: ${(error as Error).message}`,
      pathMeta.fullPath,
      error as Error
    );
  }
}

/**
 * Extract YAML frontmatter from markdown
 */
function extractFrontmatter(
  lines: string[],
  filePath: string
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
  } catch (error) {
    throw new ParserError(
      `Invalid YAML frontmatter: ${(error as Error).message}`,
      filePath,
      error as Error
    );
  }

  return {
    frontmatter,
    contentStartIndex: endIndex + 1,
  };
}

/**
 * Extract title from markdown content
 * Priority: frontmatter.title → frontmatter.archetype_name → H1 → H2 → path-based → empty
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
    const h1Match = matchLine(line, /^#\s+(.+)$/);
    if (h1Match) {
      return cleanHeadingText(h1Match[1]);
    }
  }

  // Priority 4: First H2 heading
  for (const line of lines) {
    const h2Match = matchLine(line, /^##\s+(.+)$/);
    if (h2Match) {
      return cleanHeadingText(h2Match[1]);
    }
  }

  // Priority 5: Generate from path metadata
  return generateTitleFromPath(pathMeta);
}

/**
 * Clean heading text (remove markdown formatting)
 */
function cleanHeadingText(text: string): string {
  return text.trim().replace(/\*\*/g, '');
}

/**
 * Generate a title from path metadata
 */
function generateTitleFromPath(pathMeta: PathMetadata): string {
  const parts: string[] = [];

  if (pathMeta.userType) {
    parts.push(formatPathPart(pathMeta.userType));
  }

  if (pathMeta.epic && pathMeta.epic !== 'other') {
    parts.push(formatPathPart(pathMeta.epic));
  }

  if (pathMeta.baseName && !isGenericBaseName(pathMeta.baseName)) {
    parts.push(formatPathPart(pathMeta.baseName));
  }

  if (parts.length === 0) {
    return pathMeta.baseName || '';
  }

  return parts.join(' - ');
}

/**
 * Check if base name is generic (readme, epic, index)
 */
function isGenericBaseName(baseName: string): boolean {
  return ['readme', 'epic', 'index'].includes(baseName.toLowerCase());
}

/**
 * Format path part for display (Title Case)
 */
function formatPathPart(part: string): string {
  return part
    .replace(/[-_]/g, ' ')
    .split(' ')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Extract sections with headings and hierarchy
 */
function extractSections(lines: string[]): ParsedSection[] {
  const sections: ParsedSection[] = [];
  let currentSection: ParsedSection | null = null;
  const sectionStack: ParsedSection[] = [];

  for (const line of lines) {
    const headingMatch = matchLine(line, /^(#{1,6})\s+(.+)$/);

    if (headingMatch) {
      const level = headingMatch[1].length;
      const title = cleanHeadingText(headingMatch[2]);
      const anchor = generateAnchor(title);

      const newSection: ParsedSection = {
        level,
        title,
        anchor,
        content: '',
        children: [],
      };

      // Find parent section
      while (sectionStack.length > 0 && sectionStack.at(-1)!.level >= level) {
        sectionStack.pop();
      }

      if (sectionStack.length > 0) {
        // Add as child of current parent
        sectionStack.at(-1)!.children.push(newSection);
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
