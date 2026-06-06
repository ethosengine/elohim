import { TestBed } from '@angular/core/testing';
import { ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { signal } from '@angular/core';
import { vi } from 'vitest';

import { lamadIdentityGuard } from './lamad-identity.guard';
import { LAMAD_EPR_NAV, LAMAD_IDENTITY } from '../interfaces/cross-pillar.interface';

describe('lamadIdentityGuard', () => {
  let eprNavSpy: { navigate: ReturnType<typeof vi.fn>; ownsPath: ReturnType<typeof vi.fn>; recordHandoff: ReturnType<typeof vi.fn> };
  let identityServiceSpy: { mode: ReturnType<typeof signal>; isAuthenticated: ReturnType<typeof vi.fn> };

  const mockRoute = {} as ActivatedRouteSnapshot;
  const mockState = (url: string) => ({ url } as RouterStateSnapshot);

  const runGuard = (url: string) =>
    TestBed.runInInjectionContext(() =>
      lamadIdentityGuard(mockRoute, mockState(url))
    );

  beforeEach(async () => {
    eprNavSpy = { navigate: vi.fn(), ownsPath: vi.fn(() => true), recordHandoff: vi.fn() };
    identityServiceSpy = {
      mode: signal('hosted'),
      isAuthenticated: vi.fn().mockReturnValue(true),
    };

    await TestBed.configureTestingModule({
      providers: [
        { provide: LAMAD_EPR_NAV, useValue: eprNavSpy },
        { provide: LAMAD_IDENTITY, useValue: identityServiceSpy },
      ],
    }).compileComponents();
  });

  it('should return true for authenticated network users', () => {
    const result = runGuard('/some/path');
    expect(result).toBe(true);
    expect(eprNavSpy.navigate).not.toHaveBeenCalled();
  });

  it('should navigate to login and return false for unauthenticated users', () => {
    identityServiceSpy.isAuthenticated.mockReturnValue(false);

    const url = '/some/protected/path';
    const result = runGuard(url);

    expect(result).toBe(false);
    expect(eprNavSpy.navigate).toHaveBeenCalledWith(
      `/identity/login?returnUrl=${encodeURIComponent('/lamad' + url)}`
    );
  });

  it('should navigate to login and return false for session mode (non-network)', () => {
    identityServiceSpy.mode = signal('session');
    identityServiceSpy.isAuthenticated.mockReturnValue(false);

    const url = '/content/123';
    const result = runGuard(url);

    expect(result).toBe(false);
    expect(eprNavSpy.navigate).toHaveBeenCalledWith(
      `/identity/login?returnUrl=${encodeURIComponent('/lamad' + url)}`
    );
  });
});
