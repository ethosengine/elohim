import { ComponentFixture, TestBed } from '@angular/core/testing';
import { SimpleChange } from '@angular/core';
import { IframeRendererComponent } from './iframe-renderer.component';
import { ContentNode } from '../../models/content-node.model';

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
    metadata
  });

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [IframeRendererComponent]
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
        node: new SimpleChange(null, component.node, true)
      });

      expect(component.safeUrl).toBeTruthy();
    });

    it('should handle empty URL', () => {
      component.node = createContentNode('');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });

      expect(component.safeUrl).toBeTruthy();
    });

    it('should handle non-string content', () => {
      const node = createContentNode('');
      (node as any).content = { invalid: 'object' };
      component.node = node;
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });

      expect(component.safeUrl).toBeTruthy();
    });
  });

  describe('sandbox attribute', () => {
    it('should have static sandbox attribute on iframe', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });
      fixture.detectChanges();

      const iframe = fixture.nativeElement.querySelector('iframe');
      expect(iframe.getAttribute('sandbox')).toBe('allow-scripts allow-same-origin');
    });
  });

  describe('template rendering', () => {
    it('should render iframe element', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });
      fixture.detectChanges();

      const iframe = fixture.nativeElement.querySelector('iframe');
      expect(iframe).toBeTruthy();
    });

    it('should have iframe-container class', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });
      fixture.detectChanges();

      const container = fixture.nativeElement.querySelector('.iframe-container');
      expect(container).toBeTruthy();
    });

    it('should have iframe-content class on iframe', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });
      fixture.detectChanges();

      const iframe = fixture.nativeElement.querySelector('iframe.iframe-content');
      expect(iframe).toBeTruthy();
    });

    it('should have allowfullscreen attribute', () => {
      component.node = createContentNode('https://example.com');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });
      fixture.detectChanges();

      const iframe = fixture.nativeElement.querySelector('iframe');
      expect(iframe.hasAttribute('allowfullscreen')).toBeTrue();
    });
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
        node: new SimpleChange(null, component.node, true)
      });

      const firstUrl = component.safeUrl;

      component.node = createContentNode('https://second.com');
      component.ngOnChanges({
        node: new SimpleChange(createContentNode('https://first.com'), component.node, false)
      });

      expect(component.safeUrl).not.toBe(firstUrl);
    });
  });

  describe('YouTube embed URLs', () => {
    it('should handle YouTube embed URLs', () => {
      component.node = createContentNode('https://www.youtube.com/embed/dQw4w9WgXcQ');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });

      expect(component.safeUrl).toBeTruthy();
    });
  });

  describe('Vimeo embed URLs', () => {
    it('should handle Vimeo embed URLs', () => {
      component.node = createContentNode('https://player.vimeo.com/video/123456789');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });

      expect(component.safeUrl).toBeTruthy();
    });
  });

  describe('security', () => {
    it('should bypass security for trusted URLs', () => {
      component.node = createContentNode('https://trusted-domain.com/embed');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true)
      });

      // The safeUrl should be a SafeResourceUrl type, not a plain string
      expect(component.safeUrl).toBeTruthy();
      expect(typeof component.safeUrl).not.toBe('string');
    });
  });
});
