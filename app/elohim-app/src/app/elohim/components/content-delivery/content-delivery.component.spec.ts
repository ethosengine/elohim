import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, provideRouter } from '@angular/router';
import { of, throwError } from 'rxjs';
import { vi } from 'vitest';

import { ContentDeliveryComponent } from './content-delivery.component';
import { ContentService } from '@app/lamad/services/content.service';
import { RendererRegistryService } from '@app/lamad/renderers/renderer-registry.service';
import { SeoService } from '../../../services/seo.service';

describe('ContentDeliveryComponent', () => {
  let component: ContentDeliveryComponent;
  let fixture: ComponentFixture<ContentDeliveryComponent>;
  let contentServiceSpy: { getContentBySlug: ReturnType<typeof vi.fn> };
  let seoServiceSpy: { updateForContent: ReturnType<typeof vi.fn> };

  const mockNode = {
    id: 'manifesto',
    title: 'The Elohim Protocol Manifesto',
    description: 'Founding document',
    contentType: 'epic',
    contentFormat: 'markdown',
    content: '# The Manifesto',
    reach: 'commons',
    stewardedBy: [
      { humanId: 'genesis', role: 'steward', affinity: 0.8 },
    ],
    tags: [],
    relatedNodeIds: [],
    metadata: {},
  };

  beforeEach(async () => {
    contentServiceSpy = {
      getContentBySlug: vi.fn().mockReturnValue(of(mockNode)),
    };

    const rendererRegistrySpy = {
      getRenderer: vi.fn().mockReturnValue(null),
    };

    seoServiceSpy = {
      updateForContent: vi.fn(),
    };

    await TestBed.configureTestingModule({
      imports: [ContentDeliveryComponent],
      providers: [
        provideRouter([]),
        { provide: ContentService, useValue: contentServiceSpy },
        { provide: RendererRegistryService, useValue: rendererRegistrySpy },
        { provide: SeoService, useValue: seoServiceSpy },
        {
          provide: ActivatedRoute,
          useValue: { params: of({ slug: 'manifesto' }) },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ContentDeliveryComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('loads content by slug', () => {
    expect(contentServiceSpy.getContentBySlug).toHaveBeenCalledWith('manifesto');
  });

  it('sets toolbar content address from node id', () => {
    expect(component.contentAddress).toBe('manifesto');
  });

  it('extracts steward data for toolbar', () => {
    expect(component.omnibarStewards.length).toBe(1);
    expect(component.omnibarStewards[0].humanId).toBe('genesis');
    expect(component.omnibarStewards[0].ratio).toBe(0.8);
  });

  it('sets reach for toolbar', () => {
    expect(component.reach).toBe('commons');
  });

  it('updates SEO metadata', () => {
    expect(seoServiceSpy.updateForContent).toHaveBeenCalled();
  });

  it('shows error state for missing content', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(of(null));
    // Re-trigger load
    component.isLoading = true;
    component.error = null;
    component.ngOnInit();
    expect(component.error).toBeTruthy();
  });

  it('shows error on service failure', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(
      throwError(() => new Error('Network error'))
    );
    component.isLoading = true;
    component.error = null;
    component.ngOnInit();
    expect(component.error).toBe('Failed to load content');
  });
});
