import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { SimpleChange, ChangeDetectorRef } from '@angular/core';
import { IframeRendererComponent, Html5AppContent } from './iframe-renderer.component';
import { ContentNode } from '../../models/content-node.model';
import { DomSanitizer } from '@angular/platform-browser';

describe('IframeRendererComponent', () => {
  let component: IframeRendererComponent;
  let fixture: ComponentFixture<IframeRendererComponent>;

  const createContentNode = (url: string, metadata: Record<string, unknown> = {}): ContentNode => ({
    id: 'test-iframe',
    title: 'Test Iframe',
    description: 'Test iframe content',
    contentType: 'video',
    contentFormat: 'video-embed',
    content: url,
    tags: ['iframe'],
    relatedNodeIds: [],
    metadata,
  });

  const createHtml5AppNode = (
    content: Html5AppContent,
    metadata: Record<string, unknown> = {}
  ): ContentNode => ({
    id: 'test-html5-app',
    title: 'Test HTML5 App',
    description: 'Test HTML5 app content',
    contentType: 'simulation',
    contentFormat: 'html5-app',
    content,
    tags: ['html5-app', 'interactive'],
    relatedNodeIds: [],
    metadata,
  });

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [IframeRendererComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(IframeRendererComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('URL handling', () => {
    it('should set safe URL from content', () => {
      component.node = createContentNode('https://example.com/embed');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.safeUrl).toBeTruthy();
    });

    it('should handle empty URL with error message', () => {
      component.node = createContentNode('');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      // Empty URL results in error, not a safeUrl
      expect(component.safeUrl).toBeNull();
      expect(component.errorMessage).toBe('No content URL available');
      expect(component.loading).toBeFalse();
    });

    it('should handle non-string content with error message', () => {
      const node = createContentNode('');
      (node as any).content = { invalid: 'object' };
      component.node = node;
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      // Invalid content (not Html5AppContent or URL string) results in error
      expect(component.safeUrl).toBeNull();
      expect(component.errorMessage).toBe('No content URL available');
    });
  });

  describe('sandbox attribute', () => {
    it('should have static sandbox attribute on iframe', fakeAsync(() => {
      // Set up component with a valid URL
      const sanitizer = TestBed.inject(DomSanitizer);
      component.safeUrl = sanitizer.bypassSecurityTrustResourceUrl('about:blank');
      component.loading = false;
      fixture.detectChanges();
      tick();

      const iframe = fixture.nativeElement.querySelector('iframe');
      // Static sandbox policy for security (Angular doesn't allow dynamic sandbox)
      expect(iframe.getAttribute('sandbox')).toBe(
        'allow-scripts allow-same-origin allow-forms allow-popups'
      );
    }));
  });

  describe('template rendering', () => {
    let sanitizer: DomSanitizer;

    beforeEach(() => {
      sanitizer = TestBed.inject(DomSanitizer);
      // Set component to stable rendering state
      component.loading = false;
      component.safeUrl = sanitizer.bypassSecurityTrustResourceUrl('about:blank');
    });

    it('should render iframe element', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const iframe = fixture.nativeElement.querySelector('iframe');
      expect(iframe).toBeTruthy();
    }));

    it('should have iframe-container class', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const container = fixture.nativeElement.querySelector('.iframe-container');
      expect(container).toBeTruthy();
    }));

    it('should have iframe-content class on iframe', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const iframe = fixture.nativeElement.querySelector('iframe.iframe-content');
      expect(iframe).toBeTruthy();
    }));

    it('should have allowfullscreen attribute', fakeAsync(() => {
      fixture.detectChanges();
      tick();

      const iframe = fixture.nativeElement.querySelector('iframe');
      expect(iframe.hasAttribute('allowfullscreen')).toBeTrue();
    }));
  });

  describe('ngOnChanges', () => {
    it('should not configure if node is not changed', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({});

      // safeUrl should remain null if ngOnChanges wasn't properly triggered
      expect(component.safeUrl).toBeNull();
    });

    it('should reconfigure when node changes', () => {
      component.node = createContentNode('https://first.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      const firstUrl = component.safeUrl;

      component.node = createContentNode('https://second.com');
      component.ngOnChanges({
        node: new SimpleChange(createContentNode('https://first.com'), component.node, false),
      });

      expect(component.safeUrl).not.toBe(firstUrl);
    });
  });

  describe('YouTube embed URLs', () => {
    it('should handle YouTube embed URLs', () => {
      component.node = createContentNode('https://www.youtube.com/embed/dQw4w9WgXcQ');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.safeUrl).toBeTruthy();
    });
  });

  describe('Vimeo embed URLs', () => {
    it('should handle Vimeo embed URLs', () => {
      component.node = createContentNode('https://player.vimeo.com/video/123456789');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.safeUrl).toBeTruthy();
    });
  });

  describe('security', () => {
    it('should bypass security for trusted URLs', () => {
      component.node = createContentNode('https://trusted-domain.com/embed');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      // The safeUrl should be a SafeResourceUrl type, not a plain string
      expect(component.safeUrl).toBeTruthy();
      expect(typeof component.safeUrl).not.toBe('string');
    });
  });

  describe('HTML5 App mode', () => {
    it('should handle Html5AppContent structure', () => {
      component.node = createHtml5AppNode({
        appId: 'evolution-of-trust',
        entryPoint: 'index.html',
      });
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.safeUrl).toBeTruthy();
    });

    it('should set fallback URL when provided', () => {
      component.node = createHtml5AppNode({
        appId: 'evolution-of-trust',
        entryPoint: 'index.html',
        fallbackUrl: 'https://ncase.me/trust/',
      });
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.fallbackUrl).toBe('https://ncase.me/trust/');
    });

    it('should use static sandbox policy (Angular security restriction)', fakeAsync(() => {
      // Note: Angular doesn't allow dynamic sandbox bindings, so we use a static policy
      // that's permissive enough for HTML5 apps
      // Set component to stable state to test DOM structure
      const sanitizer = TestBed.inject(DomSanitizer);
      component.loading = false;
      component.safeUrl = sanitizer.bypassSecurityTrustResourceUrl('about:blank');
      fixture.detectChanges();
      tick();

      const iframe = fixture.nativeElement.querySelector('iframe');
      expect(iframe.getAttribute('sandbox')).toBe(
        'allow-scripts allow-same-origin allow-forms allow-popups'
      );
    }));
  });

  describe('loading state', () => {
    it('should start in loading state', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      expect(component.loading).toBeTrue();
    });

    it('should exit loading state on iframe load', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      component.onIframeLoad();

      expect(component.loading).toBeFalse();
    });

    it('should reset loading state when node changes', () => {
      component.node = createContentNode('https://first.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      component.onIframeLoad();
      expect(component.loading).toBeFalse();

      component.node = createContentNode('https://second.com');
      component.ngOnChanges({
        node: new SimpleChange(createContentNode('https://first.com'), component.node, false),
      });

      expect(component.loading).toBeTrue();
    });
  });

  describe('error state', () => {
    it('should set error message on iframe error', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });

      component.onIframeError();

      expect(component.errorMessage).toBe('Failed to load application');
      expect(component.loading).toBeFalse();
    });

    it('should clear error message when node changes', () => {
      component.node = createContentNode('https://first.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      component.onIframeError();
      expect(component.errorMessage).toBeTruthy();

      component.node = createContentNode('https://second.com');
      component.ngOnChanges({
        node: new SimpleChange(createContentNode('https://first.com'), component.node, false),
      });

      expect(component.errorMessage).toBeNull();
    });
  });
});
