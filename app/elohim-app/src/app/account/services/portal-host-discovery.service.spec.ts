import { TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';

import { PortalHostDiscoveryService } from './portal-host-discovery.service';
import { AccountService } from './account.service';
import type { AccountView, PortalHostView } from '@app/generated/account-view';

function makePortalHost(overrides: Partial<PortalHostView> = {}): PortalHostView {
  return {
    humanId: 'uhCkk-human-anchor',
    hostUrl: 'https://matthew.steward.example/account',
    label: null,
    addedAt: '2026-05-01T00:00:00Z',
    lastReachableAt: '2026-06-04T00:00:00Z',
    reach: 'trusted',
    dhtAnchorHash: 'uhCkk-portal-host',
    ...overrides,
  };
}

function makeAccount(overrides: Partial<AccountView> = {}): AccountView {
  return {
    human: {
      id: 'human-1',
      displayName: 'Matthew',
      affinities: [],
      profileReach: 'trusted',
      hAppId: 'happ-1',
      createdAt: '2026-05-01T00:00:00Z',
      updatedAt: '2026-05-01T00:00:00Z',
    },
    recentRevocations: [],
    pendingRecoveryRequests: [],
    emergencyContacts: [],
    portalHosts: [],
    isSteward: false,
    hasLocalConductor: false,
    ...overrides,
  };
}

describe('PortalHostDiscoveryService — reachability projection', () => {
  let accountSignal: ReturnType<typeof signal<AccountView | null>>;
  let service: PortalHostDiscoveryService;

  beforeEach(() => {
    accountSignal = signal<AccountView | null>(null);
    TestBed.configureTestingModule({
      providers: [
        PortalHostDiscoveryService,
        { provide: AccountService, useValue: { account: accountSignal } },
      ],
    });
    service = TestBed.inject(PortalHostDiscoveryService);
  });

  it('reachablePortalHost is null when there are no hosts', () => {
    accountSignal.set(makeAccount({ portalHosts: [] }));
    expect(service.reachablePortalHost()).toBeNull();
  });

  it('reachablePortalHost is null when the only host has never been reachable', () => {
    accountSignal.set(
      makeAccount({ portalHosts: [makePortalHost({ lastReachableAt: null })] }),
    );
    expect(service.reachablePortalHost()).toBeNull();
  });

  it('reachablePortalHost returns the first host with a lastReachableAt', () => {
    accountSignal.set(
      makeAccount({
        portalHosts: [
          makePortalHost({ hostUrl: 'https://a.example', lastReachableAt: null }),
          makePortalHost({ hostUrl: 'https://b.example', lastReachableAt: '2026-06-04T00:00:00Z' }),
        ],
      }),
    );
    expect(service.reachablePortalHost()?.hostUrl).toBe('https://b.example');
  });

  it('hasReachablePortalHost reflects reachable presence', () => {
    accountSignal.set(makeAccount({ portalHosts: [makePortalHost({ lastReachableAt: null })] }));
    expect(service.hasReachablePortalHost()).toBe(false);

    accountSignal.set(makeAccount({ portalHosts: [makePortalHost()] }));
    expect(service.hasReachablePortalHost()).toBe(true);
  });

  it('shouldOfferStewardRedirect requires BOTH isSteward and a reachable host', () => {
    // steward but unreachable
    accountSignal.set(
      makeAccount({ isSteward: true, portalHosts: [makePortalHost({ lastReachableAt: null })] }),
    );
    expect(service.shouldOfferStewardRedirect()).toBe(false);

    // reachable but not steward
    accountSignal.set(makeAccount({ isSteward: false, portalHosts: [makePortalHost()] }));
    expect(service.shouldOfferStewardRedirect()).toBe(false);

    // both
    accountSignal.set(makeAccount({ isSteward: true, portalHosts: [makePortalHost()] }));
    expect(service.shouldOfferStewardRedirect()).toBe(true);
  });
});
