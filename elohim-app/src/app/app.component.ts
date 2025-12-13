import { Component, OnInit, inject } from '@angular/core';
import { RouterOutlet, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { filter } from 'rxjs/operators';
import { ThemeToggleComponent } from './components/theme-toggle/theme-toggle.component';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { HolochainContentService } from './elohim/services/holochain-content.service';
import { environment } from '../environments/environment';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, ThemeToggleComponent, CommonModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit {
  title = 'elohim-app';
  /** Show floating theme toggle only on root landing page */
  showFloatingToggle = false;

  private readonly holochainService = inject(HolochainClientService);
  private readonly holochainContent = inject(HolochainContentService);

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

  /**
   * Initialize Holochain connection on app startup.
   *
   * Attempts full connection flow:
   * 1. Connect to admin interface (via dev-proxy in Che)
   * 2. Discover existing app interfaces
   * 3. Authorize signing credentials
   * 4. Connect to app interface for zome calls
   * 5. Test content service availability
   *
   * Runs in background without blocking app initialization.
   */
  private async initializeHolochainConnection(): Promise<void> {
    // Only attempt connection if holochain config exists
    if (!environment.holochain?.adminUrl) {
      console.log('[Holochain] Config not found, skipping auto-connect');
      return;
    }

    try {
      console.log('[Holochain] Initializing full connection...');

      // Attempt full connection (admin + app interface)
      await this.holochainService.connect();

      if (this.holochainService.isConnected()) {
        console.log('[Holochain] Connection successful, testing content availability...');

        // Test if content service can make zome calls
        const contentAvailable = await this.holochainContent.testAvailability();

        if (contentAvailable) {
          console.log('[Holochain] Content service available - Holochain data ready!');
        } else {
          console.warn('[Holochain] Connected but content service unavailable');
        }
      }
    } catch (err) {
      // Holochain connection is required - no fallback
      console.error('[Holochain] Connection failed:', err);
      throw err;
    }
  }

  /** Check if URL is the root landing page (/ or empty) */
  private isRootLandingPage(url: string): boolean {
    // Strip query params and fragments
    const path = url.split('?')[0].split('#')[0];
    return path === '/' || path === '';
  }
}
