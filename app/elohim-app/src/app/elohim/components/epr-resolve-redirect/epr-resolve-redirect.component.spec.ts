/**
 * EprResolveRedirectComponent spec — protocol-handler redirect.
 *
 * Verifies the component strips the web+epr: prefix, resolves via
 * BUNDLE_ROUTE_CONTEXT, and navigates to the correct Angular route.
 */
import { vi } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

import { BUNDLE_ROUTE_CONTEXT } from '@elohim/service';

import { EprResolveRedirectComponent } from './epr-resolve-redirect.component';

describe('EprResolveRedirectComponent', () => {
  let fixture: ComponentFixture<EprResolveRedirectComponent>;
  let routerNavSpy: ReturnType<typeof vi.spyOn>;

  function makeRoute(uri: string): Partial<ActivatedRoute> {
    return {
      snapshot: {
        queryParamMap: {
          get: (key: string) => (key === 'uri' ? uri : null),
        },
      } as any,
    };
  }

  async function setupWith(uri: string): Promise<void> {
    await TestBed.configureTestingModule({
      imports: [EprResolveRedirectComponent, RouterModule.forRoot([])],
      providers: [
        { provide: ActivatedRoute, useValue: makeRoute(uri) },
        {
          provide: BUNDLE_ROUTE_CONTEXT,
          useValue: { claims: [], ownsUniversalRoute: true },
        },
      ],
    }).compileComponents();

    routerNavSpy = vi
      .spyOn(TestBed.inject(Router), 'navigate')
      .mockResolvedValue(true as any);

    fixture = TestBed.createComponent(EprResolveRedirectComponent);
    fixture.detectChanges(); // triggers ngOnInit
  }

  afterEach(() => {
    TestBed.resetTestingModule();
  });

  it('should create', async () => {
    await setupWith('epr:manifesto-foundations');
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('navigates to /epr/{id} for a plain epr: URI (shell owns universal route)', async () => {
    await setupWith('epr:manifesto-foundations');
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'manifesto-foundations']);
  });

  it('strips the web+epr: prefix before resolving', async () => {
    await setupWith('web+epr:rea-foundations');
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'rea-foundations']);
  });

  it('falls back to /epr/{id} when uri param is absent', async () => {
    await setupWith('');
    // parseEpr('') → id '' — eprToRoute with ownsUniversalRoute → ['/epr', '']
    // The navigate call must still be made (no crash)
    expect(routerNavSpy).toHaveBeenCalled();
  });

  it('handles a bare epr: URI with a fragment', async () => {
    await setupWith('epr:elohim-protocol#step/2');
    // step fragment implies path contentType; shell has no path claim → /epr/{id}
    expect(routerNavSpy).toHaveBeenCalledWith(['/epr', 'elohim-protocol']);
  });
});
