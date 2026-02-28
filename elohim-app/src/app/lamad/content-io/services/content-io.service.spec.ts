import { TestBed } from '@angular/core/testing';
import { ContentIOService } from './content-io.service';
import { ContentFormatRegistryService } from './content-format-registry.service';
import { ContentFormatPlugin } from '../interfaces/content-format-plugin.interface';
import { FormatMetadata } from '../interfaces/format-metadata.interface';
import { vi, Mock } from 'vitest';

describe('ContentIOService', () => {
  let service: ContentIOService;
  let registrySpy: any;

  const mockPlugin: Partial<ContentFormatPlugin> = {
    formatId: 'markdown',
    displayName: 'Markdown',
    fileExtensions: ['.md'],
    mimeTypes: ['text/markdown'],
    canImport: true,
    canExport: true,
    canValidate: true,
    import: vi.fn().mockReturnValue(
      Promise.resolve({
        nodes: [{ id: 'test', title: 'Test', content: '# Test' }],
        warnings: [],
      })
    ),
    export: vi.fn().mockReturnValue(Promise.resolve('# Exported')),
    validate: vi.fn().mockReturnValue(
      Promise.resolve({
        valid: true,
        errors: [],
        warnings: [],
      })
    ),
    getFormatMetadata: () => ({
      formatId: 'markdown',
      displayName: 'Markdown',
      description: 'Markdown format',
      fileExtensions: ['.md'],
      mimeTypes: ['text/markdown'],
      canImport: true,
      canExport: true,
      canValidate: true,
      category: 'document',
      supportsRoundTrip: true,
    }),
  };

  const mockPluginNoImport: Partial<ContentFormatPlugin> = {
    formatId: 'readonly',
    displayName: 'Read Only',
    fileExtensions: ['.ro'],
    mimeTypes: ['text/plain'],
    canImport: false,
    canExport: true,
    canValidate: false,
    import: vi.fn()
      .mockImplementation(() => Promise.reject(new Error('Not supported'))),
    export: vi.fn().mockReturnValue(Promise.resolve('exported')),
    validate: vi.fn()
      .mockReturnValue(Promise.resolve({ valid: true, errors: [], warnings: [] })),
    getFormatMetadata: () => ({
      formatId: 'readonly',
      displayName: 'Read Only',
      description: 'Read only format',
      fileExtensions: ['.ro'],
      mimeTypes: ['text/plain'],
      canImport: false,
      canExport: true,
      canValidate: false,
      category: 'document',
      supportsRoundTrip: false,
    }),
  };

  beforeEach(() => {
    const registrySpyObj = {
    detectFormat: vi.fn(),
    detectFormatFromContent: vi.fn(),
    getPlugin: vi.fn(),
    getImportableFormats: vi.fn(),
    getExportableFormats: vi.fn(),
    getExportableFormatsForContent: vi.fn(),
  };

    TestBed.configureTestingModule({
      providers: [
        ContentIOService,
        { provide: ContentFormatRegistryService, useValue: registrySpyObj },
      ],
    });

    service = TestBed.inject(ContentIOService);
    registrySpy = TestBed.inject(
      ContentFormatRegistryService
    ) as { [K in keyof ContentFormatRegistryService]?: Mock };

    // Default spy returns
    registrySpy.getPlugin.mockReturnValue(mockPlugin as ContentFormatPlugin);
    registrySpy.detectFormat.mockReturnValue(Promise.resolve('markdown'));
    registrySpy.detectFormatFromContent.mockReturnValue('markdown');
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('importFile', () => {
    it('should import a file with auto-detected format', async () => {
      const file = new File(['# Test'], 'test.md', { type: 'text/markdown' });

      const result = await service.importFile(file);

      expect(registrySpy.detectFormat).toHaveBeenCalledWith(file);
      expect(result).toBeTruthy();
    });

    it('should throw error if format cannot be detected', async () => {
      registrySpy.detectFormat.mockReturnValue(Promise.resolve(null));
      const file = new File(['unknown'], 'test.xyz', { type: 'application/octet-stream' });

      await await expect(service.importFile(file)).rejects.toThrow('Cannot detect format for file: test.xyz');
    });
  });

  describe('importFileAs', () => {
    it('should import file with specified format', async () => {
      const file = new File(['# Test'], 'test.md', { type: 'text/markdown' });

      const result = await service.importFileAs(file, 'markdown');

      expect(registrySpy.getPlugin).toHaveBeenCalledWith('markdown');
      expect(mockPlugin.import).toHaveBeenCalledWith(file);
      expect(result).toBeTruthy();
    });

    it('should throw error if plugin not found', async () => {
      registrySpy.getPlugin.mockReturnValue(undefined);
      const file = new File(['content'], 'test.xyz');

      await await expect(service.importFileAs(file, 'unknown')).rejects.toThrow('No plugin found for format: unknown');
    });

    it('should throw error if plugin does not support import', async () => {
      registrySpy.getPlugin.mockReturnValue(mockPluginNoImport as ContentFormatPlugin);
      const file = new File(['content'], 'test.ro');

      await await expect(service.importFileAs(file, 'readonly')).rejects.toThrow("Plugin 'readonly' does not support import");
    });
  });

  describe('importString', () => {
    it('should import content string with specified format', async () => {
      const result = await service.importString('# Test', 'markdown');

      expect(mockPlugin.import).toHaveBeenCalledWith('# Test');
      expect(result).toBeTruthy();
    });

    it('should throw error if plugin not found', async () => {
      registrySpy.getPlugin.mockReturnValue(undefined);

      await await expect(service.importString('content', 'unknown')).rejects.toThrow('No plugin found for format: unknown');
    });

    it('should throw error if plugin does not support import', async () => {
      registrySpy.getPlugin.mockReturnValue(mockPluginNoImport as ContentFormatPlugin);

      await await expect(service.importString('content', 'readonly')).rejects.toThrow("Plugin 'readonly' does not support import");
    });
  });

  describe('importStringAutoDetect', () => {
    it('should import string with auto-detected format', async () => {
      const result = await service.importStringAutoDetect('# Test');

      expect(registrySpy.detectFormatFromContent).toHaveBeenCalledWith('# Test');
      expect(result).toBeTruthy();
    });

    it('should throw error if format cannot be detected', async () => {
      registrySpy.detectFormatFromContent.mockReturnValue(null);

      await await expect(service.importStringAutoDetect('unknown')).rejects.toThrow('Cannot detect format from content');
    });
  });

  describe('exportToFormat', () => {
    it('should export node to specified format as Blob', async () => {
      const node = { id: 'test', title: 'Test', content: '# Test', contentFormat: 'markdown' };

      const result = await service.exportToFormat(node, 'markdown');

      expect(result instanceof Blob).toBe(true);
    });

    it('should throw error if plugin not found', async () => {
      registrySpy.getPlugin.mockReturnValue(undefined);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'unknown' };

      await await expect(service.exportToFormat(node, 'unknown')).rejects.toThrow('No plugin found for format: unknown');
    });

    it('should throw error if plugin does not support export', async () => {
      const noExportPlugin = { ...mockPlugin, canExport: false };
      registrySpy.getPlugin.mockReturnValue(noExportPlugin as ContentFormatPlugin);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      await await expect(service.exportToFormat(node, 'markdown')).rejects.toThrow("Plugin 'markdown' does not support export");
    });

    it('should return Blob directly if export returns Blob', async () => {
      const blob = new Blob(['test'], { type: 'text/plain' });
      const blobPlugin = {
        ...mockPlugin,
        export: vi.fn().mockReturnValue(Promise.resolve(blob)),
      };
      registrySpy.getPlugin.mockReturnValue(blobPlugin as ContentFormatPlugin);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const result = await service.exportToFormat(node, 'markdown');

      expect(result).toBe(blob);
    });
  });

  describe('exportToString', () => {
    it('should export node to string', async () => {
      const node = { id: 'test', title: 'Test', content: '# Test', contentFormat: 'markdown' };

      const result = await service.exportToString(node, 'markdown');

      expect(result).toBe('# Exported');
    });

    it('should throw error if plugin not found', async () => {
      registrySpy.getPlugin.mockReturnValue(undefined);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'unknown' };

      await await expect(service.exportToString(node, 'unknown')).rejects.toThrow('No plugin found for format: unknown');
    });
  });

  describe('validateFile', () => {
    it('should validate file with auto-detected format', async () => {
      const file = new File(['# Test'], 'test.md', { type: 'text/markdown' });

      const result = await service.validateFile(file);

      expect(result.valid).toBe(true);
    });

    it('should return error if format cannot be detected', async () => {
      registrySpy.detectFormat.mockReturnValue(Promise.resolve(null));
      const file = new File(['unknown'], 'test.xyz');

      const result = await service.validateFile(file);

      expect(result.valid).toBe(false);
      expect(result.errors[0].code).toBe('UNKNOWN_FORMAT');
    });
  });

  describe('validateFileAs', () => {
    it('should validate file with specified format', async () => {
      const file = new File(['# Test'], 'test.md');

      const result = await service.validateFileAs(file, 'markdown');

      expect(result.valid).toBe(true);
    });

    it('should return error if plugin not found', async () => {
      registrySpy.getPlugin.mockReturnValue(undefined);
      const file = new File(['content'], 'test.xyz');

      const result = await service.validateFileAs(file, 'unknown');

      expect(result.valid).toBe(false);
      expect(result.errors[0].code).toBe('NO_PLUGIN');
    });

    it('should return warning if plugin does not support validation', async () => {
      registrySpy.getPlugin.mockReturnValue(mockPluginNoImport as ContentFormatPlugin);
      const file = new File(['content'], 'test.ro');

      const result = await service.validateFileAs(file, 'readonly');

      expect(result.valid).toBe(true);
      expect(result.warnings[0].code).toBe('NO_VALIDATION');
    });
  });

  describe('validateString', () => {
    it('should validate string content', async () => {
      const result = await service.validateString('# Test', 'markdown');

      expect(result.valid).toBe(true);
    });

    it('should return error if plugin not found', async () => {
      registrySpy.getPlugin.mockReturnValue(undefined);

      const result = await service.validateString('content', 'unknown');

      expect(result.valid).toBe(false);
      expect(result.errors[0].code).toBe('NO_PLUGIN');
    });

    it('should return warning if plugin does not support validation', async () => {
      registrySpy.getPlugin.mockReturnValue(mockPluginNoImport as ContentFormatPlugin);

      const result = await service.validateString('content', 'readonly');

      expect(result.valid).toBe(true);
      expect(result.warnings[0].code).toBe('NO_VALIDATION');
    });
  });

  describe('getImportableFormats', () => {
    it('should delegate to registry', () => {
      const formats: FormatMetadata[] = [];
      registrySpy.getImportableFormats.mockReturnValue(formats);

      const result = service.getImportableFormats();

      expect(registrySpy.getImportableFormats).toHaveBeenCalled();
      expect(result).toBe(formats);
    });
  });

  describe('getExportableFormats', () => {
    it('should delegate to registry', () => {
      const formats: FormatMetadata[] = [];
      registrySpy.getExportableFormats.mockReturnValue(formats);

      const result = service.getExportableFormats();

      expect(registrySpy.getExportableFormats).toHaveBeenCalled();
      expect(result).toBe(formats);
    });
  });

  describe('getExportableFormatsForContent', () => {
    it('should delegate to registry', () => {
      const formats: FormatMetadata[] = [];
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };
      registrySpy.getExportableFormatsForContent.mockReturnValue(formats);

      const result = service.getExportableFormatsForContent(node);

      expect(registrySpy.getExportableFormatsForContent).toHaveBeenCalledWith(node);
      expect(result).toBe(formats);
    });
  });

  describe('canExport', () => {
    it('should return true if plugin supports export', () => {
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const result = service.canExport(node);

      expect(result).toBe(true);
    });

    it('should return false if plugin not found', () => {
      registrySpy.getPlugin.mockReturnValue(undefined);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'unknown' };

      const result = service.canExport(node);

      expect(result).toBe(false);
    });

    it('should return false if plugin does not support export', () => {
      const noExportPlugin = { ...mockPlugin, canExport: false };
      registrySpy.getPlugin.mockReturnValue(noExportPlugin as ContentFormatPlugin);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const result = service.canExport(node);

      expect(result).toBe(false);
    });
  });

  describe('getSourceFormat', () => {
    it('should return format if plugin supports export', () => {
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const result = service.getSourceFormat(node);

      expect(result).toBe('markdown');
    });

    it('should return null if plugin not found', () => {
      registrySpy.getPlugin.mockReturnValue(undefined);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'unknown' };

      const result = service.getSourceFormat(node);

      expect(result).toBeNull();
    });
  });

  describe('getExportedContent', () => {
    it('should return exported content with metadata', async () => {
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const result = await service.getExportedContent(node);

      expect(result).not.toBeNull();
      expect(result!.content).toBe('# Exported');
      expect(result!.filename).toContain('test');
      expect(result!.mimeType).toBe('text/markdown');
    });

    it('should return null if plugin does not support export', async () => {
      const noExportPlugin = { ...mockPlugin, canExport: false };
      registrySpy.getPlugin.mockReturnValue(noExportPlugin as ContentFormatPlugin);
      const node = { id: 'test', title: 'Test', content: 'test', contentFormat: 'markdown' };

      const result = await service.getExportedContent(node);

      expect(result).toBeNull();
    });

    it('should use node id if title is not available', async () => {
      const node = { id: 'test-id', title: '', content: 'test', contentFormat: 'markdown' };

      const result = await service.getExportedContent(node);

      expect(result!.filename).toBeTruthy();
    });
  });
});
