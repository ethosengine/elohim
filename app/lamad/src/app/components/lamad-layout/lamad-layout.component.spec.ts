import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';
import { provideRouter } from '@angular/router';
import { Router } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { ELOHIM_CLIENT, GOVERNANCE, CONTENT_ATTESTATION } from '@elohim/service';
import { LAMAD_STORAGE_CLIENT } from '../../interfaces/storage.interface';
import { DataLoaderService } from '../../services/data-loader.service';
import { RendererInitializerService } from '../../renderers/renderer-initializer.service';
import { of } from 'rxjs';
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
        { provide: CONTENT_ATTESTATION, useValue: {} },
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
});
