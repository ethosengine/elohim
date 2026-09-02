import { Component, EventEmitter, Input, Output } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { ContentDeliveryComponent } from './content-delivery.component';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { SeoService } from '../../../services/seo.service';

@Component({ selector: 'app-epr-focal', standalone: true, template: '' })
class EprFocalStub {
  @Input() slug = '';
  @Output() nodeLoaded = new EventEmitter<unknown>();
  @Output() notFound = new EventEmitter<string>();
  @Output() failed = new EventEmitter<string>();
}

describe('ContentDeliveryComponent', () => {
  let component: ContentDeliveryComponent;
  let fixture: ComponentFixture<ContentDeliveryComponent>;
  let seoServiceSpy: { updateForContent: ReturnType<typeof vi.fn> };

  const mockNode = {
    id: 'manifesto',
    title: 'The Elohim Protocol Manifesto',
    description: 'Founding document',
    contentType: 'epic',
    contentFormat: 'markdown',
    content: '# The Manifesto',
    reach: 'commons',
    stewardedBy: [{ humanId: 'genesis', role: 'steward', affinity: 0.8 }],
    tags: [],
    relatedNodeIds: [],
    metadata: {},
  };

  beforeEach(async () => {
    seoServiceSpy = { updateForContent: vi.fn() };
    await TestBed.configureTestingModule({
      imports: [ContentDeliveryComponent],
      providers: [
        provideRouter([]),
        { provide: SeoService, useValue: seoServiceSpy },
        { provide: ActivatedRoute, useValue: { params: of({ slug: 'manifesto' }) } },
      ],
    })
      .overrideComponent(ContentDeliveryComponent, {
        remove: { imports: [EprFocalComponent] },
        add: { imports: [EprFocalStub] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(ContentDeliveryComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('hands the route slug to the focal slot', () => {
    expect(component.slug).toBe('manifesto');
    expect(fixture.nativeElement.querySelector('app-epr-focal')).not.toBeNull();
  });

  it('sets toolbar content address from the loaded node', () => {
    component.onNodeLoaded(mockNode as never);
    expect(component.contentAddress).toBe('manifesto');
  });

  it('extracts steward data for toolbar', () => {
    component.onNodeLoaded(mockNode as never);
    expect(component.omnibarStewards).toEqual([
      { humanId: 'genesis', displayName: 'genesis', ratio: 0.8 },
    ]);
  });

  it('sets reach for toolbar', () => {
    component.onNodeLoaded(mockNode as never);
    expect(component.reach).toBe('commons');
  });

  it('updates SEO metadata', () => {
    component.onNodeLoaded(mockNode as never);
    expect(seoServiceSpy.updateForContent).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'manifesto', title: 'The Elohim Protocol Manifesto' })
    );
  });

  it('shows error state when the focal reports not found', () => {
    component.onNotFound();
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('[data-testid="delivery-error"]')).not.toBeNull();
  });

  it('shows error on focal failure', () => {
    component.onFailed();
    expect(component.error).toBe('Failed to load content');
  });
});
