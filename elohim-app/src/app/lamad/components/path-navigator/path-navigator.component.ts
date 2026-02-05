import { CommonModule, DOCUMENT } from '@angular/common';
import {
  Component,
  OnInit,
  OnDestroy,
  ChangeDetectorRef,
  inject,
  HostListener,
  Inject,
} from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

// @coverage: 81.9% (2026-02-05)

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { AgentService } from '@app/elohim/services/agent.service';
import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';

import { SeoService } from '../../../services/seo.service';
import { MasteryLevel } from '../../models/content-mastery.model';
import { PathContext } from '../../models/exploration-context.model';
import {
  PathStepView,
  LearningPath,
  PathChapter,
  PathModule,
  PathSection,
} from '../../models/learning-path.model';
import { RendererCompletionEvent } from '../../renderers/renderer-registry.service';
import { PathContextService } from '../../services/path-context.service';
import { PathService } from '../../services/path.service';
import { getIconForContent, inferContentTypeFromId } from '../../utils/content-icons';
import { FocusedViewToggleComponent } from '../focused-view-toggle/focused-view-toggle.component';
import { LessonViewComponent } from '../lesson-view/lesson-view.component';

/**
 * Concept item in the lesson sidebar - represents one concept within the current section/lesson
 */
interface LessonConcept {
  conceptId: string;
  title: string;
  isCompleted: boolean;
  isCurrent: boolean;
  index: number; // Index within the lesson
  icon: string; // Icon based on content type
}

/**
 * Current lesson context - tracks where we are in the hierarchy
 */
interface LessonContext {
  chapter: PathChapter;
  chapterIndex: number;
  module: PathModule;
  moduleIndex: number;
  section: PathSection;
  sectionIndex: number;
  concepts: LessonConcept[];
  currentConceptIndex: number;
}

/**
 * Parameters for building section context - groups related data
 */
interface SectionContextParams {
  chapter: PathChapter;
  chapterIndex: number;
  module: PathModule;
  moduleIndex: number;
  section: PathSection;
  sectionIndex: number;
  globalIndex: number;
  currentConceptIndex: number;
}

/**
 * PathNavigatorComponent - The main learning interface.
 *
 * Implements a split-view "Course Player" layout:
 * - Left Sidebar: Current lesson's concepts (focused, not overwhelming)
 * - Sidebar Header: Module/Lesson title for context
 * - Main Area: Breadcrumbs, Content, Navigation Footer
 *
 * Hierarchy: Path → Chapter → Module → Section (Lesson) → Concepts
 * The sidebar shows concepts within the current Section (Lesson).
 *
 * Route: /lamad/path/:pathId/step/:stepIndex
 */
@Component({
  selector: 'app-path-navigator',
  standalone: true,
  imports: [CommonModule, RouterModule, LessonViewComponent, FocusedViewToggleComponent],
  templateUrl: './path-navigator.component.html',
  styleUrls: ['./path-navigator.component.css'],
})
export class PathNavigatorComponent implements OnInit, OnDestroy {
  // Route params
  pathId = '';
  stepIndex = 0; // Global concept index across all sections

  // Data
  stepView: PathStepView | null = null;
  path: LearningPath | null = null;

  // Lesson Context - where we are in the hierarchy
  lessonContext: LessonContext | null = null;

  // Bloom's Mastery State (Prototype)
  currentBloomLevel: MasteryLevel = 'not_started';
  readonly BLOOM_LEVELS: MasteryLevel[] = [
    'not_started',
    'seen',
    'remember',
    'understand',
    'apply',
    'analyze',
    'evaluate',
    'create',
  ];

  // UI state
  isLoading = true;
  error: string | null = null;
  sidebarOpen = true; // Default open, click backdrop to dismiss on mobile

  // Focused view (immersive mode) state
  isFocusedView = false;
  contentRefreshKey = 0; // Increment to trigger content reload
  private readonly TRANSITION_DURATION = 300; // Match CSS transition duration

