/**
 * EprLinkComponent spec — thin Angular wrapper around <elohim-epr-link>.
 *
 * Replaces the prior pre-refactor spec (commit 10516614e — "EprLinkComponent
 * now thin Angular wrapper around elohim-core Lit element") which tested the
 * old internal resolution / popover / display-mode logic. That logic moved
 * into the Lit element and is tested at the elohim-core layer.
 *
 * This spec asserts only the wrapper's contract:
 *   - Renders <elohim-epr-link> with epr/display attributes
 *   - Wires the resolver property as a DOM property after init
 *   - Translates the Lit 'navigate' CustomEvent into Router.navigate
 *   - Removes the listener on destroy
 */
import { vi } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, RouterModule } from '@angular/router';

import { of } from 'rxjs';

import { EprLinkComponent } from './epr-link.component';
import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';

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

describe('EprLinkComponent (thin Lit wrapper)', () => {
  let fixture: ComponentFixture<EprLinkComponent>;
  let component: EprLinkComponent;
  let resolverSpy: { resolve: ReturnType<typeof vi.fn> };
  let routerNavSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(async () => {
    resolverSpy = { resolve: vi.fn().mockReturnValue(of(mockResolved)) };

    await TestBed.configureTestingModule({
      imports: [EprLinkComponent, RouterModule.forRoot([])],
      providers: [{ provide: EprResolverService, useValue: resolverSpy }],
    }).compileComponents();

    fixture = TestBed.createComponent(EprLinkComponent);
    component = fixture.componentInstance;
    routerNavSpy = vi
      .spyOn(TestBed.inject(Router), 'navigate')
      .mockResolvedValue(true as any);
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render <elohim-epr-link> with epr/display attributes', () => {
    component.epr = 'epr:manifesto';
    component.display = 'card';
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector('elohim-epr-link');
    expect(lit).toBeTruthy();
    expect(lit?.getAttribute('epr')).toBe('epr:manifesto');
    expect(lit?.getAttribute('display')).toBe('card');
  });

  it('should attach a resolver function to the Lit element after init', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector(
      'elohim-epr-link',
    ) as HTMLElement & { resolver?: unknown };
    expect(typeof lit.resolver).toBe('function');
  });

  it('should navigate via Router when a navigate event is dispatched', async () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('navigate', { detail: { epr: 'epr:manifesto' }, bubbles: true }),
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(resolverSpy.resolve).toHaveBeenCalledWith('epr:manifesto');
    expect(routerNavSpy).toHaveBeenCalledWith(['/resource', 'manifesto']);
  });

  it('should not throw on ngOnDestroy', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    expect(() => component.ngOnDestroy()).not.toThrow();
  });
});
