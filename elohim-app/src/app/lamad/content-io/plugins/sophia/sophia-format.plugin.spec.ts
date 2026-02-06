import { SophiaFormatPlugin } from './sophia-format.plugin';
import { SophiaRendererComponent } from './sophia-renderer.component';

describe('SophiaFormatPlugin', () => {
  let plugin: SophiaFormatPlugin;

  beforeEach(() => {
    plugin = new SophiaFormatPlugin();
  });

  describe('Identity', () => {
    it('should have correct formatId', () => {
      expect(plugin.formatId).toBe('sophia-quiz-json');
    });

    it('should have correct displayName', () => {
      expect(plugin.displayName).toBe('Sophia Assessment');
    });

    it('should have correct file extensions', () => {
      expect(plugin.fileExtensions).toEqual(['.sophia.json', '.sophia-quiz.json']);
    });

    it('should have correct MIME types', () => {
      expect(plugin.mimeTypes).toEqual(['application/vnd.sophia.assessment+json']);
    });

    it('should have alias formats', () => {
      expect(plugin.aliasFormats).toEqual(['sophia', 'perseus-quiz-json']);
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

    it('should not support editing', () => {
      expect(plugin.canEdit).toBeFalse();
    });
  });

  describe('getRendererComponent', () => {
    it('should return SophiaRendererComponent', () => {
      expect(plugin.getRendererComponent()).toBe(SophiaRendererComponent);
    });
  });

  describe('getRendererPriority', () => {
    it('should return priority 5', () => {
      expect(plugin.getRendererPriority()).toBe(5);
    });
  });

  describe('validate', () => {
    it('should validate valid Moment format content', async () => {
      const validMoment = JSON.stringify({
        purpose: 'mastery',
        content: {
          content: 'What is 2+2?',
          widgets: { radio: { options: ['3', '4', '5'] } },
        },
      });

      const result = await plugin.validate(validMoment);

      expect(result.valid).toBeTrue();
      expect(result.errors).toEqual([]);
    });

    it('should validate valid Perseus-compatible format', async () => {
      const validPerseus = JSON.stringify({
        question: {
          content: 'What is the capital of France?',
          widgets: { dropdown: { options: ['Paris', 'London'] } },
        },
      });

      const result = await plugin.validate(validPerseus);

      expect(result.valid).toBeTrue();
      expect(result.errors).toEqual([]);
    });

    it('should validate array of moments', async () => {
      const momentArray = JSON.stringify([
        {
          purpose: 'mastery',
          content: { content: 'Q1', widgets: {} },
        },
        {
          purpose: 'reflection',
          content: { content: 'Q2', widgets: {} },
        },
      ]);

      const result = await plugin.validate(momentArray);

      expect(result.valid).toBeTrue();
    });

    it('should reject invalid structure', async () => {
      const invalid = JSON.stringify({
        foo: 'bar',
      });

      const result = await plugin.validate(invalid);

      expect(result.valid).toBeFalse();
      expect(result.errors[0].code).toBe('INVALID_STRUCTURE');
    });

    it('should reject malformed JSON', async () => {
      const result = await plugin.validate('not valid json');

      expect(result.valid).toBeFalse();
      expect(result.errors[0].code).toBe('PARSE_ERROR');
    });

    it('should handle File input', async () => {
      const content = JSON.stringify({
        purpose: 'mastery',
        content: { content: 'Question', widgets: {} },
      });
      const file = new File([content], 'test.sophia.json', {
        type: 'application/json',
      });

      const result = await plugin.validate(file);

      expect(result.valid).toBeTrue();
    });

    it('should reject empty content', async () => {
      const result = await plugin.validate(JSON.stringify(null));

      expect(result.valid).toBeFalse();
    });

    it('should reject content with missing widgets', async () => {
      const invalid = JSON.stringify({
        purpose: 'mastery',
        content: { content: 'Question' },
      });

      const result = await plugin.validate(invalid);

      expect(result.valid).toBeFalse();
    });
  });

  describe('import', () => {
    it('should import valid Moment content', async () => {
      const content = JSON.stringify({
        purpose: 'mastery',
        content: { content: 'Question', widgets: {} },
        metadata: { title: 'My Quiz' },
      });

      const result = await plugin.import(content);

      expect(result.contentFormat).toBe('sophia-quiz-json');
      expect(result.title).toBe('My Quiz');
      expect(result.metadata?.['assessmentPurpose']).toBe('mastery');
    });

    it('should extract title from metadata', async () => {
      const content = JSON.stringify({
        purpose: 'mastery',
        content: { content: 'Q', widgets: {} },
        metadata: { title: 'Custom Title' },
      });

      const result = await plugin.import(content);

      expect(result.title).toBe('Custom Title');
    });

    it('should use default title for discovery assessments', async () => {
      const content = JSON.stringify({
        purpose: 'discovery',
        content: { content: 'Q', widgets: {} },
      });

      const result = await plugin.import(content);

      expect(result.title).toBe('Discovery Assessment');
    });

    it('should use default title for mastery assessments', async () => {
      const content = JSON.stringify({
        purpose: 'mastery',
        content: { content: 'Q', widgets: {} },
      });

      const result = await plugin.import(content);

      expect(result.title).toBe('Sophia Assessment');
    });

    it('should handle File input', async () => {
      const content = JSON.stringify({
        purpose: 'reflection',
        content: { content: 'Reflect on...', widgets: {} },
      });
      const file = new File([content], 'quiz.sophia.json');

      const result = await plugin.import(file);

      expect(result.contentFormat).toBe('sophia-quiz-json');
      expect(result.metadata?.['assessmentPurpose']).toBe('reflection');
    });

    it('should throw on invalid content', async () => {
      await expectAsync(plugin.import('not json')).toBeRejectedWithError();
    });

    it('should throw on invalid structure', async () => {
      await expectAsync(plugin.import(JSON.stringify({ invalid: true }))).toBeRejectedWithError(
        'Invalid Sophia assessment format'
      );
    });

    it('should detect discovery mode from discoveryMode flag', async () => {
      const content = JSON.stringify({
        discoveryMode: true,
        question: { content: 'Q', widgets: {} },
      });

      const result = await plugin.import(content);

      expect(result.metadata?.['assessmentPurpose']).toBe('discovery');
    });
  });

  describe('export', () => {
    it('should export content as JSON blob', async () => {
      const node = {
        content: {
          purpose: 'mastery',
          content: { content: 'Question', widgets: {} },
        },
        title: 'Test Quiz',
        contentFormat: 'sophia-quiz-json',
      };

      const result = await plugin.export(node as never);

      expect(result).toBeInstanceOf(Blob);
      expect((result as Blob).type).toBe('application/json');

      const text = await (result as Blob).text();
      const parsed = JSON.parse(text);
      expect(parsed.purpose).toBe('mastery');
    });
  });

  describe('getFormatMetadata', () => {
    it('should return correct metadata', () => {
      const metadata = plugin.getFormatMetadata();

      expect(metadata.formatId).toBe('sophia-quiz-json');
      expect(metadata.displayName).toBe('Sophia Assessment');
      expect(metadata.icon).toBe('quiz');
      expect(metadata.category).toBe('data');
      expect(metadata.supportsRoundTrip).toBeTrue();
      expect(metadata.priority).toBe(10);
    });
  });

  describe('getEditorConfig', () => {
    it('should return code editor config with live preview', () => {
      const config = plugin.getEditorConfig();

      expect(config.editorMode).toBe('code');
      expect(config.supportsLivePreview).toBeTrue();
    });
  });

  describe('detectFormat', () => {
    it('should return 0.95 for reflection/discovery content', () => {
      const content = JSON.stringify({
        purpose: 'reflection',
        content: { content: 'Q', widgets: {} },
      });

      expect(plugin.detectFormat(content)).toBe(0.95);
    });

    it('should return 0.95 for discovery purpose', () => {
      const content = JSON.stringify({
        purpose: 'discovery',
        content: { content: 'Q', widgets: {} },
      });

      expect(plugin.detectFormat(content)).toBe(0.95);
    });

    it('should return 0.85 for mastery content', () => {
      const content = JSON.stringify({
        purpose: 'mastery',
        content: { content: 'Q', widgets: {} },
      });

      expect(plugin.detectFormat(content)).toBe(0.85);
    });

    it('should return 0.7 for Perseus-like content with hints', () => {
      const content = JSON.stringify({
        question: { content: 'Q', widgets: {} },
        hints: [],
      });

      expect(plugin.detectFormat(content)).toBe(0.7);
    });

    it('should return 0.5 for valid but unmarked content', () => {
      const content = JSON.stringify({
        question: { content: 'Q', widgets: {} },
      });

      expect(plugin.detectFormat(content)).toBe(0.5);
    });

    it('should return null for invalid JSON', () => {
      expect(plugin.detectFormat('not json')).toBeNull();
    });

    it('should return null for non-matching structure', () => {
      const content = JSON.stringify({ foo: 'bar' });

      expect(plugin.detectFormat(content)).toBeNull();
    });
  });
});
