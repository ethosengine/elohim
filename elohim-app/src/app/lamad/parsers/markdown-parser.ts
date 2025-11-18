import { EpicNode, EpicSection, EmbeddedReference, NodeType } from '../models';

/**
 * Parser for Markdown epic documents
 * Extracts structure, sections, and embedded references
 */
export class MarkdownParser {
  /**
   * Parse a markdown file into an EpicNode
   */
  static parseEpic(content: string, sourcePath: string): EpicNode {
    const lines = content.split('\n');

    // Extract frontmatter if present
    const frontmatter = this.extractFrontmatter(lines);
    const contentStart = frontmatter ? this.findContentStart(lines) : 0;

    // Extract title from first heading or filename
    const title = this.extractTitle(lines, contentStart) ?? this.getTitleFromPath(sourcePath);

    // Extract sections
    const sections = this.extractSections(lines, contentStart);

    // Extract tags from frontmatter or content
    const tags = this.extractTags(frontmatter, content);

    // Extract epic ID from filename
    const epicId = this.generateEpicId(sourcePath);

    // Extract feature and scenario references
    const { featureIds, relatedEpicIds } = this.extractReferences(content, tags);

    // Extract metadata
    const metadata = {
      ...frontmatter,
      wordCount: this.countWords(content),
      headingCount: sections.length
    };

    return {
      id: epicId,
      type: NodeType.EPIC,
      title,
      description: this.generateDescription(sections),
      tags,
      sourcePath,
      content,
      relatedNodeIds: [...featureIds, ...relatedEpicIds],
      metadata,
      authors: frontmatter?.['authors'] ?? [],
      version: frontmatter?.['version'] ?? '1.0',
      category: frontmatter?.['category'] ?? this.inferCategory(title, tags),
      featureIds,
      relatedEpicIds,
      markdownContent: content,
      sections
    };
  }

  /**
   * Extract YAML frontmatter from markdown
   */
  private static extractFrontmatter(lines: string[]): Record<string, any> | null {
    if (lines[0]?.trim() !== '---') return null;

    const endIndex = lines.findIndex((line, i) => i > 0 && line.trim() === '---');
    if (endIndex === -1) return null;

    const frontmatterLines = lines.slice(1, endIndex);
    const frontmatter: Record<string, any> = {};

    for (const line of frontmatterLines) {
      const match = /^(\w+):\s*(.+)$/.exec(line);
      if (match) {
        const [, key, value] = match;
        // Handle arrays (comma-separated or bracket notation)
        if (value.includes(',') || value.startsWith('[')) {
          frontmatter[key] = value
            .replace(/[[\]]/g, '')
            .split(',')
            .map(v => v.trim());
        } else {
          frontmatter[key] = value.trim();
        }
      }
    }

    return frontmatter;
  }

  private static findContentStart(lines: string[]): number {
    const secondSeparator = lines.findIndex((line, i) => i > 0 && line.trim() === '---');
    return secondSeparator >= 0 ? secondSeparator + 1 : 0;
  }

  /**
   * Extract title from first h1 heading
   */
  private static extractTitle(lines: string[], startIndex: number): string | null {
    for (let i = startIndex; i < lines.length; i++) {
      const match = /^#\s+(.+)$/.exec(lines[i]);
      if (match) return match[1].trim().replace(/\*\*/g, '');
    }
    return null;
  }

  private static getTitleFromPath(sourcePath: string): string {
    return sourcePath
      .split('/')
      .pop()!
      .replace(/\.md$/, '')
      .split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  /**
   * Extract sections with headings
   */
  private static extractSections(lines: string[], startIndex: number): EpicSection[] {
    const sections: EpicSection[] = [];
    let currentSection: EpicSection | null = null;

    for (let i = startIndex; i < lines.length; i++) {
      const headingMatch = /^(#{1,6})\s+(.+)$/.exec(lines[i]);

      if (headingMatch) {
        // Save previous section if exists
        if (currentSection) {
          sections.push(currentSection);
        }

        const level = headingMatch[1].length;
        const title = headingMatch[2].trim().replace(/\*\*/g, '');
        const anchor = this.generateAnchor(title);

        currentSection = {
          title,
          level,
          anchor,
          content: '',
          embeddedReferences: []
        };
      } else if (currentSection) {
        // Add line to current section
        currentSection.content += lines[i] + '\n';

        // Check for embedded references
        const references = this.findEmbeddedReferences(lines[i], i);
        currentSection.embeddedReferences.push(...references);
      }
    }

    // Add final section
    if (currentSection) {
      sections.push(currentSection);
    }

    return sections;
  }

  /**
   * Find embedded feature/scenario references in text
   * Looks for patterns like [Feature: xxx] or [Scenario: xxx]
   */
  private static findEmbeddedReferences(line: string, position: number): EmbeddedReference[] {
    const references: EmbeddedReference[] = [];
    const featurePattern = /\[Feature:\s*([^\]]+)\]/g;
    const scenarioPattern = /\[Scenario:\s*([^\]]+)\]/g;

    let match;

    while ((match = featurePattern.exec(line)) !== null) {
      references.push({
        type: 'feature',
        nodeId: this.textToId('feature', match[1]),
        position,
        displayText: match[1].trim()
      });
    }

    while ((match = scenarioPattern.exec(line)) !== null) {
      references.push({
        type: 'scenario',
        nodeId: this.textToId('scenario', match[1]),
        position,
        displayText: match[1].trim()
      });
    }

    return references;
  }

