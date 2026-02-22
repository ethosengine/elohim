import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, effect } from '@angular/core';
import { RouterOutlet, Router, NavigationEnd } from '@angular/router';

import { filter } from 'rxjs/operators';

import { environment } from '../environments/environment';

import { ThemeToggleComponent } from './components/theme-toggle/theme-toggle.component';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { HolochainContentService } from './elohim/services/holochain-content.service';
import { AuthService } from './imagodei/services/auth.service';
import { TauriAuthService } from './imagodei/services/tauri-auth.service';
import { BlobBootstrapService } from './lamad/services/blob-bootstrap.service';

// @coverage: 90.9% (2026-02-05)

/** Connection retry configuration */
interface RetryConfig {
  maxAttempts: number;
  baseDelayMs: number;
  maxDelayMs: number;
  backoffMultiplier: number;
}

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, ThemeToggleComponent, CommonModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css',
})
export class AppComponent implements OnInit, OnDestroy {
  title = 'elohim-app';
  /** Show floating theme toggle only on root landing page */
  showFloatingToggle = false;

  private readonly holochainService = inject(HolochainClientService);
  private readonly holochainContent = inject(HolochainContentService);
  private readonly blobBootstrap = inject(BlobBootstrapService);
  private readonly authService = inject(AuthService);
  private readonly tauriAuth = inject(TauriAuthService);

  /** Retry configuration for connection attempts */
  private readonly retryConfig: RetryConfig = {
    maxAttempts: 5,
    baseDelayMs: 1000,
    maxDelayMs: 30000,
    backoffMultiplier: 2,
  };

  /** Track retry state */
  private connectionAttempt = 0;
  private retryTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private isDestroyed = false;
  private holochainInitialized = false;

  constructor(private readonly router: Router) {
    // Post-unlock trigger (Tauri): when status transitions to 'authenticated' after needs_unlock,
    // initialize Holochain connection (which was deferred during lock gate)
    effect(() => {
      if (this.tauriAuth.status() === 'authenticated' && !this.holochainInitialized) {
        this.holochainInitialized = true;
        void this.initializeHolochainConnection();
      }
    });

    // Post-login trigger (Browser): when auth state transitions to authenticated,
    // initialize Holochain connection (deferred until login for browser visitors)
    effect(() => {
      if (
        !this.tauriAuth.isTauriEnvironment() &&
        this.authService.isAuthenticated() &&
        !this.holochainInitialized
      ) {
        this.holochainInitialized = true;
        void this.initializeHolochainConnection();
      }
    });
  }

  ngOnInit(): void {
    // Track route changes to show floating toggle only on root landing page
    this.router.events
      .pipe(filter(event => event instanceof NavigationEnd))
      .subscribe((event: NavigationEnd) => {
        this.showFloatingToggle = this.isRootLandingPage(event.url);
      });

    // Check initial route
    this.showFloatingToggle = this.isRootLandingPage(this.router.url);

    // Initialize auth and connections
    void this.initializeApp();
  }

  /**
   * Initialize the application - check for Tauri session first, then connections.
   */
  private async initializeApp(): Promise<void> {
    // In Tauri environment, check for existing session first
    if (this.tauriAuth.isTauriEnvironment()) {
      await this.tauriAuth.initialize();

      if (this.tauriAuth.needsLogin()) {
        void this.router.navigate(['/identity/login'], {
          queryParams: { launcher: 'true' },
        });
        return;
      }

      if (this.tauriAuth.needsUnlock()) {
        // Route to lock screen — don't init Holochain yet (deferred until unlock)
        void this.router.navigate(['/identity/login']);
        return;
      }

      // Authenticated (standalone mode) — mark as initialized so effect doesn't double-fire
      this.holochainInitialized = true;
      await this.initializeHolochainConnection();
      return;
    }

    // Browser path: start content services (doorway + blobs), defer Holochain to post-auth.
    // Content is served via the doorway projection tier — no Holochain needed for visitors.
    await this.initializeContentServices();

    // Only connect to Holochain if already authenticated (returning user with valid token)
    if (this.authService.isAuthenticated()) {
      this.holochainInitialized = true;
      await this.initializeHolochainConnection();
    }
    // Otherwise: Holochain connection deferred until login (via browser auth effect)
  }

  ngOnDestroy(): void {
    this.isDestroyed = true;
    if (this.retryTimeoutId) {
      clearTimeout(this.retryTimeoutId);
      this.retryTimeoutId = null;
    }
    // Clean up Tauri event listeners
    this.tauriAuth.destroy();
  }

