import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { PathStepView, LearningPath } from '../../models/learning-path.model';

/**
 * PathNavigatorComponent - The main learning interface.
 *
 * Renders a single step within a learning path with:
 * - Step narrative and learning objectives
 * - Content node (rendered based on contentFormat)
 * - Previous/Next navigation
 * - Progress tracking
 *
 * Route: /lamad/path/:pathId/step/:stepIndex
 *
 * Implements Section 1.1 of LAMAD_API_SPECIFICATION_v1.0.md
 */
@Component({
  selector: 'app-path-navigator',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './path-navigator.component.html',
  styleUrls: ['./path-navigator.component.css']
})
export class PathNavigatorComponent implements OnInit, OnDestroy {
  // Route params
  pathId: string = '';
  stepIndex: number = 0;

  // Data
  stepView: PathStepView | null = null;
  path: LearningPath | null = null;

  // UI state
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
    // Subscribe to route param changes
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      this.pathId = params['pathId'];
      this.stepIndex = parseInt(params['stepIndex'], 10) || 0;
      this.loadStep();
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Load the current step using PathService.getPathStep()
   */
  private loadStep(): void {
    this.isLoading = true;
    this.error = null;

    // Load path metadata for header/progress
    this.pathService.getPath(this.pathId).pipe(
      takeUntil(this.destroy$)
    ).subscribe({
      next: path => {
        this.path = path;
      },
      error: err => {
        console.error('[PathNavigator] Failed to load path:', err);
      }
    });

    // Load specific step with content (lazy loading)
    this.pathService.getPathStep(this.pathId, this.stepIndex).pipe(
      takeUntil(this.destroy$)
    ).subscribe({
      next: stepView => {
        this.stepView = stepView;
        this.isLoading = false;
      },
      error: err => {
        this.error = err.message || 'Failed to load step';
        this.isLoading = false;
        console.error('[PathNavigator] Failed to load step:', err);
      }
    });
  }

  /**
   * Navigate to previous step
   */
  goToPrevious(): void {
    if (this.stepView?.hasPrevious && this.stepView.previousStepIndex !== undefined) {
      this.router.navigate(['/lamad/path', this.pathId, 'step', this.stepView.previousStepIndex]);
    }
  }

  /**
   * Navigate to next step
   */
  goToNext(): void {
    if (this.stepView?.hasNext && this.stepView.nextStepIndex !== undefined) {
      this.router.navigate(['/lamad/path', this.pathId, 'step', this.stepView.nextStepIndex]);
    }
  }

  /**
   * Navigate to path overview
   */
  goToPathOverview(): void {
    this.router.navigate(['/lamad/path', this.pathId]);
  }

  /**
   * Mark current step as complete
   */
  markComplete(): void {
    this.agentService.completeStep(this.pathId, this.stepIndex).subscribe({
      next: () => {
        // Reload to get updated progress
        this.loadStep();
      },
      error: (err: Error) => {
        console.error('[PathNavigator] Failed to mark step complete:', err);
      }
    });
  }

  /**
   * Get progress percentage
   */
  getProgressPercentage(): number {
    if (!this.path) return 0;
    return Math.round(((this.stepIndex + 1) / this.path.steps.length) * 100);
  }

  /**
   * Get content as string for rendering
   */
  getContentString(): string {
    if (!this.stepView?.content?.content) return '';
    const content = this.stepView.content.content;
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }

  /**
   * Check if content is markdown
   */
  isMarkdown(): boolean {
    return this.stepView?.content?.contentFormat === 'markdown';
  }

  /**
   * Check if content is a quiz
   */
  isQuiz(): boolean {
    return this.stepView?.content?.contentFormat === 'quiz-json' ||
           this.stepView?.content?.contentType === 'assessment';
  }

  /**
   * Check if content is gherkin
   */
  isGherkin(): boolean {
    return this.stepView?.content?.contentFormat === 'gherkin';
  }

  /**
   * Render markdown to HTML (basic implementation)
   */
  renderMarkdown(content: string): string {
    let html = content;

    // Headers
    html = html.replace(/^### (.*$)/gim, '<h3>$1</h3>');
    html = html.replace(/^## (.*$)/gim, '<h2>$1</h2>');
    html = html.replace(/^# (.*$)/gim, '<h1>$1</h1>');

    // Bold
    html = html.replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>');

    // Italic
    html = html.replace(/\*(.*?)\*/g, '<em>$1</em>');

    // Links
    html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank">$1</a>');

    // Code blocks
    html = html.replace(/```([^`]+)```/gs, '<pre><code>$1</code></pre>');

    // Inline code
    html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

    // Blockquotes
    html = html.replace(/^> (.*$)/gim, '<blockquote>$1</blockquote>');

    // Unordered lists
    html = html.replace(/^\- (.*$)/gim, '<li>$1</li>');

    // Paragraphs (double newline)
    html = html.replace(/\n\n/g, '</p><p>');

    // Single line breaks
    html = html.replace(/\n/g, '<br>');

    return `<p>${html}</p>`;
  }
}
