/**
 * Identity Default Redirect Guard Tests
 *
 * Tests auth-aware redirect for the /identity root path.
 */

import { TestBed } from '@angular/core/testing';
import { Router, ActivatedRouteSnapshot, RouterStateSnapshot, UrlTree } from '@angular/router';

import { identityDefaultRedirectGuard } from './identity-redirect.guard';
import { IdentityService } from '../services/identity.service';
import { vi, Mock } from 'vitest';

describe('identityDefaultRedirectGuard', () => {
  let mockRouter: any;
  let mockIdentityService: any;
  let mockRoute: ActivatedRouteSnapshot;
  let mockState: RouterStateSnapshot;

  beforeEach(() => {
    mockRouter = {
      createUrlTree: vi.fn(),
    };

    mockIdentityService = {
      isAuthenticated: vi.fn(),
      mode: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        { provide: Router, useValue: mockRouter },
        { provide: IdentityService, useValue: mockIdentityService },
      ],
    });

    mockRoute = {} as ActivatedRouteSnapshot;
    mockState = { url: '/identity' } as RouterStateSnapshot;
  });

  it('should redirect authenticated hosted users to /identity/profile', () => {
    (mockIdentityService.mode as Mock).mockReturnValue('hosted');
    mockIdentityService.isAuthenticated.mockReturnValue(true);

    const profileTree = {} as UrlTree;
    mockRouter.createUrlTree.mockReturnValue(profileTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(profileTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/profile']);
  });

  it('should redirect authenticated steward users to /identity/profile', () => {
    (mockIdentityService.mode as Mock).mockReturnValue('steward');
    mockIdentityService.isAuthenticated.mockReturnValue(true);

    const profileTree = {} as UrlTree;
    mockRouter.createUrlTree.mockReturnValue(profileTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(profileTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/profile']);
  });

  it('should redirect unauthenticated users to /identity/login', () => {
    (mockIdentityService.mode as Mock).mockReturnValue('session');
    mockIdentityService.isAuthenticated.mockReturnValue(false);

    const loginTree = {} as UrlTree;
    mockRouter.createUrlTree.mockReturnValue(loginTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(loginTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login']);
  });

  it('should redirect session-mode users to /identity/login', () => {
    (mockIdentityService.mode as Mock).mockReturnValue('session');
    mockIdentityService.isAuthenticated.mockReturnValue(true);

    const loginTree = {} as UrlTree;
    mockRouter.createUrlTree.mockReturnValue(loginTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(loginTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login']);
  });

  it('should redirect anonymous users to /identity/login', () => {
    (mockIdentityService.mode as Mock).mockReturnValue('anonymous');
    mockIdentityService.isAuthenticated.mockReturnValue(false);

    const loginTree = {} as UrlTree;
    mockRouter.createUrlTree.mockReturnValue(loginTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(loginTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login']);
  });
});
