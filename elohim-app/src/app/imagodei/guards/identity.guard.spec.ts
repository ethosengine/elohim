/**
 * Identity Guards Tests
 *
 * Tests route guards for authentication-based access control.
 */

import { TestBed } from '@angular/core/testing';
import { Router, ActivatedRouteSnapshot, RouterStateSnapshot, UrlTree } from '@angular/router';
import { signal } from '@angular/core';

import { identityGuard, sessionOrAuthGuard, attestationGuard } from './identity.guard';
import { IdentityService } from '../services/identity.service';
import { SessionHumanService } from '../services/session-human.service';
import type { IdentityMode } from '../models/identity.model';

describe('Identity Guards', () => {
  let mockRouter: jasmine.SpyObj<Router>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockRoute: ActivatedRouteSnapshot;
  let mockState: RouterStateSnapshot;

  beforeEach(() => {
    // Create mocks
    mockRouter = jasmine.createSpyObj('Router', ['createUrlTree']);

    mockIdentityService = jasmine.createSpyObj('IdentityService', ['isAuthenticated'], {
      mode: jasmine.createSpy('mode'),
      attestations: jasmine.createSpy('attestations'),
    });

    mockSessionHumanService = jasmine.createSpyObj('SessionHumanService', ['hasSession']);

    // Setup TestBed
    TestBed.configureTestingModule({
      providers: [
        { provide: Router, useValue: mockRouter },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
      ],
    });

    // Create mock route snapshots
    mockRoute = {} as ActivatedRouteSnapshot;
    mockState = {
      url: '/protected/resource',
    } as RouterStateSnapshot;
  });

  // ==========================================================================
  // identityGuard
  // ==========================================================================

  describe('identityGuard', () => {
    it('should allow access when authenticated via network (hosted mode)', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(true);
    });

    it('should allow access when authenticated via network (steward mode)', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('steward');
      mockIdentityService.isAuthenticated.and.returnValue(true);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(true);
    });

    it('should redirect to register when not authenticated', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockIdentityService.isAuthenticated.and.returnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/protected/resource' },
      });
    });

    it('should redirect when in session mode even if authenticated', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockIdentityService.isAuthenticated.and.returnValue(true);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
    });

    it('should redirect when in visitor mode', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('visitor');
      mockIdentityService.isAuthenticated.and.returnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
    });

    it('should redirect when authenticated but not in network mode', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockIdentityService.isAuthenticated.and.returnValue(true);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
    });
  });

  // ==========================================================================
  // sessionOrAuthGuard
  // ==========================================================================

  describe('sessionOrAuthGuard', () => {
    it('should allow access when authenticated via network', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);

      const result = TestBed.runInInjectionContext(() =>
        sessionOrAuthGuard(mockRoute, mockState)
      );

      expect(result).toBe(true);
    });

    it('should allow access when has session', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockIdentityService.isAuthenticated.and.returnValue(false);
      mockSessionHumanService.hasSession.and.returnValue(true);

      const result = TestBed.runInInjectionContext(() =>
        sessionOrAuthGuard(mockRoute, mockState)
      );

      expect(result).toBe(true);
    });

    it('should redirect when neither session nor authentication exists', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('visitor');
      mockIdentityService.isAuthenticated.and.returnValue(false);
      mockSessionHumanService.hasSession.and.returnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        sessionOrAuthGuard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login']);
    });

    it('should prefer network auth over session', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);
      mockSessionHumanService.hasSession.and.returnValue(true);

      const result = TestBed.runInInjectionContext(() =>
        sessionOrAuthGuard(mockRoute, mockState)
      );

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
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);
      (mockIdentityService.attestations as jasmine.Spy).and.returnValue([
        'content-creator',
        'verified-human',
      ]);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() =>
        guard(mockRoute, mockState)
      );

      expect(result).toBe(true);
    });

    it('should redirect to register when not authenticated', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('visitor');
      mockIdentityService.isAuthenticated.and.returnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() =>
        guard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/protected/resource' },
      });
    });

    it('should redirect to register when in session mode', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('session');
      mockIdentityService.isAuthenticated.and.returnValue(true);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() =>
        guard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
    });

    it('should redirect to access-denied when missing attestation', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);
      (mockIdentityService.attestations as jasmine.Spy).and.returnValue([
        'verified-human',
      ]);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() =>
        guard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/access-denied'], {
        queryParams: {
          required: 'content-creator',
          returnUrl: '/protected/resource',
        },
      });
    });

    it('should handle empty attestations list', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);
      (mockIdentityService.attestations as jasmine.Spy).and.returnValue([]);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() =>
        guard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/access-denied'], {
        queryParams: {
          required: 'content-creator',
          returnUrl: '/protected/resource',
        },
      });
    });

    it('should create guard functions for different attestations', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);
      (mockIdentityService.attestations as jasmine.Spy).and.returnValue([
        'steward',
        'community-moderator',
      ]);

      const stewardGuard = attestationGuard('steward');
      const moderatorGuard = attestationGuard('community-moderator');
      const creatorGuard = attestationGuard('content-creator');

      const stewardResult = TestBed.runInInjectionContext(() =>
        stewardGuard(mockRoute, mockState)
      );
      expect(stewardResult).toBe(true);

      const moderatorResult = TestBed.runInInjectionContext(() =>
        moderatorGuard(mockRoute, mockState)
      );
      expect(moderatorResult).toBe(true);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const creatorResult = TestBed.runInInjectionContext(() =>
        creatorGuard(mockRoute, mockState)
      );
      expect(creatorResult).toBe(mockUrlTree);
    });
  });

  // ==========================================================================
  // Edge Cases
  // ==========================================================================

  describe('edge cases', () => {
    it('should handle null mode gracefully', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue(null);
      mockIdentityService.isAuthenticated.and.returnValue(false);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState)
      );

      expect(result).toBe(mockUrlTree);
    });

    it('should handle undefined attestations', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('hosted');
      mockIdentityService.isAuthenticated.and.returnValue(true);
      (mockIdentityService.attestations as jasmine.Spy).and.returnValue(undefined);

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const guard = attestationGuard('content-creator');
      const result = TestBed.runInInjectionContext(() =>
        guard(mockRoute, mockState)
      );

      // Should redirect when attestations are undefined
      expect(result).toBe(mockUrlTree);
    });

    it('should preserve returnUrl query parameter', () => {
      (mockIdentityService.mode as jasmine.Spy).and.returnValue('visitor');
      mockIdentityService.isAuthenticated.and.returnValue(false);

      const mockState2: RouterStateSnapshot = {
        url: '/deeply/nested/protected/resource?foo=bar',
      } as RouterStateSnapshot;

      const mockUrlTree = {} as UrlTree;
      mockRouter.createUrlTree.and.returnValue(mockUrlTree);

      const result = TestBed.runInInjectionContext(() =>
        identityGuard(mockRoute, mockState2)
      );

      expect(mockRouter.createUrlTree).toHaveBeenCalledWith(['/identity/login'], {
        queryParams: { returnUrl: '/deeply/nested/protected/resource?foo=bar' },
      });
    });
  });
});