  /** CSS class for focused view mode */
  private readonly FOCUSED_VIEW_MODE_CLASS = 'focused-view-mode';

  /** Base route for path navigation */
  private readonly PATH_ROUTE = '/lamad/path';

  private readonly destroy$ = new Subject<void>();
  private readonly seoService = inject(SeoService);

  // Learning signal tracking
  private contentViewStartTime: number | null = null;

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly pathService: PathService,
    private readonly agentService: AgentService,
    private readonly governanceSignalService: GovernanceSignalService,
    private readonly pathContextService: PathContextService,
    private readonly cdr: ChangeDetectorRef,
    @Inject(DOCUMENT) private readonly document: Document
  ) {}

  ngOnInit(): void {
    // Subscribe to route param changes
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      this.pathId = params['pathId'] as string;
      const parsed = Number.parseInt(params['stepIndex'] as string, 10);
      this.stepIndex = Number.isNaN(parsed) ? 0 : parsed;
      this.loadContext();
    });
  }

  ngOnDestroy(): void {
    // Emit progress signal before destroying
    this.emitProgressSignal();

    this.destroy$.next();
    this.destroy$.complete();
    // Exit path context when leaving the navigator
    this.pathContextService.exitPath();
    // Clean up focused view mode if active
    this.document.body.classList.remove(this.FOCUSED_VIEW_MODE_CLASS);
  }

  /**
   * Load path context and current step
   */
  private loadContext(): void {
    this.isLoading = true;
    this.error = null;

    // Load path metadata first
    this.pathService
      .getPath(this.pathId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: path => {
          this.path = path;

          // Build lesson context (find current position in hierarchy)
          this.buildLessonContext(path);

          // Load the current concept's content
          if (this.lessonContext) {
            const currentConcept =
              this.lessonContext.concepts[this.lessonContext.currentConceptIndex];
            this.loadConceptContent(currentConcept.conceptId, path);
          } else {
            // Fallback for paths without hierarchical structure
            this.pathService.getPathStep(this.pathId, this.stepIndex).subscribe({
              next: stepView => this.handleStepLoaded(stepView, path),
              error: err => this.handleError(err),
            });
          }
        },
        error: err => this.handleError(err),
      });
  }

  /**
   * Load content for a specific concept ID
   */
  private loadConceptContent(conceptId: string, path: LearningPath): void {
    // Use the content service to load the concept
    this.pathService
      .getContentById(conceptId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: content => {
          // Handle case where content is not found
          if (!content) {
            this.error = `Content not found: ${conceptId}`;
            this.isLoading = false;
            return;
          }

          // Build a PathStepView-like structure from the concept content
          this.stepView = {
            step: {
              order: this.stepIndex,
              resourceId: conceptId,
              stepTitle: content.title ?? this.formatConceptTitle(conceptId),
              stepNarrative: content.description ?? '',
              learningObjectives: [],
              optional: false,
              completionCriteria: [],
            },
            content: content,
            isCompleted: false,
            hasPrevious: this.stepIndex > 0,
            hasNext: this.stepIndex < this.getTotalConcepts() - 1,
            previousStepIndex: this.stepIndex > 0 ? this.stepIndex - 1 : undefined,
            nextStepIndex:
              this.stepIndex < this.getTotalConcepts() - 1 ? this.stepIndex + 1 : undefined,
          };

          // Update SEO
          const conceptTitle = content?.title ?? this.formatConceptTitle(conceptId);
          const lessonTitle = this.lessonContext?.section.title ?? '';
          this.seoService.updateSeo({
            title: `${conceptTitle} - ${lessonTitle} - ${path.title}`,
            description: content?.description ?? `Learning ${conceptTitle}`,
            openGraph: {
              ogType: 'article',
              ogImage: content?.metadata?.['thumbnailUrl'] ?? path.thumbnailUrl,
              articleSection: 'Learning',
            },
          });

          // Mark content as seen and emit learning signal
          if (content) {
            this.agentService
              .markContentSeen(conceptId)
              .pipe(takeUntil(this.destroy$))
              .subscribe({
                error: (err: unknown) => {
                  console.error('Failed to mark content as seen:', err);
                },
              });
            this.agentService
              .getContentMastery(conceptId)
              .pipe(takeUntil(this.destroy$))
              .subscribe({
                next: level => {
                  this.currentBloomLevel = level;
                },
                error: (err: unknown) => {
                  console.error('Failed to get content mastery:', err);
                },
              });

            // Track view start time for learning signals
            this.contentViewStartTime = Date.now();

            // Emit content viewed learning signal
            this.governanceSignalService
              .recordLearningSignal({
                contentId: conceptId,
                signalType: 'content_viewed',
                payload: {
                  pathId: this.pathId,
                  stepIndex: this.stepIndex,
                  contentType: content.contentType,
                  chapter: this.lessonContext?.chapter.title,
                  module: this.lessonContext?.module.title,
                  section: this.lessonContext?.section.title,
                },
              })
              .pipe(takeUntil(this.destroy$))
              .subscribe({
                error: (err: unknown) => {
                  console.error('Failed to record learning signal:', err);
                },
              });
          }

          this.isLoading = false;
          this.pathContextService.enterPath(this.buildPathContext());
          this.cdr.detectChanges();
        },
        error: err => this.handleError(err),
      });
  }

  /**
   * Handle step loaded (fallback for non-hierarchical paths)
   */
  private handleStepLoaded(stepView: PathStepView, path: LearningPath): void {
    // Handle case where content is not found
    if (!stepView.content) {
      this.error = `Content not found: ${stepView.step.resourceId}`;
      this.isLoading = false;
      return;
    }

    this.stepView = stepView;

    const stepTitle = stepView.content?.title ?? stepView.step.stepTitle;
    this.seoService.updateSeo({
      title: `${stepTitle} - ${path.title}`,
      description: stepView.content?.description ?? `Step ${this.stepIndex + 1} of ${path.title}`,
      openGraph: {
        ogType: 'article',
        ogImage: stepView.content?.metadata?.['thumbnailUrl'] ?? path.thumbnailUrl,
        articleSection: 'Learning',
      },
    });

    if (stepView.step.resourceId) {
      this.agentService
        .markContentSeen(stepView.step.resourceId)
        .pipe(takeUntil(this.destroy$))
        .subscribe({
          error: (err: unknown) => {
            console.error('Failed to mark content as seen:', err);
          },
        });
      this.agentService
        .getContentMastery(stepView.step.resourceId)
        .pipe(takeUntil(this.destroy$))
        .subscribe({
          next: level => {
            this.currentBloomLevel = level;
          },
          error: (err: unknown) => {
            console.error('Failed to get content mastery:', err);
          },
        });
    }

    this.isLoading = false;
    this.pathContextService.enterPath(this.buildPathContext());
    this.cdr.detectChanges();
  }

  /**
   * Get total concept count across all sections.
   * Handles both 4-level (modules) and 2-level (steps) formats.
   */
  getTotalConcepts(): number {
    if (!this.path?.chapters) return 0;
    let total = 0;
    for (const chapter of this.path.chapters) {
      // 2-level format: chapters with direct steps
      if (chapter.steps && chapter.steps.length > 0) {
        total += chapter.steps.length;
      } else {
        // 4-level format: modules → sections → conceptIds
        for (const module of chapter.modules ?? []) {
          for (const section of module.sections ?? []) {
            total += section.conceptIds?.length ?? 0;
          }
        }
      }
    }
    return total;
  }

  private handleError(err: unknown): void {
    this.error = err instanceof Error ? err.message : 'Failed to load learning path';
    this.isLoading = false;
  }

  /**
   * Build the lesson context - find where we are in the hierarchy
   * and build the concept list for the current section/lesson.
   *
   * Handles both:
   * - 4-level: chapters → modules → sections → conceptIds
   * - 2-level: chapters → steps (each step has resourceId)
   */
  private buildLessonContext(path: LearningPath): void {
    if (!path.chapters || path.chapters.length === 0) {
      this.lessonContext = null;
      return;
    }

    // Check if we have 2-level format (chapters with direct steps)
    const firstChapter = path.chapters[0];
    const is2Level = firstChapter.steps && firstChapter.steps.length > 0;

    if (is2Level) {
      this.buildLessonContext2Level(path);
    } else {
      this.buildLessonContext4Level(path);
    }
  }

  /**
   * Build lesson context for 2-level hierarchy (chapters → steps)
   */
  private buildLessonContext2Level(path: LearningPath): void {
    let globalIndex = 0;
    for (let ci = 0; ci < path.chapters!.length; ci++) {
      const chapter = path.chapters![ci];
      const chapterSteps = chapter.steps ?? [];

      // Check if current stepIndex falls within this chapter
      if (this.stepIndex >= globalIndex && this.stepIndex < globalIndex + chapterSteps.length) {
        const currentConceptIndex = this.stepIndex - globalIndex;

        // Build concepts list from chapter steps
        const concepts: LessonConcept[] = chapterSteps.map((step, idx) => ({
          conceptId: step.resourceId,
          title: step.stepTitle ?? this.formatConceptTitle(step.resourceId),
          isCompleted: false, // TODO: Load from progress
          isCurrent: idx === currentConceptIndex,
          index: globalIndex + idx,
          icon: getIconForContent(step.resourceId, inferContentTypeFromId(step.resourceId)),
        }));

        // Create a synthetic section for the UI
        const syntheticSection = {
          id: `${chapter.id}-section`,
          title: chapter.title,
          order: 0,
          conceptIds: chapterSteps.map(s => s.resourceId),
        };

        this.lessonContext = {
          chapter,
          chapterIndex: ci,
          module: {
            id: `${chapter.id}-module`,
            title: chapter.title,
            order: 0,
            sections: [syntheticSection],
          },
          moduleIndex: 0,
          section: syntheticSection,
          sectionIndex: 0,
          concepts,
          currentConceptIndex,
        };
        return;
      }
      globalIndex += chapterSteps.length;
    }

    // Fallback: use first chapter
    if (path.chapters!.length > 0) {
      const chapter = path.chapters![0];
      const chapterSteps = chapter.steps ?? [];
      if (chapterSteps.length > 0) {
        const syntheticSection = {
          id: `${chapter.id}-section`,
          title: chapter.title,
          order: 0,
          conceptIds: chapterSteps.map(s => s.resourceId),
        };
        this.lessonContext = {
          chapter,
          chapterIndex: 0,
          module: {
            id: `${chapter.id}-module`,
            title: chapter.title,
            order: 0,
            sections: [syntheticSection],
          },
          moduleIndex: 0,
          section: syntheticSection,
          sectionIndex: 0,
          concepts: chapterSteps.map((step, idx) => ({
            conceptId: step.resourceId,
            title: step.stepTitle ?? this.formatConceptTitle(step.resourceId),
            isCompleted: false,
            isCurrent: idx === 0,
            index: idx,
            icon: getIconForContent(step.resourceId, inferContentTypeFromId(step.resourceId)),
          })),
          currentConceptIndex: 0,
        };
      }
    }
  }

  /**
   * Build lesson context for 4-level hierarchy (chapters -> modules -> sections -> conceptIds)
   */
  private buildLessonContext4Level(path: LearningPath): void {
    const context = this.findLessonContextInPath(path);
    if (context) {
      this.lessonContext = context;
      return;
    }
    // Fallback: stepIndex out of range, use first section
    this.lessonContext = this.buildDefaultLessonContext(path);
  }

  /**
   * Search through path hierarchy to find context for current stepIndex.
   */
  private findLessonContextInPath(path: LearningPath): LessonContext | null {
    let globalIndex = 0;
    const chapters = path.chapters ?? [];

    for (let ci = 0; ci < chapters.length; ci++) {
      const chapter = chapters[ci];
      const result = this.findContextInChapter(chapter, ci, globalIndex);
      if (result.context) {
        return result.context;
      }
      globalIndex = result.nextGlobalIndex;
    }
    return null;
  }

  /**
   * Search within a chapter for the current stepIndex context.
   */
  private findContextInChapter(
    chapter: PathChapter,
    chapterIndex: number,
    globalIndex: number
  ): { context: LessonContext | null; nextGlobalIndex: number } {
    const modules = chapter.modules ?? [];

    for (let mi = 0; mi < modules.length; mi++) {
      const module = modules[mi];
      const result = this.findContextInModule(chapter, chapterIndex, module, mi, globalIndex);
      if (result.context) {
        return result;
      }
      globalIndex = result.nextGlobalIndex;
    }
    return { context: null, nextGlobalIndex: globalIndex };
  }

  /**
   * Search within a module for the current stepIndex context.
   */
  private findContextInModule(
    chapter: PathChapter,
    chapterIndex: number,
    module: PathModule,
    moduleIndex: number,
    globalIndex: number
  ): { context: LessonContext | null; nextGlobalIndex: number } {
    const sections = module.sections ?? [];

    for (let si = 0; si < sections.length; si++) {
      const section = sections[si];
      const conceptCount = section.conceptIds?.length ?? 0;

      if (this.stepIndex >= globalIndex && this.stepIndex < globalIndex + conceptCount) {
        const currentConceptIndex = this.stepIndex - globalIndex;
        return {
          context: this.buildSectionContext({
            chapter,
            chapterIndex,
            module,
            moduleIndex,
            section,
            sectionIndex: si,
            globalIndex,
            currentConceptIndex,
          }),
          nextGlobalIndex: globalIndex + conceptCount,
        };
      }
      globalIndex += conceptCount;
    }
    return { context: null, nextGlobalIndex: globalIndex };
  }

  /**
   * Build lesson context for a specific section.
   */
  private buildSectionContext(params: SectionContextParams): LessonContext {
    const concepts: LessonConcept[] = (params.section.conceptIds ?? []).map((conceptId, idx) => ({
      conceptId,
      title: this.formatConceptTitle(conceptId),
      isCompleted: false, // TODO: Load from progress
      isCurrent: idx === params.currentConceptIndex,
      index: params.globalIndex + idx,
      icon: getIconForContent(conceptId, inferContentTypeFromId(conceptId)),
    }));

    return {
      chapter: params.chapter,
      chapterIndex: params.chapterIndex,
      module: params.module,
      moduleIndex: params.moduleIndex,
      section: params.section,
      sectionIndex: params.sectionIndex,
      concepts,
      currentConceptIndex: params.currentConceptIndex,
    };
  }

  /**
   * Build default lesson context using first available section.
   */
  private buildDefaultLessonContext(path: LearningPath): LessonContext | null {
    const chapters = path.chapters ?? [];
    if (chapters.length === 0) return null;

    const chapter = chapters[0];
    const modules = chapter.modules ?? [];
    if (modules.length === 0) return null;

    const module = modules[0];
    const sections = module.sections ?? [];
    if (sections.length === 0) return null;

    return this.buildSectionContext({
      chapter,
      chapterIndex: 0,
      module,
      moduleIndex: 0,
      section: sections[0],
      sectionIndex: 0,
      globalIndex: 0,
      currentConceptIndex: 0,
    });
  }

  /**
   * Format concept ID to display title (kebab-case to Title Case)
   */
  private formatConceptTitle(conceptId: string): string {
    return conceptId
      .split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  toggleSidebar(): void {
    this.sidebarOpen = !this.sidebarOpen;
  }

  /**
   * Navigate to a specific concept in the lesson
   */
  goToConcept(globalIndex: number): void {
    this.router
      .navigate([this.PATH_ROUTE, this.pathId, 'step', globalIndex])
      .catch((err: unknown) => {
        console.error('Navigation failed:', err);
      });
  }

  /**
   * Navigation Methods
   */
  goToStep(index: number): void {
    // Emit progress signal before navigating
    this.emitProgressSignal();
    this.router.navigate([this.PATH_ROUTE, this.pathId, 'step', index]).catch((err: unknown) => {
      console.error('Navigation failed:', err);
    });
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
    this.router.navigate([this.PATH_ROUTE, this.pathId]).catch((err: unknown) => {
      console.error('Navigation failed:', err);
    });
  }

  /**
   * Toggle through Bloom's Taxonomy levels (Prototype)
   */
  markComplete(): void {
    // Cycle to next level
    const currentIndex = this.BLOOM_LEVELS.indexOf(this.currentBloomLevel);
    const nextIndex = (currentIndex + 1) % this.BLOOM_LEVELS.length;
    this.currentBloomLevel = this.BLOOM_LEVELS[nextIndex];

    // Persist basic completion to backend if we reach a 'completed' state
    // For prototype, let's say 'remember' is enough to mark the step complete navigation-wise
    if (this.currentBloomLevel === 'remember' || this.currentBloomLevel === 'apply') {
      this.agentService
        .completeStep(this.pathId, this.stepIndex)
        .pipe(takeUntil(this.destroy$))
        .subscribe({
          next: () => {
            // Refresh sidebar
            this.loadContext();
          },
          error: (err: unknown) => {
            console.error('Failed to complete step:', err);
          },
        });
    }
  }

  /**
   * Helper: Get current chapter title for breadcrumbs
   */
  getCurrentChapterTitle(): string | undefined {
    return this.lessonContext?.chapter.title;
  }

  /**
   * Helper: Get current module/lesson title for sidebar header
   */
  getCurrentModuleTitle(): string | undefined {
    return this.lessonContext?.module.title;
  }

  /**
   * Helper: Get current section title
   */
  getCurrentSectionTitle(): string | undefined {
    return this.lessonContext?.section.title;
  }

  /**
   * Get progress percentage
   */
  getProgressPercentage(): number {
    const total = this.getTotalConcepts();
    if (total === 0) return 0;
    return Math.round(((this.stepIndex + 1) / total) * 100);
  }

  /**
   * Get progress within current lesson (section)
   */
  getLessonProgressPercentage(): number {
    if (!this.lessonContext) return 0;
    const total = this.lessonContext.concepts.length;
    if (total === 0) return 0;
    return Math.round(((this.lessonContext.currentConceptIndex + 1) / total) * 100);
  }

  /**
   * Format Bloom level for display
   */
  getBloomDisplay(): string {
    return this.currentBloomLevel.replace('_', ' ').toUpperCase();
  }

  // =========================================================================
  // Path Context & Exploration Methods
  // =========================================================================

  /**
   * Build path context for LessonView and exploration tracking.
   */
  buildPathContext(): PathContext {
    return {
      pathId: this.pathId,
      pathTitle: this.path?.title ?? '',
      stepIndex: this.stepIndex,
      totalSteps: this.getTotalSteps(),
      chapterTitle: this.getCurrentChapterTitle(),
      returnRoute: [this.PATH_ROUTE, this.pathId, 'step', String(this.stepIndex)],
    };
  }

  /**
   * Get total step count for the path (alias for getTotalConcepts).
   */
  getTotalSteps(): number {
    return this.getTotalConcepts();
  }

  /**
   * Handle exploration of related content from LessonView.
   */
  onExploreContent(contentId: string): void {
    // Track the detour in path context
    this.pathContextService.startDetour({
      fromContentId: this.stepView?.content?.id ?? '',
      toContentId: contentId,
      detourType: 'related',
      timestamp: new Date().toISOString(),
    });

    // Navigate to the content
    this.router.navigate(['/lamad/resource', contentId]).catch((err: unknown) => {
      console.error('Navigation failed:', err);
    });
  }

  /**
   * Handle "Explore in Full Graph" from LessonView.
   */
  onExploreInGraph(): void {
    const contentId = this.stepView?.content?.id;
    if (!contentId) return;

    // Track the detour
    this.pathContextService.startDetour({
      fromContentId: contentId,
      toContentId: contentId,
      detourType: 'graph-explore',
      timestamp: new Date().toISOString(),
    });

    // Navigate to graph explorer with context
    this.router
      .navigate(['/lamad/explore'], {
        queryParams: {
          focus: contentId,
          fromPath: this.pathId,
          returnStep: this.stepIndex,
        },
      })
      .catch((err: unknown) => {
        console.error('Navigation failed:', err);
      });
  }

  /**
   * Handle completion event from LessonView (interactive content).
   */
  onLessonComplete(event: RendererCompletionEvent): void {
    const contentId = this.stepView?.content?.id;
    if (!contentId) return;

    // Calculate time spent on this content
    const timeSpent = this.contentViewStartTime
      ? Math.round((Date.now() - this.contentViewStartTime) / 1000)
      : 0;

    // Emit interactive completion learning signal
    this.governanceSignalService
      .recordInteractiveCompletion({
        contentId,
        interactionType: event.type ?? 'interactive',
        passed: event.passed,
        score: event.score,
        details: {
          ...event.details,
          pathId: this.pathId,
          stepIndex: this.stepIndex,
          timeSpentSeconds: timeSpent,
          chapter: this.lessonContext?.chapter.title,
          module: this.lessonContext?.module.title,
        },
      })
      .pipe(takeUntil(this.destroy$))
      .subscribe();

    // If passed, advance mastery level
    if (event.passed) {
      this.markComplete();
    }
  }

  /**
   * Emit progress signal when navigating away (time spent tracking).
   */
  private emitProgressSignal(): void {
    const contentId = this.stepView?.content?.id;
    if (!contentId || !this.contentViewStartTime) return;

    const timeSpent = Math.round((Date.now() - this.contentViewStartTime) / 1000);

    // Only emit if meaningful time was spent (>5 seconds)
    if (timeSpent > 5) {
      this.governanceSignalService
        .recordLearningSignal({
          contentId,
          signalType: 'progress_update',
          payload: {
            pathId: this.pathId,
            stepIndex: this.stepIndex,
            timeSpentSeconds: timeSpent,
            masteryLevel: this.currentBloomLevel,
            progressPercent: this.getProgressPercentage(),
          },
        })
        .pipe(takeUntil(this.destroy$))
        .subscribe();
    }

    this.contentViewStartTime = null;
  }

  // =========================================================================
  // Focused View (Immersive Mode) Methods
  // =========================================================================

  /**
   * Handle escape key to exit focused view mode.
   */
  @HostListener('document:keydown.escape')
  onEscapeKey(): void {
    if (this.isFocusedView) {
      this.onFocusedViewToggle(false);
    }
  }

  /**
   * Toggle focused view mode.
   * Waits for CSS transition to complete before reloading content
   * so iframes can measure the new viewport dimensions correctly.
   */
  onFocusedViewToggle(active: boolean): void {
    this.isFocusedView = active;

    // Hide sidebar in focused view
    if (active) {
      this.sidebarOpen = false;
      this.document.body.classList.add(this.FOCUSED_VIEW_MODE_CLASS);
    } else {
      this.document.body.classList.remove(this.FOCUSED_VIEW_MODE_CLASS);
    }

    // Wait for CSS transition to complete, then trigger content reload
    // This ensures iframes get the correct viewport dimensions
    setTimeout(() => {
      this.contentRefreshKey = Date.now();
      this.cdr.detectChanges();
    }, this.TRANSITION_DURATION);
  }
}
