import { TestBed } from '@angular/core/testing';
import { RendererRegistryService } from './renderer-registry.service';
import { ContentFormat } from '../models/content-node.model';

describe('RendererRegistryService', () => {
  let service: RendererRegistryService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [RendererRegistryService]
    });
    service = TestBed.inject(RendererRegistryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getRendererType', () => {
    it('should return markdown renderer for markdown format', () => {
      const renderer = service.getRendererType('markdown');
      expect(renderer).toBeDefined();
    });

    it('should return gherkin renderer for gherkin format', () => {
      const renderer = service.getRendererType('gherkin');
      expect(renderer).toBeDefined();
    });

    it('should return iframe renderer for html5-app format', () => {
      const renderer = service.getRendererType('html5-app');
      expect(renderer).toBeDefined();
    });

    it('should return video renderer for video formats', () => {
      expect(service.getRendererType('video-embed')).toBeDefined();
      expect(service.getRendererType('video-file')).toBeDefined();
    });

    it('should return quiz renderer for quiz-json format', () => {
      const renderer = service.getRendererType('quiz-json');
      expect(renderer).toBeDefined();
    });

    it('should handle all supported formats', () => {
      const formats: ContentFormat[] = [
        'markdown', 'html5-app', 'video-embed', 'video-file',
        'quiz-json', 'external-link', 'epub', 'gherkin', 'html', 'plaintext'
      ];

      formats.forEach(format => {
        const renderer = service.getRendererType(format);
        expect(renderer).toBeDefined(`Expected renderer for ${format}`);
      });
    });
  });

  describe('registerRenderer', () => {
    it('should register custom renderer', () => {
      const customRenderer = { type: 'custom' } as any;
      service.registerRenderer('custom' as ContentFormat, customRenderer);

      const renderer = service.getRendererType('custom' as ContentFormat);
      expect(renderer).toBe(customRenderer);
    });
  });

  describe('getSupportedFormats', () => {
    it('should return list of supported formats', () => {
      const formats = service.getSupportedFormats();
      expect(formats).toBeDefined();
      expect(Array.isArray(formats)).toBe(true);
      expect(formats.length).toBeGreaterThan(0);
    });

    it('should include common formats', () => {
      const formats = service.getSupportedFormats();
      expect(formats).toContain('markdown');
      expect(formats).toContain('gherkin');
    });
  });

  describe('isFormatSupported', () => {
    it('should return true for supported formats', () => {
      expect(service.isFormatSupported('markdown')).toBe(true);
      expect(service.isFormatSupported('gherkin')).toBe(true);
    });

    it('should return false for unsupported formats', () => {
      expect(service.isFormatSupported('unknown-format' as ContentFormat)).toBe(false);
    });
  });
});
