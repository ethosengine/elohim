import { Component, OnInit, OnDestroy } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subject } from 'rxjs';
import { filter, takeUntil } from 'rxjs/operators';
import { DataLoaderService } from '../../services/data-loader.service';
import { SessionUserService } from '../../services/session-user.service';
import { RendererInitializerService } from '../../renderers/renderer-initializer.service';
import { ThemeToggleComponent } from '../../../components/theme-toggle/theme-toggle.component';
import { SessionUser, HolochainUpgradePrompt } from '../../models/session-user.model';

@Component({
  selector: 'app-lamad-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive, FormsModule, ThemeToggleComponent],
  templateUrl: './lamad-layout.component.html',
  styleUrls: ['./lamad-layout.component.css']
})
export class LamadLayoutComponent implements OnInit, OnDestroy {
  searchQuery = '';
  isReady = false;
  isHomePage = false;

  // Session human state
  session: SessionUser | null = null;
  activeUpgradePrompt: HolochainUpgradePrompt | null = null;
  showUpgradeModal = false;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly sessionUserService: SessionUserService,
    private readonly router: Router,
    // Injecting RendererInitializerService triggers renderer registration
    private readonly _rendererInit: RendererInitializerService
  ) {}

  ngOnInit(): void {
    // Verify data is loadable by fetching the content index
    this.dataLoader.getContentIndex().pipe(takeUntil(this.destroy$)).subscribe({
      next: (index) => {
        console.log('Lamad data ready:', index.nodes?.length || 0, 'content nodes');
        this.isReady = true;
      },
      error: (err: Error) => {
        console.error('Failed to load Lamad data:', err);
        this.isReady = true; // Still mark ready to show error state
      }
    });

    // Subscribe to session human state
    this.sessionUserService.session$.pipe(takeUntil(this.destroy$)).subscribe(session => {
      this.session = session;
    });

    // Subscribe to upgrade prompts
    this.sessionUserService.upgradePrompts$.pipe(takeUntil(this.destroy$)).subscribe(prompts => {
      // Show the most recent non-dismissed prompt
      this.activeUpgradePrompt = prompts.find(p => !p.dismissed) || null;
    });

    // Track route for UI state
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      takeUntil(this.destroy$)
    ).subscribe(() => {
      this.checkIfHomePage();
    });

    this.checkIfHomePage();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate(['/lamad/search'], {
        queryParams: { q: this.searchQuery }
      });
    }
  }

  private checkIfHomePage(): void {
    this.isHomePage = this.router.url === '/lamad' || this.router.url === '/lamad/';
  }

  // =========================================================================
  // Session Human UI Methods
  // =========================================================================

  /**
   * Get display name for session human.
   */
  getDisplayName(): string {
    return this.session?.displayName || 'Traveler';
  }

  /**
   * Get session stats summary.
   */
  getStatsSummary(): string {
    if (!this.session) return '';
    const stats = this.session.stats;
    const parts: string[] = [];
    if (stats.nodesViewed > 0) parts.push(`${stats.nodesViewed} explored`);
    if (stats.pathsStarted > 0) parts.push(`${stats.pathsStarted} paths`);
    return parts.join(' · ') || 'New traveler';
  }

  /**
   * Show the upgrade modal.
   */
  openUpgradeModal(): void {
    this.showUpgradeModal = true;
  }

  /**
   * Close the upgrade modal.
   */
  closeUpgradeModal(): void {
    this.showUpgradeModal = false;
  }

  /**
   * Dismiss the current upgrade prompt.
   */
  dismissUpgradePrompt(): void {
    if (this.activeUpgradePrompt) {
      this.sessionUserService.dismissUpgradePrompt(this.activeUpgradePrompt.id);
    }
  }

  /**
   * Handle "Join Network" action.
   * For MVP, shows the upgrade modal with Holochain install info.
   */
  onJoinNetwork(): void {
    this.openUpgradeModal();
  }
}
