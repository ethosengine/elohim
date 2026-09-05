/**
 * LoginComponent (Lit wrapper) — spec
 *
 * The route is a REDIRECTOR: it discovers which doorway owns the human and
 * hands them to that doorway's portal through OAuth. There is no in-app
 * credential path, so these tests assert the hand-off and its inputs, never a
 * password submission. The Lit elements themselves are not rendered (JSDOM
 * treats custom elements as HTMLElement stubs), so assertions target
 * component state and service call expectations.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { signal } from '@angular/core';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { LoginComponent } from './login.component';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

describe('LoginComponent (Lit wrapper)', () => {
  let fixture: ComponentFixture<LoginComponent>;
  let component: LoginComponent;

  let mockAuthService: {
    hasProvider: ReturnType<typeof vi.fn>;
    registerProvider: ReturnType<typeof vi.fn>;
    isAuthenticated: ReturnType<typeof vi.fn>;
    login: ReturnType<typeof vi.fn>;
  };
  let mockOAuthProvider: {
    initiateLogin: ReturnType<typeof vi.fn>;
    storeReturnUrl: ReturnType<typeof vi.fn>;
    isFlowInProgress: ReturnType<typeof signal<boolean>>;
  };
  let mockDoorwayRegistry: {
    selectDoorwayByUrl: ReturnType<typeof vi.fn>;
    selectProbedDoorwayUrl: ReturnType<typeof vi.fn>;
    selected: ReturnType<typeof signal<null>>;
    selectedUrl: ReturnType<typeof signal<null>>;
    hasSelection: ReturnType<typeof signal<boolean>>;
  };
  let mockActivatedRoute: { queryParams: ReturnType<typeof of> };
  let router: Router;

  beforeEach(async () => {
    mockAuthService = {
      hasProvider: vi.fn().mockReturnValue(false),
      registerProvider: vi.fn(),
      isAuthenticated: vi.fn().mockReturnValue(false),
      login: vi.fn().mockResolvedValue({ success: true }),
    };

    mockOAuthProvider = {
      initiateLogin: vi.fn(),
      storeReturnUrl: vi.fn(),
      isFlowInProgress: signal(false),
    };

    mockDoorwayRegistry = {
      selectDoorwayByUrl: vi.fn(),
      selectProbedDoorwayUrl: vi.fn().mockResolvedValue(true),
      selected: signal(null),
      selectedUrl: signal(null),
      hasSelection: signal(false),
    };

    mockActivatedRoute = { queryParams: of({}) };

    await TestBed.configureTestingModule({
      imports: [LoginComponent],
      providers: [
        provideRouter([]),
        { provide: AuthService, useValue: mockAuthService },
        { provide: OAuthAuthProvider, useValue: mockOAuthProvider },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
      ],
    }).compileComponents();

    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate').mockResolvedValue(true);

    fixture = TestBed.createComponent(LoginComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ==========================================================================
  // Initial state
  // ==========================================================================

  it('renders the portal-shell at the resolve step initially', () => {
    expect(component.step).toBe('resolve');
    const shell = fixture.nativeElement.querySelector('elohim-imagodei-portal-shell');
    expect(shell).toBeTruthy();
  });

  it('starts with no error message', () => {
    expect(component.errorMessage).toBe('');
  });

  it('registers NO credential provider — the app is a relying party, not a portal', () => {
    // The one place a hosted human's password is seen is the doorway's own
    // origin. This route must not stand up an in-app credential path at all.
    expect(mockAuthService.registerProvider).not.toHaveBeenCalled();
    expect((component as unknown as Record<string, unknown>)['passwordProvider']).toBeUndefined();
    expect((component as unknown as Record<string, unknown>)['onPasswordSubmit']).toBeUndefined();
  });

  it('renders no login card and no password field', () => {
    const html = fixture.nativeElement.innerHTML as string;
    expect(html).not.toContain('elohim-imagodei-login-card');
    expect(html).not.toContain('allow-password');
    expect(fixture.nativeElement.querySelector('input[type="password"]')).toBeNull();
  });

  it('redirects immediately when already authenticated', () => {
    mockAuthService.isAuthenticated.mockReturnValue(true);

    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();

    expect(router.navigate).toHaveBeenCalledWith(['/']);
  });

  it('pre-fills identifier from localStorage', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockReturnValue('matthew@alpha.elohim.host');

    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();

    expect(newFixture.componentInstance.identifier).toBe('matthew@alpha.elohim.host');
  });

  // ==========================================================================
  // Step: resolve → login
  // ==========================================================================

  it('hands off to the doorway portal and stores the identifier on resolved', () => {
    component.onResolved(
      new CustomEvent('resolved', {
        detail: {
          identifier: 'matthew@alpha.elohim.host',
          doorwayUrl: 'https://alpha.elohim.host',
        },
      })
    );

    expect(component.step).toBe('login');
    expect(component.identifier).toBe('matthew@alpha.elohim.host');
    // login_hint carries the identifier so the human does not retype it.
    expect(mockOAuthProvider.initiateLogin).toHaveBeenCalledWith(
      'https://alpha.elohim.host',
      expect.stringContaining('/auth/callback'),
      'matthew@alpha.elohim.host'
    );
  });

  it('stores the return URL before leaving for the portal', () => {
    component.onResolved(
      new CustomEvent('resolved', {
        detail: {
          identifier: 'matthew@alpha.elohim.host',
          doorwayUrl: 'https://alpha.elohim.host',
        },
      })
    );

    expect(mockOAuthProvider.storeReturnUrl).toHaveBeenCalled();
  });

  it('adopts the resolved doorway through the PROBED path, not the trusted-only setter', () => {
    component.onResolved(
      new CustomEvent('resolved', {
        detail: {
          identifier: 'matthew@alpha.elohim.host',
          doorwayUrl: 'https://alpha.elohim.host',
        },
      })
    );

    // `selectDoorwayByUrl` accepts only a doorway the app ALREADY trusts, so a
    // host the resolver element legitimately probed would be silently refused
    // by it. The probed path is the one that can adopt a new doorway.
    expect(mockDoorwayRegistry.selectProbedDoorwayUrl).toHaveBeenCalledWith(
      'https://alpha.elohim.host',
      true
    );
    expect(mockDoorwayRegistry.selectDoorwayByUrl).not.toHaveBeenCalled();
  });

  it('selects nothing and redirects nowhere when no doorway declares the typed gateway', () => {
    component.onResolved(
      new CustomEvent('resolved', {
        detail: { identifier: 'someone@x.evil.tld', doorwayUrl: '' },
      })
    );

    // The lookup returns null for an undeclared host, and null is the answer —
    // an unknown host is never invented into a redirect target.
    expect(mockDoorwayRegistry.selectProbedDoorwayUrl).not.toHaveBeenCalled();
    expect(mockDoorwayRegistry.selectDoorwayByUrl).not.toHaveBeenCalled();
    expect(mockOAuthProvider.initiateLogin).not.toHaveBeenCalled();
    expect(component.step).toBe('resolve');
    expect(component.errorMessage).toBeTruthy();
  });

  it('clears errorMessage on a successful resolved event', () => {
    component.errorMessage = 'stale error';

    component.onResolved(
      new CustomEvent('resolved', {
        detail: {
          identifier: 'matthew@alpha.elohim.host',
          doorwayUrl: 'https://alpha.elohim.host',
        },
      })
    );

    expect(component.errorMessage).toBe('');
  });

  // ==========================================================================
  // Step: resolve error
  // ==========================================================================

  it('sets errorMessage on resolve-error event', () => {
    component.onResolveError(
      new CustomEvent('resolve-error', { detail: { reason: 'unknown-host' } })
    );

    expect(component.errorMessage).toContain('unknown-host');
  });

  it('does not advance step on resolve error', () => {
    component.onResolveError(new CustomEvent('resolve-error', { detail: { reason: 'timeout' } }));

    expect(component.step).toBe('resolve');
  });

  // ==========================================================================
  // A doorway that already proved itself skips the resolver entirely
  // ==========================================================================

  it('redirects on init, with no resolver step, when a doorway is already proven', () => {
    mockDoorwayRegistry.selectedUrl = signal('https://alpha.elohim.host') as ReturnType<
      typeof signal<null>
    >;
    mockOAuthProvider.initiateLogin.mockClear();

    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();

    expect(mockOAuthProvider.initiateLogin).toHaveBeenCalledWith(
      'https://alpha.elohim.host',
      expect.stringContaining('/auth/callback'),
      undefined
    );
    expect(newFixture.componentInstance.step).toBe('login');
  });

  it('carries a remembered identifier as the login hint on the init redirect', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockReturnValue('matthew@alpha.elohim.host');
    mockDoorwayRegistry.selectedUrl = signal('https://alpha.elohim.host') as ReturnType<
      typeof signal<null>
    >;
    mockOAuthProvider.initiateLogin.mockClear();

    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();

    expect(mockOAuthProvider.initiateLogin).toHaveBeenCalledWith(
      'https://alpha.elohim.host',
      expect.stringContaining('/auth/callback'),
      'matthew@alpha.elohim.host'
    );
  });

  it('does not redirect an already-authenticated human — it navigates them home', () => {
    mockAuthService.isAuthenticated.mockReturnValue(true);
    mockDoorwayRegistry.selectedUrl = signal('https://alpha.elohim.host') as ReturnType<
      typeof signal<null>
    >;
    mockOAuthProvider.initiateLogin.mockClear();

    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();

    expect(mockOAuthProvider.initiateLogin).not.toHaveBeenCalled();
    expect(router.navigate).toHaveBeenCalledWith(['/']);
  });

  // ==========================================================================
  // Hand-off failure
  // ==========================================================================

  it('surfaces an error and returns to resolve when the hand-off throws', () => {
    mockOAuthProvider.initiateLogin.mockImplementation(() => {
      throw new Error('sessionStorage unavailable');
    });

    component.redirectToPortal('https://alpha.elohim.host', 'matthew@alpha.elohim.host');

    expect(component.step).toBe('resolve');
    expect(component.errorMessage).toContain('sessionStorage unavailable');
  });

  // ==========================================================================
  // Legacy state-machine is gone
  // ==========================================================================

  it('has no currentStep signal (old multi-step machine removed)', () => {
    // The old 7-step LoginStep union is replaced by the 2-value Step type.
    // currentStep was the old signal name — it must not exist.
    expect((component as unknown as Record<string, unknown>)['currentStep']).toBeUndefined();
  });

  it('has no credentials step (old Tauri-specific step removed)', () => {
    // 'credentials' was one of the 7 old LoginStep values.
    // The new Step type only allows 'resolve' | 'login'.
    const validSteps: string[] = ['resolve', 'login'];
    expect(validSteps).toContain(component.step);
  });
  // ==========================================================================
  // Authority pre-fetch — the trust chip on an ANONYMOUS sign-in page
  // ==========================================================================

  describe('authority pre-fetch', () => {
    let originalLocation: Location;

    /** Reach the private pre-fetch without waiting on ngOnInit's fire-and-forget. */
    const prefetch = (): Promise<void> =>
      (component as unknown as { _prefetchAuthority(): Promise<void> })._prefetchAuthority();

    beforeEach(() => {
      component.authority = null;
      originalLocation = globalThis.location;
      Object.defineProperty(globalThis, 'location', {
        value: {
          origin: 'https://doorway-alpha.elohim.host',
          hostname: 'doorway-alpha.elohim.host',
          protocol: 'https:',
          href: 'https://doorway-alpha.elohim.host/identity/login',
        },
        writable: true,
        configurable: true,
      });
    });

    afterEach(() => {
      Object.defineProperty(globalThis, 'location', {
        value: originalLocation,
        writable: true,
        configurable: true,
      });
    });

    const jsonResponse = (body: unknown) => ({ ok: true, json: async () => body });
    const unauthorized = { ok: false, status: 401, json: async () => ({}) };

    it('falls back to the discovery document when /auth/me is 401 (every anonymous visitor)', async () => {
      const fetchMock = vi.fn(async (url: string) =>
        url.endsWith('/auth/me')
          ? unauthorized
          : jsonResponse({ version: 1, doorwayId: 'alpha.elohim.host', portal: '/threshold/login' })
      );
      vi.stubGlobal('fetch', fetchMock);

      await prefetch();

      expect(fetchMock).toHaveBeenCalledTimes(2);
      // The chip used to read "Hosted via" followed by NOTHING here.
      expect(component.authority).not.toBeNull();
      expect(component.authority?.authority.label).toBe('alpha.elohim.host');
      expect(component.authority?.authority.id).toBe('alpha.elohim.host');
      expect(component.authority?.trustMode).toBe('doorway-host');
    });

    it('names the page hostname when the discovery document declares no doorwayId', async () => {
      vi.stubGlobal(
        'fetch',
        vi.fn(async (url: string) =>
          url.endsWith('/auth/me')
            ? unauthorized
            : jsonResponse({ version: 1, portal: '/threshold/login' })
        )
      );

      await prefetch();

      expect(component.authority?.authority.label).toBe('doorway-alpha.elohim.host');
      expect(component.authority?.authority.id).toBeUndefined();
    });

    it('prefers the session answer when /auth/me actually resolves an authority', async () => {
      const fetchMock = vi.fn(async () =>
        jsonResponse({
          trustMode: 'peer-native',
          authority: { label: 'matthew.steward.example', id: 'matthew' },
          flywheelHint: true,
        })
      );
      vi.stubGlobal('fetch', fetchMock);

      await prefetch();

      expect(fetchMock).toHaveBeenCalledTimes(1); // discovery not consulted
      expect(component.authority?.authority.label).toBe('matthew.steward.example');
      expect(component.authority?.trustMode).toBe('peer-native');
      expect(component.authority?.flywheelHint).toBe(true);
    });

    it('leaves authority null when neither endpoint answers, so the shell can ask', async () => {
      vi.stubGlobal(
        'fetch',
        vi.fn(async () => {
          throw new Error('offline');
        })
      );

      await prefetch();

      expect(component.authority).toBeNull();
    });
  });
});
