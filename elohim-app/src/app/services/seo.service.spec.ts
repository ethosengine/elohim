import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { Title, Meta } from '@angular/platform-browser';
import { Router, ActivatedRoute, NavigationEnd } from '@angular/router';
import { DOCUMENT } from '@angular/common';
import { Subject, of } from 'rxjs';
import { SeoService, SeoConfig } from './seo.service';

describe('SeoService', () => {
  let service: SeoService;
  let titleService: jasmine.SpyObj<Title>;
  let metaService: jasmine.SpyObj<Meta>;
  let mockDocument: Document;
  let routerEventsSubject: Subject<any>;
  let mockActivatedRoute: any;

  beforeEach(() => {
    const titleSpy = jasmine.createSpyObj('Title', ['setTitle', 'getTitle']);
    const metaSpy = jasmine.createSpyObj('Meta', ['updateTag', 'addTag', 'removeTag']);

    routerEventsSubject = new Subject();

    mockActivatedRoute = {
      firstChild: null,
      outlet: 'primary',
      data: of({})
    };

    // Create a mock document
    mockDocument = document.implementation.createHTMLDocument('test');

    TestBed.configureTestingModule({
      providers: [
        SeoService,
        { provide: Title, useValue: titleSpy },
        { provide: Meta, useValue: metaSpy },
        {
          provide: Router,
          useValue: {
            events: routerEventsSubject.asObservable(),
            url: '/test-path'
          }
        },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
        { provide: DOCUMENT, useValue: mockDocument }
      ]
    });

    titleService = TestBed.inject(Title) as jasmine.SpyObj<Title>;
    metaService = TestBed.inject(Meta) as jasmine.SpyObj<Meta>;

    service = TestBed.inject(SeoService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // setTitle
  // =========================================================================

  describe('setTitle', () => {
    it('should set title with site name suffix', () => {
      service.setTitle('Test Page');
      expect(titleService.setTitle).toHaveBeenCalledWith('Test Page | Elohim Protocol');
    });

    it('should use only site name when title is empty', () => {
      service.setTitle('');
      expect(titleService.setTitle).toHaveBeenCalledWith('Elohim Protocol');
    });
  });

  // =========================================================================
  // updateMetaDescription
  // =========================================================================

  describe('updateMetaDescription', () => {
    it('should update meta description tag', () => {
      service.updateMetaDescription('Test description');
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'description',
        content: 'Test description'
      });
    });
  });

  // =========================================================================
  // updateMetaKeywords
  // =========================================================================

  describe('updateMetaKeywords', () => {
    it('should join keywords with commas', () => {
      service.updateMetaKeywords(['keyword1', 'keyword2', 'keyword3']);
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'keywords',
        content: 'keyword1, keyword2, keyword3'
      });
    });
  });

  // =========================================================================
  // updateOpenGraphTags
  // =========================================================================

  describe('updateOpenGraphTags', () => {
    it('should update basic OG tags', () => {
      service.updateOpenGraphTags({
        ogTitle: 'Test Title',
        ogDescription: 'Test Description',
        ogUrl: 'https://example.com/page'
      });

      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:title',
        content: 'Test Title'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:description',
        content: 'Test Description'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:url',
        content: 'https://example.com/page'
      });
    });

    it('should use default image when not provided', () => {
      service.updateOpenGraphTags({});
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:image',
        content: 'https://elohim.host/images/elohim_logo_light.png'
      });
    });

    it('should set custom image when provided', () => {
      service.updateOpenGraphTags({
        ogImage: 'https://example.com/image.png'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:image',
        content: 'https://example.com/image.png'
      });
    });

    it('should set og:type to website by default', () => {
      service.updateOpenGraphTags({});
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:type',
        content: 'website'
      });
    });

    it('should update site name', () => {
      service.updateOpenGraphTags({});
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:site_name',
        content: 'Elohim Protocol'
      });
    });

    it('should handle article type with timestamps', () => {
      service.updateOpenGraphTags({
        ogType: 'article',
        articlePublishedTime: '2025-01-01T00:00:00.000Z',
        articleModifiedTime: '2025-01-02T00:00:00.000Z',
        articleSection: 'Learning'
      });

      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:type',
        content: 'article'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'article:published_time',
        content: '2025-01-01T00:00:00.000Z'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'article:modified_time',
        content: '2025-01-02T00:00:00.000Z'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'article:section',
        content: 'Learning'
      });
    });

    it('should handle profile type', () => {
      service.updateOpenGraphTags({
        ogType: 'profile',
        profileUsername: 'testuser',
        profileFirstName: 'Test',
        profileLastName: 'User'
      });

      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'profile:username',
        content: 'testuser'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'profile:first_name',
        content: 'Test'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'profile:last_name',
        content: 'User'
      });
    });
  });

  // =========================================================================
  // updateJsonLd
  // =========================================================================

  describe('updateJsonLd', () => {
    it('should add JSON-LD script to head', () => {
      const jsonLd = {
        '@context': 'https://schema.org',
        '@type': 'Article',
        name: 'Test Article'
      };

      service.updateJsonLd(jsonLd);

      const script = mockDocument.getElementById('seo-json-ld');
      expect(script).toBeTruthy();
      expect(script?.getAttribute('type')).toBe('application/ld+json');
      expect(script?.textContent).toContain('Test Article');
    });

    it('should replace existing JSON-LD script', () => {
      service.updateJsonLd({ '@context': 'https://schema.org', '@type': 'Article', name: 'First' });
      service.updateJsonLd({ '@context': 'https://schema.org', '@type': 'Article', name: 'Second' });

      const scripts = mockDocument.querySelectorAll('#seo-json-ld');
      expect(scripts.length).toBe(1);
      expect(scripts[0].textContent).toContain('Second');
    });
  });

  describe('removeJsonLd', () => {
    it('should remove JSON-LD script', () => {
      service.updateJsonLd({ '@context': 'https://schema.org', '@type': 'Article', name: 'Test' });
      expect(mockDocument.getElementById('seo-json-ld')).toBeTruthy();

      service.removeJsonLd();
      expect(mockDocument.getElementById('seo-json-ld')).toBeNull();
    });
  });

  // =========================================================================
  // updateCanonicalUrl
  // =========================================================================

  describe('updateCanonicalUrl', () => {
    it('should create canonical link if not exists', () => {
      service.updateCanonicalUrl('https://example.com/page');

      const link = mockDocument.querySelector('link[rel="canonical"]') as HTMLLinkElement;
      expect(link).toBeTruthy();
      expect(link.href).toBe('https://example.com/page');
    });

    it('should update existing canonical link', () => {
      service.updateCanonicalUrl('https://example.com/first');
      service.updateCanonicalUrl('https://example.com/second');

      const links = mockDocument.querySelectorAll('link[rel="canonical"]');
      expect(links.length).toBe(1);
      expect((links[0] as HTMLLinkElement).href).toBe('https://example.com/second');
    });
  });

  // =========================================================================
  // noIndex
  // =========================================================================

  describe('setNoIndex', () => {
    it('should add noindex meta tag', () => {
      service.setNoIndex();
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'robots',
        content: 'noindex, nofollow'
      });
    });
  });

  describe('removeNoIndex', () => {
    it('should remove noindex meta tag', () => {
      service.removeNoIndex();
      expect(metaService.removeTag).toHaveBeenCalledWith('name="robots"');
    });
  });

  // =========================================================================
  // updateSeo (comprehensive)
  // =========================================================================

  describe('updateSeo', () => {
    it('should update all SEO components', () => {
      const config: SeoConfig = {
        title: 'Test Page',
        description: 'Test description',
        keywords: ['key1', 'key2'],
        canonicalUrl: 'https://example.com/test',
        openGraph: {
          ogType: 'article'
        }
      };

      service.updateSeo(config);

      expect(titleService.setTitle).toHaveBeenCalledWith('Test Page | Elohim Protocol');
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'description',
        content: 'Test description'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'keywords',
        content: 'key1, key2'
      });
    });

    it('should handle noIndex flag', () => {
      service.updateSeo({
        title: 'Private Page',
        description: 'Private content',
        noIndex: true
      });

      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'robots',
        content: 'noindex, nofollow'
      });
    });

    it('should remove noIndex when not set', () => {
      service.updateSeo({
        title: 'Public Page',
        description: 'Public content',
        noIndex: false
      });

      expect(metaService.removeTag).toHaveBeenCalledWith('name="robots"');
    });

    it('should update JSON-LD when provided', () => {
      const jsonLd = {
        '@context': 'https://schema.org',
        '@type': 'Article',
        name: 'Test'
      };

      service.updateSeo({
        title: 'Test',
        description: 'Test',
        jsonLd
      });

      const script = mockDocument.getElementById('seo-json-ld');
      expect(script).toBeTruthy();
    });

    it('should remove JSON-LD when not provided', () => {
      service.updateJsonLd({ '@context': 'https://schema.org', '@type': 'Article', name: 'Test' });

      service.updateSeo({
        title: 'Test',
        description: 'Test'
        // No jsonLd
      });

      expect(mockDocument.getElementById('seo-json-ld')).toBeNull();
    });
  });

  // =========================================================================
  // resetToDefaults
  // =========================================================================

  describe('resetToDefaults', () => {
    it('should reset all SEO to default values', () => {
      service.resetToDefaults();

      expect(titleService.setTitle).toHaveBeenCalledWith('Elohim Protocol');
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'description',
        content: 'Digital guardians for human flourishing. Technology organized around love.'
      });
    });
  });

  // =========================================================================
  // Content-specific methods
  // =========================================================================

  describe('updateForPath', () => {
    it('should set SEO for learning path', () => {
      service.updateForPath({
        id: 'path-123',
        title: 'Test Learning Path',
        description: 'A great learning path'
      });

      expect(titleService.setTitle).toHaveBeenCalledWith('Test Learning Path | Elohim Protocol');
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'description',
        content: 'A great learning path'
      });

      // Check JSON-LD
      const script = mockDocument.getElementById('seo-json-ld');
      expect(script?.textContent).toContain('Course');
      expect(script?.textContent).toContain('Test Learning Path');
    });

    it('should include difficulty and duration in JSON-LD', () => {
      service.updateForPath({
        id: 'path-123',
        title: 'Test Path',
        description: 'Description',
        difficulty: 'beginner',
        estimatedDuration: 'PT2H'
      });

      const script = mockDocument.getElementById('seo-json-ld');
      expect(script?.textContent).toContain('beginner');
      expect(script?.textContent).toContain('PT2H');
    });
  });

  describe('updateForContent', () => {
    it('should set SEO for content resource', () => {
      service.updateForContent({
        id: 'content-123',
        title: 'Test Content',
        contentType: 'concept',
        summary: 'A test concept'
      });

      expect(titleService.setTitle).toHaveBeenCalledWith('Test Content | Elohim Protocol');
      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'description',
        content: 'A test concept'
      });
    });

    it('should use default description when no summary', () => {
      service.updateForContent({
        id: 'content-123',
        title: 'Test Content',
        contentType: 'feature'
      });

      expect(metaService.updateTag).toHaveBeenCalledWith({
        name: 'description',
        content: 'Test Content - feature content'
      });
    });

    it('should include timestamps in OG tags', () => {
      service.updateForContent({
        id: 'content-123',
        title: 'Test',
        contentType: 'epic',
        createdAt: '2025-01-01T00:00:00.000Z',
        updatedAt: '2025-01-02T00:00:00.000Z'
      });

      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'article:published_time',
        content: '2025-01-01T00:00:00.000Z'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'article:modified_time',
        content: '2025-01-02T00:00:00.000Z'
      });
    });
  });

  describe('updateForProfile', () => {
    it('should set SEO for profile page', () => {
      service.updateForProfile({
        username: 'testuser',
        displayName: 'Test User',
        bio: 'A great learner'
      });

      expect(titleService.setTitle).toHaveBeenCalledWith('Test User | Elohim Protocol');
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'og:type',
        content: 'profile'
      });
      expect(metaService.updateTag).toHaveBeenCalledWith({
        property: 'profile:username',
        content: 'testuser'
      });
    });

    it('should use username as title when no displayName', () => {
      service.updateForProfile({
        username: 'testuser'
      });

      expect(titleService.setTitle).toHaveBeenCalledWith('testuser | Elohim Protocol');
    });
  });

  // =========================================================================
  // Route listener
  // =========================================================================

  describe('route listener', () => {
    it('should update SEO from route data', fakeAsync(() => {
      mockActivatedRoute.data = of({
        seo: {
          title: 'Route Title',
          description: 'Route description'
        }
      });

      routerEventsSubject.next(new NavigationEnd(1, '/test', '/test'));
      tick();

      expect(titleService.setTitle).toHaveBeenCalledWith('Route Title | Elohim Protocol');
    }));

    it('should use simple title from route data', fakeAsync(() => {
      mockActivatedRoute.data = of({
        title: 'Simple Title'
      });

      routerEventsSubject.next(new NavigationEnd(1, '/test', '/test'));
      tick();

      expect(titleService.setTitle).toHaveBeenCalledWith('Simple Title | Elohim Protocol');
    }));
  });
});
