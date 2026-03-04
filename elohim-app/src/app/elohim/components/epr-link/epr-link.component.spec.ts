import { vi } from 'vitest';
import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { RouterModule } from '@angular/router';

import { of } from 'rxjs';

import { EprLinkComponent } from './epr-link.component';
import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';

describe('EprLinkComponent', () => {
  let component: EprLinkComponent;
  let fixture: ComponentFixture<EprLinkComponent>;
  let resolverSpy: {
    resolve: ReturnType<typeof vi.fn>;
    resolveUrl: ReturnType<typeof vi.fn>;
    resolveEprHead: ReturnType<typeof vi.fn>;
  };

  const mockResolved: ResolvedContent = {
    ref: { id: 'manifesto', tier: 'doc' },
    content: {
      id: 'manifesto',
      title: 'Elohim Protocol Manifesto',
      description: 'The foundational document',
      contentType: 'article',
      contentBody: 'body',
      contentFormat: 'markdown',
      reach: 'public',
      tags: ['protocol', 'manifesto'],
    } as any,
    blobUrl: null,
    route: ['/resource', 'manifesto'],
  };

  beforeEach(async () => {
    resolverSpy = {
      resolve: vi.fn().mockReturnValue(of(mockResolved)),
      resolveUrl: vi.fn().mockReturnValue({
        ref: { id: 'manifesto', tier: 'doc' },
        url: 'https://doorway.host/db/content/manifesto',
        route: ['/resource', 'manifesto'],
      }),
      resolveEprHead: vi.fn().mockReturnValue(of(null)),
    };

    await TestBed.configureTestingModule({
      imports: [EprLinkComponent, RouterModule.forRoot([])],
      providers: [{ provide: EprResolverService, useValue: resolverSpy }],
    }).compileComponents();

    fixture = TestBed.createComponent(EprLinkComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('inline display', () => {
    it('should render inline link after resolution', () => {
      component.epr = 'epr:manifesto';
      component.display = 'inline';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const link = el.querySelector('[data-testid="epr-link-inline"]');
      expect(link).toBeTruthy();
      expect(link?.textContent?.trim()).toBe('Elohim Protocol Manifesto');
    });

    it('should set data-epr attribute', () => {
      component.epr = 'epr:manifesto';
      component.display = 'inline';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const link = el.querySelector('[data-testid="epr-link-inline"]');
      expect(link?.getAttribute('data-epr')).toBe('epr:manifesto');
    });

    it('should set data-content-type attribute', () => {
      component.epr = 'epr:manifesto';
      component.display = 'inline';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const link = el.querySelector('[data-testid="epr-link-inline"]');
      expect(link?.getAttribute('data-content-type')).toBe('article');
    });
  });

  describe('chip display', () => {
    it('should render chip with type and title', () => {
      component.epr = 'epr:manifesto';
      component.display = 'chip';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const chip = el.querySelector('[data-testid="epr-link-chip"]');
      expect(chip).toBeTruthy();

      const type = el.querySelector('.epr-chip-type');
      const title = el.querySelector('.epr-chip-title');
      expect(type?.textContent?.trim()).toBe('article');
      expect(title?.textContent?.trim()).toBe('Elohim Protocol Manifesto');
    });
  });

  describe('card display', () => {
    it('should render card with title and description', () => {
      component.epr = 'epr:manifesto';
      component.display = 'card';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const card = el.querySelector('[data-testid="epr-link-card"]');
      expect(card).toBeTruthy();

      const title = el.querySelector('.epr-card-title');
      const desc = el.querySelector('.epr-card-desc');
      expect(title?.textContent?.trim()).toBe('Elohim Protocol Manifesto');
      expect(desc?.textContent?.trim()).toBe('The foundational document');
    });

    it('should render reach and tags', () => {
      component.epr = 'epr:manifesto';
      component.display = 'card';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const reach = el.querySelector('.epr-card-reach');
      const tags = el.querySelector('.epr-card-tags');
      expect(reach?.textContent?.trim()).toBe('public');
      expect(tags?.textContent?.trim()).toBe('protocol / manifesto');
    });

    it('should not render thumbnail when blobUrl is null', () => {
      component.epr = 'epr:manifesto';
      component.display = 'card';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      expect(el.querySelector('.epr-card-thumbnail')).toBeNull();
    });

    it('should render thumbnail when blobUrl is present', () => {
      resolverSpy.resolve.mockReturnValue(
        of({ ...mockResolved, blobUrl: 'https://doorway.host/blob/abc' })
      );

      component.epr = 'epr:manifesto';
      component.display = 'card';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const img = el.querySelector('.epr-card-thumbnail') as HTMLImageElement;
      expect(img).toBeTruthy();
      expect(img.src).toBe('https://doorway.host/blob/abc');
    });
  });

  describe('loading and error states', () => {
    it('should show loading indicator before resolution', () => {
      // Don't trigger ngOnChanges yet — component has no resolved data
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      expect(el.querySelector('.epr-link-loading')).toBeTruthy();
    });

    it('should show error when resolution returns null', () => {
      resolverSpy.resolve.mockReturnValue(of(null));

      component.epr = 'epr:nonexistent';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      const el = fixture.nativeElement as HTMLElement;
      const error = el.querySelector('.epr-link-error');
      expect(error).toBeTruthy();
      expect(error?.textContent?.trim()).toBe('epr:nonexistent');
    });
  });

  describe('EPR URI parsing', () => {
    it('should call resolver.resolve with the input', () => {
      component.epr = 'epr:rea-foundations';
      component.ngOnChanges({ epr: {} as any });

      expect(resolverSpy.resolve).toHaveBeenCalledWith('epr:rea-foundations');
    });

    it('should accept bare ID format', () => {
      component.epr = 'manifesto';
      component.ngOnChanges({ epr: {} as any });

      expect(resolverSpy.resolve).toHaveBeenCalledWith('manifesto');
    });
  });

  describe('cleanup', () => {
    it('should complete destroy$ on ngOnDestroy', () => {
      component.epr = 'epr:manifesto';
      component.ngOnChanges({ epr: {} as any });
      fixture.detectChanges();

      // Should not throw
      expect(() => component.ngOnDestroy()).not.toThrow();
    });
  });
});
