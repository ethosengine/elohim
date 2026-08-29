/**
 * LoginComponent (Lit wrapper) — spec
 *
 * Tests cover the thin Angular bridge: event wiring, step transitions, and
 * service delegation. The Lit elements themselves are not rendered (JSDOM
 * treats custom elements as HTMLElement stubs), so assertions target
 * component state and service call expectations.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { signal } from '@angular/core';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { LoginComponent } from './login.component';
import { AUTH_IDENTIFIER_KEY } from '../../models/auth.model';
import { AuthService } from '../../services/auth.service';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { IdentityService } from '../../services/identity.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { PasswordAuthProvider } from '../../services/providers/password-auth.provider';

describe('LoginComponent (Lit wrapper)', () => {
  let fixture: ComponentFixture<LoginComponent>;
  let component: LoginComponent;

  let mockAuthService: {
    hasProvider: ReturnType<typeof vi.fn>;
    registerProvider: ReturnType<typeof vi.fn>;
    isAuthenticated: ReturnType<typeof vi.fn>;
    login: ReturnType<typeof vi.fn>;
  };
  let mockPasswordProvider: { login: ReturnType<typeof vi.fn> };
  let mockOAuthProvider: {
    initiateLogin: ReturnType<typeof vi.fn>;
    storeReturnUrl: ReturnType<typeof vi.fn>;
    isFlowInProgress: ReturnType<typeof signal<boolean>>;
  };
  let mockIdentityService: { waitForAuthenticatedState: ReturnType<typeof vi.fn> };
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

    mockPasswordProvider = { login: vi.fn().mockResolvedValue({}) };

    mockOAuthProvider = {
      initiateLogin: vi.fn(),
      storeReturnUrl: vi.fn(),
      isFlowInProgress: signal(false),
    };

    mockIdentityService = {
      waitForAuthenticatedState: vi.fn().mockResolvedValue(true),
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
        { provide: PasswordAuthProvider, useValue: mockPasswordProvider },
        { provide: OAuthAuthProvider, useValue: mockOAuthProvider },
        { provide: IdentityService, useValue: mockIdentityService },
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

  it('registers password provider with AuthService on init', () => {
    expect(mockAuthService.registerProvider).toHaveBeenCalledWith(mockPasswordProvider);
  });

  it('does not re-register if provider is already registered', async () => {
    mockAuthService.hasProvider.mockReturnValue(true);
    mockAuthService.registerProvider.mockClear();

    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();

    expect(mockAuthService.registerProvider).not.toHaveBeenCalled();
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

  it('advances to login step and stores identifier on resolved event', () => {
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

  it('selects nothing when no doorway declares the typed gateway', () => {
    component.onResolved(
      new CustomEvent('resolved', {
        detail: { identifier: 'someone@x.evil.tld', doorwayUrl: '' },
      })
    );

    // The lookup returns null for an undeclared host, and null is the answer.
    expect(mockDoorwayRegistry.selectProbedDoorwayUrl).not.toHaveBeenCalled();
    expect(mockDoorwayRegistry.selectDoorwayByUrl).not.toHaveBeenCalled();
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
  // Password submit
  // ==========================================================================

  it('calls AuthService.login with password credentials on password-submit', async () => {
    component.identifier = 'matthew@alpha.elohim.host';

    await component.onPasswordSubmit(
      new CustomEvent('password-submit', {
        detail: { identifier: 'matthew@alpha.elohim.host', password: 'hunter2', remember: false },
      })
    );

    expect(mockAuthService.login).toHaveBeenCalledWith('password', {
      type: 'password',
      identifier: 'matthew@alpha.elohim.host',
      password: 'hunter2',
    });
  });

  it('navigates to returnUrl after successful password submit', async () => {
    mockActivatedRoute.queryParams = of({ returnUrl: '/dashboard' });
    const newFixture = TestBed.createComponent(LoginComponent);
    newFixture.componentInstance.ngOnInit();
    await newFixture.whenStable();

    await newFixture.componentInstance.onPasswordSubmit(
      new CustomEvent('password-submit', {
        detail: { identifier: 'matthew@alpha.elohim.host', password: 'pw', remember: false },
      })
    );

    expect(router.navigate).toHaveBeenCalledWith(['/dashboard']);
  });

  it('persists identifier in localStorage when remember is true', async () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {});
    component.identifier = 'matthew@alpha.elohim.host';

    await component.onPasswordSubmit(
      new CustomEvent('password-submit', {
        detail: { identifier: 'matthew@alpha.elohim.host', password: 'pw', remember: true },
      })
    );

    expect(setItem).toHaveBeenCalledWith(AUTH_IDENTIFIER_KEY, 'matthew@alpha.elohim.host');
  });

  it('removes identifier from localStorage when remember is false', async () => {
    const removeItem = vi.spyOn(Storage.prototype, 'removeItem').mockImplementation(() => {});
    component.identifier = 'matthew@alpha.elohim.host';

    await component.onPasswordSubmit(
      new CustomEvent('password-submit', {
        detail: { identifier: 'matthew@alpha.elohim.host', password: 'pw', remember: false },
      })
    );

    expect(removeItem).toHaveBeenCalledWith(AUTH_IDENTIFIER_KEY);
  });

  it('surfaces error to errorMessage on password failure (AuthResult failure)', async () => {
    mockAuthService.login.mockResolvedValue({ success: false, error: 'bad credentials' });
    component.identifier = 'matthew@alpha.elohim.host';

    await component.onPasswordSubmit(
      new CustomEvent('password-submit', {
        detail: { identifier: 'matthew@alpha.elohim.host', password: 'wrong', remember: false },
      })
    );

    expect(component.errorMessage).toContain('bad credentials');
  });

  it('surfaces error to errorMessage on thrown exception during password submit', async () => {
    mockAuthService.login.mockRejectedValue(new Error('network error'));
    component.identifier = 'matthew@alpha.elohim.host';

    await component.onPasswordSubmit(
      new CustomEvent('password-submit', {
        detail: { identifier: 'matthew@alpha.elohim.host', password: 'pw', remember: false },
      })
    );

    expect(component.errorMessage).toContain('network error');
  });

  // ==========================================================================
  // OAuth start
  // ==========================================================================

  it('calls oauthProvider.initiateLogin on oauth-start event', async () => {
    component.identifier = 'matthew@alpha.elohim.host';
    // Provide doorwayUrl via the registry so the handler can resolve it
    (mockDoorwayRegistry.selectedUrl as ReturnType<typeof signal<string | null>>) = signal(
      'https://alpha.elohim.host'
    );

    await component.onOAuthStart(
      new CustomEvent('oauth-start', {
        detail: { providerId: 'github', doorwayUrl: 'https://alpha.elohim.host' },
      })
    );

    expect(mockOAuthProvider.initiateLogin).toHaveBeenCalled();
  });

  it('calls oauthProvider.storeReturnUrl before initiating OAuth', async () => {
    await component.onOAuthStart(
      new CustomEvent('oauth-start', {
        detail: { providerId: 'github', doorwayUrl: 'https://alpha.elohim.host' },
      })
    );

    expect(mockOAuthProvider.storeReturnUrl).toHaveBeenCalled();
  });

  it('sets errorMessage when no doorway URL is available on oauth-start', async () => {
    component.identifier = '';
    (mockDoorwayRegistry.selectedUrl as ReturnType<typeof signal<string | null>>) = signal(null);

    await component.onOAuthStart(
      new CustomEvent('oauth-start', { detail: { providerId: 'github' } })
    );

    expect(component.errorMessage).toBeTruthy();
    expect(mockOAuthProvider.initiateLogin).not.toHaveBeenCalled();
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
});
