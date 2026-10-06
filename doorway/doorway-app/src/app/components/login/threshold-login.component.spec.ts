import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { IDENTITY_ELEMENTS } from '../../elements/identity-elements';
import { AuthStateService } from '../../services/auth-state.service';

import { ThresholdLoginComponent } from './threshold-login.component';

/** Let pending zone promises settle so the next HttpClient request is issued. */
const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

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
        { provide: IDENTITY_ELEMENTS, useValue: () => Promise.resolve(false) },
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

/**
 * Returning a person to the page that sent them here (authGuard's returnUrl) —
 * how the device approval page gets its person back after sign-in.
 */
describe('ThresholdLoginComponent — return to the page that asked', () => {
  let httpMock: HttpTestingController;
  let mockRouter: {
    navigate: ReturnType<typeof vi.fn>;
    navigateByUrl: ReturnType<typeof vi.fn>;
  };

  const AUTH_RESPONSE = {
    token: 'jwt',
    humanId: 'human-matthew',
    agentPubKey: 'uhCAk-matthew',
    expiresAt: '2026-06-05T00:00:00Z',
    identifier: 'matthew',
    isSteward: false,
  };
  const CONSENT_URL = '/consent/device?request=eyJhIjoxfQ';

  function setup(params: Record<string, string>) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ThresholdLoginComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ActivatedRoute, useValue: { snapshot: { queryParams: params } } },
        { provide: Router, useValue: mockRouter },
        {
          provide: AuthStateService,
          useValue: { storeToken: vi.fn(), refresh: vi.fn().mockResolvedValue(undefined) },
        },
        { provide: IDENTITY_ELEMENTS, useValue: () => Promise.resolve(false) },
      ],
    });
    httpMock = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(ThresholdLoginComponent);
    fixture.detectChanges();
    const component = fixture.componentInstance;
    component.form.identifier = 'matthew';
    component.form.password = 'pw';
    return { fixture, component };
  }

  beforeEach(() => {
    mockRouter = {
      navigate: vi.fn().mockResolvedValue(true),
      navigateByUrl: vi.fn().mockResolvedValue(true),
    };
  });

  afterEach(() => httpMock.verify());

  it('brings the person back to the device approval page after signing in', async () => {
    const { component } = setup({ returnUrl: CONSENT_URL });
    const submit = component.onSubmit();
    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE);
    await submit;

    expect(mockRouter.navigateByUrl).toHaveBeenCalledWith(CONSENT_URL);
    expect(mockRouter.navigate).not.toHaveBeenCalled();
  });

  it('says why sign-in is needed when a device is waiting, and nothing more', () => {
    const { fixture } = setup({ returnUrl: CONSENT_URL });
    const line = fixture.nativeElement.querySelector(
      '[data-testid="threshold-device-consent-context"]'
    );
    expect(line?.textContent?.trim()).toBe(
      'Sign in to review a device that is asking to act for you.'
    );
  });

  it('shows no device line for an ordinary sign-in', () => {
    const { fixture } = setup({});
    expect(
      fixture.nativeElement.querySelector('[data-testid="threshold-device-consent-context"]')
    ).toBeNull();
  });

  it('ignores a return target that would leave the app', async () => {
    const { component } = setup({ returnUrl: 'https://evil.example/' });
    const submit = component.onSubmit();
    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE);
    await submit;

    expect(mockRouter.navigateByUrl).not.toHaveBeenCalled();
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
  });

  it('explains an empty form instead of sending it', () => {
    const { fixture, component } = setup({});
    component.form.identifier = '';
    component.form.password = '';
    void component.onSubmit(false);
    fixture.detectChanges();

    const identifier: HTMLInputElement = fixture.nativeElement.querySelector('#identifier');
    expect(identifier.getAttribute('aria-invalid')).toBe('true');
    expect(identifier.getAttribute('aria-describedby')).toContain('identifier-error');
    expect(fixture.nativeElement.querySelector('#identifier-error')?.textContent).toContain(
      'Enter your username.'
    );
    httpMock.expectNone('/auth/login');
  });
});

/**
 * The witness trail takes the place of the waiting indicator after the person
 * clicks Sign in — never before, and it never holds navigation back.
 */
