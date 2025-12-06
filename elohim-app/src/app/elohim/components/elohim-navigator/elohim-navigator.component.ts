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

/**\n * Context app configuration\n */\nexport interface ContextAppConfig {\n  id: ContextApp;\n  name: string;\n  icon: string;\n  route: string;\n  tagline: string;\n  available: boolean;\n}\n\n/**\n * ElohimNavigatorComponent - Unified navigation for all context apps\n *\n * Implements the \"Google Account\" pattern:\n * - Imago Dei as the identity layer (profile bubble / Traveler)\n * - Lamad/Qahal/Shefa as context apps (like YouTube/Gmail/News)\n * - Settings tray accessible from profile bubble\n * - Context switcher (app tray) for navigating between pillars\n */\n@Component({\n  selector: \'app-elohim-navigator\',\n  standalone: true,\n  imports: [CommonModule, RouterLink, FormsModule, ThemeToggleComponent],\n  templateUrl: \'./elohim-navigator.component.html\',\n  styleUrls: [\'./elohim-navigator.component.css\']\n})\nexport class ElohimNavigatorComponent implements OnInit, OnDestroy {\n  /** Current context app */\n  @Input() context: ContextApp = \'lamad\';\n\n  /** Whether to show search bar */\n  @Input() showSearch = true;\n\n  /** Search query */\n  searchQuery = \'\';\n\n  /** Session human state */\n  session: SessionHuman | null = null;\n  activeUpgradePrompt: HolochainUpgradePrompt | null = null;\n\n  /** UI state */\n  showProfileTray = false;\n  showContextSwitcher = false;\n  showUpgradeModal = false;\n\n  /** Available context apps */\n  readonly contextApps: ContextAppConfig[] = [\n    {\n      id: \'lamad\',\n      name: \'Lamad\',\n      icon: \'📚\',\n      route: \'/lamad\',\n      tagline: \'Learning & Content\',\n      available: true\n    },\n    {\n      id: \'community\',\n      name: \'Qahal\',\n      icon: \'👥\',\n      route: \'/community\',\n      tagline: \'Community & Governance\',\n      available: true\n    },\
    {\n      id: \'shefa\',\n      name: \'Shefa\',\n      icon: \'✨\',\n      route: \'/shefa\',\n      tagline: \'Economics of Flourishing\',\n      available: true\n    }\n  ];

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
    if (!app.available) return;
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
  public closeAllTrays(): void {
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
