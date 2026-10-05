import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { DeviceConsentService, type ConsentViewResponse } from './device-consent.service';

describe('DeviceConsentService — the /auth/consent wire', () => {
  let service: DeviceConsentService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(DeviceConsentService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('posts the request, unchanged, to /auth/consent/view', async () => {
    const request = { clientId: 'epr-cli', label: 'workspace' };
    const view: ConsentViewResponse = {
      clientId: 'epr-cli',
      label: 'workspace',
      deviceFingerprint: 'uhCAk…',
      askedActs: ['device.enroll'],
    };
    const pending = service.view(request);

    const req = http.expectOne('/auth/consent/view');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toBe(request);
    req.flush(view);

    await expect(pending).resolves.toEqual(view);
  });

  it('posts the request and the agreed acts to /auth/consent/agree', async () => {
    const body = { request: { clientId: 'epr-cli' }, agreedActs: ['device.enroll' as const] };
    const pending = service.agree(body);

    const req = http.expectOne('/auth/consent/agree');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(body);
    req.flush({ returnTarget: { kind: 'display', value: 'K7QF' }, expiresAt: 1 });

    await expect(pending).resolves.toEqual({
      returnTarget: { kind: 'display', value: 'K7QF' },
      expiresAt: 1,
    });
  });
});
