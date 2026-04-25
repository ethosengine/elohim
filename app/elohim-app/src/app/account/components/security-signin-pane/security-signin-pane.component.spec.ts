import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { SecuritySigninPaneComponent } from './security-signin-pane.component';
import { AccountService } from '../../services/account.service';
import { RevocationService } from '../../services/revocation.service';

const mockAccountService = {
  account: signal(null),
  loading: signal(false),
  error: signal(null),
  refresh: async () => {},
};

const mockRevocationService = {
  revocations: signal([]),
  pendingRecovery: signal([]),
  error: signal(null),
  selfRevoke: async () => false,
  voteOnRecovery: async () => false,
};

describe('SecuritySigninPaneComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [SecuritySigninPaneComponent],
      providers: [
        provideRouter([]),
        { provide: AccountService, useValue: mockAccountService },
        { provide: RevocationService, useValue: mockRevocationService },
      ],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render pane title with data-testid', () => {
    const fixture = TestBed.createComponent(SecuritySigninPaneComponent);
    fixture.detectChanges();
    const title = (fixture.nativeElement as HTMLElement).querySelector(
      '[data-testid="pane-title-security"]',
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
});
