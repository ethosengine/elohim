import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';

import { VoteAsEcComponent } from './vote-as-ec.component';
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
  activeKeyRotation: null,
  recentRevocations: [],
  pendingRecoveryRequests: [],
  emergencyContacts: [],
  portalHosts: [],
  isSteward: false,
  hasLocalConductor: false,
};

describe('VoteAsEcComponent', () => {
  const mockAccountService = {
    account: signal<AccountView | null>(baseAccount),
    loading: signal(false),
    error: signal(null),
    refresh: async () => {},
  };

  const mockRevocationService = {
    error: signal<string | null>(null),
    voteOnRecovery: async (_id: string, _decision: 'approve' | 'reject') => true as boolean,
    revocations: signal([]),
    pendingRecovery: signal([]),
  };

  beforeEach(async () => {
    mockAccountService.account.set(baseAccount);
    mockRevocationService.error.set(null);
    await TestBed.configureTestingModule({
      imports: [VoteAsEcComponent],
      providers: [
        { provide: AccountService, useValue: mockAccountService },
        { provide: RevocationService, useValue: mockRevocationService },
      ],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(VoteAsEcComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render vote-empty when no pending requests', () => {
    const fixture = TestBed.createComponent(VoteAsEcComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="vote-empty"]')).toBeTruthy();
  });

  it('should render pending request card with approve/reject buttons', () => {
    mockAccountService.account.set({
      ...baseAccount,
      pendingRecoveryRequests: [
        {
          dhtAnchorHash: 'req-anchor-abc',
          humanAgentPubkey: 'human-pub',
          newAgentPubkey: 'new-pub',
          hostingDoorwayPubkey: 'doorway-pub',
          proposedAuthorityKind: 'intimateQuorum',
          proposedAuthorityJson: '{}',
          requestNonce: [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
          humanId: 'human-2',
          requiredWitnessCount: 2,
          createdAt: '2024-07-01T00:00:00Z',
        },
      ],
    });
    const fixture = TestBed.createComponent(VoteAsEcComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="pending-req-anchor-abc"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="approve-req-anchor-abc"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="reject-req-anchor-abc"]')).toBeTruthy();
  });

  it('should display vote-error on 503 (service returns false + error signal)', async () => {
    const errorMsg = '[M5 stub — recovery voting lands in Phase 11]';
    mockRevocationService.voteOnRecovery = async () => {
      mockRevocationService.error.set(errorMsg);
      return false;
    };
    mockAccountService.account.set({
      ...baseAccount,
      pendingRecoveryRequests: [
        {
          dhtAnchorHash: 'req-anchor-abc',
          humanAgentPubkey: 'human-pub',
          newAgentPubkey: 'new-pub',
          hostingDoorwayPubkey: 'doorway-pub',
          proposedAuthorityKind: 'intimateQuorum',
          proposedAuthorityJson: '{}',
          requestNonce: [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
          humanId: 'human-2',
          requiredWitnessCount: 2,
          createdAt: '2024-07-01T00:00:00Z',
        },
      ],
    });

    const fixture = TestBed.createComponent(VoteAsEcComponent);
    fixture.detectChanges();

    await fixture.componentInstance.vote('req-anchor-abc', 'approve');
    fixture.detectChanges();

    const errorEl = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="vote-error"]',
    );
    expect(errorEl).toBeTruthy();
    expect(errorEl?.textContent).toContain('Phase 11');
  });
});
