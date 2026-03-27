import { TestBed } from '@angular/core/testing';
import { RendererInitializerService } from './renderer-initializer.service';
import { RendererRegistryService } from './renderer-registry.service';

describe('RendererInitializerService', () => {
  let service: RendererInitializerService;
  let registry: RendererRegistryService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [RendererInitializerService],
    });
    service = TestBed.inject(RendererInitializerService);
    registry = TestBed.inject(RendererRegistryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should register renderers from manifest on construction', () => {
    expect(registry.isInitialized()).toBe(true);
  });

  it('should register markdown format', () => {
    expect(registry.canRender('markdown')).toBe(true);
  });

  it('should register sophia-quiz-json format', () => {
    expect(registry.canRender('sophia-quiz-json')).toBe(true);
  });

  it('should register sophia alias', () => {
    expect(registry.canRender('sophia')).toBe(true);
  });

  it('should register perseus-quiz-json alias', () => {
    expect(registry.canRender('perseus-quiz-json')).toBe(true);
  });

  it('should register html5-app format', () => {
    expect(registry.canRender('html5-app')).toBe(true);
  });

  it('should register gherkin format', () => {
    expect(registry.canRender('gherkin')).toBe(true);
  });

  it('should register html format (via markdown renderer)', () => {
    expect(registry.canRender('html')).toBe(true);
  });

  it('should register plaintext format (via markdown renderer)', () => {
    expect(registry.canRender('plaintext')).toBe(true);
  });

  it('should register video-embed format', () => {
    expect(registry.canRender('video-embed')).toBe(true);
  });
});
