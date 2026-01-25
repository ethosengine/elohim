import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { SimpleChange } from '@angular/core';
import { MarkdownRendererComponent, TocEntry } from './markdown-renderer.component';
import { ContentNode } from '../../models/content-node.model';
import { StorageClientService } from '@app/elohim/services/storage-client.service';

describe('MarkdownRendererComponent', () => {
  let component: MarkdownRendererComponent;
  let fixture: ComponentFixture<MarkdownRendererComponent>;

  // Mock StorageClientService
  const mockStorageClientService = {
    getBlobUrl: (hash: string) => `https://test-doorway.example.com/api/blob/${hash}`,
  };

  const createContentNode = (content: string): ContentNode => ({
    id: 'test-markdown',
    title: 'Test Markdown',
    description: 'Test markdown content',
    contentType: 'concept',
    contentFormat: 'markdown',
    content,
    tags: ['markdown'],
    relatedNodeIds: [],
    metadata: {},
  });

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [MarkdownRendererComponent],
      providers: [{ provide: StorageClientService, useValue: mockStorageClientService }],
    }).compileComponents();

    fixture = TestBed.createComponent(MarkdownRendererComponent);
    component = fixture.componentInstance;
  });

  afterEach(() => {
    // Clean up any scroll listeners
    if (component.ngOnDestroy) {
      component.ngOnDestroy();
    }
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('markdown rendering', () => {
    it('should render basic markdown', fakeAsync(() => {
      component.node = createContentNode('# Hello World\n\nThis is a test.');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.renderedContent).toBeTruthy();
    }));

    it('should render markdown with multiple headings', fakeAsync(() => {
      const markdown = `# Heading 1
## Heading 2
### Heading 3`;

      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.tocEntries.length).toBe(3);
    }));

    it('should handle code blocks', fakeAsync(() => {
      const markdown = '```javascript\nconst x = 1;\n```';

      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.renderedContent).toBeTruthy();
    }));

    it('should warn for non-string content', fakeAsync(() => {
      spyOn(console, 'warn');
      const node = createContentNode('');
      (node as any).content = { invalid: 'object' };
      component.node = node;
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(console.warn).toHaveBeenCalledWith('Markdown renderer expects string content');
    }));
  });

  describe('table of contents', () => {
    it('should generate TOC from headings', fakeAsync(() => {
      const markdown = `# First
## Second
### Third`;

      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.tocEntries.length).toBe(3);
      expect(component.tocEntries[0].level).toBe(1);
      expect(component.tocEntries[1].level).toBe(2);
      expect(component.tocEntries[2].level).toBe(3);
    }));

    it('should emit tocGenerated event', fakeAsync(() => {
      let emittedToc: TocEntry[] = [];
      component.tocGenerated.subscribe((toc: TocEntry[]) => {
        emittedToc = toc;
      });

      component.node = createContentNode('# Test');
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(emittedToc.length).toBe(1);
    }));

    it('should generate unique IDs for duplicate headings', fakeAsync(() => {
      const markdown = `# Test
# Test
# Test`;

      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      const ids = component.tocEntries.map(e => e.id);
      const uniqueIds = new Set(ids);
      expect(uniqueIds.size).toBe(3);
    }));

    it('should toggle TOC visibility', () => {
      expect(component.tocVisible).toBeFalse();

      component.toggleToc();
      expect(component.tocVisible).toBeTrue();

      component.toggleToc();
      expect(component.tocVisible).toBeFalse();
    });
  });

  describe('scroll behavior', () => {
    it('should scroll to heading when link clicked', fakeAsync(() => {
      const markdown = '# Test Heading';
      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      // Create a mock element
      const mockElement = document.createElement('div');
      mockElement.id = component.tocEntries[0].id;
      document.body.appendChild(mockElement);
      spyOn(mockElement, 'scrollIntoView');

      const event = new Event('click');
      spyOn(event, 'preventDefault');

      component.scrollToHeading(event, component.tocEntries[0].id);

      expect(event.preventDefault).toHaveBeenCalled();
      expect(mockElement.scrollIntoView).toHaveBeenCalledWith({
        behavior: 'smooth',
        block: 'start',
      });
      expect(component.activeHeadingId).toBe(component.tocEntries[0].id);

      document.body.removeChild(mockElement);
    }));

    it('should close TOC on mobile after clicking link', fakeAsync(() => {
      const markdown = '# Test';
      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      // Mock mobile viewport
      spyOnProperty(window, 'innerWidth').and.returnValue(500);

      // Create mock element
      const mockElement = document.createElement('div');
      mockElement.id = component.tocEntries[0].id;
      document.body.appendChild(mockElement);

      component.tocVisible = true;
      const event = new Event('click');
      component.scrollToHeading(event, component.tocEntries[0].id);

      expect(component.tocVisible).toBeFalse();

      document.body.removeChild(mockElement);
    }));

    it('should scroll to top when scrollToTop called', () => {
      spyOn(window, 'scrollTo');
      component.scrollToTop();
      expect(window.scrollTo).toHaveBeenCalled();
    });
  });

  describe('embedded mode', () => {
    it('should apply embedded class when embedded is true', fakeAsync(() => {
      component.embedded = true;
      component.node = createContentNode('# Test');
      fixture.detectChanges();
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();
      fixture.detectChanges();

      const wrapper = fixture.nativeElement.querySelector('.markdown-wrapper');
      expect(wrapper.classList.contains('embedded')).toBeTrue();
    }));
  });

  describe('lifecycle', () => {
    it('should set up scroll listener after view init', () => {
      spyOn(window, 'addEventListener');
      component.ngAfterViewInit();
      expect(window.addEventListener).toHaveBeenCalledWith('scroll', jasmine.any(Function), {
        passive: true,
      });
    });

    it('should remove scroll listener on destroy', () => {
      spyOn(window, 'removeEventListener');
      component.ngAfterViewInit();
      component.ngOnDestroy();
      expect(window.removeEventListener).toHaveBeenCalled();
    });
  });

  describe('ID generation', () => {
    it('should generate lowercase IDs', fakeAsync(() => {
      const markdown = '# UPPERCASE HEADING';
      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.tocEntries[0].id).toBe(component.tocEntries[0].id.toLowerCase());
    }));

    it('should replace spaces with hyphens', fakeAsync(() => {
      const markdown = '# Test Heading Here';
      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.tocEntries[0].id).toContain('-');
      expect(component.tocEntries[0].id).not.toContain(' ');
    }));

    it('should remove special characters', fakeAsync(() => {
      const markdown = '# Test! @Heading#';
      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.tocEntries[0].id).not.toContain('!');
      expect(component.tocEntries[0].id).not.toContain('@');
      expect(component.tocEntries[0].id).not.toContain('#');
    }));

    it('should truncate long IDs', fakeAsync(() => {
      const longHeading = '# ' + 'A'.repeat(100);
      component.node = createContentNode(longHeading);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.tocEntries[0].id.length).toBeLessThanOrEqual(50);
    }));
  });

  describe('GFM support', () => {
    it('should render task lists', fakeAsync(() => {
      const markdown = `- [ ] Task 1
- [x] Task 2`;

      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.renderedContent).toBeTruthy();
    }));

    it('should render tables', fakeAsync(() => {
      const markdown = `| Header 1 | Header 2 |
| --- | --- |
| Cell 1 | Cell 2 |`;

      component.node = createContentNode(markdown);
      component.ngOnChanges({
        node: new SimpleChange(null, component.node, true),
      });
      tick();

      expect(component.renderedContent).toBeTruthy();
    }));
  });

  describe('back to top button', () => {
    it('should initially hide back to top button', () => {
      expect(component.showBackToTop).toBeFalse();
    });

    it('should show back to top button after scrolling', fakeAsync(() => {
      component.ngAfterViewInit();
      tick();

      // Simulate scroll event
      Object.defineProperty(window, 'scrollY', { value: 400, writable: true });
      window.dispatchEvent(new Event('scroll'));
      tick();

      expect(component.showBackToTop).toBeTrue();
    }));
  });
});
