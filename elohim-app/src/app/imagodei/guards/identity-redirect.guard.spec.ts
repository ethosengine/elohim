/**
 * Identity Default Redirect Guard Tests
 *
 * Tests auth-aware redirect for the /identity root path.
 */

import { TestBed } from '@angular/core/testing';
import { Router, ActivatedRouteSnapshot, RouterStateSnapshot, UrlTree } from '@angular/router';

import { identityDefaultRedirectGuard } from './identity-redirect.guard';
import { IdentityService } from '../services/identity.service';

describe('identityDefaultRedirectGuard', () => {
  let mockRouter: jasmine.SpyObj<Router>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockRoute: ActivatedRouteSnapshot;
  let mockState: RouterStateSnapshot;

  beforeEach(() => {
    mockRouter = jasmine.createSpyObj('Router', ['createUrlTree']);

    mockIdentityService = jasmine.createSpyObj('IdentityService', ['isAuthenticated'], {
      mode: jasmine.createSpy('mode'),
    });

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
    (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
    mockIdentityService.isAuthenticated.and.returnValue(true);

    const profileTree = {} as UrlTree;
    mockRouter.createUrlTree.and.returnValue(profileTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(profileTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/profile']);
  });

  it('should redirect authenticated steward users to /identity/profile', () => {
    (mockIdentityService.mode as jasmine.Spy).and.returnValue('steward');
    mockIdentityService.isAuthenticated.and.returnValue(true);

    const profileTree = {} as UrlTree;
    mockRouter.createUrlTree.and.returnValue(profileTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(profileTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/profile']);
  });

  it('should redirect unauthenticated users to /identity/register', () => {
    (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
    mockIdentityService.isAuthenticated.and.returnValue(false);

    const registerTree = {} as UrlTree;
    mockRouter.createUrlTree.and.returnValue(registerTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(registerTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/register']);
  });

  it('should redirect session-mode users to /identity/register', () => {
    (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
    mockIdentityService.isAuthenticated.and.returnValue(true);

    const registerTree = {} as UrlTree;
    mockRouter.createUrlTree.and.returnValue(registerTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(registerTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/register']);
  });

  it('should redirect anonymous users to /identity/register', () => {
    (mockIdentityService.mode as jasmine.Spy).and.returnValue('anonymous');
    mockIdentityService.isAuthenticated.and.returnValue(false);

    const registerTree = {} as UrlTree;
    mockRouter.createUrlTree.and.returnValue(registerTree);

    const result = TestBed.runInInjectionContext(() =>
      identityDefaultRedirectGuard(mockRoute, mockState)
    );

    expect(result).toBe(registerTree);
    expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/register']);
  });
});
