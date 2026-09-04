import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AuthStateService } from '../../services/auth-state.service';

import { ThresholdRegisterComponent } from './threshold-register.component';

/**
 * Registration wire-shape contract.
 *
 * doorway's `RegisterRequest` is `#[serde(rename_all = "camelCase")]`
 * (doorway-service/src/routes/auth_routes.rs). Unknown keys deserialize into
 * serde defaults SILENTLY — so a snake_case body posted a Display Name the
 * doorway never saw, and the new profile was named after the identifier's
 * local-part instead. These tests pin the camelCase body so that regression
 * cannot return unnoticed.
 */
describe('ThresholdRegisterComponent — /auth/register wire shape', () => {
  let httpMock: HttpTestingController;
  let originalLocation: Location;
  let locationSink: { href: string; origin: string; hostname: string };
  let mockAuthState: { storeToken: ReturnType<typeof vi.fn>; refresh: ReturnType<typeof vi.fn> };
  let mockRouter: { navigate: ReturnType<typeof vi.fn> };

  const AUTH_RESPONSE = {
    token: 'issued-token',
    humanId: 'human-ada',
    agentPubKey: 'uhCAk-ada',
    expiresAt: '2026-06-05T00:00:00Z',
    identifier: 'ada@alpha.elohim.host',
    isSteward: false,
  };

  function setup(): ThresholdRegisterComponent {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ThresholdRegisterComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ActivatedRoute, useValue: { snapshot: { queryParams: {} } } },
        { provide: Router, useValue: mockRouter },
        { provide: AuthStateService, useValue: mockAuthState },
      ],
    });
    httpMock = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(ThresholdRegisterComponent);
    const component = fixture.componentInstance;
    fixture.detectChanges();
    return component;
  }

  beforeEach(() => {
    mockAuthState = { storeToken: vi.fn(), refresh: vi.fn().mockResolvedValue(undefined) };
    mockRouter = { navigate: vi.fn().mockResolvedValue(true) };

    originalLocation = globalThis.location;
    locationSink = {
      href: 'https://doorway-alpha.elohim.host/threshold/register',
      origin: 'https://doorway-alpha.elohim.host',
      hostname: 'doorway-alpha.elohim.host',
    };
    Object.defineProperty(globalThis, 'location', {
      value: locationSink,
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

  it('posts camelCase keys so the typed Display Name actually reaches the doorway', async () => {
    const component = setup();
    component.form.displayName = 'Ada Lovelace';
    component.form.email = 'ada';
    component.form.password = 'a-long-enough-secret';
    component.form.confirmPassword = 'a-long-enough-secret';

    const submit = component.onSubmit();
    const req = httpMock.expectOne('/auth/register');

    expect(req.request.method).toBe('POST');
    const body = req.request.body as Record<string, unknown>;

    expect(body['displayName']).toBe('Ada Lovelace');
    expect(body['identifierType']).toBe('email');
    expect(body['identifier']).toBe('ada');
    expect(Object.keys(body).sort()).toEqual([
      'agentPubKey',
      'displayName',
      'humanId',
      'identifier',
      'identifierType',
      'password',
    ]);

    // The snake_case shape serde silently discards must never come back.
    for (const dropped of ['display_name', 'identifier_type', 'human_id', 'agent_pub_key']) {
      expect(body[dropped]).toBeUndefined();
    }

    req.flush(AUTH_RESPONSE);
    await submit;

    expect(mockRouter.navigate).toHaveBeenCalledWith(['/dashboard']);
  });

  it('never shows a "creating your identity" step — the doorway creates it inside /auth/register', async () => {
    const component = setup();
    component.form.displayName = 'Ada Lovelace';
    component.form.email = 'ada';
    component.form.password = 'a-long-enough-secret';
    component.form.confirmPassword = 'a-long-enough-secret';

    const submit = component.onSubmit();

    // The only in-flight state is the real one: the HTTP call now pending.
    expect(component.state()).toBe('registering');

    httpMock.expectOne('/auth/register').flush(AUTH_RESPONSE);
    await submit;
  });

  it('strips a typed @domain, because the doorway re-qualifies every identifier', () => {
    const component = setup();

    component.onIdentifierChange('ada@gmail.com');
    expect(component.form.email).toBe('ada');

    component.onIdentifierChange('ada');
    expect(component.form.email).toBe('ada');
  });

  it('names the gateway domain the account is actually created at', () => {
    const component = setup();
    // doorway-alpha.elohim.host is the doorway; accounts live at alpha.elohim.host.
    expect(component.gatewayDomain()).toBe('alpha.elohim.host');
  });
});