  /**
   * Initialize content services that work without Holochain.
   * Doorway projection tier serves content for all visitors (authenticated or not).
   */
  private async initializeContentServices(): Promise<void> {
    // Test doorway availability (non-blocking, informational)
    await this.testDoorwayConnection();

    // Start blob bootstrap regardless of connectivity
    // It will serve cached blobs and upgrade to streaming when services connect
    this.blobBootstrap.startBootstrap();
  }

  /**
   * Initialize Holochain connection for agent-centric data.
   *
   * Architecture:
   * - CONTENT: Served from doorway projection (SQLite) or local storage (Tauri)
   *   - ElohimClient handles mode-aware routing automatically
   *   - App works fully offline with cached/local content
   *
   * - AGENT DATA: Holochain DHT (identity, attestations, points)
   *   - Optional - app degrades gracefully without it
   *   - Uses exponential backoff retry for connection
   *
   * Graceful degradation scenarios:
   * - Doorway + Holochain: Full functionality
   * - Doorway only: Content works, agent features degraded
   * - Tauri (local): Full offline functionality via local SQLite
   * - Neither: Cached content only, no writes
   */
  private async initializeHolochainConnection(): Promise<void> {
    // Ensure content services are running (idempotent for Tauri path)
    await this.initializeContentServices();

    // Only attempt Holochain connection if config exists
    // Holochain is only for agent-centric data (identity, attestations, points)
    if (!environment.holochain?.adminUrl) {
      // Holochain config not found - running in content-only mode
      return;
    }

    await this.attemptConnection();
  }

  /**
   * Test doorway projection API availability.
   * Non-blocking - returns false on any error.
   */
  private async testDoorwayConnection(): Promise<boolean> {
    const doorwayUrl = environment.client?.doorwayUrl;
    if (!doorwayUrl) {
      return false;
    }

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), 3000);

      const response = await fetch(`${doorwayUrl}/health`, {
        signal: controller.signal,
      });

      clearTimeout(timeoutId);
      return response.ok;
    } catch {
      // Silent fail - graceful degradation
      return false;
    }
  }

  /**
   * Attempt Holochain connection with retry support.
   * Holochain is used for agent-centric data only (identity, attestations, points).
   */
  private async attemptConnection(): Promise<void> {
    if (this.isDestroyed) return;

    this.connectionAttempt++;
    const { maxAttempts, baseDelayMs, maxDelayMs, backoffMultiplier } = this.retryConfig;

    try {
      // Connection attempt ${this.connectionAttempt}/${maxAttempts}

      // Attempt full connection (admin + app interface)
      await this.holochainService.connect();

      if (this.holochainService.isConnected()) {
        // Connection successful, testing zome availability

        // Test if we can make zome calls (for agent-centric data)
        await this.holochainContent.testAvailability();

        // Reset retry counter on success
        this.connectionAttempt = 0;
      }
    } catch {
      if (this.connectionAttempt < maxAttempts) {
        // Calculate delay with exponential backoff + deterministic jitter
        const exponentialDelay =
          baseDelayMs * Math.pow(backoffMultiplier, this.connectionAttempt - 1);
        // Use deterministic jitter based on attempt number (avoids Math.random)
        const jitter = (this.connectionAttempt * 200) % 1000;
        const delay = Math.min(exponentialDelay + jitter, maxDelayMs);
        // Schedule retry
        this.retryTimeoutId = setTimeout(() => {
          if (!this.isDestroyed) {
            void this.attemptConnection();
          }
        }, delay);
      } else {
        // Max retries reached - log and continue without throwing
        // Reset for potential future reconnection attempts
        this.connectionAttempt = 0;
      }
    }
  }

  /**
   * Public method to manually retry connection (can be called from UI)
   */
  public retryConnection(): void {
    // Reset counter for fresh retry sequence
    this.connectionAttempt = 0;

    // Cancel any pending retry
    if (this.retryTimeoutId) {
      clearTimeout(this.retryTimeoutId);
      this.retryTimeoutId = null;
    }

    // Attempt connection
    void this.initializeHolochainConnection();
  }

  /** Check if URL is the root landing page (/ or empty) */
  private isRootLandingPage(url: string): boolean {
    // Strip query params and fragments
    const path = url.split('?')[0].split('#')[0];
    return path === '/' || path === '';
  }
}
