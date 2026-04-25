import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';

import { SelfRevokeComponent } from './self-revoke.component';
import { AccountService } from '../../../services/account.service';
import { RevocationService } from '../../../services/revocation.service';
import type { AccountView } from '@app/generated/account-view';

const baseAccount: AccountView = {
  human: {
    id: 'human-1',
    displayName: 'Test Human',
    affinities: [],
    profileReach: 'trusted',
    hAppId: 'happ-1',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  },
  activeKeyRotation: {
    dhtAnchorHash: 'anchor-1',
    humanAgentPubkey: 'human-pub',
    newAgentPubkey: 'new-pub-key-abcdef',
    supersededAgentPubkey: 'old-pub',
    recoveryRequestHash: 'req-hash',
    authorityKind: 'intimateQuorum',
    authorityJson: '{}',
    rotatedAt: '2024-06-01T00:00:00Z',
  },
  recentRevocations: [],
  pendingRecoveryRequests: [],
  emergencyContacts: [],
  portalHosts: [],
  isSteward: false,
  hasLocalConductor: false,
};

describe('SelfRevokeComponent', () => {
  const mockAccountService = {
    account: signal<AccountView | null>(baseAccount),
    loading: signal(false),
    error: signal(null),
    refresh: async () => {},
  };

  const mockRevocationService = {
    error: signal<string | null>(null),
    selfRevoke: async (_key: string) => true as boolean,
    revocations: signal([]),
    pendingRecovery: signal([]),
  };

  beforeEach(async () => {
    mockAccountService.account.set(baseAccount);
    mockRevocationService.error.set(null);
    await TestBed.configureTestingModule({
      imports: [SelfRevokeComponent],
      providers: [
        { provide: AccountService, useValue: mockAccountService },
        { provide: RevocationService, useValue: mockRevocationService },
      ],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(SelfRevokeComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render self-revoke-start button when an active key exists', () => {
    const fixture = TestBed.createComponent(SelfRevokeComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="self-revoke-start"]')).toBeTruthy();
  });

  it('should show confirm and cancel buttons after clicking start', () => {
    const fixture = TestBed.createComponent(SelfRevokeComponent);
    fixture.detectChanges();
    const start = fixture.nativeElement.querySelector(
      '[data-testid="self-revoke-start"]',
    ) as HTMLButtonElement;
    start.click();
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="self-revoke-confirm"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="self-revoke-cancel"]')).toBeTruthy();
  });

  it('should display self-revoke-error on 503 (service returns false + error signal)', async () => {
    const errorMsg = '[M5 stub — full self-revocation lands in Phase 11]';
    mockRevocationService.selfRevoke = async (_key: string) => {
      mockRevocationService.error.set(errorMsg);
      return false;
    };

    const fixture = TestBed.createComponent(SelfRevokeComponent);
    fixture.detectChanges();

    // Open confirm step
    (fixture.nativeElement.querySelector('[data-testid="self-revoke-start"]') as HTMLElement).click();
    fixture.detectChanges();

    // Trigger confirm
    await fixture.componentInstance.confirm();
    fixture.detectChanges();

    const errorEl = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="self-revoke-error"]',
    );
    expect(errorEl).toBeTruthy();
    expect(errorEl?.textContent).toContain('Phase 11');
  });
});
