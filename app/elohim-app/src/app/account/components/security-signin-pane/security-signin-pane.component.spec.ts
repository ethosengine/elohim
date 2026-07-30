import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { SecuritySigninPaneComponent } from './security-signin-pane.component';
import { AccountService } from '../../services/account.service';
import { HandoffService } from '../../services/handoff.service';
import { RevocationService } from '../../services/revocation.service';
import type { AccountView, PortalHostView } from '@app/generated/account-view';

// ---------------------------------------------------------------------------
// Fixture builders
// ---------------------------------------------------------------------------

function makePortalHost(overrides: Partial<PortalHostView> = {}): PortalHostView {
  return {
    humanId: 'uhCkk-human-anchor',
    hostUrl: 'https://matthew.steward.example/account',
    label: 'Matthew’s steward',
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

const mockRevocationService = {
  revocations: signal([]),
  pendingRecovery: signal([]),
  error: signal(null),
  selfRevoke: async () => false,
  voteOnRecovery: async () => false,
};

const REDIRECT_TESTID = '[data-testid="portal-host-redirect"]';

describe('SecuritySigninPaneComponent', () => {
  let accountSignal: ReturnType<typeof signal<AccountView | null>>;
  let mockAccountService: {
    account: typeof accountSignal;
    loading: ReturnType<typeof signal<boolean>>;
    error: ReturnType<typeof signal<string | null>>;
    refresh: () => Promise<void>;
  };
  let mintResult: string | null;
  let mintCalls: number;
  let mockHandoffService: { mintHandoffToken: () => Promise<string | null> };
  let originalLocation: Location;
  let hrefSink: { href: string; origin: string };

  beforeEach(async () => {
    accountSignal = signal<AccountView | null>(null);
    mockAccountService = {
      account: accountSignal,
      loading: signal(false),
      error: signal<string | null>(null),
      refresh: async () => {},
    };
    // Mint seam: the redirect must carry a single-use code minted via
    // HandoffService (GET /auth/session-token) — never the JWT itself.
    mintResult = 'minted-single-use-code';
    mintCalls = 0;
    mockHandoffService = {
      mintHandoffToken: async () => {
        mintCalls++;
        return mintResult;
      },
    };

    // Stub globalThis.location so the client-driven redirect is observable and
    // does not navigate the jsdom page. The test URL carries OAuth params so we
    // can assert they are preserved.
    originalLocation = globalThis.location;
    hrefSink = {
      href: 'http://localhost:4200/account/security?client_id=elohim-app&redirect_uri=https%3A%2F%2Fdoorway.example%2Fcb&response_type=code&state=xyz',
      origin: 'http://localhost:4200',
    };
    Object.defineProperty(globalThis, 'location', {
      value: hrefSink,
      writable: true,
      configurable: true,
    });

    await TestBed.configureTestingModule({
      imports: [SecuritySigninPaneComponent],
      providers: [
        provideRouter([]),
        { provide: AccountService, useValue: mockAccountService },
        { provide: RevocationService, useValue: mockRevocationService },
        { provide: HandoffService, useValue: mockHandoffService },
      ],
    }).compileComponents();
  });

  afterEach(() => {
    Object.defineProperty(globalThis, 'location', {
      value: originalLocation,
      writable: true,
      configurable: true,
    });
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render pane title with data-testid', () => {
    const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
    fixture.detectChanges();
    const title = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="pane-title-security"]'
    );
    expect(title).toBeTruthy();
    expect(title?.textContent?.trim()).toBe('Security & sign-in');
  });

  it('should render all 4 section cards with data-testid', () => {
    const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="my-keys-section"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="self-revoke-section"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="vote-as-ec-section"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="lost-key-section"]')).toBeTruthy();
  });

  // ─────────────────────────────────────────────────────────────────────────
  // GAP-3 — "Manage from your steward →" portal-host redirect
  // Spec: genesis/a2o/features/auth/recovery/recovery-m5-doorway-handoff-to-steward.feature
  // ─────────────────────────────────────────────────────────────────────────

  describe('portal-host redirect (steward + reachable host)', () => {
    it('renders the redirect button when isSteward and a reachable portal host exists', () => {
      accountSignal.set(
        makeAccount({
          isSteward: true,
          portalHosts: [makePortalHost({ lastReachableAt: '2026-06-04T00:00:00Z' })],
        })
      );
      const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
      fixture.detectChanges();

      const btn = (fixture.nativeElement as HTMLElement).querySelector(REDIRECT_TESTID);
      expect(btn).toBeTruthy();
      expect(btn?.textContent?.trim()).toBe('Manage from your steward →');
    });

    it('does NOT render the redirect button when isSteward is false', () => {
      accountSignal.set(
        makeAccount({
          isSteward: false,
          portalHosts: [makePortalHost({ lastReachableAt: '2026-06-04T00:00:00Z' })],
        })
      );
      const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
      fixture.detectChanges();

      const btn = (fixture.nativeElement as HTMLElement).querySelector(REDIRECT_TESTID);
      expect(btn).toBeNull();
    });

    it('does NOT render the button for a steward with NO reachable host, hosted view intact', () => {
      accountSignal.set(
        makeAccount({
          isSteward: true,
          // Host present but never reachable (probe failed / fall-through scenario).
          portalHosts: [makePortalHost({ lastReachableAt: null })],
        })
      );
      const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
      fixture.detectChanges();
      const el = fixture.nativeElement as HTMLElement;

      expect(el.querySelector(REDIRECT_TESTID)).toBeNull();
      // Hosted security view still renders normally.
      expect(el.querySelector('[data-testid="pane-title-security"]')).toBeTruthy();
      expect(el.querySelector('[data-testid="my-keys-section"]')).toBeTruthy();
    });

    it('does NOT render the button for a steward with no portal hosts at all', () => {
      accountSignal.set(makeAccount({ isSteward: true, portalHosts: [] }));
      const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
      fixture.detectChanges();

      expect((fixture.nativeElement as HTMLElement).querySelector(REDIRECT_TESTID)).toBeNull();
    });

    it('carries accessible labeling (aria-label) on the redirect button', () => {
      accountSignal.set(
        makeAccount({
          isSteward: true,
          portalHosts: [makePortalHost()],
        })
      );
      const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
      fixture.detectChanges();

      const btn = (fixture.nativeElement as HTMLElement).querySelector(REDIRECT_TESTID);
      expect(btn?.getAttribute('aria-label')).toBeTruthy();
    });
  });

  describe('portal-host redirect (click behavior)', () => {
    async function clickRedirect(): Promise<void> {
      accountSignal.set(
        makeAccount({
          isSteward: true,
          portalHosts: [
            makePortalHost({
              hostUrl: 'https://matthew.steward.example/account',
              lastReachableAt: '2026-06-04T00:00:00Z',
            }),
          ],
        })
      );
      const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
      fixture.detectChanges();
      const btn = (fixture.nativeElement as HTMLElement).querySelector(
        REDIRECT_TESTID
      ) as HTMLButtonElement;
      btn.click();
      // Let the async mint-then-redirect handler settle.
      await new Promise(resolve => setTimeout(resolve, 0));
    }

    it('redirects to the portal host URL with a MINTED single-use code, never the JWT', async () => {
      await clickRedirect();
      const target = new URL(hrefSink.href);
      expect(`${target.origin}${target.pathname}`).toBe('https://matthew.steward.example/account');
      expect(mintCalls).toBe(1);
      expect(target.searchParams.get('session_token')).toBe('minted-single-use-code');
      // The redeeming portal needs the issuer origin for the back-channel call.
      expect(target.searchParams.get('doorway_url')).toBe('http://localhost:4200');
    });

    it('preserves OAuth params present on the current URL', async () => {
      await clickRedirect();
      const target = new URL(hrefSink.href);
      expect(target.searchParams.get('client_id')).toBe('elohim-app');
      expect(target.searchParams.get('redirect_uri')).toBe('https://doorway.example/cb');
      expect(target.searchParams.get('response_type')).toBe('code');
      expect(target.searchParams.get('state')).toBe('xyz');
    });

    it('stays on the hosted view when the mint fails — JWT never falls back onto the URL', async () => {
      mintResult = null;
      const before = hrefSink.href;
      await clickRedirect();
      expect(mintCalls).toBe(1);
      expect(hrefSink.href).toBe(before);
    });
  });
});
