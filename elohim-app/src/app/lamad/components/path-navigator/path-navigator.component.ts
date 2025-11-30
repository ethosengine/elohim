import { Component, OnInit, OnDestroy, ViewChild, ViewContainerRef, ComponentRef, ChangeDetectorRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';
import { Subject, Subscription } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { PathService } from '../../services/path.service';
import { AgentService } from '../../services/agent.service';
import { PathStepView, LearningPath, PathChapter } from '../../models/learning-path.model';
import { AgentProgress } from '../../models/agent.model';
import { BLOOM_LEVEL_VALUES, BloomMasteryLevel } from '../../models/content-mastery.model';
import { RendererRegistryService, ContentRenderer } from '../../renderers/renderer-registry.service';
import { ContentNode } from '../../models/content-node.model';

interface SidebarStep {
  index: number;
  title: string;
  isCompleted: boolean;
  isLocked: boolean;
  isCurrent: boolean;
  isGlobalCompletion: boolean; // True if completed in another path
  icon: string;
}

interface SidebarChapter {
  id: string;
  title: string;
  steps: SidebarStep[];
  isExpanded: boolean;
}

/**
 * PathNavigatorComponent - The main learning interface.
 *
 * Implements a split-view "Course Player" layout:
 * - Left Sidebar: Contextual navigation (Chapters/Steps)
 * - Main Area: Breadcrumbs, Content, Navigation Footer
 *
 * Route: /lamad/path/:pathId/step/:stepIndex
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
  
  // Sidebar State
  sidebarChapters: SidebarChapter[] = [];
  flatSidebarSteps: SidebarStep[] = []; // Fallback for paths without chapters
  currentChapterId: string | null = null;

  // Bloom's Mastery State (Prototype)
  currentBloomLevel: BloomMasteryLevel = 'not_started';
  readonly BLOOM_LEVELS: BloomMasteryLevel[] = [
    'not_started',
    'seen',
    'remember',
    'understand',
    'apply',
    'analyze',
    'evaluate',
    'create'
  ];

  // UI state
  isLoading = true;
  error: string | null = null;
  sidebarOpen = false; // Default closed on mobile, toggled by user

  // Dynamic renderer hosting
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;
  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;

  /** Whether we have a registered renderer for the current content format */
  hasRegisteredRenderer = false;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly pathService: PathService,
    private readonly agentService: AgentService,
    private readonly rendererRegistry: RendererRegistryService,
    private readonly cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    // Subscribe to route param changes
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      this.pathId = params['pathId'];
      this.stepIndex = parseInt(params['stepIndex'], 10) || 0;
      this.loadContext();
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
  }

  /**
   * Clean up the current renderer instance
   */
  private destroyRenderer(): void {
    if (this.rendererSubscription) {
      this.rendererSubscription.unsubscribe();
      this.rendererSubscription = null;
    }
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }

  /**
   * Dynamically instantiate the appropriate renderer for the current content.
   * Called after the step is loaded and the view is ready.
   */
  private loadRenderer(): void {
    if (!this.stepView?.content || !this.rendererHost) {
      this.hasRegisteredRenderer = false;
      return;
    }

    // Clean up previous renderer
    this.destroyRenderer();
    this.rendererHost.clear();

    // Get the renderer component for this content format
    const rendererComponent = this.rendererRegistry.getRenderer(this.stepView.content);

    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      return;
    }

    this.hasRegisteredRenderer = true;

    // Create the renderer component
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);

    // Set the node input using setInput to trigger ngOnChanges
    this.rendererRef.setInput('node', this.stepView.content);

    // Set embedded mode if the renderer supports it (e.g., markdown renderer)
    // This tells the renderer to adapt to the container rather than imposing its own layout
    if ('embedded' in this.rendererRef.instance) {
      this.rendererRef.setInput('embedded', true);
    }
  }

  /**
   * Load path context and current step
   */
  private loadContext(): void {
    this.isLoading = true;
    this.error = null;

    // Load full path context for sidebar (using the efficient bulk method)
    this.pathService.getAllStepsWithCompletionStatus(this.pathId).subscribe({
      next: (stepsWithStatus) => {
        // 1. Load the Path Metadata
        this.pathService.getPath(this.pathId).pipe(takeUntil(this.destroy$)).subscribe(path => {
          this.path = path;
          
          // 2. Build Sidebar Structure
          this.buildSidebar(path, stepsWithStatus);
          
          // 3. Set Current Step Data
          this.pathService.getPathStep(this.pathId, this.stepIndex).subscribe({
            next: (stepView) => {
              this.stepView = stepView;
              
              // Initialize Bloom level based on completion status (Prototype)
              if (stepView.isCompleted) {
                // If already completed, assume at least 'apply' for demo purposes
                // Real impl would fetch actual ContentMastery
                this.currentBloomLevel = 'apply';
              } else {
                this.currentBloomLevel = 'not_started';
              }

              this.determineCurrentChapter();
              this.isLoading = false;

              // Ensure view updates so @if blocks resolve and rendererHost is available
              this.cdr.detectChanges();

              // Load the appropriate renderer for this content format
              this.loadRenderer();
            },
            error: (err) => this.handleError(err)
          });
        });
      },
      error: (err) => this.handleError(err)
    });
  }

  private handleError(err: any): void {
    this.error = err.message || 'Failed to load learning path';
    this.isLoading = false;
    console.error('[PathNavigator] Error:', err);
  }

  /**
   * Construct the sidebar data structure
   */
  private buildSidebar(path: LearningPath, stepsWithStatus: any[]): void {
    if (path.chapters && path.chapters.length > 0) {
      // Structure by chapters
      let globalStepIndex = 0;
      this.sidebarChapters = path.chapters.map(chapter => {
        const chapterSteps: SidebarStep[] = chapter.steps.map((step, idx) => {
          const status = stepsWithStatus[globalStepIndex];
          const sidebarStep: SidebarStep = {
            index: globalStepIndex,
            title: step.stepTitle,
            isCompleted: status.isCompleted,
            isLocked: false, // Todo: wire up exact locking logic if needed beyond basic completion
            isCurrent: globalStepIndex === this.stepIndex,
            isGlobalCompletion: status.completedInOtherPath,
            icon: this.getContentIcon(status.content?.contentType)
          };
          globalStepIndex++;
          return sidebarStep;
        });

        return {
          id: chapter.id,
          title: chapter.title,
          steps: chapterSteps,
          isExpanded: true // Logic to expand current chapter later
        };
      });
    } else {
      // Flat list
      this.flatSidebarSteps = stepsWithStatus.map((status, index) => ({
        index: index,
        title: path.steps[index].stepTitle,
        isCompleted: status.isCompleted,
        isLocked: false,
        isCurrent: index === this.stepIndex,
        isGlobalCompletion: status.completedInOtherPath,
        icon: this.getContentIcon(status.content?.contentType)
      }));
    }
  }

  private determineCurrentChapter(): void {
    if (!this.sidebarChapters.length) return;

    // Find which chapter contains the current step
    for (const chapter of this.sidebarChapters) {
      if (chapter.steps.some(s => s.index === this.stepIndex)) {
        this.currentChapterId = chapter.id;
        chapter.isExpanded = true;
        break;
      }
    }
  }

  private getContentIcon(contentType: string): string {
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

  toggleSidebar(): void {
    this.sidebarOpen = !this.sidebarOpen;
  }

  toggleChapter(chapterId: string): void {
    const chapter = this.sidebarChapters.find(c => c.id === chapterId);
    if (chapter) {
      chapter.isExpanded = !chapter.isExpanded;
    }
  }

  /**
   * Navigation Methods
   */
  goToStep(index: number): void {
    this.router.navigate(['/lamad/path', this.pathId, 'step', index]);
  }

  goToPrevious(): void {
    if (this.stepView?.hasPrevious && this.stepView.previousStepIndex !== undefined) {
      this.goToStep(this.stepView.previousStepIndex);
    }
  }

  goToNext(): void {
    if (this.stepView?.hasNext && this.stepView.nextStepIndex !== undefined) {
      this.goToStep(this.stepView.nextStepIndex);
    }
  }

  goToPathOverview(): void {
    this.router.navigate(['/lamad/path', this.pathId]);
  }

  /**
   * Toggle through Bloom's Taxonomy levels (Prototype)
   */
  markComplete(): void {
    // Cycle to next level
    const currentIndex = this.BLOOM_LEVELS.indexOf(this.currentBloomLevel);
    const nextIndex = (currentIndex + 1) % this.BLOOM_LEVELS.length;
    this.currentBloomLevel = this.BLOOM_LEVELS[nextIndex];

    console.log(`[Prototype] Mastery Level: ${this.currentBloomLevel}`);

    // Persist basic completion to backend if we reach a 'completed' state
    // For prototype, let's say 'remember' is enough to mark the step complete navigation-wise
    if (this.currentBloomLevel === 'remember' || this.currentBloomLevel === 'apply') {
      this.agentService.completeStep(this.pathId, this.stepIndex).subscribe({
        next: () => {
          // Refresh sidebar
          this.loadContext(); 
        }
      });
    }
  }

  /**
   * Helper: Get current chapter title for breadcrumbs
   */
  getCurrentChapterTitle(): string | undefined {
    return this.sidebarChapters.find(c => c.id === this.currentChapterId)?.title;
  }

  /**
   * Get progress percentage
   */
  getProgressPercentage(): number {
    if (!this.path) return 0;
    return Math.round(((this.stepIndex + 1) / this.path.steps.length) * 100);
  }

  /**
   * Format Bloom level for display
   */
  getBloomDisplay(): string {
    return this.currentBloomLevel.replace('_', ' ').toUpperCase();
  }

  /**
   * Content Rendering Helpers
   */
  getContentString(): string {
    if (!this.stepView?.content?.content) return '';
    const content = this.stepView.content.content;
    return typeof content === 'string' ? content : JSON.stringify(content, null, 2);
  }

  isMarkdown(): boolean {
    return this.stepView?.content?.contentFormat === 'markdown';
  }

  isQuiz(): boolean {
    return this.stepView?.content?.contentFormat === 'quiz-json' ||
           this.stepView?.content?.contentType === 'assessment';
  }

  isGherkin(): boolean {
    return this.stepView?.content?.contentFormat === 'gherkin';
  }

}
