import { Component, OnInit, OnDestroy, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink, Router, NavigationEnd } from '@angular/router';
import { FormsModule } from '@angular/forms';
import { Subject } from 'rxjs';
import { filter, takeUntil } from 'rxjs/operators';
import { SessionHumanService } from '../../../imagodei/services/session-human.service';
import { SessionHuman, HolochainUpgradePrompt } from '../../../imagodei/models/session-human.model';
import { ThemeToggleComponent } from '../../../components/theme-toggle/theme-toggle.component';

/**
 * Context app identifiers for the Elohim Protocol
 */
export type ContextApp = 'lamad' | 'community' | 'shefa';

/**
 * Context app configuration
 */
export interface ContextAppConfig {
  id: ContextApp;
  name: string;
  icon: string;
  route: string;
  tagline: string;
  available: boolean;
}

/**
 * ElohimNavigatorComponent - Unified navigation for all context apps
 *
 * Implements the "Google Account" pattern:
 * - Imago Dei as the identity layer (profile bubble / Traveler)
 * - Lamad/Qahal/Shefa as context apps (like YouTube/Gmail/News)
 * - Settings tray accessible from profile bubble
 * - Context switcher (app tray) for navigating between pillars
 */
@Component({
  selector: 'app-elohim-navigator',
  standalone: true,
  imports: [CommonModule, RouterLink, FormsModule, ThemeToggleComponent],
  templateUrl: './elohim-navigator.component.html',
  styleUrls: ['./elohim-navigator.component.css']
})
export class ElohimNavigatorComponent implements OnInit, OnDestroy {
  /** Current context app */
  @Input() context: ContextApp = 'lamad';

  /** Whether to show search bar */
  @Input() showSearch = true;

  /** Search query */
  searchQuery = '';

  /** Session human state */
  session: SessionHuman | null = null;
  activeUpgradePrompt: HolochainUpgradePrompt | null = null;

  /** UI state */
  showProfileTray = false;
  showContextSwitcher = false;
  showUpgradeModal = false;

  /** Available context apps */
  readonly contextApps: ContextAppConfig[] = [
    {
      id: 'lamad',
      name: 'Lamad',
      icon: '📚',
      route: '/lamad',
      tagline: 'Learning & Content',
      available: true
    },
    {
      id: 'community',
      name: 'Qahal',
      icon: '👥',
      route: '/community',
      tagline: 'Community & Governance',
      available: true
    },
    {
      id: 'shefa',
      name: 'Shefa',
      icon: '✨',
      route: '/shefa',
      tagline: 'Economics of Flourishing',
      available: true
    }
  ];

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly sessionHumanService: SessionHumanService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    // Subscribe to session human state
    this.sessionHumanService.session$.pipe(takeUntil(this.destroy$)).subscribe(session => {
      this.session = session;
    });

    // Subscribe to upgrade prompts
    this.sessionHumanService.upgradePrompts$.pipe(takeUntil(this.destroy$)).subscribe(prompts => {
      this.activeUpgradePrompt = prompts.find(p => !p.dismissed) || null;
    });

    // Close trays on navigation
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      takeUntil(this.destroy$)
    ).subscribe(() => {
      this.closeAllTrays();
    });

    // Close trays on click outside
    document.addEventListener('click', this.handleOutsideClick.bind(this));
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    document.removeEventListener('click', this.handleOutsideClick.bind(this));
  }

  // =========================================================================
  // Context App Methods
  // =========================================================================

  /**
   * Get the current context app config
   */
  get currentApp(): ContextAppConfig {
    return this.contextApps.find(app => app.id === this.context) || this.contextApps[0];
  }

  /**
   * Get other available context apps
   */
  get otherApps(): ContextAppConfig[] {
    return this.contextApps.filter(app => app.id !== this.context);
  }

  /**
   * Toggle context switcher visibility
   */
  toggleContextSwitcher(event: Event): void {
    event.stopPropagation();
    this.showContextSwitcher = !this.showContextSwitcher;
    if (this.showContextSwitcher) {
      this.showProfileTray = false;
    }
  }

  /**
   * Switch to a different context app
   */
  switchContext(app: ContextAppConfig): void {
    this.showContextSwitcher = false;
    this.router.navigate([app.route]);
  }

  // =========================================================================
  // Profile Bubble Methods
  // =========================================================================

  /**
   * Toggle profile tray visibility
   */
  toggleProfileTray(event: Event): void {
    event.stopPropagation();
    this.showProfileTray = !this.showProfileTray;
    if (this.showProfileTray) {
      this.showContextSwitcher = false;
    }
  }

  /**
   * Get display name for session human
   */
  getDisplayName(): string {
    return this.session?.displayName ?? 'Traveler';
  }

  /**
   * Get initials for avatar
   */
  getInitials(): string {
    const name = this.getDisplayName();
    return name.charAt(0).toUpperCase();
  }

  /**
   * Get session stats summary
   */
  getStatsSummary(): string {
    if (!this.session) return 'New traveler';
    const stats = this.session.stats;
    const parts: string[] = [];
    if (stats.nodesViewed > 0) parts.push(`${stats.nodesViewed} explored`);
    if (stats.pathsStarted > 0) parts.push(`${stats.pathsStarted} paths`);
    return parts.join(' · ') || 'New traveler';
  }

  /**
   * Navigate to profile page (context-specific)
   */
  goToProfile(): void {
    this.showProfileTray = false;
    this.router.navigate([`/${this.context}/human`]);
  }

  // =========================================================================
  // Search Methods
  // =========================================================================

  /**
   * Handle search submission
   */
  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate([`/${this.context}/search`], {
        queryParams: { q: this.searchQuery }
      });
    }
  }

  // =========================================================================
  // Upgrade Modal Methods
  // =========================================================================

  /**
   * Show the upgrade modal
   */
  openUpgradeModal(): void {
    this.showProfileTray = false;
    this.showUpgradeModal = true;
  }

  /**
   * Close the upgrade modal
   */
  closeUpgradeModal(): void {
    this.showUpgradeModal = false;
  }

  /**
   * Dismiss the current upgrade prompt
   */
  dismissUpgradePrompt(): void {
    if (this.activeUpgradePrompt) {
      this.sessionHumanService.dismissUpgradePrompt(this.activeUpgradePrompt.id);
    }
  }

  /**
   * Handle "Join Network" action
   */
  onJoinNetwork(): void {
    this.openUpgradeModal();
  }

  // =========================================================================
  // Private Methods
  // =========================================================================

  /**
   * Close all dropdown trays
   */
  private closeAllTrays(): void {
    this.showProfileTray = false;
    this.showContextSwitcher = false;
  }

  /**
   * Handle clicks outside of trays
   */
  private handleOutsideClick(event: Event): void {
    const target = event.target as HTMLElement;
    if (!target.closest('.profile-bubble-container') && !target.closest('.context-switcher-container')) {
      this.closeAllTrays();
    }
  }
}
