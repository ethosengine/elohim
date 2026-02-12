/**
 * AuthCallbackComponent Tests
 *
 * Tests for OAuth callback handler component.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';

import { AuthCallbackComponent } from './auth-callback.component';
import { SeoService } from '../../../services/seo.service';
import { AuthService } from '../../services/auth.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

describe('AuthCallbackComponent', () => {
  let component: AuthCallbackComponent;
  let fixture: ComponentFixture<AuthCallbackComponent>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockOAuthProvider: jasmine.SpyObj<OAuthAuthProvider>;
  let mockSeoService: jasmine.SpyObj<SeoService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let originalLocation: Location;

  beforeEach(async () => {
    // Save original location
    originalLocation = window.location;

    // Create mocks with correct methods
    mockAuthService = jasmine.createSpyObj('AuthService', ['setAuthFromResult']);

    mockOAuthProvider = jasmine.createSpyObj('OAuthAuthProvider', [
      'getCallbackParams',
      'handleCallback',
      'clearCallbackParams',
    ]);

    mockSeoService = jasmine.createSpyObj('SeoService', ['setTitle']);
    mockRouter = jasmine.createSpyObj('Router', ['navigate']);

    // Configure default mock returns
    mockOAuthProvider.getCallbackParams.and.returnValue(null);

    await TestBed.configureTestingModule({
      imports: [AuthCallbackComponent],
      providers: [
        { provide: AuthService, useValue: mockAuthService },
        { provide: OAuthAuthProvider, useValue: mockOAuthProvider },
        { provide: SeoService, useValue: mockSeoService },
        { provide: Router, useValue: mockRouter },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(AuthCallbackComponent);
    component = fixture.componentInstance;
  });

  afterEach(() => {
    // Restore location if it was modified
    if (window.location !== originalLocation) {
      Object.defineProperty(window, 'location', {
        value: originalLocation,
        writable: true,
        configurable: true,
      });
    }
  });

  /**
   * Helper to set window.location using history API
   */
  function setWindowUrl(url: string): void {
    const urlObj = new URL(url);
    window.history.pushState({}, '', urlObj.pathname + urlObj.search);
  }

  // ==========================================================================
  // Component Creation
  // ==========================================================================

  it('should create', () => {
    // Don't call detectChanges yet - we need to prevent ngOnInit from running
    expect(component).toBeTruthy();
  });

  // ==========================================================================
  // Signals
  // ==========================================================================

  it('should have status signal', () => {
    expect(component.status).toBeDefined();
  });

  it('should have errorMessage signal', () => {
    expect(component.errorMessage).toBeDefined();
  });

  // ==========================================================================
  // Public Methods
  // ==========================================================================

  it('should have retry method', () => {
    expect(component.retry).toBeDefined();
    expect(typeof component.retry).toBe('function');
  });

  it('should have goHome method', () => {
    expect(component.goHome).toBeDefined();
    expect(typeof component.goHome).toBe('function');
  });

  // ==========================================================================
  // Go Home
  // ==========================================================================

  it('should navigate to home when goHome is called', () => {
    component.goHome();
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
  });

  // ==========================================================================
  // Initial State
  // ==========================================================================

  it('should start with processing status', () => {
    expect(component.status()).toBe('processing');
  });

  it('should have no error message initially', () => {
    expect(component.errorMessage()).toBe('');
  });

  // ==========================================================================
  // Retry Navigation
  // ==========================================================================

  it('should navigate to identity page when retry is called', () => {
    component.retry();
    expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity']);
  });

  // ==========================================================================
  // OAuth Callback Handling - Success Flow
  // ==========================================================================

  describe('OAuth Callback - Success Flow', () => {
    beforeEach(() => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-auth-code-123',
        state: 'test-state-456',
      });

      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({
          success: true,
          token: 'test-jwt-token',
          humanId: 'human-123',
          agentPubKey: 'agent-pub-key-123',
          identifier: 'user@example.com',
          expiresAt: Date.now() + 3600000,
        })
      );

      jasmine.clock().install();
    });

    afterEach(() => {
      jasmine.clock().uninstall();
    });

    it('should process callback on initialization', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockOAuthProvider.getCallbackParams).toHaveBeenCalled();
    });

    it('should exchange code for token', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockOAuthProvider.handleCallback).toHaveBeenCalledWith(
        'test-auth-code-123',
        'test-state-456'
      );
    });

    it('should update auth service on success', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockAuthService.setAuthFromResult).toHaveBeenCalled();
    });

    it('should set status to success', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.status()).toBe('success');
    });

    it('should clear callback params after processing', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockOAuthProvider.clearCallbackParams).toHaveBeenCalled();
    });

    it('should redirect to lamad after delay', async () => {
      fixture.detectChanges();
      await fixture.whenStable();

      jasmine.clock().tick(1500);

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/lamad']);
    });
  });

  // ==========================================================================
  // OAuth Callback Handling - Error Flows
  // ==========================================================================

  describe('OAuth Callback - Error Flows', () => {
    it('should handle no callback params', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue(null);
      setWindowUrl('http://localhost/auth/callback');

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.status()).toBe('error');
      expect(component.errorMessage()).toContain('Invalid callback');
    });

    it('should handle OAuth error in URL', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue(null);
      setWindowUrl('http://localhost/auth/callback?error=access_denied');

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.status()).toBe('error');
    });

    it('should handle token exchange failure', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });

      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({
          success: false,
          error: 'Token exchange failed',
        })
      );

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.status()).toBe('error');
      expect(component.errorMessage()).toBe('Token exchange failed');
    });

    it('should handle exception during callback', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });

      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.reject(new Error('Network error'))
      );

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.status()).toBe('error');
      expect(component.errorMessage()).toBe('Network error');
    });

    it('should handle non-Error exception', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });

      mockOAuthProvider.handleCallback.and.returnValue(Promise.reject('String error'));

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.status()).toBe('error');
      expect(component.errorMessage()).toBe('An unexpected error occurred');
    });
  });

  // ==========================================================================
  // OAuth Error Messages
  // ==========================================================================

  describe('OAuth Error Messages', () => {
    beforeEach(() => {
      mockOAuthProvider.getCallbackParams.and.returnValue(null);
    });

    const errorCases = [
      { error: 'access_denied', expectedMessage: 'You denied access to your account.' },
      { error: 'invalid_request', expectedMessage: 'The authorization request was invalid.' },
      {
        error: 'unauthorized_client',
        expectedMessage: 'This application is not authorized.',
      },
      {
        error: 'unsupported_response_type',
        expectedMessage: 'The authorization server does not support this response type.',
      },
      { error: 'invalid_scope', expectedMessage: 'The requested permissions are invalid.' },
      {
        error: 'server_error',
        expectedMessage: 'The authorization server encountered an error.',
      },
      {
        error: 'temporarily_unavailable',
        expectedMessage: 'The authorization server is temporarily unavailable.',
      },
      { error: 'unknown_error', expectedMessage: 'Authorization failed: unknown_error' },
    ];

    errorCases.forEach(({ error, expectedMessage }) => {
      it(`should display correct message for ${error}`, async () => {
        setWindowUrl(`http://localhost/auth/callback?error=${error}`);

        fixture.detectChanges();
        await fixture.whenStable();

        expect(component.status()).toBe('error');
        expect(component.errorMessage()).toBe(expectedMessage);
      });
    });

    it('should use error_description when provided', async () => {
      setWindowUrl(
        'http://localhost/auth/callback?error=access_denied&error_description=Custom+error+message'
      );

      fixture.detectChanges();
      await fixture.whenStable();

      expect(component.errorMessage()).toBe('Custom error message');
    });
  });

  // ==========================================================================
  // Template State Rendering
  // ==========================================================================

  describe('Template State Rendering', () => {
    it('should show processing state initially', () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });
      mockOAuthProvider.handleCallback.and.returnValue(new Promise(() => {})); // Never resolves

      fixture.detectChanges();

      const processingCard = fixture.nativeElement.querySelector('.callback-card.processing');
      expect(processingCard).toBeTruthy();
      expect(processingCard.textContent).toContain('Completing sign in');
    });

    it('should show success state after successful auth', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });
      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({
          success: true,
          token: 'test-token',
          humanId: 'human-123',
          agentPubKey: 'agent-pub-key-123',
          identifier: 'user@example.com',
          expiresAt: Date.now() + 3600000,
        })
      );

      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const successCard = fixture.nativeElement.querySelector('.callback-card.success');
      expect(successCard).toBeTruthy();
      expect(successCard.textContent).toContain('Welcome back');
    });

    it('should show error state on failure', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });
      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({ success: false, error: 'Test error' })
      );

      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const errorCard = fixture.nativeElement.querySelector('.callback-card.error');
      expect(errorCard).toBeTruthy();
      expect(errorCard.textContent).toContain('Sign in failed');
      expect(errorCard.textContent).toContain('Test error');
    });

    it('should show retry and home buttons on error', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });
      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({ success: false, error: 'Test error' })
      );

      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const buttons = fixture.nativeElement.querySelectorAll('.actions button');
      expect(buttons.length).toBe(2);
      expect(buttons[0].textContent).toContain('Try Again');
      expect(buttons[1].textContent).toContain('Go Home');
    });

    it('should trigger retry when retry button clicked', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });
      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({ success: false, error: 'Test error' })
      );

      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const retryButton = fixture.nativeElement.querySelector('.btn-primary');
      retryButton.click();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity']);
    });

    it('should trigger goHome when home button clicked', async () => {
      mockOAuthProvider.getCallbackParams.and.returnValue({
        code: 'test-code',
        state: 'test-state',
      });
      mockOAuthProvider.handleCallback.and.returnValue(
        Promise.resolve({ success: false, error: 'Test error' })
      );

      fixture.detectChanges();
      await fixture.whenStable();
      fixture.detectChanges();

      const homeButton = fixture.nativeElement.querySelector('.btn-secondary');
      homeButton.click();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/']);
    });
  });
});
