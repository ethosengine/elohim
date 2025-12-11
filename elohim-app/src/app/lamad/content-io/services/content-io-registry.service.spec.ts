import { TestBed } from '@angular/core/testing';
import { ContentIORegistryService } from './content-io-registry.service';
import { ContentIOPlugin } from '../interfaces/content-io-plugin.interface';

describe('ContentIORegistryService', () => {
  let service: ContentIORegistryService;

  const createMockPlugin = (overrides: Partial<ContentIOPlugin> = {}): ContentIOPlugin => ({
    formatId: 'test-format',
    displayName: 'Test Format',
    fileExtensions: ['.test'],
    mimeTypes: ['text/test'],
    canImport: true,
    canExport: true,
    canValidate: true,
    import: jasmine.createSpy('import').and.returnValue(Promise.resolve({ nodes: [], warnings: [] })),
    export: jasmine.createSpy('export').and.returnValue(Promise.resolve('exported')),
    validate: jasmine.createSpy('validate').and.returnValue(Promise.resolve({ valid: true, errors: [], warnings: [] })),
    getFormatMetadata: () => ({
      formatId: overrides.formatId ?? 'test-format',
      displayName: overrides.displayName ?? 'Test Format',
      description: 'Test format description',
      fileExtensions: overrides.fileExtensions ?? ['.test'],
      mimeTypes: overrides.mimeTypes ?? ['text/test'],
      canImport: overrides.canImport ?? true,
      canExport: overrides.canExport ?? true,
      canValidate: overrides.canValidate ?? false,
      priority: 50,
      category: 'document',
      supportsRoundTrip: true
    }),
    ...overrides
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ContentIORegistryService]
    });
    service = TestBed.inject(ContentIORegistryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('register', () => {
    it('should register a plugin', () => {
      const plugin = createMockPlugin();

      service.register(plugin);

      expect(service.getPlugin('test-format')).toBe(plugin);
    });

    it('should replace existing plugin with same formatId', () => {
      const plugin1 = createMockPlugin();
      const plugin2 = createMockPlugin({ displayName: 'New Test Format' });

      service.register(plugin1);
      service.register(plugin2);

      expect(service.getPlugin('test-format')).toBe(plugin2);
    });

    it('should index by file extension', () => {
      const plugin = createMockPlugin({ fileExtensions: ['.md', '.markdown'] });

      service.register(plugin);

      expect(service.getPluginsForExtension('.md')).toContain(plugin);
      expect(service.getPluginsForExtension('.markdown')).toContain(plugin);
    });

    it('should normalize file extensions', () => {
      const plugin = createMockPlugin({ fileExtensions: ['md', '.MD'] });

      service.register(plugin);

      expect(service.getPluginsForExtension('.md')).toContain(plugin);
      expect(service.getPluginsForExtension('md')).toContain(plugin);
    });

    it('should index by MIME type', () => {
      const plugin = createMockPlugin({ mimeTypes: ['text/markdown', 'text/x-markdown'] });

      service.register(plugin);

      expect(service.getPluginsForMimeType('text/markdown')).toContain(plugin);
      expect(service.getPluginsForMimeType('text/x-markdown')).toContain(plugin);
    });

    it('should normalize MIME types', () => {
      const plugin = createMockPlugin({ mimeTypes: ['Text/Markdown'] });

      service.register(plugin);

      expect(service.getPluginsForMimeType('text/markdown')).toContain(plugin);
    });
  });

  describe('unregister', () => {
    it('should unregister a plugin', () => {
      const plugin = createMockPlugin();
      service.register(plugin);

      service.unregister('test-format');

      expect(service.getPlugin('test-format')).toBeUndefined();
    });

    it('should remove from extension map', () => {
      const plugin = createMockPlugin({ fileExtensions: ['.test'] });
      service.register(plugin);

      service.unregister('test-format');

      expect(service.getPluginsForExtension('.test')).toEqual([]);
    });

    it('should remove from MIME type map', () => {
      const plugin = createMockPlugin({ mimeTypes: ['text/test'] });
      service.register(plugin);

      service.unregister('test-format');

      expect(service.getPluginsForMimeType('text/test')).toEqual([]);
    });

    it('should do nothing if plugin not registered', () => {
      expect(() => service.unregister('nonexistent')).not.toThrow();
    });
  });

  describe('getPlugin', () => {
    it('should return plugin by formatId', () => {
      const plugin = createMockPlugin();
      service.register(plugin);

      expect(service.getPlugin('test-format')).toBe(plugin);
    });

    it('should return undefined for unknown formatId', () => {
      expect(service.getPlugin('unknown')).toBeUndefined();
    });
  });

  describe('getAllPlugins', () => {
    it('should return all registered plugins', () => {
      const plugin1 = createMockPlugin({ formatId: 'format1' });
      const plugin2 = createMockPlugin({ formatId: 'format2' });
      service.register(plugin1);
      service.register(plugin2);

      const plugins = service.getAllPlugins();

      expect(plugins).toContain(plugin1);
      expect(plugins).toContain(plugin2);
    });

    it('should return empty array when no plugins registered', () => {
      expect(service.getAllPlugins()).toEqual([]);
    });
  });

  describe('getRegisteredFormats', () => {
    it('should return all registered format IDs', () => {
      const plugin1 = createMockPlugin({ formatId: 'markdown' });
      const plugin2 = createMockPlugin({ formatId: 'gherkin' });
      service.register(plugin1);
      service.register(plugin2);

      const formats = service.getRegisteredFormats();

      expect(formats).toContain('markdown');
      expect(formats).toContain('gherkin');
    });
  });

  describe('getPluginsForExtension', () => {
    it('should return plugins matching extension', () => {
      const plugin = createMockPlugin({ fileExtensions: ['.md'] });
      service.register(plugin);

      expect(service.getPluginsForExtension('.md')).toContain(plugin);
    });

    it('should handle extension without dot', () => {
      const plugin = createMockPlugin({ fileExtensions: ['.md'] });
      service.register(plugin);

      expect(service.getPluginsForExtension('md')).toContain(plugin);
    });

    it('should return empty array for unknown extension', () => {
      expect(service.getPluginsForExtension('.unknown')).toEqual([]);
    });

    it('should return multiple plugins for same extension', () => {
      const plugin1 = createMockPlugin({ formatId: 'format1', fileExtensions: ['.txt'] });
      const plugin2 = createMockPlugin({ formatId: 'format2', fileExtensions: ['.txt'] });
      service.register(plugin1);
      service.register(plugin2);

      const plugins = service.getPluginsForExtension('.txt');

      expect(plugins.length).toBe(2);
    });
  });

  describe('getPluginsForMimeType', () => {
    it('should return plugins matching MIME type', () => {
      const plugin = createMockPlugin({ mimeTypes: ['text/markdown'] });
      service.register(plugin);

      expect(service.getPluginsForMimeType('text/markdown')).toContain(plugin);
    });

    it('should be case insensitive', () => {
      const plugin = createMockPlugin({ mimeTypes: ['text/markdown'] });
      service.register(plugin);

      expect(service.getPluginsForMimeType('Text/Markdown')).toContain(plugin);
    });

    it('should return empty array for unknown MIME type', () => {
      expect(service.getPluginsForMimeType('application/unknown')).toEqual([]);
    });
  });

  describe('getImportableFormats', () => {
    it('should return only formats that support import', () => {
      const importable = createMockPlugin({ formatId: 'importable', canImport: true });
      const notImportable = createMockPlugin({ formatId: 'not-importable', canImport: false });
      service.register(importable);
      service.register(notImportable);

      const formats = service.getImportableFormats();

      expect(formats.find(f => f.formatId === 'importable')).toBeTruthy();
      expect(formats.find(f => f.formatId === 'not-importable')).toBeFalsy();
    });

    it('should sort by priority', () => {
      const lowPriority = createMockPlugin({
        formatId: 'low',
        canImport: true,
        getFormatMetadata: () => ({
          formatId: 'low',
          displayName: 'Low',
          description: '',
          fileExtensions: [],
          mimeTypes: [],
          canImport: true,
          canExport: false,
          canValidate: false,
          priority: 10,
          category: 'document',
          supportsRoundTrip: true
        })
      });
      const highPriority = createMockPlugin({
        formatId: 'high',
        canImport: true,
        getFormatMetadata: () => ({
          formatId: 'high',
          displayName: 'High',
          description: '',
          fileExtensions: [],
          mimeTypes: [],
          canImport: true,
          canExport: false,
          canValidate: false,
          priority: 100,
          category: 'document',
          supportsRoundTrip: true
        })
      });
      service.register(lowPriority);
      service.register(highPriority);

      const formats = service.getImportableFormats();

      expect(formats[0].formatId).toBe('high');
    });
  });

  describe('getExportableFormats', () => {
    it('should return only formats that support export', () => {
      const exportable = createMockPlugin({ formatId: 'exportable', canExport: true });
      const notExportable = createMockPlugin({ formatId: 'not-exportable', canExport: false });
      service.register(exportable);
      service.register(notExportable);

      const formats = service.getExportableFormats();

      expect(formats.find(f => f.formatId === 'exportable')).toBeTruthy();
      expect(formats.find(f => f.formatId === 'not-exportable')).toBeFalsy();
    });
  });

  describe('getExportableFormatsForContent', () => {
    it('should return all exportable formats', () => {
      const plugin = createMockPlugin({ canExport: true });
      service.register(plugin);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const formats = service.getExportableFormatsForContent(node);

      expect(formats.length).toBeGreaterThan(0);
    });
  });

  describe('detectFormat', () => {
    it('should detect format by extension when single plugin matches', async () => {
      const plugin = createMockPlugin({ formatId: 'markdown', fileExtensions: ['.md'] });
      service.register(plugin);
      const file = new File(['# Test'], 'test.md', { type: 'text/markdown' });

      const result = await service.detectFormat(file);

      expect(result).toBe('markdown');
    });

    it('should use content detection when multiple plugins match extension', async () => {
      const plugin1 = createMockPlugin({
        formatId: 'format1',
        fileExtensions: ['.txt'],
        detectFormat: jasmine.createSpy('detectFormat').and.returnValue(0.5)
      });
      const plugin2 = createMockPlugin({
        formatId: 'format2',
        fileExtensions: ['.txt'],
        detectFormat: jasmine.createSpy('detectFormat').and.returnValue(0.9)
      });
      service.register(plugin1);
      service.register(plugin2);
      const file = new File(['content'], 'test.txt', { type: 'text/plain' });

      const result = await service.detectFormat(file);

      expect(result).toBe('format2');
    });

    it('should detect by MIME type when extension not matched', async () => {
      const plugin = createMockPlugin({
        formatId: 'markdown',
        fileExtensions: [],
        mimeTypes: ['text/markdown']
      });
      service.register(plugin);
      const file = new File(['# Test'], 'test', { type: 'text/markdown' });

      const result = await service.detectFormat(file);

      expect(result).toBe('markdown');
    });

    it('should return null if no format detected', async () => {
      const file = new File(['unknown'], 'test.xyz', { type: 'application/octet-stream' });

      const result = await service.detectFormat(file);

      expect(result).toBeNull();
    });
  });

  describe('detectFormatFromContent', () => {
    it('should return format with highest confidence', () => {
      const plugin1 = createMockPlugin({
        formatId: 'format1',
        detectFormat: () => 0.5
      });
      const plugin2 = createMockPlugin({
        formatId: 'format2',
        detectFormat: () => 0.9
      });
      service.register(plugin1);
      service.register(plugin2);

      const result = service.detectFormatFromContent('# Test content');

      expect(result).toBe('format2');
    });

    it('should return null if no plugin detects format', () => {
      const plugin = createMockPlugin({
        detectFormat: () => null
      });
      service.register(plugin);

      const result = service.detectFormatFromContent('unknown content');

      expect(result).toBeNull();
    });

    it('should use provided candidates instead of all plugins', () => {
      const plugin1 = createMockPlugin({
        formatId: 'format1',
        detectFormat: () => 0.9
      });
      const plugin2 = createMockPlugin({
        formatId: 'format2',
        detectFormat: () => 0.5
      });
      service.register(plugin1);
      service.register(plugin2);

      const result = service.detectFormatFromContent('content', [plugin2]);

      expect(result).toBe('format2');
    });

    it('should ignore zero confidence', () => {
      const plugin = createMockPlugin({
        detectFormat: () => 0
      });
      service.register(plugin);

      const result = service.detectFormatFromContent('content');

      expect(result).toBeNull();
    });

    it('should skip plugins without detectFormat method', () => {
      const pluginWithDetect = createMockPlugin({
        formatId: 'with-detect',
        detectFormat: () => 0.8
      });
      const pluginWithoutDetect = createMockPlugin({
        formatId: 'without-detect'
      });
      service.register(pluginWithDetect);
      service.register(pluginWithoutDetect);

      const result = service.detectFormatFromContent('content');

      expect(result).toBe('with-detect');
    });
  });
});
