import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { Subject, forkJoin } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { LearningPath, PathStep } from '../../models/learning-path.model';
import { AgentProgress } from '../../models/agent.model';

/**
 * PathOverviewComponent - Landing page for a learning path.
 *
 * Displays:
 * - Path metadata (title, description, purpose)
 * - Step outline with progress indicators
 * - Continue/Begin Journey buttons
 * - Estimated duration and difficulty
 *
 * Route: /lamad/path/:pathId
 *
 * Implements Section 1.1 of LAMAD_API_SPECIFICATION_v1.0.md (Path Overview Route)
 */
@Component({
  selector: 'app-path-overview',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './path-overview.component.html',
  styleUrls: ['./path-overview.component.css']
})
export class PathOverviewComponent implements OnInit, OnDestroy {
  pathId: string = '';
  path: LearningPath | null = null;
  progress: AgentProgress | null = null;
  accessibleSteps: number[] = [];

  isLoading = true;
  error: string | null = null;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly pathService: PathService,
    private readonly agentService: AgentService
  ) {}

  ngOnInit(): void {
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      this.pathId = params['pathId'];
      this.loadPath();
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  private loadPath(): void {
    this.isLoading = true;
    this.error = null;

    forkJoin({
      path: this.pathService.getPath(this.pathId),
      progress: this.agentService.getProgressForPath(this.pathId),
      accessible: this.pathService.getAccessibleSteps(this.pathId)
    }).pipe(takeUntil(this.destroy$)).subscribe({
      next: ({ path, progress, accessible }) => {
        this.path = path;
        this.progress = progress;
        this.accessibleSteps = accessible;
        this.isLoading = false;
      },
      error: err => {
        this.error = err.message || 'Failed to load path';
        this.isLoading = false;
        console.error('[PathOverview] Failed to load path:', err);
      }
    });
  }

  /**
   * Get the current step index for the user
   */
  getCurrentStepIndex(): number {
    if (!this.progress || this.progress.completedStepIndices.length === 0) {
      return 0;
    }
    const maxCompleted = Math.max(...this.progress.completedStepIndices);
    return Math.min(maxCompleted + 1, (this.path?.steps.length || 1) - 1);
  }

  /**
   * Check if user has started this path
   */
  hasStarted(): boolean {
    return this.progress !== null && this.progress.completedStepIndices.length > 0;
  }

  /**
   * Check if user has completed this path
   */
  isCompleted(): boolean {
    if (!this.path || !this.progress) return false;
    const requiredSteps = this.path.steps.filter(s => !s.optional);
    return requiredSteps.every((_, i) =>
      this.progress!.completedStepIndices.includes(i)
    );
  }

  /**
   * Get completion percentage
   */
  getCompletionPercentage(): number {
    if (!this.path || !this.progress) return 0;
    const total = this.path.steps.length;
    const completed = this.progress.completedStepIndices.length;
    return Math.round((completed / total) * 100);
  }

  /**
   * Check if a step is completed
   */
  isStepCompleted(stepIndex: number): boolean {
    return this.progress?.completedStepIndices.includes(stepIndex) ?? false;
  }

  /**
   * Check if a step is accessible (fog-of-war)
   */
  isStepAccessible(stepIndex: number): boolean {
    return this.accessibleSteps.includes(stepIndex);
  }

  /**
   * Check if a step is locked (fog-of-war)
   */
  isStepLocked(stepIndex: number): boolean {
    return !this.isStepAccessible(stepIndex);
  }

  /**
   * Navigate to begin the journey
   */
  beginJourney(): void {
    this.router.navigate(['/lamad/path', this.pathId, 'step', 0]);
  }

  /**
   * Continue from current position
   */
  continueJourney(): void {
    const currentStep = this.getCurrentStepIndex();
    this.router.navigate(['/lamad/path', this.pathId, 'step', currentStep]);
  }

  /**
   * Navigate to a specific step
   */
  goToStep(stepIndex: number): void {
    if (this.isStepAccessible(stepIndex)) {
      this.router.navigate(['/lamad/path', this.pathId, 'step', stepIndex]);
    }
  }

  /**
   * Go back to home
   */
  goHome(): void {
    this.router.navigate(['/lamad']);
  }

  /**
   * Get difficulty display
   */
  getDifficultyDisplay(): string {
    const displays: Record<string, string> = {
      'beginner': 'Beginner',
      'intermediate': 'Intermediate',
      'advanced': 'Advanced'
    };
    return displays[this.path?.difficulty || ''] || this.path?.difficulty || '';
  }

  /**
   * Get step status class for styling
   */
  getStepStatusClass(stepIndex: number): string {
    if (this.isStepCompleted(stepIndex)) return 'completed';
    if (this.isStepLocked(stepIndex)) return 'locked';
    if (stepIndex === this.getCurrentStepIndex()) return 'current';
    return 'accessible';
  }
}
