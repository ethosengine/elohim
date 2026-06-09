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

import { Subject, of } from 'rxjs';

import type { PullStatusInfo } from '../../services/acquisition.service';

import { type ContextMenuAction } from '@app/qahal';

import { EprLinkComponent } from './epr-link.component';
import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';
import { EprNavService } from '../../services/epr-nav.service';
import { AcquisitionService } from '../../services/acquisition.service';

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
  let acquisitionSpy: {
    download: ReturnType<typeof vi.fn>;
    capability: ReturnType<typeof vi.fn>;
    pinAsPeer: ReturnType<typeof vi.fn>;
    pullStatus$: ReturnType<typeof vi.fn>;
  };

  beforeEach(async () => {
    resolverSpy = { resolve: vi.fn().mockReturnValue(of(mockResolved)) };
    eprNavSpy = { navigate: vi.fn() };
    acquisitionSpy = {
      download: vi.fn().mockResolvedValue('browser'),
      capability: vi.fn().mockReturnValue('browser'),
      pinAsPeer: vi.fn().mockResolvedValue(undefined),
      pullStatus$: vi.fn().mockReturnValue(of()),
    };

    await TestBed.configureTestingModule({
      imports: [EprLinkComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EprResolverService, useValue: resolverSpy },
        { provide: EprNavService, useValue: eprNavSpy },
        { provide: AcquisitionService, useValue: acquisitionSpy },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(EprLinkComponent);
    component = fixture.componentInstance;
    routerNavSpy = vi.spyOn(TestBed.inject(Router), 'navigate').mockResolvedValue(true as any);
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
      'elohim-epr-link'
    ) as HTMLElement & { resolver?: unknown };
    expect(typeof lit.resolver).toBe('function');
  });

  it('should navigate via Router when a navigate event is dispatched', async () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('navigate', { detail: { epr: 'epr:manifesto' }, bubbles: true })
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(resolverSpy.resolve).toHaveBeenCalledWith('epr:manifesto');
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'manifesto']);
  });

  it('should navigate via EprNavService when route is null (cross-bundle)', async () => {
    resolverSpy.resolve.mockReturnValue(
      of({ ...mockResolved, route: null, href: '/epr/manifesto' })
    );

    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('navigate', { detail: { epr: 'epr:manifesto' }, bubbles: true })
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
      'elohim-epr-link'
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
      })
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
      })
    );

    expect(emitted).toEqual([{ entityType: 'epr', entityId: 'epr:manifesto', action: 'flag' }]);
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
      })
    );

    expect(emitted[0].action).toBe('open-feedback');
  });

  it('should remove the epr-menu-select listener on destroy', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    const removeSpy = vi.spyOn(host, 'removeEventListener');
    component.ngOnDestroy();

    expect(removeSpy).toHaveBeenCalledWith('epr-menu-select', expect.any(Function));
  });

  it('should include "open-in" and "download" in the context menu action list', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector(
      'elohim-epr-link'
    ) as HTMLElement & { contextMenuItems?: { id: string }[] };
    const ids = lit.contextMenuItems!.map(i => i.id);
    expect(ids).toContain('open-in');
    expect(ids).toContain('download');
    // Built-in three remain first.
    expect(ids.slice(0, 3)).toEqual(['open', 'about', 'copy']);
  });

  it('should call AcquisitionService.download with the epr on a "download" selection', async () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'download', epr: 'epr:test-1' },
        bubbles: true,
      })
    );
    // Allow the microtask queue to drain so the void-promise handler runs.
    await Promise.resolve();

    expect(acquisitionSpy.download).toHaveBeenCalledWith('epr:test-1');
  });

  it('should navigate via navigateToResource on an "open-in" selection', async () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'open-in', epr: 'epr:manifesto' },
        bubbles: true,
      })
    );
    await Promise.resolve();
    await Promise.resolve();

    expect(resolverSpy.resolve).toHaveBeenCalledWith('epr:manifesto');
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'manifesto']);
  });

  it('does NOT offer "pin-as-peer" on a browser node', () => {
    acquisitionSpy.capability.mockReturnValue('browser');
    component.epr = 'epr:manifesto';
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector(
      'elohim-epr-link'
    ) as HTMLElement & { contextMenuItems?: { id: string }[] };
    const ids = lit.contextMenuItems!.map(i => i.id);
    expect(ids).not.toContain('pin-as-peer');
  });

  it('offers "pin-as-peer" on a peer node for commons-reach content', async () => {
    acquisitionSpy.capability.mockReturnValue('peer');
    resolverSpy.resolve.mockReturnValue(
      of({ ...mockResolved, content: { ...mockResolved.content, reach: 'commons' } })
    );

    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    // pin-as-peer gating resolves reach asynchronously; let the resolver settle.
    await Promise.resolve();
    await Promise.resolve();
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector(
      'elohim-epr-link'
    ) as HTMLElement & { contextMenuItems?: { id: string }[] };
    const ids = lit.contextMenuItems!.map(i => i.id);
    expect(ids).toContain('pin-as-peer');
  });

  it('does NOT offer "pin-as-peer" on a peer node for non-commons content', async () => {
    acquisitionSpy.capability.mockReturnValue('peer');
    // mockResolved.content.reach is 'public' (non-commons) by default.
    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    await Promise.resolve();
    await Promise.resolve();
    fixture.detectChanges();

    const lit = (fixture.nativeElement as HTMLElement).querySelector(
      'elohim-epr-link'
    ) as HTMLElement & { contextMenuItems?: { id: string }[] };
    const ids = lit.contextMenuItems!.map(i => i.id);
    expect(ids).not.toContain('pin-as-peer');
  });

  it('routes a "pin-as-peer" selection to AcquisitionService.pinAsPeer', async () => {
    acquisitionSpy.capability.mockReturnValue('peer');
    resolverSpy.resolve.mockReturnValue(
      of({ ...mockResolved, content: { ...mockResolved.content, reach: 'commons' } })
    );
    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    await Promise.resolve();
    await Promise.resolve();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', {
        detail: { id: 'pin-as-peer', epr: 'epr:strawberry-guide' },
        bubbles: true,
      })
    );
    await Promise.resolve();

    expect(acquisitionSpy.pinAsPeer).toHaveBeenCalledWith('epr:strawberry-guide');
  });

  // ── Rung-4 visible-delivery wiring: <app-pin-progress> in the host ──────────

  /** Drive a peer pin-as-peer selection and return the status Subject feeding it. */
  async function selectPinAsPeer(epr = 'epr:strawberry-guide'): Promise<Subject<PullStatusInfo>> {
    const status$ = new Subject<PullStatusInfo>();
    acquisitionSpy.capability.mockReturnValue('peer');
    acquisitionSpy.pullStatus$.mockReturnValue(status$);
    resolverSpy.resolve.mockReturnValue(
      of({ ...mockResolved, content: { ...mockResolved.content, reach: 'commons' } })
    );
    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    await Promise.resolve();
    await Promise.resolve();

    const host = fixture.nativeElement as HTMLElement;
    host.dispatchEvent(
      new CustomEvent('epr-menu-select', { detail: { id: 'pin-as-peer', epr }, bubbles: true })
    );
    // pinAsPeer resolves, then subscribePull subscribes (two microtask hops).
    await Promise.resolve();
    await Promise.resolve();
    fixture.detectChanges();
    return status$;
  }

  it('subscribes to pullStatus$ and renders <app-pin-progress> on a pin-as-peer selection', async () => {
    const status$ = await selectPinAsPeer();
    expect(acquisitionSpy.pullStatus$).toHaveBeenCalledWith('epr:strawberry-guide');
    expect(status$.observed).toBe(true);

    status$.next({ total: 4, fetched: 1, pending: 3, failed: 0, caughtUp: false });
    fixture.detectChanges();

    const bar = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="epr-link-pin-progress"]'
    );
    expect(bar).toBeTruthy();
    const frac = bar?.querySelector('[data-testid="pin-progress-fraction"]');
    expect(frac?.textContent).toContain('1');
    expect(frac?.textContent).toContain('4');
  });

  it('does not render the progress bar before a pin-as-peer selection', () => {
    component.epr = 'epr:manifesto';
    fixture.detectChanges();
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="epr-link-pin-progress"]')
    ).toBeFalsy();
  });

  it('stops polling on caughtUp===true but KEEPS the element (serving badge)', async () => {
    const status$ = await selectPinAsPeer();
    expect(status$.observed).toBe(true);

    status$.next({ total: 2, fetched: 2, pending: 0, failed: 0, caughtUp: true });
    fixture.detectChanges();

    // Polling stopped: the host unsubscribed.
    expect(status$.observed).toBe(false);
    // Element still rendered, showing the caught-up badge (null≠done preserved —
    // total was computable, so caughtUp surfaces).
    const bar = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="epr-link-pin-progress"]'
    );
    expect(bar).toBeTruthy();
    expect(bar?.querySelector('[data-testid="pin-progress-caught-up"]')).toBeTruthy();
  });

  it('does NOT stop polling when caughtUp is true but total is null (null≠done)', async () => {
    const status$ = await selectPinAsPeer();
    status$.next({ total: null, fetched: 0, pending: 0, failed: 0, caughtUp: true });
    fixture.detectChanges();

    // null total = not computable: keep polling, never surface "serving".
    expect(status$.observed).toBe(true);
    const bar = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="epr-link-pin-progress"]'
    );
    expect(bar?.querySelector('[data-testid="pin-progress-caught-up"]')).toBeFalsy();
    expect(bar?.querySelector('[data-testid="pin-progress-waiting"]')).toBeTruthy();
  });

  it('clears the bar and unsubscribes on cancel (no leaked subscription)', async () => {
    const status$ = await selectPinAsPeer();
    status$.next({ total: 4, fetched: 1, pending: 3, failed: 0, caughtUp: false });
    fixture.detectChanges();

    const cancelBtn = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="pin-progress-cancel"]'
    ) as HTMLButtonElement;
    expect(cancelBtn).toBeTruthy();
    cancelBtn.click();
    fixture.detectChanges();

    // Unsubscribed (no leaked poll) AND the bar is gone.
    expect(status$.observed).toBe(false);
    expect(
      (fixture.nativeElement as HTMLElement).querySelector('[data-testid="epr-link-pin-progress"]')
    ).toBeFalsy();
  });

  it('re-issues pinAsPeer and re-subscribes on retry', async () => {
    const status$ = await selectPinAsPeer();
    status$.next({ total: 3, fetched: 1, pending: 1, failed: 1, caughtUp: false });
    fixture.detectChanges();

    const retryBtn = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="pin-progress-retry"]'
    ) as HTMLButtonElement;
    expect(retryBtn).toBeTruthy();

    acquisitionSpy.pinAsPeer.mockClear();
    acquisitionSpy.pullStatus$.mockClear();
    const status2$ = new Subject<PullStatusInfo>();
    acquisitionSpy.pullStatus$.mockReturnValue(status2$);

    retryBtn.click();
    await Promise.resolve();
    await Promise.resolve();
    fixture.detectChanges();

    expect(acquisitionSpy.pinAsPeer).toHaveBeenCalledWith('epr:strawberry-guide');
    expect(status2$.observed).toBe(true);
  });

  it('unsubscribes the poll on ngOnDestroy (no leaked interval)', async () => {
    const status$ = await selectPinAsPeer();
    expect(status$.observed).toBe(true);
    component.ngOnDestroy();
    expect(status$.observed).toBe(false);
  });
});
