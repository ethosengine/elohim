import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthStateService } from '../../services/auth-state.service';

import { ThresholdLoginComponent } from './threshold-login.component';

/** Let pending zone promises settle so the next HttpClient request is issued. */
const settle = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

/**
 * Steward portal-handoff contract (GAP-2c).
 *
 * When /auth/login reports a reachable portalHostUrl, the component mints a
 * single-use transfer code from the EXISTING GET /auth/session-token endpoint
 * (Bearer doorway JWT, 60s TTL, consumed exactly once) and redirects with that
 * code — the doorway JWT must NEVER ride a URL. The issuer origin rides along
 * as doorway_url so the portal's storage can redeem the code server-to-server
 * via GET {doorway_url}/auth/exchange-session. The portal-handoff deliberately
 * REUSES the session-transfer pair rather than minting a parallel mechanism.
 */
describe('ThresholdLoginComponent — steward portal-handoff', () => {
  let httpMock: HttpTestingController;
  let originalLocation: Location;
  let hrefSink: { href: string; origin: string; hostname: string };
  let mockAuthState: { storeToken: ReturnType<typeof vi.fn>; refresh: ReturnType<typeof vi.fn> };
  let mockRouter: { navigate: ReturnType<typeof vi.fn> };

  const AUTH_RESPONSE_STEWARD = {
    token: 'jwt-secret-never-in-url',
    humanId: 'human-matthew',
    agentPubKey: 'uhCAk-matthew',
    expiresAt: '2026-06-05T00:00:00Z',
    identifier: 'matthew',
    isSteward: true,
    portalHostUrl: 'https://matthew.steward.example/account',
  };

  function setup(params: Record<string, string> = {}): ThresholdLoginComponent {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ThresholdLoginComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ActivatedRoute, useValue: { snapshot: { queryParams: params } } },
        { provide: Router, useValue: mockRouter },
        { provide: AuthStateService, useValue: mockAuthState },
      ],
    });
    httpMock = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(ThresholdLoginComponent);
    const component = fixture.componentInstance;
    fixture.detectChanges(); // ngOnInit → parseOAuthParams
    component.form.identifier = 'matthew';
    component.form.password = 'pw';
    return component;
  }

  beforeEach(() => {
    mockAuthState = { storeToken: vi.fn(), refresh: vi.fn().mockResolvedValue(undefined) };
    mockRouter = { navigate: vi.fn().mockResolvedValue(true) };

    // Stub location so the client-driven redirect is observable and does not
    // navigate the jsdom page (same pattern as elohim-app's security pane spec).
    originalLocation = globalThis.location;
    hrefSink = {
      href: 'https://alpha.elohim.host/threshold/login',
      origin: 'https://alpha.elohim.host',
      hostname: 'alpha.elohim.host',
    };
    Object.defineProperty(globalThis, 'location', {
      value: hrefSink,
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
    httpMock.verify();
  });

  it('redirects a steward to the portal host with a MINTED code, never the JWT', async () => {
    const component = setup();
    const submit = component.onSubmit();

    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE_STEWARD);
    await settle(); // let the mint request fire

    const mint = httpMock.expectOne('/auth/session-token');
    expect(mint.request.method).toBe('GET');
    expect(mint.request.headers.get('Authorization')).toBe('Bearer jwt-secret-never-in-url');
    mint.flush({ sessionToken: 'minted-single-use-code', expiresAt: 1780000000 });

    await submit;

    const target = new URL(hrefSink.href);
    expect(target.origin + target.pathname).toBe('https://matthew.steward.example/account');
    expect(target.searchParams.get('session_token')).toBe('minted-single-use-code');
    expect(target.searchParams.get('doorway_url')).toBe('https://alpha.elohim.host');
    // The JWT must appear nowhere in the redirect URL.
    expect(hrefSink.href).not.toContain('jwt-secret-never-in-url');
  });

  it('preserves OAuth params (incl. scope) on the handoff redirect when present', async () => {
    const component = setup({
      client_id: 'elohim-app',
      redirect_uri: 'https://app.example/cb',
      response_type: 'code',
      state: 'xyz-state',
      scope: 'openid profile',
    });
    const submit = component.onSubmit();

    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE_STEWARD);
    await settle();
    httpMock
      .expectOne('/auth/session-token')
      .flush({ sessionToken: 'minted-code', expiresAt: 1780000000 });
    await submit;

    const target = new URL(hrefSink.href);
    expect(target.searchParams.get('client_id')).toBe('elohim-app');
    expect(target.searchParams.get('redirect_uri')).toBe('https://app.example/cb');
    expect(target.searchParams.get('response_type')).toBe('code');
    expect(target.searchParams.get('state')).toBe('xyz-state');
    expect(target.searchParams.get('scope')).toBe('openid profile');
  });

  it('falls through to local auth when the mint fails (login never blocked)', async () => {
    const component = setup();
    const submit = component.onSubmit();

    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE_STEWARD);
    await settle();
    httpMock
      .expectOne('/auth/session-token')
      .flush({ error: 'unavailable' }, { status: 503, statusText: 'Service Unavailable' });
    await submit;

    // No portal redirect happened — the JWT was never placed on a URL.
    expect(hrefSink.href).toBe('https://alpha.elohim.host/threshold/login');
    // The local (non-handoff) path completed instead.
    expect(mockAuthState.storeToken).toHaveBeenCalledWith('jwt-secret-never-in-url');
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
  });

  it('non-steward login never touches the mint endpoint', async () => {
    const component = setup();
    const submit = component.onSubmit();

    httpMock
      .expectOne('/auth/login')
      .flush({ ...AUTH_RESPONSE_STEWARD, isSteward: false, portalHostUrl: undefined });
    await submit;

    httpMock.expectNone('/auth/session-token');
    expect(hrefSink.href).toBe('https://alpha.elohim.host/threshold/login');
    expect(mockAuthState.storeToken).toHaveBeenCalledWith('jwt-secret-never-in-url');
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
  });
});
