import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { Subject, forkJoin, of } from 'rxjs';
import { takeUntil, catchError } from 'rxjs/operators';
import { PathService } from '../../services/path.service';
import { ProfileService } from '../../services/profile.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { PathIndex, PathIndexEntry } from '../../models/learning-path.model';
import { CurrentFocus } from '../../models/profile.model';

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
  styleUrls: ['./lamad-home.component.css']
})
export class LamadHomeComponent implements OnInit, OnDestroy {
  paths: PathIndexEntry[] = [];
  featuredPath: PathIndexEntry | null = null;
  activeFocus: CurrentFocus | null = null;

  /** Map of pathId to progress percentage (0-100) */
  pathProgressMap: Map<string, number> = new Map();

  isLoading = true;
  error: string | null = null;

  // View mode toggle
  viewMode: 'paths' | 'explore' = 'paths';

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly pathService: PathService,
    private readonly router: Router,
    private readonly profileService: ProfileService,
    private readonly agentService: AgentService
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

    const isAuth = this.agentService.getCurrentAgentId() !== 'anonymous';

    const tasks = isAuth
      ? [
          this.pathService.listPaths(),
          this.profileService.getCurrentFocus().pipe(catchError(() => of([]))),
          this.agentService.getAgentProgress().pipe(catchError(() => of([])))
        ]
      : [this.pathService.listPaths()];

    forkJoin(tasks).pipe(
      takeUntil(this.destroy$)
    ).subscribe({
      next: (results) => {
        const index = results[0] as PathIndex;
        this.paths = index.paths || [];

        if (isAuth && results.length > 1 && results[1] && Array.isArray(results[1]) && results[1].length > 0) {
            // Sort by most recent activity
            const focus = (results[1] as CurrentFocus[]).sort((a, b) =>
                new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime()
            );
            this.activeFocus = focus[0];
        }

        // Build progress map from agent progress records
        if (isAuth && results.length > 2 && results[2]) {
          const progressRecords = results[2] as Array<{ pathId: string; completedStepIndices: number[]; completedAt?: string }>;
          this.pathProgressMap.clear();

          // Calculate progress for each path
          progressRecords
            .filter(p => p.pathId !== '__global__') // Skip global progress record
            .forEach(progress => {
              // Find the path to get total step count
              const path = this.paths.find(p => p.id === progress.pathId);
              if (path && path.stepCount > 0) {
                const percent = progress.completedAt
                  ? 100
                  : Math.round((progress.completedStepIndices.length / path.stepCount) * 100);
                this.pathProgressMap.set(progress.pathId, percent);
              }
            });
        }

        // Feature the Elohim Protocol path
        this.featuredPath = this.paths.find(p => p.id === 'elohim-protocol') || this.paths[0] || null;

        this.isLoading = false;
      },
      error: () => {
        this.error = 'Unable to load learning paths';
        this.isLoading = false;
      }
    });
  }

  /**
   * Navigate to a path overview
   */
  goToPath(pathId: string): void {
    this.router.navigate(['/lamad/path', pathId]);
  }

  /**
   * Start the featured path directly at step 0
   */
  startFeaturedPath(): void {
    if (this.featuredPath) {
      this.router.navigate(['/lamad/path', this.featuredPath.id, 'step', 0]);
    }
  }

  /**
   * Continue the active journey
   */
  continueActiveJourney(): void {
    if (this.activeFocus) {
      this.router.navigate(['/lamad/path', this.activeFocus.pathId, 'step', this.activeFocus.currentStepIndex]);
    }
  }

  /**
   * Navigate to explore/map view
   */
  goToExplore(): void {
    this.router.navigate(['/lamad/explore']);
  }

  /**
   * Navigate to search
   */
  goToSearch(): void {
    this.router.navigate(['/lamad/search']);
  }

  /**
   * Navigate to learner dashboard
   */
  goToDashboard(): void {
    this.router.navigate(['/lamad/me']);
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
      'beginner': 'Beginner',
      'intermediate': 'Intermediate',
      'advanced': 'Advanced'
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
      this.router.navigate(['/lamad/explore']);
    }
  }
}