  /**
   * Extract tags from frontmatter or @tags in content
   */
  private static extractTags(frontmatter: Record<string, any> | null, content: string): string[] {
    const tags = new Set<string>();

    // From frontmatter
    if (frontmatter?.['tags']) {
      const fmTags = Array.isArray(frontmatter['tags']) ? frontmatter['tags'] : [frontmatter['tags']];
      fmTags.forEach(tag => tags.add(tag));
    }

    // From content (@tag patterns)
    const tagMatches = content.matchAll(/@([\w-]+)/g);
    for (const match of tagMatches) {
      tags.add(match[1]);
    }

    return Array.from(tags);
  }

  /**
   * Extract feature and epic references from content
   */
  private static extractReferences(
    content: string,
    tags: string[]
  ): { featureIds: string[]; relatedEpicIds: string[] } {
    const featureIds = new Set<string>();
    const relatedEpicIds = new Set<string>();

    // From tags
    tags.forEach(tag => {
      if (tag.startsWith('feature:')) {
        featureIds.add(tag.substring(8));
      } else if (tag.startsWith('epic:')) {
        relatedEpicIds.add(tag.substring(5));
      }
    });

    return {
      featureIds: Array.from(featureIds),
      relatedEpicIds: Array.from(relatedEpicIds)
    };
  }

  /**
   * Generate description from first section or first paragraph
   */
  private static generateDescription(sections: EpicSection[]): string {
    if (sections.length === 0) return '';

    const firstSection = sections[0];
    const sentences = firstSection.content.split(/[.!?]\s+/);

    return sentences.slice(0, 2).join('. ').trim() + (sentences.length > 2 ? '...' : '');
  }

  /**
   * Generate anchor ID from heading text
   */
  private static generateAnchor(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, '')
      .replace(/\s+/g, '-')
      .replace(/-+/g, '-')
      .replace(/(^-)|(-$)/g, '');
  }

  /**
   * Generate epic ID from file path
   */
  private static generateEpicId(sourcePath: string): string {
    return sourcePath
      .split('/')
      .pop()!
      .replace(/\.md$/, '')
      .toLowerCase()
      .replace(/[^a-z0-9-]/g, '_');
  }

  /**
   * Convert text to ID format
   */
  private static textToId(type: string, text: string): string {
    const normalized = text
      .toLowerCase()
      .replace(/[^a-z0-9\s-]/g, '')
      .replace(/\s+/g, '_')
      .replace(/_+/g, '_')
      .replace(/(^_)|(_$)/g, '');

    return `${type}_${normalized}`;
  }

  /**
   * Infer category from title and tags
   */
  private static inferCategory(title: string, tags: string[]): string {
    const lowerTitle = title.toLowerCase();

    if (lowerTitle.includes('observer')) return 'observer';
    if (lowerTitle.includes('shopping') || lowerTitle.includes('value scanner')) return 'value-scanner';
    if (lowerTitle.includes('autonomous') || lowerTitle.includes('workplace')) return 'autonomous-entity';
    if (lowerTitle.includes('social') || lowerTitle.includes('medium')) return 'social';

    // Check tags
    for (const tag of tags) {
      if (tag.includes('observer')) return 'observer';
      if (tag.includes('value')) return 'value-scanner';
      if (tag.includes('autonomous')) return 'autonomous-entity';
    }

    return 'general';
  }

  /**
   * Count words in content
   */
  private static countWords(content: string): number {
    return content.split(/\s+/).filter(word => word.length > 0).length;
  }
}
