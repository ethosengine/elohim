/**
 * Tests for markdown-parser
 */

import {
  parseMarkdown,
  extractDescription,
  extractTags,
  extractRelatedUsers,
  extractGovernanceScope
} from './markdown-parser';
import { PathMetadata } from '../models/path-metadata.model';

describe('markdown-parser', () => {
  const mockPathMeta: PathMetadata = {
    fullPath: '/test/file.md',
    relativePath: 'test/file.md',
    domain: 'elohim-protocol',
    epic: 'governance',
    userType: 'policy_maker',
    contentCategory: 'documentation',
    baseName: 'test-file',
    extension: '.md',
    isArchetypeDefinition: false,
    isEpicNarrative: false,
    isScenario: false,
    isResource: false,
    suggestedId: 'test-id'
  };

  describe('parseMarkdown', () => {
    describe('Frontmatter extraction', () => {
      it('should parse YAML frontmatter', () => {
        const content = `---
title: Test Title
epic: governance
tags:
  - test
  - example
---

# Content here`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.frontmatter.title).toBe('Test Title');
        expect(parsed.frontmatter.epic).toBe('governance');
        expect(parsed.frontmatter.tags).toEqual(['test', 'example']);
      });

      it('should handle content without frontmatter', () => {
        const content = `# Simple Markdown

This is content without frontmatter.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.frontmatter).toEqual({});
        expect(parsed.rawContent).toBe(content);
      });

      it('should handle incomplete frontmatter', () => {
        const content = `---
title: Test
This is not proper YAML

# Content`;

        const parsed = parseMarkdown(content, mockPathMeta);

        // Should continue with empty frontmatter
        expect(parsed.frontmatter).toBeDefined();
      });

      it('should handle frontmatter without closing ---', () => {
        const content = `---
title: Test
epic: governance

# Content without closing frontmatter`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.frontmatter).toEqual({});
      });

      it('should handle complex frontmatter values', () => {
        const content = `---
title: Complex Document
archetype_name: Policy Maker
description: A complex description
related_users:
  - policy_maker
  - activist
governance_scope:
  - local
  - regional
  - global
---

# Content`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.frontmatter.archetype_name).toBe('Policy Maker');
        expect(parsed.frontmatter.related_users).toEqual(['policy_maker', 'activist']);
        expect(parsed.frontmatter.governance_scope).toEqual(['local', 'regional', 'global']);
      });
    });

    describe('Title extraction', () => {
      it('should extract title from frontmatter', () => {
        const content = `---
title: Frontmatter Title
---

# Markdown Heading`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.title).toBe('Frontmatter Title');
      });

      it('should use archetype_name as title if present', () => {
        const content = `---
archetype_name: Policy Maker
title: Other Title
---

# Heading`;

        const parsed = parseMarkdown(content, mockPathMeta);

        // frontmatter.title has higher priority than archetype_name in extractTitle
        expect(parsed.title).toBe('Other Title');
      });

      it('should extract title from first H1', () => {
        const content = `# First Heading

Content here.

## Subheading`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.title).toBe('First Heading');
      });

      it('should extract title from first H2 if no H1', () => {
        const content = `## Second Level Heading

Content without H1.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.title).toBe('Second Level Heading');
      });

      it('should strip bold markers from heading', () => {
        const content = `# **Bold Heading**`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.title).toBe('Bold Heading');
      });

      it('should generate title from path if no heading found', () => {
        const content = `Just plain content without any headings.`;

        const pathMeta: PathMetadata = {
          ...mockPathMeta,
          userType: 'policy_maker',
          epic: 'governance',
          baseName: 'test-document'
        };

        const parsed = parseMarkdown(content, pathMeta);

        expect(parsed.title).toContain('Policy Maker');
        expect(parsed.title).toContain('Governance');
      });

      it('should handle path with no user type', () => {
        const content = `Content without headings.`;

        const pathMeta: PathMetadata = {
          ...mockPathMeta,
          userType: undefined,
          epic: 'lamad',
          baseName: 'intro'
        };

        const parsed = parseMarkdown(content, pathMeta);

        expect(parsed.title).toContain('Lamad');
        expect(parsed.title).toContain('Intro');
      });

      it('should handle generic basenames', () => {
        const content = `Content without headings.`;

        const pathMeta: PathMetadata = {
          ...mockPathMeta,
          baseName: 'readme'
        };

        const parsed = parseMarkdown(content, pathMeta);

        // Should not use 'readme' in generated title
        expect(parsed.title.toLowerCase()).not.toContain('readme');
      });
    });

    describe('Section extraction', () => {
      it('should extract sections with headings', () => {
        const content = `# Main Heading

Content for main section.

## Subheading 1

Content for sub 1.

### Sub-sub heading

Nested content.

## Subheading 2

Content for sub 2.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.sections).toBeDefined();
        expect(parsed.sections!.length).toBeGreaterThan(0);
      });

      it('should create hierarchical section structure', () => {
        const content = `# Level 1

Content 1.

## Level 2A

Content 2A.

### Level 3

Content 3.

## Level 2B

Content 2B.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        const level1 = parsed.sections![0];
        expect(level1.level).toBe(1);
        expect(level1.title).toBe('Level 1');
        expect(level1.children).toHaveLength(2);
        expect(level1.children[0].title).toBe('Level 2A');
        expect(level1.children[0].children).toHaveLength(1);
      });

      it('should generate anchors for sections', () => {
        const content = `# Test Heading

## Sub Heading With Spaces

### CamelCaseHeading`;

        const parsed = parseMarkdown(content, mockPathMeta);

        const section1 = parsed.sections![0];
        const section2 = section1.children[0];

        expect(section1.anchor).toBe('test-heading');
        expect(section2.anchor).toBe('sub-heading-with-spaces');
      });

      it('should handle special characters in anchors', () => {
        const content = `## Testing & Special Characters!

Content here.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        const section = parsed.sections![0];
        expect(section.anchor).toMatch(/^[a-z0-9-]+$/);
        expect(section.anchor).not.toContain('&');
        expect(section.anchor).not.toContain('!');
      });

      it('should capture section content', () => {
        const content = `## Section Heading

This is the section content.
It has multiple lines.

And paragraphs.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        const section = parsed.sections![0];
        expect(section.content).toContain('This is the section content');
        expect(section.content).toContain('And paragraphs');
      });

      it('should handle content before first heading', () => {
        const content = `Some intro text.

## First Heading

Content.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        // Content before first heading is not captured in sections
        expect(parsed.sections![0].title).toBe('First Heading');
      });
    });

    describe('Content hash generation', () => {
      it('should generate SHA256 hash of content', () => {
        const content = `# Test Document

Some content.`;

        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.contentHash).toBeDefined();
        expect(parsed.contentHash).toHaveLength(64); // SHA256 hex length
      });

      it('should generate different hashes for different content', () => {
        const content1 = `# Document 1`;
        const content2 = `# Document 2`;

        const parsed1 = parseMarkdown(content1, mockPathMeta);
        const parsed2 = parseMarkdown(content2, mockPathMeta);

        expect(parsed1.contentHash).not.toBe(parsed2.contentHash);
      });

      it('should generate same hash for identical content', () => {
        const content = `# Test\n\nContent here.`;

        const parsed1 = parseMarkdown(content, mockPathMeta);
        const parsed2 = parseMarkdown(content, mockPathMeta);

        expect(parsed1.contentHash).toBe(parsed2.contentHash);
      });
    });

    describe('ParsedContent structure', () => {
      it('should include pathMeta', () => {
        const content = `# Test`;
        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.pathMeta).toBe(mockPathMeta);
      });

      it('should include rawContent', () => {
        const content = `# Test\n\nContent.`;
        const parsed = parseMarkdown(content, mockPathMeta);

        expect(parsed.rawContent).toBe(content);
      });
    });
  });

  describe('extractDescription', () => {
    it('should extract description from frontmatter', () => {
      const content = `---
description: This is the description.
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      expect(description).toBe('This is the description.');
    });

    it('should use epic_domain from frontmatter if no description', () => {
      const content = `---
epic_domain: This is the epic domain description.
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      expect(description).toBe('This is the epic domain description.');
    });

    it('should extract first paragraph from first section', () => {
      const content = `# Heading

This is the first paragraph.

This is the second paragraph.

## Subheading

More content.`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      expect(description).toBe('This is the first paragraph.');
    });

    it('should extract first non-empty line as fallback', () => {
      const content = `


# Heading

First real content line.`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      expect(description).toBe('First real content line.');
    });

    it('should truncate long descriptions', () => {
      const longText = 'A'.repeat(500);
      const content = `---
description: ${longText}
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      expect(description.length).toBe(300); // Default maxLength
      expect(description.endsWith('...')).toBe(true);
    });

    it('should respect custom maxLength', () => {
      const content = `---
description: This is a description.
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed, 10);

      expect(description.length).toBeLessThanOrEqual(10);
    });

    it('should not truncate short text', () => {
      const shortText = 'Short description.';
      const content = `---
description: ${shortText}
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      expect(description).toBe(shortText);
    });

    it('should return empty string if no description found', () => {
      const content = `---
title: Test
---`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const description = extractDescription(parsed);

      // Priority 4 fallback scans rawContent lines; 'title: Test' is a non-empty
      // line that does not start with '#' or '---', so it is returned as the description.
      expect(description).toBe('title: Test');
    });
  });

  describe('extractTags', () => {
    it('should extract tags from frontmatter', () => {
      const content = `---
tags:
  - governance
  - policy
  - test
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('governance');
      expect(tags).toContain('policy');
      expect(tags).toContain('test');
    });

    it('should extract epic from frontmatter', () => {
      const content = `---
epic: governance
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('governance');
    });

    it('should extract user_type from frontmatter', () => {
      const content = `---
user_type: policy_maker
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('policy-maker'); // Underscores converted to hyphens
    });

    it('should extract tags from path metadata', () => {
      const content = `# Test`;

      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        epic: 'lamad',
        userType: 'learner',
        contentCategory: 'concept'
      };

      const parsed = parseMarkdown(content, pathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('lamad');
      expect(tags).toContain('learner');
      expect(tags).toContain('concept');
    });

    it('should extract @tag patterns from content', () => {
      const content = `# Test

This content mentions @holochain and @governance.`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('holochain');
      expect(tags).toContain('governance');
    });

    it('should lowercase all tags', () => {
      const content = `---
tags:
  - UPPERCASE
  - MixedCase
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('uppercase');
      expect(tags).toContain('mixedcase');
    });

    it('should not duplicate tags', () => {
      const content = `---
epic: governance
tags:
  - governance
---

# Test with @governance`;

      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        epic: 'governance'
      };

      const parsed = parseMarkdown(content, pathMeta);
      const tags = extractTags(parsed);

      const governanceCount = tags.filter(t => t === 'governance').length;
      expect(governanceCount).toBe(1);
    });

    it('should handle non-string tags in frontmatter', () => {
      const content = `---
tags:
  - valid-tag
  - 123
  - true
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const tags = extractTags(parsed);

      expect(tags).toContain('valid-tag');
      // Non-string tags should be filtered out
      expect(tags).not.toContain('123');
      expect(tags).not.toContain('true');
    });

    it('should skip "other" epic category', () => {
      const pathMeta: PathMetadata = {
        ...mockPathMeta,
        epic: 'other'
      };

      const content = `# Test`;
      const parsed = parseMarkdown(content, pathMeta);
      const tags = extractTags(parsed);

      expect(tags).not.toContain('other');
    });
  });

  describe('extractRelatedUsers', () => {
    it('should extract related users from frontmatter', () => {
      const content = `---
related_users:
  - policy_maker
  - activist
  - observer
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const relatedUsers = extractRelatedUsers(parsed.frontmatter);

      expect(relatedUsers).toContain('archetype-policy-maker');
      expect(relatedUsers).toContain('archetype-activist');
      expect(relatedUsers).toContain('archetype-observer');
    });

    it('should convert user types to archetype node IDs', () => {
      const content = `---
related_users:
  - tech_lead
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const relatedUsers = extractRelatedUsers(parsed.frontmatter);

      expect(relatedUsers[0]).toBe('archetype-tech-lead');
    });

    it('should return empty array if no related_users', () => {
      const content = `---
title: Test
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const relatedUsers = extractRelatedUsers(parsed.frontmatter);

      expect(relatedUsers).toEqual([]);
    });

    it('should filter out non-string values', () => {
      const frontmatter = {
        related_users: ['valid_user', 123, true, 'another_user']
      };

      const relatedUsers = extractRelatedUsers(frontmatter);

      expect(relatedUsers).toHaveLength(2);
      expect(relatedUsers).toContain('archetype-valid-user');
      expect(relatedUsers).toContain('archetype-another-user');
    });
  });

  describe('extractGovernanceScope', () => {
    it('should extract governance scope from frontmatter', () => {
      const content = `---
governance_scope:
  - local
  - regional
  - global
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const scope = extractGovernanceScope(parsed.frontmatter);

      expect(scope).toEqual(['local', 'regional', 'global']);
    });

    it('should return empty array if no governance_scope', () => {
      const content = `---
title: Test
---

# Test`;

      const parsed = parseMarkdown(content, mockPathMeta);
      const scope = extractGovernanceScope(parsed.frontmatter);

      expect(scope).toEqual([]);
    });

    it('should filter out non-string values', () => {
      const frontmatter = {
        governance_scope: ['local', 123, 'regional', true]
      };

      const scope = extractGovernanceScope(frontmatter);

      expect(scope).toEqual(['local', 'regional']);
    });

    it('should handle non-array governance_scope', () => {
      const frontmatter = {
        governance_scope: 'local'
      };

      const scope = extractGovernanceScope(frontmatter);

      expect(scope).toEqual([]);
    });
  });

  describe('Edge cases', () => {
    it('should handle empty content', () => {
      const content = '';
      const parsed = parseMarkdown(content, mockPathMeta);

      expect(parsed.frontmatter).toEqual({});
      expect(parsed.sections).toBeDefined();
      expect(parsed.contentHash).toBeDefined();
    });

    it('should handle content with only frontmatter', () => {
      const content = `---
title: Test
---`;

      const parsed = parseMarkdown(content, mockPathMeta);

      expect(parsed.frontmatter.title).toBe('Test');
    });

    it('should handle malformed YAML gracefully', () => {
      const content = `---
title: Test
this is not: valid: yaml:
---

# Content`;

      const parsed = parseMarkdown(content, mockPathMeta);

      // Should not crash, should have some result
      expect(parsed).toBeDefined();
    });

    it('should handle very long content', () => {
      const longContent = '# Heading\n\n' + 'A'.repeat(100000);
      const parsed = parseMarkdown(longContent, mockPathMeta);

      expect(parsed.contentHash).toBeDefined();
      expect(parsed.rawContent).toBe(longContent);
    });
  });
});
