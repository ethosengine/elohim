import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy } from '@angular/core';
import { Router, RouterModule } from '@angular/router';

// @coverage: 86.3% (2026-02-05)

import { takeUntil, catchError } from 'rxjs/operators';

import { Subject, forkJoin, of } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';
import { ProfileService } from '@app/elohim/services/profile.service';
import { IdentityService, isNetworkMode } from '@app/imagodei/services/identity.service';

import { PathIndex, PathIndexEntry } from '../../models/learning-path.model';
import { CurrentFocus } from '../../models/profile.model';
import { PathFilterService } from '../../services/path-filter.service';
import { PathService } from '../../services/path.service';

/**
 * LamadHomeComponent - Dual-mode landing page.
 *
 * Supports two view modes:
 * - 'paths': Path-centric view (guided journeys)
 * - 'explore': Link to graph exploration (Khan Academy style)
 *
 * According to spec Section 1.6:
 * - For unauthenticated: Show featured paths
 * - For authenticated: Show "My Learning" + "Recommended"
 * - Explore mode available for discovery
 *
 * Route: /lamad
 */
@Component({
  selector: 'app-lamad-home',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './lamad-home.component.html',
  styleUrls: ['./lamad-home.component.css'],
})
export class LamadHomeComponent implements OnInit, OnDestroy {
  /** Featured paths shown initially (limited for performance) */
  paths: PathIndexEntry[] = [];

  /** Full path list (loaded on demand) */
  private allPaths: PathIndexEntry[] = [];

  /** Whether to show all paths or just featured */
  showAllPaths = false;

  /** Number of featured paths to display initially */
  private readonly FEATURED_PATH_LIMIT = 6;

  /** Base route for path navigation */
  private readonly PATH_ROUTE = '/lamad/path';

  featuredPath: PathIndexEntry | null = null;
  activeFocus: CurrentFocus | null = null;

  /** Map of pathId to progress percentage (0-100) */
  pathProgressMap = new Map<string, number>();

  isLoading = true;
  error: string | null = null;

  // View mode toggle
  viewMode: 'paths' | 'explore' = 'paths';

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly pathService: PathService,
    private readonly pathFilterService: PathFilterService,
    private readonly router: Router,
    private readonly profileService: ProfileService,
    private readonly agentService: AgentService,
    private readonly identityService: IdentityService
  ) {
    // Load saved view mode preference
    const savedMode = localStorage.getItem('lamad-view-mode');
    if (savedMode === 'explore' || savedMode === 'paths') {
      this.viewMode = savedMode;
    }
  }

  ngOnInit(): void {
    this.loadPaths();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  private loadPaths(): void {
    this.isLoading = true;
    this.error = null;

    // Use IdentityService for auth check - properly reflects doorway login state
    const isAuth = isNetworkMode(this.identityService.mode());

    const tasks = isAuth
      ? [
          this.pathService.listPaths(),
          this.profileService.getCurrentFocus().pipe(catchError(() => of([]))),
          this.agentService.getAgentProgress().pipe(catchError(() => of([]))),
        ]
      : [this.pathService.listPaths()];

    forkJoin(tasks)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: results => {
          const index = results[0] as PathIndex;
          // Store full path list for "View All" expansion
          this.allPaths = index.paths || [];

          // Show only featured paths initially (performance optimization)
          this.paths = this.pathFilterService.getFeaturedPaths(
            this.allPaths,
            this.FEATURED_PATH_LIMIT
          );

          if (
            isAuth &&
            results.length > 1 &&
            results[1] &&
            Array.isArray(results[1]) &&
            results[1].length > 0
          ) {
            // Sort by most recent activity
            const focus = (results[1] as CurrentFocus[]).sort(
              (a, b) => new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime()
            );
            this.activeFocus = focus[0];
          }

          // Build progress map from agent progress records
          if (isAuth && results.length > 2 && results[2]) {
            const progressRecords = results[2] as {
              pathId: string;
              completedStepIndices: number[];
              completedAt?: string;
            }[];
            this.pathProgressMap.clear();

            // Calculate progress for each path (use allPaths for lookup)
            progressRecords
              .filter(p => p.pathId !== '__global__') // Skip global progress record
              .forEach(progress => {
                // Find the path to get total step count
                const path = this.allPaths.find(p => p.id === progress.pathId);
                if (path && path.stepCount > 0) {
                  const percent = progress.completedAt
                    ? 100
                    : Math.round((progress.completedStepIndices.length / path.stepCount) * 100);
                  this.pathProgressMap.set(progress.pathId, percent);
                }
              });
          }

          // Feature the Elohim Protocol path
          this.featuredPath =
            this.allPaths.find(p => p.id === 'elohim-protocol') ?? this.allPaths[0] ?? null;

          this.isLoading = false;
        },
        error: () => {
          this.error = 'Unable to load learning paths';
          this.isLoading = false;
        },
      });
  }

  /**
   * Toggle between featured paths and all paths
   */
  toggleShowAllPaths(): void {
    this.showAllPaths = !this.showAllPaths;
    this.paths = this.showAllPaths
      ? this.allPaths
      : this.pathFilterService.getFeaturedPaths(this.allPaths, this.FEATURED_PATH_LIMIT);
  }

  /**
   * Get the total number of paths available
   */
  get totalPathCount(): number {
    return this.allPaths.length;
  }

  /**
   * Check if there are more paths to show
   */
  get hasMorePaths(): boolean {
    return this.allPaths.length > this.FEATURED_PATH_LIMIT;
  }

  /**
   * Navigate to a path overview
   */
  goToPath(pathId: string): void {
    void this.router.navigate([this.PATH_ROUTE, pathId]);
  }

  /**
   * Start the featured path directly at step 0
   */
  startFeaturedPath(): void {
    if (this.featuredPath) {
      void this.router.navigate([this.PATH_ROUTE, this.featuredPath.id, 'step', 0]);
    }
  }

  /**
   * Continue the active journey
   */
  continueActiveJourney(): void {
    if (this.activeFocus) {
      void this.router.navigate([
        this.PATH_ROUTE,
        this.activeFocus.pathId,
        'step',
        this.activeFocus.currentStepIndex,
      ]);
    }
  }

  /**
   * Navigate to explore/map view
   */
  goToExplore(): void {
    void this.router.navigate(['/lamad/explore']);
  }

  /**
   * Navigate to search
   */
  goToSearch(): void {
    void this.router.navigate(['/lamad/search']);
  }

  /**
   * Navigate to learner dashboard
   */
  goToDashboard(): void {
    void this.router.navigate(['/lamad/me']);
  }

  /**
   * Get difficulty badge class
   */
  getDifficultyClass(difficulty: string): string {
    return difficulty || 'beginner';
  }

  /**
   * Format difficulty for display
   */
  formatDifficulty(difficulty: string): string {
    const displays: Record<string, string> = {
      beginner: 'Beginner',
      intermediate: 'Intermediate',
      advanced: 'Advanced',
    };
    return displays[difficulty] || difficulty;
  }

  /**
   * Get progress percentage for a path (0-100, or null if no progress)
   */
  getPathProgress(pathId: string): number | null {
    return this.pathProgressMap.has(pathId) ? this.pathProgressMap.get(pathId)! : null;
  }

  /**
   * Set view mode and save preference
   */
  setViewMode(mode: 'paths' | 'explore'): void {
    this.viewMode = mode;
    localStorage.setItem('lamad-view-mode', mode);

    // If explore mode, navigate to graph explorer
    if (mode === 'explore') {
      void this.router.navigate(['/lamad/explore']);
    }
  }
}
