import { NgZone, provideZoneChangeDetection } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';
import { SyncStatusService } from '../../services/sync-status.service';
import { provideRouter } from '@angular/router';
import { Router } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { ELOHIM_CLIENT, GOVERNANCE } from '@elohim/service';
import { LAMAD_STORAGE_CLIENT } from '../../interfaces/storage.interface';
import { DataLoaderService } from '../../services/data-loader.service';
import { RendererInitializerService } from '../../renderers/renderer-initializer.service';
import { of, Subject } from 'rxjs';
import { vi } from 'vitest';

describe('LamadLayoutComponent', () => {
  let component: LamadLayoutComponent;
  let fixture: ComponentFixture<LamadLayoutComponent>;

  const mockElohimClient = {
    get: vi.fn().mockReturnValue(Promise.resolve(null)),
    query: vi.fn().mockReturnValue(Promise.resolve([])),
    supportsOffline: vi.fn().mockReturnValue(false),
    backpressure: vi.fn().mockReturnValue(Promise.resolve(0)),
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LamadLayoutComponent],
      providers: [
        provideRouter([]),
        provideHttpClient(),
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
        { provide: GOVERNANCE, useValue: {} },
        {
          provide: LAMAD_STORAGE_CLIENT,
          useValue: {
            getBlobUrl: (h: string) => `https://test/blob/${h}`,
            getStorageBaseUrl: () => 'https://test',
          },
        },
        {
          provide: DataLoaderService,
          useValue: {
            checkReadiness: vi.fn().mockReturnValue(of(true)),
            getContentIndex: vi.fn().mockReturnValue(of({ nodes: [] })),
            getContent: vi.fn(),
          },
        },
        { provide: RendererInitializerService, useValue: {} },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(LamadLayoutComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // =========================================================================
  // onNavigatorNavigate
  // =========================================================================

  describe('onNavigatorNavigate', () => {
    let router: Router;
    let navigateSpy: ReturnType<typeof vi.spyOn>;

    beforeEach(() => {
      router = TestBed.inject(Router);
      navigateSpy = vi.spyOn(router, 'navigateByUrl').mockResolvedValue(true);
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it('should call router.navigateByUrl with stripped path for lamad-owned route', () => {
      const event = new CustomEvent('navigator-navigate', {
        detail: { route: '/lamad/path/x' },
      });
      component.onNavigatorNavigate(event);
      expect(navigateSpy).toHaveBeenCalledWith('/path/x');
    });

    it('should call router.navigateByUrl with "/" for bare /lamad route', () => {
      const event = new CustomEvent('navigator-navigate', {
        detail: { route: '/lamad' },
      });
      component.onNavigatorNavigate(event);
      expect(navigateSpy).toHaveBeenCalledWith('/');
    });

    it('should call location.assign for cross-bundle route', () => {
      const assignSpy = vi.fn();
      vi.stubGlobal('location', { ...globalThis.location, assign: assignSpy });

      try {
        const event = new CustomEvent('navigator-navigate', {
          detail: { route: '/identity/login' },
        });
        component.onNavigatorNavigate(event);
        expect(assignSpy).toHaveBeenCalledWith('/identity/login');
        expect(navigateSpy).not.toHaveBeenCalled();
      } finally {
        vi.unstubAllGlobals();
      }
    });

    it('should do nothing when event has no detail.route', () => {
      const assignSpy = vi.fn();
      vi.stubGlobal('location', { ...globalThis.location, assign: assignSpy });

      try {
        const event = new CustomEvent('navigator-navigate', { detail: {} });
        component.onNavigatorNavigate(event);
        expect(navigateSpy).not.toHaveBeenCalled();
        expect(assignSpy).not.toHaveBeenCalled();
      } finally {
        vi.unstubAllGlobals();
      }
    });
  });

  // =========================================================================
  // Readiness gate — OnPush subscribe-mutation regression (@regression)
  //
  // Angular 22 made OnPush the implicit change-detection default. This layout
  // flips `isReady` from a plain `.subscribe()` callback, which marks NO view
  // dirty — so on an implicit-OnPush component the `*ngIf="isReady"` gate never
  // re-evaluates and the learner is stranded on "Loading content..." forever
  // even though `/db/content?limit=1` already returned. That is exactly what
  // shipped to alpha after the Eager-removal wave.
  //
  // This test deliberately does NOT call `fixture.detectChanges()` after the
  // async emission: an explicit `detectChanges()` forces an unconditional check
  // and erases the very distinction OnPush enforces — the structural blindness
  // documented in backlog-onpush-eager-debt-inventory. It reproduces the browser
  // instead: zone change detection provided as the app provides it, the emission
  // driven inside `NgZone.run()` (the way a resolved fetch re-enters the zone),
  // and the assertion taken after the zone-driven tick settles.
  //
  // Negative control (verified while writing this): with the component's
  // `ChangeDetectionStrategy.Eager` stamp removed, this test fails with the DOM
  // still showing "Loading content..." — i.e. it reproduces the live hang.
  // =========================================================================

  describe('readiness gate renders without an external change-detection trigger', () => {
    it('swaps the loading state for the router outlet when readiness arrives asynchronously', async () => {
      TestBed.resetTestingModule();
      const readiness$ = new Subject<boolean>();

      TestBed.configureTestingModule({
        imports: [LamadLayoutComponent],
        providers: [
          // Mirror the real bundle (app.config.ts) — TestBed defaults to zoneless,
          // which would mask a zone-driven change-detection regression entirely.
          provideZoneChangeDetection(),
          provideRouter([]),
          provideHttpClient(),
          { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
          { provide: GOVERNANCE, useValue: {} },
          {
            provide: LAMAD_STORAGE_CLIENT,
            useValue: {
              getBlobUrl: (h: string) => `https://test/blob/${h}`,
              getStorageBaseUrl: () => 'https://test',
            },
          },
          {
            provide: DataLoaderService,
            useValue: {
              checkReadiness: vi.fn().mockReturnValue(readiness$.asObservable()),
              getContentIndex: vi.fn().mockReturnValue(of({ nodes: [] })),
              getContent: vi.fn(),
            },
          },
          { provide: RendererInitializerService, useValue: {} },
          // Caught-up sync status so the strip's poll completes and the zone can
          // reach stability (an 'unreachable' status re-polls every 4s forever).
          {
            provide: SyncStatusService,
            useValue: {
              fetch: () =>
                of({
                  connectedPeers: 1,
                  replication: { completed: 1, pending: 0, failed: 0, caughtUp: true },
                }),
            },
          },
        ],
      });

      const asyncFixture = TestBed.createComponent(LamadLayoutComponent);
      asyncFixture.autoDetectChanges(true);

      // Initial render: ngOnInit subscribes, nothing has emitted yet.
      const host = asyncFixture.nativeElement as HTMLElement;
      expect(host.textContent).toContain('Loading content');
      expect(host.querySelector('.lamad-main')).toBeNull();

      // Readiness arrives out-of-band — in the browser this is the doorway fetch
      // resolving back inside the Angular zone.
      TestBed.inject(NgZone).run(() => readiness$.next(true));
      await asyncFixture.whenStable();

      expect(host.querySelector('.lamad-main')).not.toBeNull();
      expect(host.textContent).not.toContain('Loading content');

      asyncFixture.destroy();
    });
  });
});
