import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { RouterOutlet, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { filter } from 'rxjs/operators';
import { ThemeToggleComponent } from './components/theme-toggle/theme-toggle.component';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { HolochainContentService } from './elohim/services/holochain-content.service';
import { BlobBootstrapService } from './lamad/services/blob-bootstrap.service';
import { environment } from '../environments/environment';

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
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit, OnDestroy {
  title = 'elohim-app';
  /** Show floating theme toggle only on root landing page */
  showFloatingToggle = false;

  private readonly holochainService = inject(HolochainClientService);
  private readonly holochainContent = inject(HolochainContentService);
  private readonly blobBootstrap = inject(BlobBootstrapService);

  /** Retry configuration for connection attempts */
  private readonly retryConfig: RetryConfig = {
    maxAttempts: 5,
    baseDelayMs: 1000,
    maxDelayMs: 30000,
    backoffMultiplier: 2
  };

  /** Track retry state */
  private connectionAttempt = 0;
  private retryTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private isDestroyed = false;

  constructor(private readonly router: Router) {}

  ngOnInit(): void {
    // Track route changes to show floating toggle only on root landing page
    this.router.events
      .pipe(filter(event => event instanceof NavigationEnd))
      .subscribe((event: NavigationEnd) => {
        this.showFloatingToggle = this.isRootLandingPage(event.url);
      });

    // Check initial route
    this.showFloatingToggle = this.isRootLandingPage(this.router.url);

    // Auto-connect to Edge Node if holochain config is available
    this.initializeHolochainConnection();
  }

  ngOnDestroy(): void {
    this.isDestroyed = true;
    if (this.retryTimeoutId) {
      clearTimeout(this.retryTimeoutId);
      this.retryTimeoutId = null;
    }
  }

  /**
   * Initialize Holochain connection on app startup with retry logic.
   *
   * Attempts full connection flow with exponential backoff:
   * 1. Connect to admin interface (via dev-proxy in Che)
   * 2. Discover existing app interfaces
   * 3. Authorize signing credentials
   * 4. Connect to app interface for zome calls
   * 5. Test content service availability
   *
   * Runs in background without blocking app initialization.
   * On failure, retries with exponential backoff up to maxAttempts.
   *
   * Also initializes blob streaming in degraded mode (cached blobs only)
   * if Holochain config is not available.
   */
  private async initializeHolochainConnection(): Promise<void> {
    // Only attempt connection if holochain config exists
    if (!environment.holochain?.adminUrl) {
      console.log('[Holochain] Config not found, initializing blob bootstrap in degraded mode');
      // Start blob bootstrap in degraded mode - can serve cached blobs without Holochain
      this.blobBootstrap.startBootstrap();
      return;
    }

    await this.attemptConnection();
  }

  /**
   * Attempt to connect with retry support.
   */
  private async attemptConnection(): Promise<void> {
    if (this.isDestroyed) return;

    this.connectionAttempt++;
    const { maxAttempts, baseDelayMs, maxDelayMs, backoffMultiplier } = this.retryConfig;

    try {
      console.log(`[Holochain] Connection attempt ${this.connectionAttempt}/${maxAttempts}...`);

      // Attempt full connection (admin + app interface)
      await this.holochainService.connect();

      if (this.holochainService.isConnected()) {
        console.log('[Holochain] Connection successful, testing content availability...');

        // Test if content service can make zome calls
        const contentAvailable = await this.holochainContent.testAvailability();

        if (contentAvailable) {
          console.log('[Holochain] Content service available - Holochain data ready!');

          // Initialize blob streaming now that Holochain is ready
          this.blobBootstrap.startBootstrap();
        } else {
          console.warn('[Holochain] Connected but content service unavailable');
        }

        // Reset retry counter on success
        this.connectionAttempt = 0;
      }
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Unknown error';

      if (this.connectionAttempt < maxAttempts) {
        // Calculate delay with exponential backoff + jitter
        const exponentialDelay = baseDelayMs * Math.pow(backoffMultiplier, this.connectionAttempt - 1);
        const jitter = Math.random() * 1000; // Add up to 1s of jitter
        const delay = Math.min(exponentialDelay + jitter, maxDelayMs);

        console.warn(
          `[Holochain] Connection failed (attempt ${this.connectionAttempt}/${maxAttempts}): ${errorMessage}. ` +
          `Retrying in ${Math.round(delay / 1000)}s...`
        );

        // Schedule retry
        this.retryTimeoutId = setTimeout(() => {
          if (!this.isDestroyed) {
            this.attemptConnection();
          }
        }, delay);
      } else {
        // Max retries reached - log and continue without throwing
        console.error(
          `[Holochain] Connection failed after ${maxAttempts} attempts: ${errorMessage}. ` +
          'App will continue without Holochain connectivity.'
        );
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
    this.initializeHolochainConnection();
  }

  /** Check if URL is the root landing page (/ or empty) */
  private isRootLandingPage(url: string): boolean {
    // Strip query params and fragments
    const path = url.split('?')[0].split('#')[0];
    return path === '/' || path === '';
  }
}
