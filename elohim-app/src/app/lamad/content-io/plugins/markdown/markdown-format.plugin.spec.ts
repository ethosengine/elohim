import { TestBed } from '@angular/core/testing';
import { MarkdownFormatPlugin } from './markdown-format.plugin';
import { MarkdownRendererComponent } from '../../../renderers/markdown-renderer/markdown-renderer.component';

describe('MarkdownFormatPlugin', () => {
  let plugin: MarkdownFormatPlugin;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [MarkdownFormatPlugin],
    });
    plugin = TestBed.inject(MarkdownFormatPlugin);
  });

  describe('Identity', () => {
    it('should have correct formatId', () => {
      expect(plugin.formatId).toBe('markdown');
    });

    it('should have correct displayName', () => {
      expect(plugin.displayName).toBe('Markdown');
    });

    it('should have correct file extensions', () => {
      expect(plugin.fileExtensions).toEqual(['.md', '.markdown']);
    });

    it('should have correct MIME types', () => {
      expect(plugin.mimeTypes).toContain('text/markdown');
      expect(plugin.mimeTypes).toContain('text/x-markdown');
      expect(plugin.mimeTypes).toContain('text/plain');
    });
  });

  describe('Capabilities', () => {
    it('should support import', () => {
      expect(plugin.canImport).toBeTrue();
    });

    it('should support export', () => {
      expect(plugin.canExport).toBeTrue();
    });

    it('should support validation', () => {
      expect(plugin.canValidate).toBeTrue();
    });

    it('should support rendering', () => {
      expect(plugin.canRender).toBeTrue();
    });

    it('should not support custom editing', () => {
      expect(plugin.canEdit).toBeFalse();
    });
  });

  describe('getRendererComponent', () => {
    it('should return MarkdownRendererComponent', () => {
      expect(plugin.getRendererComponent()).toBe(MarkdownRendererComponent);
    });
  });

  describe('getRendererPriority', () => {
    it('should return priority 10', () => {
      expect(plugin.getRendererPriority()).toBe(10);
    });
  });

  describe('getEditorComponent', () => {
    it('should return null for default editor', () => {
      expect(plugin.getEditorComponent()).toBeNull();
    });
  });

  describe('getEditorConfig', () => {
    it('should return code editor config', () => {
      const config = plugin.getEditorConfig();

      expect(config.editorMode).toBe('code');
      expect(config.supportsLivePreview).toBeTrue();
      expect(config.showLineNumbers).toBeTrue();
      expect(config.wordWrap).toBeTrue();
    });

    it('should include markdown toolbar actions', () => {
      const config = plugin.getEditorConfig();

      const actionIds = config.toolbar?.actions?.map(a => a.id) ?? [];
      expect(actionIds).toContain('bold');
      expect(actionIds).toContain('italic');
      expect(actionIds).toContain('heading');
      expect(actionIds).toContain('link');
      expect(actionIds).toContain('code');
    });
  });

  describe('import', () => {
    it('should parse markdown with frontmatter', async () => {
      // Note: MarkdownParser extracts title from first H1, not frontmatter
      // and uses 'type' field for contentType
      const content = `---
title: Test Document
description: A test
type: article
tags: [test, demo]
---

# Main Content

This is the body.`;

      const result = await plugin.import(content);

      expect(result.contentFormat).toBe('markdown');
      // Title comes from first H1 heading, not frontmatter
      expect(result.title).toBe('Main Content');
      // Description is generated from content, not frontmatter
      expect(result.description).toContain('body');
      // ContentType uses 'type' field from frontmatter
      expect(result.contentType).toBe('article');
      expect(result.tags).toContain('test');
      // Frontmatter is captured separately
      expect(result.frontmatter?.['title']).toBe('Test Document');
      expect(result.frontmatter?.['description']).toBe('A test');
    });

    it('should parse markdown without frontmatter', async () => {
      const content = `# Simple Document

Just some content.`;

      const result = await plugin.import(content);

      expect(result.contentFormat).toBe('markdown');
      expect(result.content).toContain('Simple Document');
    });

    it('should handle File input', async () => {
      const content = `---
title: File Test
---

# File Content`;
      const file = new File([content], 'test.md', { type: 'text/markdown' });

      const result = await plugin.import(file);

      // Title comes from H1 heading
      expect(result.title).toBe('File Content');
      // Frontmatter still captures the title
      expect(result.frontmatter?.['title']).toBe('File Test');
    });

    it('should extract frontmatter as metadata', async () => {
      const content = `---
title: Meta Test
customField: customValue
---

Content`;

      const result = await plugin.import(content);

      expect(result.frontmatter).toBeDefined();
      expect(result.frontmatter?.['title']).toBe('Meta Test');
      expect(result.frontmatter?.['customField']).toBe('customValue');
    });
  });

  describe('export', () => {
    it('should export with frontmatter', async () => {
      const node = {
        title: 'Export Test',
        description: 'Test description',
        contentType: 'concept',
        contentFormat: 'markdown',
        tags: ['tag1', 'tag2'],
        content: '# Heading\n\nContent here.',
      };

      const result = await plugin.export(node);

      expect(result).toContain('---');
      expect(result).toContain('title: "Export Test"');
      expect(result).toContain('description: "Test description"');
      expect(result).toContain('tags: [tag1, tag2]');
      expect(result).toContain('# Heading');
    });

    it('should strip existing frontmatter from content', async () => {
      const node = {
        title: 'New Title',
        contentFormat: 'markdown',
        content: `---
title: Old Title
---

# Content`,
      };

      const result = await plugin.export(node);

      // Should have new frontmatter, not old
      expect(result).toContain('title: "New Title"');
      expect(result).not.toContain('Old Title');
      expect(result).toContain('# Content');
    });

    it('should handle non-string content', async () => {
      const node = {
        title: 'Object Content',
        description: 'Has object data',
        contentFormat: 'markdown',
        content: { key: 'value', nested: { data: true } },
      };

      const result = await plugin.export(node);

      expect(result).toContain('# Object Content');
      expect(result).toContain('```json');
      expect(result).toContain('"key": "value"');
    });

    it('should truncate long descriptions', async () => {
      const longDesc = 'x'.repeat(300);
      const node = {
        title: 'Long Desc',
        description: longDesc,
        contentFormat: 'markdown',
        content: 'Body',
      };

      const result = await plugin.export(node);

      // Description should be truncated with ellipsis
      expect(result).not.toContain(longDesc);
      expect(result).toContain('...');
    });

    it('should escape quotes in title', async () => {
      const node = {
        title: 'Title with "quotes"',
        contentFormat: 'markdown',
        content: 'Body',
      };

      const result = await plugin.export(node);

      expect(result).toContain('\\"quotes\\"');
    });
  });

  describe('validate', () => {
    it('should validate correct markdown', async () => {
      const content = `---
title: Valid
---

# Heading

Content here.`;

      const result = await plugin.validate(content);

      expect(result.valid).toBeTrue();
      expect(result.errors).toEqual([]);
    });

    it('should warn when no frontmatter', async () => {
      const content = `# No Frontmatter

Just content.`;

      const result = await plugin.validate(content);

      expect(result.valid).toBeTrue();
      expect(result.warnings.some(w => w.code === 'NO_FRONTMATTER')).toBeTrue();
    });

    it('should error on unclosed frontmatter', async () => {
      const content = `---
title: Unclosed

# Content`;

      const result = await plugin.validate(content);

      expect(result.valid).toBeFalse();
      expect(result.errors.some(e => e.code === 'UNCLOSED_FRONTMATTER')).toBeTrue();
    });

    it('should warn when no title in frontmatter', async () => {
      const content = `---
description: No title field
---

# Content`;

      const result = await plugin.validate(content);

      expect(result.warnings.some(w => w.code === 'NO_TITLE')).toBeTrue();
    });

    it('should warn when no H1 heading', async () => {
      const content = `---
title: Has Title
---

## Only H2

Content`;

      const result = await plugin.validate(content);

      expect(result.warnings.some(w => w.code === 'NO_H1')).toBeTrue();
    });

    it('should warn when multiple H1 headings', async () => {
      const content = `# First H1

# Second H1

Content`;

      const result = await plugin.validate(content);

      expect(result.warnings.some(w => w.code === 'MULTIPLE_H1')).toBeTrue();
    });

    it('should warn on empty references', async () => {
      const content = `# Content

See [Feature: ] for details.`;

      const result = await plugin.validate(content);

      expect(result.warnings.some(w => w.code === 'EMPTY_REFERENCE')).toBeTrue();
    });

    it('should return stats in validation result', async () => {
      const content = `---
title: Stats Test
---

# Heading

Word one two three.

## Subheading

More words here.`;

      const result = await plugin.validate(content);

      expect(result.stats).toBeDefined();
      expect(result.stats?.['wordCount']).toBeGreaterThan(0);
      expect(result.stats?.['lineCount']).toBeGreaterThan(0);
      expect(result.stats?.['sectionCount']).toBe(2);
    });

    it('should return parsed preview on valid content', async () => {
      const content = `---
title: Preview Test
description: A description
tags: [a, b]
---

# Preview Content`;

      const result = await plugin.validate(content);

      expect(result.parsedPreview).toBeDefined();
      // Title comes from H1 heading, not frontmatter
      expect(result.parsedPreview?.['title']).toBe('Preview Content');
    });

    it('should handle File input', async () => {
      const content = `# File Content`;
      const file = new File([content], 'test.md', { type: 'text/markdown' });

      const result = await plugin.validate(file);

      expect(result.valid).toBeTrue();
    });
  });

  describe('detectFormat', () => {
    it('should return high confidence for frontmatter', () => {
      const content = `---
title: Test
---

Content`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeGreaterThanOrEqual(0.3);
    });

    it('should return high confidence for headings', () => {
      const content = `# Heading

## Subheading

Content`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeGreaterThanOrEqual(0.3);
    });

    it('should return confidence for markdown links', () => {
      const content = `Check out [this link](http://example.com)`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeGreaterThanOrEqual(0.2);
    });

    it('should return confidence for emphasis', () => {
      const content = `This is **bold** and *italic*`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeGreaterThanOrEqual(0.1);
    });

    it('should return confidence for lists', () => {
      const content = `- Item 1
- Item 2
* Item 3`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeGreaterThanOrEqual(0.1);
    });

    it('should return null for non-markdown content', () => {
      const content = `{ "json": true }`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeNull();
    });

    it('should cap confidence at 1', () => {
      const content = `---
title: Full Featured
---

# Heading

Check [link](url) and **bold** text.

- List item`;

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeLessThanOrEqual(1);
    });
  });

  describe('getFormatMetadata', () => {
    it('should return correct metadata', () => {
      const metadata = plugin.getFormatMetadata();

      expect(metadata.formatId).toBe('markdown');
      expect(metadata.displayName).toBe('Markdown');
      expect(metadata.icon).toBe('description');
      expect(metadata.category).toBe('document');
      expect(metadata.supportsRoundTrip).toBeTrue();
      expect(metadata.priority).toBe(10);
    });

    it('should include file extensions in metadata', () => {
      const metadata = plugin.getFormatMetadata();

      expect(metadata.fileExtensions).toContain('.md');
      expect(metadata.fileExtensions).toContain('.markdown');
    });
  });
});
