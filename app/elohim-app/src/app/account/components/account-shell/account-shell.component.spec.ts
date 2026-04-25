import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { AccountShellComponent } from './account-shell.component';
import { AccountService } from '../../services/account.service';

describe('AccountShellComponent', () => {
  const mockAccountService = {
    account: signal(null),
    loading: signal(false),
    error: signal(null),
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AccountShellComponent],
      providers: [
        provideRouter([]),
        { provide: AccountService, useValue: mockAccountService },
      ],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(AccountShellComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render the pane nav with data-testid', () => {
    const fixture = TestBed.createComponent(AccountShellComponent);
    fixture.detectChanges();
    const nav = (fixture.nativeElement as HTMLElement).querySelector('[data-testid="account-pane-nav"]');
    expect(nav).toBeTruthy();
  });

  it('should render all 5 nav links', () => {
    const fixture = TestBed.createComponent(AccountShellComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="nav-security"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="nav-personal-info"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="nav-data-privacy"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="nav-people-sharing"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="nav-third-party-apps"]')).toBeTruthy();
  });

  it('should render a router-outlet', () => {
    const fixture = TestBed.createComponent(AccountShellComponent);
    fixture.detectChanges();
    const outlet = (fixture.nativeElement as HTMLElement).querySelector('router-outlet');
    expect(outlet).toBeTruthy();
  });
});
