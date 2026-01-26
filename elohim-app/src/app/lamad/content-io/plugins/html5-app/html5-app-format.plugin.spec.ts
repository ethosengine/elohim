import { TestBed } from '@angular/core/testing';
import { Html5AppFormatPlugin } from './html5-app-format.plugin';
import { IframeRendererComponent } from '../../../renderers/iframe-renderer/iframe-renderer.component';

describe('Html5AppFormatPlugin', () => {
  let plugin: Html5AppFormatPlugin;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [Html5AppFormatPlugin],
    });
    plugin = TestBed.inject(Html5AppFormatPlugin);
  });

  describe('Identity', () => {
    it('should have correct formatId', () => {
      expect(plugin.formatId).toBe('html5-app');
    });

    it('should have correct displayName', () => {
      expect(plugin.displayName).toBe('HTML5 Application');
    });

    it('should have correct file extensions', () => {
      expect(plugin.fileExtensions).toEqual(['.zip']);
    });

    it('should have correct MIME types', () => {
      expect(plugin.mimeTypes).toContain('application/zip');
      expect(plugin.mimeTypes).toContain('application/x-zip-compressed');
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
    it('should return IframeRendererComponent', () => {
      expect(plugin.getRendererComponent()).toBe(IframeRendererComponent);
    });
  });

  describe('getRendererPriority', () => {
    it('should return priority 15', () => {
      expect(plugin.getRendererPriority()).toBe(15);
    });
  });

  describe('getEditorComponent', () => {
    it('should return null', () => {
      expect(plugin.getEditorComponent()).toBeNull();
    });
  });

  describe('getEditorConfig', () => {
    it('should return code editor without live preview', () => {
      const config = plugin.getEditorConfig();

      expect(config.editorMode).toBe('code');
      expect(config.supportsLivePreview).toBeFalse();
    });
  });

  describe('import', () => {
    it('should import from JSON string', async () => {
      const content = JSON.stringify({
        appId: 'test-app',
        entryPoint: 'index.html',
      });

      const result = await plugin.import(content);

      expect(result.contentFormat).toBe('html5-app');
      expect(result.contentType).toBe('simulation');
      expect((result.content as any).appId).toBe('test-app');
      expect((result.content as any).entryPoint).toBe('index.html');
    });

    it('should generate humanized title from appId', async () => {
      const content = JSON.stringify({
        appId: 'evolution-of-trust',
        entryPoint: 'index.html',
      });

      const result = await plugin.import(content);

      expect(result.title).toBe('Evolution Of Trust');
    });

    it('should import from ZIP file', async () => {
      const file = new File(['zip content'], 'my-cool-app.zip', {
        type: 'application/zip',
      });

      const result = await plugin.import(file);

      expect(result.contentFormat).toBe('html5-app');
      expect((result.content as any).appId).toBe('my-cool-app');
      expect((result.content as any).entryPoint).toBe('index.html');
      expect(result.title).toBe('My Cool App');
    });

    it('should include metadata from file import', async () => {
      const file = new File(['content'], 'test-app.zip', {
        type: 'application/zip',
      });

      const result = await plugin.import(file);

      expect(result.metadata?.['originalFilename']).toBe('test-app.zip');
      expect(result.metadata?.['sizeBytes']).toBe(7);
      expect(result.metadata?.['embedStrategy']).toBe('iframe');
    });

    it('should throw on invalid JSON', async () => {
      await expectAsync(plugin.import('not json')).toBeRejectedWithError(
        'Invalid HTML5 app JSON structure'
      );
    });

    it('should add default tags', async () => {
      const file = new File(['content'], 'app.zip');

      const result = await plugin.import(file);

      expect(result.tags).toContain('html5-app');
      expect(result.tags).toContain('interactive');
      expect(result.tags).toContain('simulation');
    });
  });

  describe('export', () => {
    it('should export content as JSON string', async () => {
      const node = {
        content: {
          appId: 'test-app',
          entryPoint: 'index.html',
          fallbackUrl: 'https://example.com/app',
        },
        title: 'Test App',
        contentFormat: 'html5-app',
      };

      const result = await plugin.export(node);

      const parsed = JSON.parse(result);
      expect(parsed.appId).toBe('test-app');
      expect(parsed.entryPoint).toBe('index.html');
      expect(parsed.fallbackUrl).toBe('https://example.com/app');
    });

    it('should format JSON with indentation', async () => {
      const node = {
        content: { appId: 'app', entryPoint: 'index.html' },
        title: 'App',
        contentFormat: 'html5-app',
      };

      const result = await plugin.export(node);

      expect(result).toContain('\n');
      expect(result).toContain('  ');
    });
  });

  describe('validate', () => {
    describe('JSON input', () => {
      it('should validate correct JSON structure', async () => {
        const content = JSON.stringify({
          appId: 'valid-app',
          entryPoint: 'index.html',
          fallbackUrl: 'https://example.com',
        });

        const result = await plugin.validate(content);

        expect(result.valid).toBeTrue();
        expect(result.errors).toEqual([]);
      });

      it('should error on invalid JSON', async () => {
        const result = await plugin.validate('not json');

        expect(result.valid).toBeFalse();
        expect(result.errors[0].code).toBe('INVALID_JSON');
      });

      it('should error on missing appId', async () => {
        const content = JSON.stringify({ entryPoint: 'index.html' });

        const result = await plugin.validate(content);

        expect(result.valid).toBeFalse();
        expect(result.errors.some(e => e.code === 'MISSING_APP_ID')).toBeTrue();
      });

      it('should error on invalid appId format', async () => {
        const content = JSON.stringify({
          appId: 'Invalid App ID!',
          entryPoint: 'index.html',
        });

        const result = await plugin.validate(content);

        expect(result.valid).toBeFalse();
        expect(result.errors.some(e => e.code === 'INVALID_APP_ID')).toBeTrue();
      });

      it('should accept valid appId with hyphens', async () => {
        const content = JSON.stringify({
          appId: 'my-cool-app-123',
          entryPoint: 'index.html',
        });

        const result = await plugin.validate(content);

        expect(result.valid).toBeTrue();
      });

      it('should error on missing entryPoint', async () => {
        const content = JSON.stringify({ appId: 'valid-app' });

        const result = await plugin.validate(content);

        expect(result.valid).toBeFalse();
        expect(result.errors.some(e => e.code === 'MISSING_ENTRY_POINT')).toBeTrue();
      });

      it('should warn on non-HTML entryPoint', async () => {
        const content = JSON.stringify({
          appId: 'valid-app',
          entryPoint: 'main.js',
        });

        const result = await plugin.validate(content);

        expect(result.valid).toBeTrue();
        expect(result.warnings.some(w => w.code === 'ENTRY_POINT_NOT_HTML')).toBeTrue();
      });

      it('should warn when no fallbackUrl', async () => {
        const content = JSON.stringify({
          appId: 'valid-app',
          entryPoint: 'index.html',
        });

        const result = await plugin.validate(content);

        expect(result.valid).toBeTrue();
        expect(result.warnings.some(w => w.code === 'NO_FALLBACK')).toBeTrue();
      });
    });

    describe('File input', () => {
      it('should validate ZIP file', async () => {
        const file = new File(['content'], 'app.zip', { type: 'application/zip' });

        const result = await plugin.validate(file);

        expect(result.valid).toBeTrue();
      });

      it('should error on non-ZIP file', async () => {
        const file = new File(['content'], 'app.txt', { type: 'text/plain' });

        const result = await plugin.validate(file);

        expect(result.valid).toBeFalse();
        expect(result.errors.some(e => e.code === 'NOT_ZIP')).toBeTrue();
      });

      it('should warn on large files', async () => {
        // Create a mock file with size > 100MB
        const largeContent = new ArrayBuffer(101 * 1024 * 1024);
        const file = new File([largeContent], 'large-app.zip', { type: 'application/zip' });

        const result = await plugin.validate(file);

        expect(result.warnings.some(w => w.code === 'LARGE_FILE')).toBeTrue();
      });

      it('should include stats in validation result', async () => {
        const file = new File(['content'], 'app.zip');

        const result = await plugin.validate(file);

        expect(result.stats?.['formatId']).toBe('html5-app');
        expect(result.stats?.['isFile']).toBeTrue();
      });
    });
  });

  describe('detectFormat', () => {
    it('should return 0.9 for appId + entryPoint structure', () => {
      const content = JSON.stringify({
        appId: 'test-app',
        entryPoint: 'index.html',
      });

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBe(0.9);
    });

    it('should return 0.95 for contentFormat html5-app', () => {
      const content = JSON.stringify({
        contentFormat: 'html5-app',
        content: { appId: 'app' },
      });

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBe(0.95);
    });

    it('should return null for invalid JSON', () => {
      const confidence = plugin.detectFormat('not json');

      expect(confidence).toBeNull();
    });

    it('should return null for non-matching structure', () => {
      const content = JSON.stringify({ foo: 'bar' });

      const confidence = plugin.detectFormat(content);

      expect(confidence).toBeNull();
    });
  });

  describe('getFormatMetadata', () => {
    it('should return correct metadata', () => {
      const metadata = plugin.getFormatMetadata();

      expect(metadata.formatId).toBe('html5-app');
      expect(metadata.displayName).toBe('HTML5 Application');
      expect(metadata.icon).toBe('web');
      expect(metadata.category).toBe('media');
      expect(metadata.supportsRoundTrip).toBeFalse();
      expect(metadata.priority).toBe(15);
    });

    it('should include description about Doorway serving', () => {
      const metadata = plugin.getFormatMetadata();

      expect(metadata.description).toContain('Doorway');
    });
  });
});
