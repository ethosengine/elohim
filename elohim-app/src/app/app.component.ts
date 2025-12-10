import { Component, OnInit, inject } from '@angular/core';
import { RouterOutlet, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { filter } from 'rxjs/operators';
import { ThemeToggleComponent } from './components/theme-toggle/theme-toggle.component';
import { HolochainClientService } from './elohim/services/holochain-client.service';
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
   * Phase 1: Tests admin proxy connectivity (list_apps only)
   * Phase 2: Will add full app interface proxying for zome calls
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
      console.log('[Holochain] Testing admin proxy connection (Phase 1)...');

      // Phase 1: Test admin connection only (list_apps)
      // Full connect() requires ADMIN permission for install_app
      // and app interface proxying which isn't implemented yet
      const result = await this.holochainService.testAdminConnection();

      if (result.success) {
        console.log('[Holochain] Admin proxy connection successful', {
          apps: result.apps
        });
      } else {
        console.warn('[Holochain] Admin proxy connection failed:', result.error);
      }
    } catch (err) {
      // Log error but don't crash the app - connection is optional
      console.warn('[Holochain] Edge Node auto-connect failed:', err);
      // Connection state will be 'error' and visible in the UI
    }
  }

  /** Check if URL is the root landing page (/ or empty) */
  private isRootLandingPage(url: string): boolean {
    // Strip query params and fragments
    const path = url.split('?')[0].split('#')[0];
    return path === '/' || path === '';
  }
}
