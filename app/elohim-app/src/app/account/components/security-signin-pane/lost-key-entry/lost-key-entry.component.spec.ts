import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { Router } from '@angular/router';

import { LostKeyEntryComponent } from './lost-key-entry.component';
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

describe('LostKeyEntryComponent', () => {
  const mockAccountService = {
    account: signal<AccountView | null>(baseAccount),
    loading: signal(false),
    error: signal(null),
    refresh: async () => {},
  };

  beforeEach(async () => {
    mockAccountService.account.set(baseAccount);
    await TestBed.configureTestingModule({
      imports: [LostKeyEntryComponent],
      providers: [
        provideRouter([]),
        { provide: AccountService, useValue: mockAccountService },
      ],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(LostKeyEntryComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render lost-key-recovery button when account is loaded', () => {
    const fixture = TestBed.createComponent(LostKeyEntryComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="lost-key-recovery"]')).toBeTruthy();
  });

  it('should navigate to /identity/recover when button clicked', () => {
    const fixture = TestBed.createComponent(LostKeyEntryComponent);
    fixture.detectChanges();
    const router = TestBed.inject(Router);
    const navigateSpy = vi.spyOn(router, 'navigate');
    const btn = fixture.nativeElement.querySelector(
      '[data-testid="lost-key-recovery"]',
    ) as HTMLButtonElement;
    btn.click();
    expect(navigateSpy).toHaveBeenCalledWith(['/identity/recover']);
  });
});
