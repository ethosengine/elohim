/**
 * IdentityApiService — the credential contract for `/api/v1/identity/me`.
 *
 * `/api/v1/identity/*` is identity-scoped: the doorway resolves the caller from
 * the bearer's claims and tells storage who is asking (`X-Agent-Cid`). A
 * credential-less request is anonymous by construction and storage answers 401
 * — swallowed by this service's `catchError`, but logged by the browser as a
 * console error on every authenticated page load. These tests pin both halves:
 * a signed-in human's request CARRIES the bearer, and an anonymous one is never
 * sent at all.
 */
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { vi } from 'vitest';

import { BrowserSessionTokenStore } from './browser-session-token.store';
import { IdentityApiService } from './identity-api.service';

import type { StoredSession } from '../models/auth.model';

const IDENTITY_ME = '/api/v1/identity/me';

const storedSession = (token: string): StoredSession => ({
  token,
  humanId: 'human-matthew-manager',
  agentPubKey: 'uhCAkIaML',
  identifier: 'matthew.dowell@localhost',
  expiresAt: Math.floor(Date.now() / 1000) + 3600,
});

describe('IdentityApiService', () => {
  let service: IdentityApiService;
  let httpMock: HttpTestingController;
  let get: ReturnType<typeof vi.fn>;

  const configure = (session: StoredSession | null): void => {
    get = vi.fn().mockReturnValue(session);
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        IdentityApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: BrowserSessionTokenStore, useValue: { get } },
      ],
    });
    service = TestBed.inject(IdentityApiService);
    httpMock = TestBed.inject(HttpTestingController);
  };

  afterEach(() => {
    httpMock.verify();
  });

  it('presents the session bearer when a human is signed in', async () => {
    configure(storedSession('jwt-abc'));

    const pending = service.getMyHuman();
    const req = httpMock.expectOne(IDENTITY_ME);

    expect(req.request.headers.get('Authorization')).toBe('Bearer jwt-abc');

    req.flush({
      id: 'human-matthew-manager',
      agentPubKey: 'uhCAkIaML',
      displayName: 'Matthew',
      bio: null,
      affinities: [],
      profileReach: 'commons',
      location: null,
      hAppId: 'elohim',
      createdAt: '2026-09-13T00:00:00Z',
      updatedAt: '2026-09-13T00:00:00Z',
    });

    const result = await pending;
    expect(result?.human.id).toBe('human-matthew-manager');
  });

  it('asks nobody "who am I" without a session — no request, no guaranteed 401', async () => {
    configure(null);

    await expect(service.getMyHuman()).resolves.toBeNull();

    httpMock.expectNone(IDENTITY_ME);
  });
});
