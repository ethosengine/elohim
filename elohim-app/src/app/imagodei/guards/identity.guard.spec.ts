/**
 * Identity Guards Tests
 *
 * Tests route guards for authentication-based access control.
 */

import { TestBed } from '@angular/core/testing';
import { Router, ActivatedRouteSnapshot, RouterStateSnapshot, UrlTree } from '@angular/router';

import { identityGuard, sessionOrAuthGuard, attestationGuard } from './identity.guard';
import { AuthService } from '../services/auth.service';
import { IdentityService } from '../services/identity.service';
import { SessionHumanService } from '../services/session-human.service';
import { vi, Mock } from 'vitest';

describe('Identity Guards', () => {
  let mockRouter: any;
  let mockIdentityService: any;
  let mockAuthService: any;
  let mockSessionHumanService: any;
  let mockRoute: ActivatedRouteSnapshot;
  let mockState: RouterStateSnapshot;

  beforeEach(() => {
    // Create mocks
    mockRouter = {
      createUrlTree: vi.fn(),
    };

    mockIdentityService = {
      isAuthenticated: vi.fn(),
      waitForAuthenticatedState: vi.fn(),
      mode: vi.fn(),
      attestations: vi.fn(),
    };

    mockAuthService = {
      isAuthenticated: vi.fn(),
    };

    mockSessionHumanService = {
      hasSession: vi.fn(),
    };

    // Setup TestBed
    TestBed.configureTestingModule({
      providers: [
        { provide: Router, useValue: mockRouter },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: AuthService, useValue: mockAuthService },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
      ],
    });

    mockRoute = {} as ActivatedRouteSnapshot;
    mockState = { url: '/protected/resource' } as RouterStateSnapshot;
  });

  // ==========================================================================
  // identityGuard
  // ==========================================================================

  describe('identityGuard', () => {
    it('should allow access when authenticated via network (hosted mode)', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(true);
    });

    it('should allow access when authenticated via network (steward mode)', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('steward');
      mockIdentityService.isAuthenticated.mockReturnValue(true);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(true);
    });

    it('should redirect when not authenticated at all', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('session');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockAuthService.isAuthenticated.mockReturnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/protected/resource' },
      });
    });

    it('should wait for identity to settle when auth is valid but mode is session', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('session');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockAuthService.isAuthenticated.mockReturnValue(true);
      mockIdentityService.waitForAuthenticatedState.mockResolvedValue(true);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(true);
      expect(mockIdentityService.waitForAuthenticatedState).toHaveBeenCalledWith(5000);
    });

    it('should redirect when auth is valid but identity fails to settle', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('session');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockAuthService.isAuthenticated.mockReturnValue(true);
      mockIdentityService.waitForAuthenticatedState.mockResolvedValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
      expect(mockIdentityService.waitForAuthenticatedState).toHaveBeenCalledWith(5000);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/protected/resource' },
      });
    });

    it('should not call waitForAuthenticatedState when already in network mode', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);

      await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(mockIdentityService.waitForAuthenticatedState).not.toHaveBeenCalled();
    });

    it('should redirect when in visitor mode and not authenticated', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('visitor');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockAuthService.isAuthenticated.mockReturnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
    });
  });

  // ==========================================================================
  // sessionOrAuthGuard
  // ==========================================================================

  describe('sessionOrAuthGuard', () => {
    it('should allow access when authenticated via network', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);

      const result = TestBed.runInInjectionContext(() => sessionOrAuthGuard(mockRoute, mockState));

      expect(result).toBe(true);
    });

    it('should allow access when has session', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('session');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockSessionHumanService.hasSession.mockReturnValue(true);

      const result = TestBed.runInInjectionContext(() => sessionOrAuthGuard(mockRoute, mockState));

      expect(result).toBe(true);
    });

    it('should redirect when neither session nor authentication exists', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('visitor');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockSessionHumanService.hasSession.mockReturnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() => sessionOrAuthGuard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login']);
    });

    it('should prefer network auth over session', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);
      mockSessionHumanService.hasSession.mockReturnValue(true);

      const result = TestBed.runInInjectionContext(() => sessionOrAuthGuard(mockRoute, mockState));

      expect(result).toBe(true);
      // Session check should not be called if network auth succeeds
      expect(mockSessionHumanService.hasSession).not.toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // attestationGuard
  // ==========================================================================

  describe('attestationGuard', () => {
    it('should allow access when user has required attestation', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);
      (mockIdentityService.attestations as Mock).mockReturnValue([
        'content-creator',
        'verified-human',
      ]);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() => guard(mockRoute, mockState));

      expect(result).toBe(true);
    });

    it('should redirect to login when not authenticated', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('visitor');
      mockIdentityService.isAuthenticated.mockReturnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() => guard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/protected/resource' },
      });
    });

    it('should redirect to login when in session mode', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('session');
      mockIdentityService.isAuthenticated.mockReturnValue(true);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() => guard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
    });

    it('should redirect to access-denied when missing attestation', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);
      (mockIdentityService.attestations as Mock).mockReturnValue(['verified-human']);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() => guard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/access-denied'], {
        queryParams: {
          required: 'content-creator',
          returnUrl: '/protected/resource',
        },
      });
    });

    it('should handle empty attestations list', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);
      (mockIdentityService.attestations as Mock).mockReturnValue([]);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() => guard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/access-denied'], {
        queryParams: {
          required: 'content-creator',
          returnUrl: '/protected/resource',
        },
      });
    });

    it('should create guard functions for different attestations', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);
      (mockIdentityService.attestations as Mock).mockReturnValue([
        'steward',
        'community-moderator',
      ]);

      const stewardGuard = attestationGuard('steward');
      const moderatorGuard = attestationGuard('community-moderator');
      const creatorGuard = attestationGuard('content-creator');

      const stewardResult = TestBed.runInInjectionContext(() => stewardGuard(mockRoute, mockState));
      expect(stewardResult).toBe(true);

      const moderatorResult = TestBed.runInInjectionContext(() =>
        moderatorGuard(mockRoute, mockState)
      );
      expect(moderatorResult).toBe(true);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const creatorResult = TestBed.runInInjectionContext(() => creatorGuard(mockRoute, mockState));
      expect(creatorResult).toBe(mockUrlTree);
    });
  });

  // ==========================================================================
  // Edge Cases
  // ==========================================================================

  describe('edge cases', () => {
    it('should handle null mode gracefully', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue(null);
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockAuthService.isAuthenticated.mockReturnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const result = await TestBed.runInInjectionContext(() => identityGuard(mockRoute, mockState));

      expect(result).toBe(mockUrlTree);
    });

    it('should handle undefined attestations', () => {
      (mockIdentityService.mode as Mock).mockReturnValue('hosted');
      mockIdentityService.isAuthenticated.mockReturnValue(true);
      (mockIdentityService.attestations as Mock).mockReturnValue(undefined);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() => guard(mockRoute, mockState));

      // Should redirect when attestations are undefined
      expect(result).toBe(mockUrlTree);
    });

    it('should preserve returnUrl query parameter', async () => {
      (mockIdentityService.mode as Mock).mockReturnValue('visitor');
      mockIdentityService.isAuthenticated.mockReturnValue(false);
      mockAuthService.isAuthenticated.mockReturnValue(false);

      const mockState2: RouterStateSnapshot = {
        url: '/deeply/nested/protected/resource?foo=bar',
      } as RouterStateSnapshot;

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.mockReturnValue(mockUrlTree);

      const result = await TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState2)
      );

      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/deeply/nested/protected/resource?foo=bar' },
      });
    });
  });
});
