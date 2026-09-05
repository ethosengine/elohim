/**
 * AuthCallbackComponent (Lit wrapper) — spec
 *
 * Tests cover the thin Angular bridge: URL param extraction, exchangeCode
 * callback wiring, auth state update, navigation, and error logging.
 *
 * The Lit element is not rendered (JSDOM treats custom elements as
 * HTMLElement stubs), so assertions target component state and service
 * call expectations — same pattern as login.component.spec.ts.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, provideRouter, Router } from '@angular/router';
import { signal } from '@angular/core';
import { vi } from 'vitest';

import { AuthCallbackComponent } from './auth-callback.component';
import { AuthService } from '../../services/auth.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';
import { SessionMigrationService } from '../../services/session-migration.service';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeQueryParamMap(params: Record<string, string>) {
  return {
    get: (k: string) => params[k] ?? null,
  };
}

function makeRoute(params: Record<string, string> = {}) {
  return {
    snapshot: { queryParamMap: makeQueryParamMap(params) },
  };
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('AuthCallbackComponent (Lit wrapper)', () => {
  let fixture: ComponentFixture<AuthCallbackComponent>;
  let component: AuthCallbackComponent;
  let router: Router;

  let mockOAuth: {
    handleCallback: ReturnType<typeof vi.fn>;
    clearCallbackParams: ReturnType<typeof vi.fn>;
    consumeReturnUrl: ReturnType<typeof vi.fn>;
    isFlowInProgress: ReturnType<typeof signal<boolean>>;
  };

  let mockAuthService: {
    setAuthFromResult: ReturnType<typeof vi.fn>;
  };

  let mockMigration: {
    canMigrate: ReturnType<typeof vi.fn>;
    applySessionToProfile: ReturnType<typeof vi.fn>;
  };

  const successResult = {
    success: true as const,
    token: 'jwt-abc',
    humanId: 'matthew',
    agentPubKey: 'agent-pub-key',
    identifier: 'matthew@alpha.elohim.host',
    expiresAt: new Date(Date.now() + 3_600_000).toISOString(),
  };

  async function setup(
    queryParams: Record<string, string> = { code: 'abc', state: 'xyz', provider: 'github' }
  ) {
    mockOAuth = {
      handleCallback: vi.fn().mockResolvedValue(successResult),
      clearCallbackParams: vi.fn(),
      consumeReturnUrl: vi.fn().mockReturnValue(null),
      isFlowInProgress: signal(false),
    };

    mockAuthService = {
      setAuthFromResult: vi.fn(),
    };

    mockMigration = {
      canMigrate: vi.fn().mockReturnValue(false),
      applySessionToProfile: vi.fn().mockResolvedValue({ success: true }),
    };

    await TestBed.configureTestingModule({
      imports: [AuthCallbackComponent],
      providers: [
        provideRouter([]),
        { provide: OAuthAuthProvider, useValue: mockOAuth },
        { provide: AuthService, useValue: mockAuthService },
        { provide: SessionMigrationService, useValue: mockMigration },
        { provide: ActivatedRoute, useValue: makeRoute(queryParams) },
      ],
    }).compileComponents();

    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate').mockResolvedValue(true);

    fixture = TestBed.createComponent(AuthCallbackComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  }

  afterEach(() => {
    vi.restoreAllMocks();
    TestBed.resetTestingModule();
  });

  // ==========================================================================
  // URL param extraction
  // ==========================================================================

  it('extracts code from query params', async () => {
    await setup();
    expect(component.code()).toBe('abc');
  });

  it('extracts state from query params', async () => {
    await setup();
    expect(component.state()).toBe('xyz');
  });

  it('extracts provider label from query params', async () => {
    await setup();
    expect(component.providerLabel()).toBe('github');
  });

  it('sets code to empty string when absent from URL', async () => {
    await setup({ state: 'xyz' });
    expect(component.code()).toBe('');
  });

  it('sets provider label to empty string when absent from URL', async () => {
    await setup({ code: 'abc', state: 'xyz' });
    expect(component.providerLabel()).toBe('');
  });

  // ==========================================================================
  // Custom element rendered in template
  // ==========================================================================

  it('renders the oauth-callback custom element', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector('elohim-imagodei-oauth-callback');
    expect(el).toBeTruthy();
  });

  it('sets code attribute on the element', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector('elohim-imagodei-oauth-callback');
    expect(el.getAttribute('code')).toBe('abc');
  });

  it('sets state attribute on the element', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector('elohim-imagodei-oauth-callback');
    expect(el.getAttribute('state')).toBe('xyz');
  });

  it('sets provider-label attribute on the element', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector('elohim-imagodei-oauth-callback');
    expect(el.getAttribute('provider-label')).toBe('github');
  });

  // ==========================================================================
  // exchangeCode callback wiring — success path
  // ==========================================================================

  it('wires exchangeCode to OAuthAuthProvider.handleCallback', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (c: string, s: string) => Promise<unknown>;

    await exchangeCode('abc', 'xyz');

    expect(mockOAuth.handleCallback).toHaveBeenCalledWith('abc', 'xyz');
  });

  it('calls AuthService.setAuthFromResult on successful exchange', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (c: string, s: string) => Promise<unknown>;

    await exchangeCode('abc', 'xyz');

    expect(mockAuthService.setAuthFromResult).toHaveBeenCalledWith(successResult);
  });

  it('clears URL params after a successful exchange', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (c: string, s: string) => Promise<unknown>;

    await exchangeCode('abc', 'xyz');

    expect(mockOAuth.clearCallbackParams).toHaveBeenCalled();
  });

  it('resolves with { session: result } on success', async () => {
    await setup();
    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (
      c: string,
      s: string
    ) => Promise<{ session: unknown }>;

    const outcome = await exchangeCode('abc', 'xyz');

    expect(outcome).toEqual({ session: successResult });
  });

  // ==========================================================================
  // exchangeCode callback wiring — failure path
  // ==========================================================================

  it('throws when OAuthAuthProvider.handleCallback returns success:false', async () => {
    await setup();
    mockOAuth.handleCallback.mockResolvedValue({
      success: false,
      error: 'invalid_grant',
      code: 'INVALID_CREDENTIALS',
    });

    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (c: string, s: string) => Promise<unknown>;

    await expect(exchangeCode('abc', 'xyz')).rejects.toThrow('invalid_grant');
  });

  it('clears URL params even on a failed exchange', async () => {
    await setup();
    mockOAuth.handleCallback.mockResolvedValue({
      success: false,
      error: 'expired',
      code: 'TOKEN_EXPIRED',
    });

    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (c: string, s: string) => Promise<unknown>;

    await exchangeCode('abc', 'xyz').catch(() => {});

    expect(mockOAuth.clearCallbackParams).toHaveBeenCalled();
  });

  it('does NOT call setAuthFromResult on a failed exchange', async () => {
    await setup();
    mockOAuth.handleCallback.mockResolvedValue({
      success: false,
      error: 'denied',
      code: 'INVALID_CREDENTIALS',
    });

    const el = fixture.nativeElement.querySelector(
      'elohim-imagodei-oauth-callback'
    ) as HTMLElement & Record<string, unknown>;
    const exchangeCode = el['exchangeCode'] as (c: string, s: string) => Promise<unknown>;

    await exchangeCode('abc', 'xyz').catch(() => {});

    expect(mockAuthService.setAuthFromResult).not.toHaveBeenCalled();
  });

  // ==========================================================================
  // onSuccess — navigation
  // ==========================================================================

  it('navigates to /lamad by default on success', async () => {
    await setup();
    mockOAuth.consumeReturnUrl.mockReturnValue(null);

    await component.onSuccess(new CustomEvent('success', { detail: { session: successResult } }));

    expect(router.navigate).toHaveBeenCalledWith(['/lamad']);
  });

  it('navigates to stored returnUrl when present', async () => {
    await setup();
    mockOAuth.consumeReturnUrl.mockReturnValue('/community/governance');

    await component.onSuccess(new CustomEvent('success', { detail: { session: successResult } }));

    expect(router.navigate).toHaveBeenCalledWith(['/community/governance']);
  });

  it('consumes the return URL (calls consumeReturnUrl) on success', async () => {
    await setup();

    await component.onSuccess(new CustomEvent('success', { detail: { session: {} } }));

    expect(mockOAuth.consumeReturnUrl).toHaveBeenCalled();
  });

  // ==========================================================================
  // Visitor → hosted carry-over — the upgrade lands HERE, not at a form
  // ==========================================================================

  it('carries a visitor session onto the fresh profile, naming the issued identifier', async () => {
    await setup();
    mockMigration.canMigrate.mockReturnValue(true);
    const exchangeCode = (component.cbRef!.nativeElement as unknown as Record<string, unknown>)[
      'exchangeCode'
    ] as (c: string, s: string) => Promise<unknown>;
    await exchangeCode('abc', 'xyz');

    await component.onSuccess(new CustomEvent('success', { detail: { session: successResult } }));

    expect(mockMigration.applySessionToProfile).toHaveBeenCalledWith('matthew@alpha.elohim.host');
  });

  it('does nothing when there is no visitor session to carry', async () => {
    await setup();
    mockMigration.canMigrate.mockReturnValue(false);

    await component.onSuccess(new CustomEvent('success', { detail: { session: successResult } }));

    expect(mockMigration.applySessionToProfile).not.toHaveBeenCalled();
    expect(router.navigate).toHaveBeenCalled();
  });

  it('still lands the human somewhere when the carry-over fails', async () => {
    await setup();
    mockMigration.canMigrate.mockReturnValue(true);
    mockMigration.applySessionToProfile.mockResolvedValue({
      success: false,
      error: 'conductor unreachable',
    });
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    await component.onSuccess(new CustomEvent('success', { detail: { session: successResult } }));

    expect(router.navigate).toHaveBeenCalled();
    expect(warn).toHaveBeenCalled();
  });

  // ==========================================================================
  // onError — logging, no crash
  // ==========================================================================

  it('logs error details on OAuth callback failure', async () => {
    await setup();
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    component.onError(
      new CustomEvent('error', { detail: { reason: 'invalid_grant', recoverable: true } })
    );

    expect(consoleSpy).toHaveBeenCalledWith('OAuth callback error:', 'invalid_grant');
    consoleSpy.mockRestore();
  });

  it('does not throw when onError is called', async () => {
    await setup();
    vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => {
      component.onError(
        new CustomEvent('error', { detail: { reason: 'network_error', recoverable: true } })
      );
    }).not.toThrow();
  });

  it('does not navigate on error', async () => {
    await setup();
    vi.spyOn(console, 'error').mockImplementation(() => {});

    component.onError(
      new CustomEvent('error', { detail: { reason: 'access_denied', recoverable: false } })
    );

    expect(router.navigate).not.toHaveBeenCalled();
  });
});
