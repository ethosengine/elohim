import { TestBed } from '@angular/core/testing';
import { Type } from '@angular/core';
import { ContentFormatRegistryService } from './content-format-registry.service';
import {
  ContentFormatPlugin,
  ContentRenderer,
  ContentEditorComponent,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
} from '../interfaces/content-format-plugin.interface';
import { ContentNode } from '../../models/content-node.model';

describe('ContentFormatRegistryService', () => {
  let service: ContentFormatRegistryService;

  // Mock renderer component
  class MockRendererComponent implements ContentRenderer {
    node!: ContentNode;
  }

  // Mock editor component
  class MockEditorComponent {
    node!: ContentNode;
    config!: EditorConfig;
    readonly = false;
  }

  const createMockPlugin = (overrides: Partial<ContentFormatPlugin> = {}): ContentFormatPlugin => ({
    formatId: 'test-format',
    displayName: 'Test Format',
    fileExtensions: ['.test'],
    mimeTypes: ['text/test'],
    canImport: true,
    canExport: true,
    canValidate: false,
    canRender: true,
    canEdit: false,
    import: jasmine
      .createSpy('import')
      .and.returnValue(Promise.resolve({ nodes: [], warnings: [] })),
    export: jasmine.createSpy('export').and.returnValue(Promise.resolve('exported')),
    validate: jasmine
      .createSpy('validate')
      .and.returnValue(Promise.resolve({ valid: true, errors: [], warnings: [] })),
    getFormatMetadata: () => ({
      formatId: overrides.formatId ?? 'test-format',
      displayName: overrides.displayName ?? 'Test Format',
      description: 'Test format description',
      fileExtensions: overrides.fileExtensions ?? ['.test'],
      mimeTypes: overrides.mimeTypes ?? ['text/test'],
      canImport: overrides.canImport ?? true,
      canExport: overrides.canExport ?? true,
      canValidate: false,
      priority: 50,
      category: 'document',
      supportsRoundTrip: true,
    }),
    getRendererComponent: jasmine
      .createSpy('getRendererComponent')
      .and.returnValue(MockRendererComponent),
    getRendererPriority: () => 0,
    getEditorComponent: jasmine.createSpy('getEditorComponent').and.returnValue(null),
    getEditorConfig: jasmine.createSpy('getEditorConfig').and.returnValue(DEFAULT_EDITOR_CONFIG),
    ...overrides,
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ContentFormatRegistryService],
    });
    service = TestBed.inject(ContentFormatRegistryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('register', () => {
    it('should register a plugin', () => {
      const plugin = createMockPlugin();

      service.register(plugin);

      const result = service.getRendererComponent('test-format');
      expect(result).toBe(MockRendererComponent);
    });

    it('should replace existing plugin with same formatId', () => {
      const plugin1 = createMockPlugin({
        getRendererComponent: () => MockRendererComponent,
      });

      class NewRendererComponent implements ContentRenderer {
        node!: ContentNode;
      }

      const plugin2 = createMockPlugin({
        getRendererComponent: () => NewRendererComponent,
      });

      service.register(plugin1);
      service.register(plugin2);

      const result = service.getRendererComponent('test-format');
      expect(result).toBe(NewRendererComponent);
    });
  });

  describe('getRendererComponent', () => {
    it('should return renderer component for registered format', () => {
      const plugin = createMockPlugin();
      service.register(plugin);

      const result = service.getRendererComponent('test-format');

      expect(result).toBe(MockRendererComponent);
    });

    it('should return null for unregistered format', () => {
      const result = service.getRendererComponent('unknown-format');

      expect(result).toBeNull();
    });

    it('should return null if plugin canRender is false', () => {
      const plugin = createMockPlugin({
        canRender: false,
      });
      service.register(plugin);

      const result = service.getRendererComponent('test-format');

      expect(result).toBeNull();
    });
  });

  describe('getRenderer', () => {
    it('should return renderer for ContentNode based on contentFormat', () => {
      const plugin = createMockPlugin();
      service.register(plugin);

      // Use 'markdown' format since we need a valid ContentFormat type
      // But register plugin under 'markdown' to match
      const mdPlugin = createMockPlugin({ formatId: 'markdown' });
      service.register(mdPlugin);

      const node = {
        id: 'test-1',
        title: 'Test',
        description: '',
        content: '',
        contentType: 'concept',
        contentFormat: 'markdown',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
      } as ContentNode;
      const result = service.getRenderer(node);

      expect(result).toBe(MockRendererComponent);
    });
  });

  describe('getEditorComponent', () => {
    it('should return editor component when plugin provides one', () => {
      const plugin = createMockPlugin({
        canEdit: true,
        getEditorComponent: () => MockEditorComponent as any,
      });
      service.register(plugin);

      const result = service.getEditorComponent('test-format');

      expect(result).toBe(MockEditorComponent as any);
    });

    it('should return default editor when plugin does not provide one', () => {
      service.registerDefaultEditor(MockEditorComponent as any);
      const plugin = createMockPlugin({
        canEdit: false,
        getEditorComponent: () => null,
      });
      service.register(plugin);

      const result = service.getEditorComponent('test-format');

      expect(result).toBe(MockEditorComponent as any);
    });

    it('should return null when no editor available and no default registered', () => {
      const plugin = createMockPlugin({
        canEdit: false,
        getEditorComponent: () => null,
      });
      service.register(plugin);

      const result = service.getEditorComponent('test-format');

      expect(result).toBeNull();
    });

    it('should return null for unregistered format when no default', () => {
      const result = service.getEditorComponent('unknown-format');

      expect(result).toBeNull();
    });
  });

  describe('getEditorConfig', () => {
    it('should return editor config from plugin', () => {
      const customConfig: EditorConfig = {
        editorMode: 'visual',
        supportsLivePreview: true,
        showLineNumbers: false,
        wordWrap: false,
        toolbar: {
          enabled: false,
          actions: [],
        },
      };
      const plugin = createMockPlugin({
        getEditorConfig: () => customConfig,
      });
      service.register(plugin);

      const result = service.getEditorConfig('test-format');

      expect(result).toBe(customConfig);
    });

    it('should return default config for unregistered format', () => {
      const result = service.getEditorConfig('unknown-format');

      expect(result).toEqual(DEFAULT_EDITOR_CONFIG);
    });
  });

  describe('detectFormat', () => {
    it('should detect format by file extension', async () => {
      const mdPlugin = createMockPlugin({
        formatId: 'markdown',
        fileExtensions: ['.md', '.markdown'],
        mimeTypes: ['text/markdown'],
      });
      service.register(mdPlugin);

      const file = new File(['# Test'], 'test.md', { type: 'text/markdown' });
      const result = await service.detectFormat(file);

      expect(result).toBe('markdown');
    });

    it('should detect format by MIME type when extension not matched', async () => {
      const mdPlugin = createMockPlugin({
        formatId: 'markdown',
        fileExtensions: [],
        mimeTypes: ['text/markdown'],
      });
      service.register(mdPlugin);

      const file = new File(['# Test'], 'test', { type: 'text/markdown' });
      const result = await service.detectFormat(file);

      expect(result).toBe('markdown');
    });

    it('should use content detection when multiple plugins match extension', async () => {
      const plugin1 = createMockPlugin({
        formatId: 'format1',
        fileExtensions: ['.txt'],
        detectFormat: () => 0.3,
      });
      const plugin2 = createMockPlugin({
        formatId: 'format2',
        fileExtensions: ['.txt'],
        detectFormat: () => 0.9,
      });
      service.register(plugin1);
      service.register(plugin2);

      const file = new File(['content'], 'test.txt', { type: 'text/plain' });
      const result = await service.detectFormat(file);

      expect(result).toBe('format2');
    });

    it('should return null if no format detected', async () => {
      const file = new File(['unknown'], 'test.xyz', { type: 'application/octet-stream' });

      const result = await service.detectFormat(file);

      expect(result).toBeNull();
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
      expect(formats.length).toBe(2);
    });

    it('should return empty array when no plugins registered', () => {
      expect(service.getRegisteredFormats()).toEqual([]);
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

  describe('unregister', () => {
    it('should remove plugin from registry', () => {
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

  describe('canRender', () => {
    it('should return true for renderable format', () => {
      const plugin = createMockPlugin({ canRender: true });
      service.register(plugin);

      expect(service.canRender('test-format')).toBeTrue();
    });

    it('should return false for non-renderable format', () => {
      const plugin = createMockPlugin({ canRender: false });
      service.register(plugin);

      expect(service.canRender('test-format')).toBeFalse();
    });

    it('should return false for unknown format', () => {
      expect(service.canRender('unknown')).toBeFalse();
    });
  });

  describe('getRenderableFormats', () => {
    it('should return only formats with renderers', () => {
      const renderable = createMockPlugin({ formatId: 'renderable', canRender: true });
      const notRenderable = createMockPlugin({ formatId: 'not-renderable', canRender: false });
      service.register(renderable);
      service.register(notRenderable);

      const formats = service.getRenderableFormats();

      expect(formats).toContain('renderable');
      expect(formats).not.toContain('not-renderable');
    });
  });

  describe('hasSpecializedEditor', () => {
    it('should return true when plugin has custom editor', () => {
      const plugin = createMockPlugin({
        canEdit: true,
        getEditorComponent: () => MockEditorComponent as any,
      });
      service.register(plugin);

      expect(service.hasSpecializedEditor('test-format')).toBeTrue();
    });

    it('should return false when plugin uses default editor', () => {
      const plugin = createMockPlugin({
        canEdit: false,
        getEditorComponent: () => null,
      });
      service.register(plugin);

      expect(service.hasSpecializedEditor('test-format')).toBeFalse();
    });
  });

  describe('getStats', () => {
    it('should return registry statistics', () => {
      const plugin1 = createMockPlugin({
        formatId: 'format1',
        canRender: true,
        canEdit: true,
        canImport: true,
        canExport: true,
      });
      const plugin2 = createMockPlugin({
        formatId: 'format2',
        canRender: false,
        canEdit: false,
        canImport: true,
        canExport: false,
      });
      service.register(plugin1);
      service.register(plugin2);
      service.registerDefaultEditor(MockEditorComponent as any);

      const stats = service.getStats();

      expect(stats.totalPlugins).toBe(2);
      expect(stats.renderableFormats).toBe(1);
      expect(stats.editableFormats).toBe(1);
      expect(stats.importableFormats).toBe(2);
      expect(stats.exportableFormats).toBe(1);
      expect(stats.hasDefaultEditor).toBeTrue();
    });
  });
});
