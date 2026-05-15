import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';

import { KeyListComponent } from './key-list.component';
import { AccountService } from '../../../services/account.service';
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

describe('KeyListComponent', () => {
  const mockAccountService = {
    account: signal<AccountView | null>(null),
    loading: signal(false),
    error: signal(null),
    refresh: async () => {},
  };

  beforeEach(async () => {
    mockAccountService.account.set(null);
    await TestBed.configureTestingModule({
      imports: [KeyListComponent],
      providers: [{ provide: AccountService, useValue: mockAccountService }],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(KeyListComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render empty state when no account loaded', () => {
    const fixture = TestBed.createComponent(KeyListComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="key-list-empty"]')).toBeTruthy();
  });

  it('should render active-key row when activeKeyRotation is present', () => {
    mockAccountService.account.set({
      ...baseAccount,
      activeKeyRotation: {
        dhtAnchorHash: 'anchor-hash-1',
        humanAgentPubkey: 'human-pub-key',
        newAgentPubkey: 'new-pub-key-abcdefgh',
        supersededAgentPubkey: 'old-pub-key',
        recoveryRequestHash: 'req-hash',
        authorityKind: 'intimateQuorum',
        authorityJson: '{}',
        rotatedAt: '2024-06-01T00:00:00Z',
      },
    });
    const fixture = TestBed.createComponent(KeyListComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="active-key"]')).toBeTruthy();
  });

  it('should render revoked key row with dynamic data-testid', () => {
    mockAccountService.account.set({
      ...baseAccount,
      recentRevocations: [
        {
          dhtAnchorHash: 'revoc-hash-xyz',
          id: 'rev-1',
          subjectHumanId: 'human-1',
          revokedKey: 'revoked-pub-key-12345678',
          reason: 'voluntary',
          triggerType: 'voluntary',
          initiatedByCid: 'human-1',
          requiredVotes: 1,
          currentVotes: 1,
          thresholdReached: true,
          createdAt: '2024-05-01T00:00:00Z',
          updatedAt: '2024-05-01T00:00:00Z',
        },
      ],
    });
    const fixture = TestBed.createComponent(KeyListComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="revoked-revoc-hash-xyz"]')).toBeTruthy();
  });
});
