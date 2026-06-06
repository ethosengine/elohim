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

import { type ContextMenuAction } from '@app/qahal';

import { EprLinkComponent } from './epr-link.component';
import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';
import { EprNavService } from '../../services/epr-nav.service';

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
  route: ['/epr', 'manifesto'],
  href: '/epr/manifesto',
};

describe('EprLinkComponent (thin Lit wrapper)', () => {
  let fixture: ComponentFixture<EprLinkComponent>;
  let component: EprLinkComponent;
  let resolverSpy: { resolve: ReturnType<typeof vi.fn> };
  let routerNavSpy: ReturnType<typeof vi.spyOn>;
  let eprNavSpy: { navigate: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    resolverSpy = { resolve: vi.fn().mockReturnValue(of(mockResolved)) };
    eprNavSpy = { navigate: vi.fn() };

    await TestBed.configureTestingModule({
      imports: [EprLinkComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EprResolverService, useValue: resolverSpy },
        { provide: EprNavService, useValue: eprNavSpy },
      ],
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
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'manifesto']);
  });

  it('should navigate via EprNavService when route is null (cross-bundle)', async () => {
    resolverSpy.resolve.mockReturnValue(
      of({ ...mockResolved, route: null, href: '/epr/manifesto' }),
    );

    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('navigate', { detail: { epr: 'epr:manifesto' }, bubbles: true }),
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(eprNavSpy.navigate).toHaveBeenCalledWith('/epr/manifesto');
    expect(routerNavSpy).not.toHaveBeenCalled();
  });

  it('should not throw on ngOnDestroy', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    expect(() => component.ngOnDestroy()).not.toThrow();
  });

  it('should inject the full action list as the Lit element contextMenuItems property', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector(
      'elohim-epr-link',
    ) as HTMLElement & { contextMenuItems?: { id: string }[] };
    expect(Array.isArray(lit.contextMenuItems)).toBe(true);
    const ids = lit.contextMenuItems!.map(i => i.id);
    // MVP three lead, then the full Epic E set.
    expect(ids.slice(0, 3)).toEqual(['open', 'about', 'copy']);
    expect(ids).toContain('network');
    expect(ids).toContain('steward');
    expect(ids).toContain('flag');
  });

  it('should navigate to the resilience route with the network fragment on a "network" selection', async () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'network', epr: 'epr:manifesto' },
        bubbles: true,
      }),
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(resolverSpy.resolve).toHaveBeenCalledWith('epr:manifesto');
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'manifesto'], {
      fragment: 'network',
    });
  });

  it('should emit the governance output with a ContextMenuAction on a "flag" selection', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const emitted: unknown[] = [];
    component.governance.subscribe(a => emitted.push(a));

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'flag', epr: 'epr:manifesto' },
        bubbles: true,
      }),
    );

    expect(emitted).toEqual([
      { entityType: 'epr', entityId: 'epr:manifesto', action: 'flag' },
    ]);
  });

  it('should map "feedback" to the open-feedback governance action', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const emitted: ContextMenuAction[] = [];
    component.governance.subscribe(a => emitted.push(a));

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'feedback', epr: 'epr:manifesto' },
        bubbles: true,
      }),
    );

    expect(emitted[0].action).toBe('open-feedback');
  });

  it('should remove the epr-menu-select listener on destroy', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    const removeSpy = vi.spyOn(host, 'removeEventListener');
    component.ngOnDestroy();

    expect(removeSpy).toHaveBeenCalledWith(
      'epr-menu-select',
      expect.any(Function),
    );
  });
});
