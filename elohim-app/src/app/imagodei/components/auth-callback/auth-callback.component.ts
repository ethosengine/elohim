/**
 * OAuth Callback Component
 *
 * Handles the OAuth redirect callback from doorway's /auth/authorize.
 * Exchanges the authorization code for a JWT token and completes login.
 *
 * Flow:
 * 1. User is redirected here from doorway with ?code=xxx&state=yyy
 * 2. Component extracts code and state from URL
 * 3. OAuthAuthProvider exchanges code for token
 * 4. On success: AuthService updates state, redirect to home/lamad
 * 5. On error: Show error message with retry option
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal } from '@angular/core';
import { Router } from '@angular/router';

// @coverage: 100.0% (2026-02-05)

import { AuthService } from '../../services/auth.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

type CallbackStatus = 'processing' | 'success' | 'error';

@Component({
  selector: 'app-auth-callback',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="callback-container">
      @switch (status()) {
        @case ('processing') {
          <div class="callback-card processing">
            <div class="spinner"></div>
            <h2>Completing sign in...</h2>
            <p>Please wait while we verify your identity.</p>
          </div>
        }

        @case ('success') {
          <div class="callback-card success">
            <div class="check-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <polyline points="20,6 9,17 4,12"></polyline>
              </svg>
            </div>
            <h2>Welcome back!</h2>
            <p>Redirecting you to Lamad...</p>
          </div>
        }

        @case ('error') {
          <div class="callback-card error">
            <div class="error-icon">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10"></circle>
                <line x1="15" y1="9" x2="9" y2="15"></line>
                <line x1="9" y1="9" x2="15" y2="15"></line>
              </svg>
            </div>
            <h2>Sign in failed</h2>
            <p class="error-message">{{ errorMessage() }}</p>
            <div class="actions">
              <button class="btn-primary" (click)="retry()">Try Again</button>
              <button class="btn-secondary" (click)="goHome()">Go Home</button>
            </div>
          </div>
        }
      }
    </div>
  `,
  styles: [
    `
      .callback-container {
        min-height: 100vh;
        display: flex;
        align-items: center;
        justify-content: center;
        background: linear-gradient(135deg, #1a1a2e 0%, #0a0a15 100%);
        padding: 1rem;
      }

      .callback-card {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid rgba(255, 255, 255, 0.1);
        border-radius: 1rem;
        padding: 3rem;
        text-align: center;
        max-width: 400px;
        width: 100%;
        backdrop-filter: blur(10px);
      }

      .callback-card h2 {
        color: #fff;
        margin: 1.5rem 0 0.5rem;
        font-size: 1.5rem;
        font-weight: 600;
      }

      .callback-card p {
        color: rgba(255, 255, 255, 0.7);
        margin: 0;
        font-size: 0.95rem;
      }

      .spinner {
        width: 48px;
        height: 48px;
        border: 3px solid rgba(255, 255, 255, 0.1);
        border-top-color: #6366f1;
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin: 0 auto;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }

      .check-icon,
      .error-icon {
        width: 64px;
        height: 64px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        margin: 0 auto;
      }

      .check-icon {
        background: rgba(34, 197, 94, 0.2);
        color: #22c55e;
      }

      .check-icon svg {
        width: 32px;
        height: 32px;
      }

      .error-icon {
        background: rgba(239, 68, 68, 0.2);
        color: #ef4444;
      }

      .error-icon svg {
        width: 32px;
        height: 32px;
      }

      .error-message {
        color: rgba(239, 68, 68, 0.9) !important;
        margin-top: 0.5rem !important;
      }

      .actions {
        display: flex;
        gap: 1rem;
        justify-content: center;
        margin-top: 2rem;
      }

      .btn-primary,
      .btn-secondary {
        padding: 0.75rem 1.5rem;
        border-radius: 0.5rem;
        font-size: 0.95rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
        border: none;
      }

      .btn-primary {
        background: #6366f1;
        color: white;
      }

      .btn-primary:hover {
        background: #4f46e5;
      }

      .btn-secondary {
        background: rgba(255, 255, 255, 0.1);
        color: white;
        border: 1px solid rgba(255, 255, 255, 0.2);
      }

      .btn-secondary:hover {
        background: rgba(255, 255, 255, 0.15);
      }
    `,
  ],
})
export class AuthCallbackComponent implements OnInit {
  private readonly router = inject(Router);
  private readonly oauthProvider = inject(OAuthAuthProvider);
  private readonly authService = inject(AuthService);

  readonly status = signal<CallbackStatus>('processing');
  readonly errorMessage = signal<string>('');

  ngOnInit(): void {
    void this.handleCallback();
  }

  private async handleCallback(): Promise<void> {
    // Get callback parameters from URL
    const params = this.oauthProvider.getCallbackParams();

    if (!params) {
      // Check for error in URL
      const url = new URL(window.location.href);
      const error = url.searchParams.get('error');
      const errorDesc = url.searchParams.get('error_description');

      if (error) {
        this.status.set('error');
        this.errorMessage.set(errorDesc ?? this.getOAuthErrorMessage(error));
        this.oauthProvider.clearCallbackParams();
        return;
      }

      // No code or error - invalid callback
      this.status.set('error');
      this.errorMessage.set('Invalid callback. No authorization code received.');
      return;
    }

    try {
      // Exchange code for token
      const result = await this.oauthProvider.handleCallback(params.code, params.state);

      // Clear URL parameters
      this.oauthProvider.clearCallbackParams();

      if (result.success) {
        // Update auth state
        this.authService.setAuthFromResult(result);

        this.status.set('success');

        // Redirect to lamad after brief delay for UX
        setTimeout(() => {
          void this.router.navigate(['/lamad']);
        }, 1500);
      } else {
        this.status.set('error');
        this.errorMessage.set(result.error);
      }
    } catch (err) {
      this.status.set('error');
      this.errorMessage.set(err instanceof Error ? err.message : 'An unexpected error occurred');
    }
  }

  retry(): void {
    // Navigate back to identity page (with doorway picker)
    void this.router.navigate(['/identity']);
  }

  goHome(): void {
    void this.router.navigate(['/']);
  }

  private getOAuthErrorMessage(error: string): string {
    switch (error) {
      case 'access_denied':
        return 'You denied access to your account.';
      case 'invalid_request':
        return 'The authorization request was invalid.';
      case 'unauthorized_client':
        return 'This application is not authorized.';
      case 'unsupported_response_type':
        return 'The authorization server does not support this response type.';
      case 'invalid_scope':
        return 'The requested permissions are invalid.';
      case 'server_error':
        return 'The authorization server encountered an error.';
      case 'temporarily_unavailable':
        return 'The authorization server is temporarily unavailable.';
      default:
        return `Authorization failed: ${error}`;
    }
  }
}
