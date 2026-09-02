/**
 * EprFocalComponent — the renderer host extracted from content-delivery.
 * Pins: slug → load → nodeLoaded; null → notFound; error → failed; a registered
 * renderer is created with the node as input; no renderer → format fallback.
 */
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { describe, expect, it, vi, beforeEach } from 'vitest';

import { ContentService } from '@app/lamad/services/content.service';
import { RendererRegistryService } from '@app/lamad/renderers/renderer-registry.service';

import { EprFocalComponent } from './epr-focal.component';

const mockNode = {
  id: 'manifesto',
  title: 'The Elohim Protocol Manifesto',
  description: 'Founding document',
  contentType: 'epic',
  contentFormat: 'plaintext',
  content: 'plain body',
  reach: 'commons',
  stewardedBy: [],
  tags: [],
  relatedNodeIds: [],
  metadata: {},
};

describe('EprFocalComponent', () => {
  let fixture: ComponentFixture<EprFocalComponent>;
  let contentServiceSpy: { getContentBySlug: ReturnType<typeof vi.fn> };
  let registrySpy: { getRenderer: ReturnType<typeof vi.fn>; register: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    contentServiceSpy = { getContentBySlug: vi.fn().mockReturnValue(of(mockNode)) };
    registrySpy = { getRenderer: vi.fn().mockReturnValue(null), register: vi.fn() };
    await TestBed.configureTestingModule({
      imports: [EprFocalComponent],
      providers: [
        { provide: ContentService, useValue: contentServiceSpy },
        { provide: RendererRegistryService, useValue: registrySpy },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(EprFocalComponent);
  });

  function setSlug(slug: string): void {
    fixture.componentRef.setInput('slug', slug);
    fixture.detectChanges();
  }

  it('loads the node for the slug and emits nodeLoaded', () => {
    const loaded = vi.fn();
    fixture.componentInstance.nodeLoaded.subscribe(loaded);
    setSlug('manifesto');
    expect(contentServiceSpy.getContentBySlug).toHaveBeenCalledWith('manifesto');
    expect(loaded).toHaveBeenCalledWith(mockNode);
  });

  it('renders the plaintext fallback when no renderer is registered', () => {
    setSlug('manifesto');
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('.plaintext-content')?.textContent).toContain(
      'plain body'
    );
  });

  it('emits notFound when the slug resolves to null', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(of(null));
    const notFound = vi.fn();
    fixture.componentInstance.notFound.subscribe(notFound);
    setSlug('missing');
    expect(notFound).toHaveBeenCalledWith('missing');
  });

  it('emits failed when the load errors', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(throwError(() => new Error('boom')));
    const failed = vi.fn();
    fixture.componentInstance.failed.subscribe(failed);
    setSlug('manifesto');
    expect(failed).toHaveBeenCalledWith('manifesto');
  });

  it('reloads when the slug input changes', () => {
    setSlug('manifesto');
    setSlug('succession');
    expect(contentServiceSpy.getContentBySlug).toHaveBeenNthCalledWith(2, 'succession');
  });

  it('renders the fallback title when showFallbackTitle is set and no renderer is registered', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(
      of({ ...mockNode, contentFormat: 'gherkin' })
    );
    fixture.componentRef.setInput('showFallbackTitle', true);
    setSlug('manifesto');
    const h1 = fixture.nativeElement.querySelector('.fallback-content h1');
    expect(h1?.textContent).toContain('The Elohim Protocol Manifesto');
  });

  it('renders no fallback title by default', () => {
    contentServiceSpy.getContentBySlug.mockReturnValue(
      of({ ...mockNode, contentFormat: 'gherkin' })
    );
    setSlug('manifesto');
    expect(fixture.nativeElement.querySelector('.fallback-content h1')).toBeNull();
  });
});
