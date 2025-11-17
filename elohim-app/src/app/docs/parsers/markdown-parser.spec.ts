import { MarkdownParser } from './markdown-parser';
import { NodeType } from '../models';

describe('MarkdownParser', () => {
  describe('parseEpic', () => {
    it('should parse basic markdown epic without frontmatter', () => {
      const content = `# Test Epic

This is a test epic description.

## Section 1

Content for section 1.`;

      const result = MarkdownParser.parseEpic(content, 'test-epic.md');

      expect(result.type).toBe(NodeType.EPIC);
      expect(result.title).toBe('Test Epic');
      expect(result.sourcePath).toBe('test-epic.md');
      expect(result.content).toBe(content);
      expect(result.sections.length).toBe(2);
    });

    it('should parse epic with YAML frontmatter', () => {
      const content = `---
title: Epic Title
version: 2.0
tags: [tag1, tag2]
---

# Epic Heading

Content here.`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.version).toBe('2.0');
      expect(result.tags).toContain('tag1');
      expect(result.tags).toContain('tag2');
    });

    it('should extract title from filename if no heading present', () => {
      const content = 'Just some content without heading';
      const result = MarkdownParser.parseEpic(content, 'my-test-epic.md');

      expect(result.title).toBe('My Test Epic');
    });

    it('should extract tags from content using @ notation', () => {
      const content = `# Epic

Content with @feature-1 and @security tags.`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.tags).toContain('feature-1');
      expect(result.tags).toContain('security');
    });

    it('should extract feature references from tags', () => {
      const content = `---
tags: [feature:user-auth, feature:payment]
---

# Epic`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.featureIds).toContain('user-auth');
      expect(result.featureIds).toContain('payment');
    });

    it('should count words in content', () => {
      const content = `# Epic

This has five words total.`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.metadata['wordCount']).toBeGreaterThan(0);
    });

    it('should generate epic ID from source path', () => {
      const result = MarkdownParser.parseEpic('# Test', 'path/to/my-epic-file.md');

      expect(result.id).toBe('my-epic-file');
    });

    it('should extract sections with correct levels', () => {
      const content = `# Epic

## Level 2 Section

### Level 3 Section

## Another Level 2`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.sections.length).toBe(4);
      expect(result.sections[0].level).toBe(1);
      expect(result.sections[1].level).toBe(2);
      expect(result.sections[2].level).toBe(3);
      expect(result.sections[3].level).toBe(2);
    });

    it('should find embedded feature references in content', () => {
      const content = `# Epic

This references [Feature: user authentication] and [Scenario: login flow].`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      const embeddedRefs = result.sections[0].embeddedReferences;
      expect(embeddedRefs.length).toBeGreaterThan(0);
      expect(embeddedRefs.some(ref => ref.type === 'feature')).toBe(true);
      expect(embeddedRefs.some(ref => ref.type === 'scenario')).toBe(true);
    });

    it('should infer category from title', () => {
      const observerEpic = MarkdownParser.parseEpic('# Observer System', 'epic.md');
      expect(observerEpic.category).toBe('observer');

      const shoppingEpic = MarkdownParser.parseEpic('# Shopping Cart', 'epic.md');
      expect(shoppingEpic.category).toBe('value-scanner');

      const autonomousEpic = MarkdownParser.parseEpic('# Autonomous Agent', 'epic.md');
      expect(autonomousEpic.category).toBe('autonomous-entity');

      const socialEpic = MarkdownParser.parseEpic('# Social Medium', 'epic.md');
      expect(socialEpic.category).toBe('social');

      const generalEpic = MarkdownParser.parseEpic('# Random Epic', 'epic.md');
      expect(generalEpic.category).toBe('general');
    });

    it('should generate anchors from section titles', () => {
      const content = `# Epic

## User Authentication System

Content here.`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.sections[1].anchor).toBe('user-authentication-system');
    });

    it('should handle content with bold text in headings', () => {
      const content = `# **Bold Epic Title**

## **Bold Section**`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.title).toBe('Bold Epic Title');
      expect(result.sections[1].title).toBe('Bold Section');
    });

    it('should handle empty content', () => {
      const result = MarkdownParser.parseEpic('', 'empty.md');

      expect(result.title).toBe('Empty');
      expect(result.sections.length).toBe(0);
      expect(result.description).toBe('');
    });

    it('should extract related epic IDs from tags', () => {
      const content = `---
tags: [epic:parent-epic, epic:related-epic]
---

# Child Epic`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.relatedEpicIds).toContain('parent-epic');
      expect(result.relatedEpicIds).toContain('related-epic');
    });

    it('should generate description from first section', () => {
      const content = `# Epic
This is the first sentence. This is the second sentence. This is the third sentence.

## Introduction

More content here.`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.description.length).toBeGreaterThan(0);
    });

    it('should handle frontmatter with comma-separated values', () => {
      const content = `---
tags: tag1, tag2, tag3
authors: John Doe, Jane Smith
---

# Epic`;

      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.tags.length).toBeGreaterThan(0);
      expect(result.authors).toBeDefined();
      if (result.authors) {
        expect(result.authors.length).toBeGreaterThan(0);
      }
    });

    it('should use default version if not specified', () => {
      const content = '# Epic';
      const result = MarkdownParser.parseEpic(content, 'epic.md');

      expect(result.version).toBe('1.0');
    });

    it('should handle special characters in epic ID generation', () => {
      const result = MarkdownParser.parseEpic('# Test', 'My Epic File!@#$.md');

      expect(result.id).toMatch(/^[a-z0-9_-]+$/);
    });
  });
});