describe('ThresholdLoginComponent — witness trail while signing in', () => {
  let httpMock: HttpTestingController;
  let navigate: ReturnType<typeof vi.fn>;

  const AUTH_RESPONSE = {
    token: 'jwt',
    humanId: 'human-matthew',
    agentPubKey: 'uhCAk-matthew',
    expiresAt: '2026-06-05T00:00:00Z',
    identifier: 'matthew',
    isSteward: false,
  };

  /** `loader` stands in for the lazily loaded element bundle. */
  function setup(loader: () => Promise<boolean>) {
    navigate = vi.fn().mockResolvedValue(true);
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ThresholdLoginComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ActivatedRoute, useValue: { snapshot: { queryParams: {} } } },
        { provide: Router, useValue: { navigate, navigateByUrl: vi.fn() } },
        {
          provide: AuthStateService,
          useValue: { storeToken: vi.fn(), refresh: vi.fn().mockResolvedValue(undefined) },
        },
        { provide: IDENTITY_ELEMENTS, useValue: loader },
      ],
    });
    httpMock = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(ThresholdLoginComponent);
    fixture.detectChanges();
    const component = fixture.componentInstance;
    component.form.identifier = 'matthew';
    component.form.password = 'pw';
    return { fixture, component };
  }

  const trailIn = (fixture: { nativeElement: HTMLElement }) =>
    fixture.nativeElement.querySelector('elohim-imagodei-witness-trail') as
      | (HTMLElement & { steps?: unknown })
      | null;

  afterEach(() => httpMock.verify());

  it('renders no trail before the person submits, and loads nothing', () => {
    const loader = vi.fn().mockResolvedValue(true);
    const { fixture, component } = setup(loader);

    expect(trailIn(fixture)).toBeNull();
    expect(component.trailSteps()).toBeNull();
    expect(loader).not.toHaveBeenCalled();
  });

  it('shows only the doorway checking the password while the request is in flight', async () => {
    const { fixture, component } = setup(() => Promise.resolve(true));
    const submit = component.onSubmit();
    await settle();
    fixture.detectChanges();

    const working = [
      {
        id: 'doorway-check',
        act: 'checked',
        relation: 'your-doorway',
        label: component.gatewayDomain(),
        state: 'working',
      },
    ];
    expect(component.trailSteps()).toEqual(working);
    expect(trailIn(fixture)?.steps).toEqual(working);
    expect(trailIn(fixture)?.getAttribute('layout')).toBe('list');
    expect(
      fixture.nativeElement.querySelector('[data-testid="threshold-submit"]').textContent
    ).toContain('Signing in…');

    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE);
    await submit;
    expect(component.trailSteps()).toEqual([{ ...working[0], state: 'done' }]);
  });

  it('marks the step failed and leaves the error where it already is', async () => {
    const { fixture, component } = setup(() => Promise.resolve(true));
    const submit = component.onSubmit();
    await settle();
    httpMock
      .expectOne('/auth/login')
      .flush({ error: 'Invalid credentials' }, { status: 401, statusText: 'Unauthorized' });
    await submit;
    fixture.detectChanges();

    const [step] = component.trailSteps() ?? [];
    expect(step.state).toBe('failed');
    expect(step.note).toBeUndefined();
    expect(
      fixture.nativeElement.querySelector('[data-testid="threshold-error"]').textContent
    ).toContain('Invalid credentials');
    expect(trailIn(fixture)).not.toBeNull();
  });

  it('shows nothing when the element has not loaded before the request finishes', async () => {
    const { fixture, component } = setup(() => new Promise<boolean>(() => undefined));
    const submit = component.onSubmit();
    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE);
    await submit;
    fixture.detectChanges();

    expect(component.trailElementReady()).toBe(false);
    expect(trailIn(fixture)).toBeNull();
  });

  it.each([
    ['absent (never loads)', () => new Promise<boolean>(() => undefined)],
    ['present (loaded)', () => Promise.resolve(true)],
  ])('navigates as soon as sign-in succeeds with the element %s', async (_label, loader) => {
    const { component } = setup(loader);
    const submit = component.onSubmit();
    expect(navigate).not.toHaveBeenCalled();

    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE);
    await submit; // resolves the moment navigation is issued — nothing awaits the trail

    expect(navigate).toHaveBeenCalledTimes(1);
    expect(navigate).toHaveBeenCalledWith(['/dashboard']);
  });

  it('sends one request even if the person clicks twice', async () => {
    const { component } = setup(() => Promise.resolve(true));
    const first = component.onSubmit();
    const second = component.onSubmit();
    httpMock.expectOne('/auth/login').flush(AUTH_RESPONSE);
    await Promise.all([first, second]);
  });
});
