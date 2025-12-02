import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { Subject, forkJoin } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { SeoService } from '../../../services/seo.service';
import { LearningPath, PathStep, PathChapter } from '../../models/learning-path.model';
import { AgentProgress } from '../../models/agent.model';
import { ContentNode } from '../../models/content-node.model';

interface EnrichedStep {
  step: PathStep;
  content: ContentNode;
  isCompleted: boolean;
  isGlobalCompletion: boolean;
  icon: string;
  isLocked: boolean;
}

interface ChapterDisplay {
  chapter: PathChapter;
  metrics: {
    totalUniqueContent: number;
    completedUniqueContent: number;
    contentCompletionPercentage: number;
    sharedContentCompleted: number;
    completedSteps: number;
    totalSteps: number;
  };
  steps: EnrichedStep[];
  absoluteStartIndex: number;
}

/**
 * PathOverviewComponent - Landing page for a learning path.
 *
 * Displays:
 * - Path metadata (title, description, purpose)
 * - Chapter/Step outline with "Territory Mastery" visualization
 * - Continue/Begin Journey buttons
 * - Estimated duration and difficulty
 *
 * Route: /lamad/path/:pathId
 *
 * Implements Section 1.1 of LAMAD_API_SPECIFICATION_v1.0.md
 * Updated to support Khan Academy-style "Territory Mastery" views.
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
  
  // Chapter Data
  chapters: ChapterDisplay[] = [];
  pathCompletion: any = null; // Territory-based completion metrics
  
  // Concept Progress Data
  conceptProgress: Array<{
    conceptId: string;
    title: string;
    totalSteps: number;
    completedSteps: number;
    completionPercentage: number;
  }> = [];

  // Flat steps for paths without chapters
  flatSteps: EnrichedStep[] = [];

  isLoading = true;
  error: string | null = null;

  private readonly destroy$ = new Subject<void>();

  private readonly seoService = inject(SeoService);

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
      accessible: this.pathService.getAccessibleSteps(this.pathId),
      completion: this.pathService.getPathCompletionByContent(this.pathId),
      chapterSummaries: this.pathService.getChapterSummariesWithContent(this.pathId),
      allSteps: this.pathService.getAllStepsWithCompletionStatus(this.pathId),
      concepts: this.pathService.getConceptProgressForPath(this.pathId)
    }).pipe(takeUntil(this.destroy$)).subscribe({
      next: ({ path, progress, accessible, completion, chapterSummaries, allSteps, concepts }) => {
        this.path = path;
        this.progress = progress;
        this.accessibleSteps = accessible;
        this.pathCompletion = completion;
        this.conceptProgress = concepts;

        // Update SEO metadata for this path
        this.seoService.updateForPath({
          id: path.id,
          title: path.title,
          description: path.description,
          thumbnailUrl: path.thumbnailUrl,
          difficulty: path.difficulty,
          estimatedDuration: path.estimatedDuration
        });
        
        // Map all steps to EnrichedStep format
        const enrichedSteps: EnrichedStep[] = allSteps.map((s, i) => ({
          step: s.step,
          content: s.content,
          isCompleted: s.isCompleted ?? false,
          isGlobalCompletion: s.completedInOtherPath,
          icon: this.getContentIcon(s.content.contentType),
          isLocked: !accessible.includes(i)
        }));

        if (chapterSummaries.length > 0) {
          // Map chapter summaries to display format with steps
          let currentIndex = 0;
          this.chapters = chapterSummaries.map(summary => {
            const chapterSteps = enrichedSteps.slice(currentIndex, currentIndex + summary.totalSteps);
            const display: ChapterDisplay = {
              chapter: summary.chapter,
              metrics: {
                totalUniqueContent: summary.totalUniqueContent,
                completedUniqueContent: summary.completedUniqueContent,
                contentCompletionPercentage: summary.contentCompletionPercentage,
                sharedContentCompleted: summary.sharedContentCompleted,
                completedSteps: summary.completedSteps,
                totalSteps: summary.totalSteps
              },
              steps: chapterSteps,
              absoluteStartIndex: currentIndex
            };
            currentIndex += summary.totalSteps;
            return display;
          });
        } else {
          this.flatSteps = enrichedSteps;
        }

        this.isLoading = false;
      },
      error: err => {
        this.error = err.message ?? 'Failed to load path';
        this.isLoading = false;
      }
    });
  }

  /**
   * Get icon for content type
   */
  getContentIcon(contentType: string): string {
    const icons: Record<string, string> = {
      'epic': '📖',
      'feature': '⚡',
      'scenario': '✓',
      'concept': '💡',
      'simulation': '🎮',
      'video': '🎥',
      'assessment': '📝',
      'organization': '🏢',
      'book-chapter': '📚',
      'tool': '🛠️'
    };
    return icons[contentType] || '📄';
  }

  /**
   * Get the current step index for the user
   */
  getCurrentStepIndex(): number {
    if (!this.progress || this.progress.completedStepIndices.length === 0) {
      return 0;
    }
    const maxCompleted = Math.max(...this.progress.completedStepIndices);
    return Math.min(maxCompleted + 1, (this.path?.steps.length ?? 1) - 1);
  }

  /**
   * Check if user has started this path
   */
  hasStarted(): boolean {
    return this.progress !== null && this.progress.completedStepIndices.length > 0;
  }

  /**
   * Check if user has completed this path.
   * Uses territory completion (100% content mastered) when available,
   * otherwise falls back to step completion logic.
   */
  isCompleted(): boolean {
    if (this.pathCompletion && this.pathCompletion.contentCompletionPercentage === 100) {
      return true;
    }
    if (!this.path || !this.progress) return false;
    const requiredSteps = this.path.steps.filter(s => !s.optional);
    return requiredSteps.length > 0 && requiredSteps.every(step =>
      this.progress!.completedStepIndices.includes(step.order)
    );
  }

  /**
   * Get completion percentage (Territory Mastery preferred)
   */
  getCompletionPercentage(): number {
    if (this.pathCompletion) {
      return this.pathCompletion.contentCompletionPercentage;
    }
    return 0;
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
   * Start specific chapter
   */
  startChapter(chapterId: string): void {
    this.pathService.getChapterFirstStep(this.pathId, chapterId).subscribe(step => {
      if (step) {
        this.router.navigate(['/lamad/path', this.pathId, 'step', step.step.order]);
      }
    });
  }

  /**
   * Navigate to a specific step
   */
  goToStep(stepIndex: number): void {
    if (this.accessibleSteps.includes(stepIndex)) {
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
    return displays[this.path?.difficulty ?? ''] ?? this.path?.difficulty ?? '';
  }

  /**
   * Handle image load error by hiding the element
   */
  onImageError(event: Event): void {
    const img = event.target as HTMLImageElement;
    img.style.display = 'none';
  }
}
