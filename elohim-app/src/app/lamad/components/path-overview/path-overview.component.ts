import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject } from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

import { takeUntil } from 'rxjs/operators';

import { Subject, forkJoin } from 'rxjs';

import { AgentProgress, MasteryLevel, MasteryTier } from '@app/elohim/models/agent.model';
import { AgentService } from '@app/elohim/services/agent.service';

import { SeoService } from '../../../services/seo.service';
import { LearningPath, PathStep, PathChapter, PathModule, PathSection } from '../../models';
import { PathService } from '../../services/path.service';
import {
  getStepTypeIcon,
  getIconForContent,
  inferContentTypeFromId,
} from '../../utils/content-icons';

/**
 * Lightweight step display data - uses step metadata, NOT content.
 * This avoids loading all content just to display the overview.
 */
interface StepDisplay {
  step: PathStep;
  stepIndex: number;
  isCompleted: boolean;
  isGlobalCompletion: boolean;
  icon: string;
  isLocked: boolean;
  masteryLevel: MasteryLevel;
  masteryTier: MasteryTier;
}

/**
 * Concept display for hierarchical paths using conceptIds
 */
interface ConceptDisplay {
  conceptId: string;
  title: string;
  description?: string;
  isCompleted: boolean;
  isGlobalCompletion: boolean;
  icon: string;
  contentType?: string;
}

/**
 * Section display - leaf level containing concepts
 */
interface SectionDisplay {
  section: PathSection;
  concepts: ConceptDisplay[];
  /** Completion percentage for progress indicator */
  completionPercentage: number;
  /** Total concepts in this section */
  totalConcepts: number;
  /** Completed concepts in this section */
  completedConcepts: number;
  /** Whether section is expanded in UI */
  isExpanded: boolean;
}

/**
 * Module display - contains sections
 */
interface ModuleDisplay {
  module: PathModule;
  sections: SectionDisplay[];
  totalConcepts: number;
  completedConcepts: number;
  /** Completion percentage for progress indicator */
  completionPercentage: number;
  /** Whether module is expanded in UI */
  isExpanded: boolean;
}

/**
 * Chapter display - hierarchical structure (Chapter → Module → Section → Concepts)
 */
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
  /** Modules in this chapter */
  modules: ModuleDisplay[];
  /** Total concepts across all modules/sections */
  totalConcepts: number;
  /** Completed concepts across all modules/sections */
  completedConcepts: number;
  /** Completion percentage for progress indicator */
  completionPercentage: number;
  /** Whether chapter is expanded in UI */
  isExpanded: boolean;
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
  styleUrls: ['./path-overview.component.css'],
})
export class PathOverviewComponent implements OnInit, OnDestroy {
  pathId = '';
  path: LearningPath | null = null;
  progress: AgentProgress | null = null;
  accessibleSteps: number[] = [];

  // Chapter Data
  chapters: ChapterDisplay[] = [];
  pathCompletion: any = null; // Territory-based completion metrics

  // Concept Progress Data
  conceptProgress: {
    conceptId: string;
    title: string;
    totalSteps: number;
    completedSteps: number;
    completionPercentage: number;
  }[] = [];

  // Flat steps for paths without chapters
  flatSteps: StepDisplay[] = [];

  isLoading = true;
  error: string | null = null;

  private readonly destroy$ = new Subject<void>();

  private readonly seoService = inject(SeoService);

  /** Base route for path navigation */
  private readonly PATH_ROUTE = '/lamad/path';

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

    // Use lightweight metadata loading - NO content loading for overview
    forkJoin({
      path: this.pathService.getPath(this.pathId),
      progress: this.agentService.getProgressForPath(this.pathId),
      accessible: this.pathService.getAccessibleSteps(this.pathId),
      completion: this.pathService.getPathCompletionByContent(this.pathId),
      chapterSummaries: this.pathService.getChapterSummariesWithContent(this.pathId),
      stepsMetadata: this.pathService.getAllStepsMetadata(this.pathId), // Lightweight!
      concepts: this.pathService.getConceptProgressForPath(this.pathId),
    })
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: ({
          path,
          progress,
          accessible,
          completion,
          chapterSummaries,
          stepsMetadata,
          concepts,
        }) => {
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
            estimatedDuration: path.estimatedDuration,
          });

          // Map steps to display format using METADATA only (no content loading)
          const displaySteps: StepDisplay[] = stepsMetadata.map(s => ({
            step: s.step,
            stepIndex: s.stepIndex,
            isCompleted: s.isCompleted,
            isGlobalCompletion: s.completedInOtherPath,
            icon: getStepTypeIcon(s.step.stepType || 'content'), // Use stepType from utility
            isLocked: !accessible.includes(s.stepIndex),
            masteryLevel: s.masteryLevel,
            masteryTier: s.masteryTier,
          }));

          if (chapterSummaries.length > 0) {
            // Map chapter summaries to display format with modules
            this.chapters = chapterSummaries.map(summary => {
              // Handle both 4-level (modules) and 2-level (steps) formats
              const moduleDisplays = this.buildModuleDisplaysFromChapter(summary.chapter);
              const totalConcepts = moduleDisplays.reduce((sum, m) => sum + m.totalConcepts, 0);
              const completedConcepts = moduleDisplays.reduce(
                (sum, m) => sum + m.completedConcepts,
                0
              );
              const completionPercentage =
                totalConcepts > 0 ? Math.round((completedConcepts / totalConcepts) * 100) : 0;

              const display: ChapterDisplay = {
                chapter: summary.chapter,
                metrics: {
                  totalUniqueContent: summary.totalUniqueContent,
                  completedUniqueContent: summary.completedUniqueContent,
                  contentCompletionPercentage: summary.contentCompletionPercentage,
                  sharedContentCompleted: summary.sharedContentCompleted,
                  completedSteps: summary.completedSteps,
                  totalSteps: summary.totalSteps,
                },
                modules: moduleDisplays,
                totalConcepts,
                completedConcepts,
                completionPercentage,
                isExpanded: true, // Default expanded
              };
              return display;
            });
          } else {
            this.flatSteps = displaySteps;
          }

          this.isLoading = false;
        },
        error: err => {
          this.error = err.message ?? 'Failed to load path';
          this.isLoading = false;
        },
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
    if (this.pathCompletion?.contentCompletionPercentage === 100) {
      return true;
    }
    if (!this.path || !this.progress) return false;
    const requiredSteps = this.path.steps.filter(s => !s.optional);
    return (
      requiredSteps.length > 0 &&
      requiredSteps.every(step => this.progress!.completedStepIndices.includes(step.order))
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
    this.router.navigate([this.PATH_ROUTE, this.pathId, 'step', 0]);
  }

  /**
   * Continue from current position
   */
  continueJourney(): void {
    const currentStep = this.getCurrentStepIndex();
    this.router.navigate([this.PATH_ROUTE, this.pathId, 'step', currentStep]);
  }

  /**
   * Start specific chapter
   */
  startChapter(chapterId: string): void {
    this.pathService.getChapterFirstStep(this.pathId, chapterId).subscribe(step => {
      if (step) {
        this.router.navigate([this.PATH_ROUTE, this.pathId, 'step', step.step.order]);
      }
    });
  }

  /**
   * Navigate to a specific step
   */
  goToStep(stepIndex: number): void {
    if (this.accessibleSteps.includes(stepIndex)) {
      this.router.navigate([this.PATH_ROUTE, this.pathId, 'step', stepIndex]);
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
      beginner: 'Beginner',
      intermediate: 'Intermediate',
      advanced: 'Advanced',
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

  /**
   * Get display text for mastery tier.
   */
  getMasteryTierLabel(tier: MasteryTier): string {
    const labels: Record<MasteryTier, string> = {
      unseen: 'Not started',
      seen: 'Seen',
      practiced: 'Practiced',
      applied: 'Applied',
      mastered: 'Mastered',
    };
    return labels[tier] ?? '';
  }

  /**
   * Get CSS class for mastery tier visual indicator.
   */
  getMasteryTierClass(tier: MasteryTier): string {
    return `mastery-tier-${tier}`;
  }

  /**
   * Get icon for mastery tier.
   */
  getMasteryTierIcon(tier: MasteryTier): string {
    const icons: Record<MasteryTier, string> = {
      unseen: '',
      seen: '👁',
      practiced: '✏️',
      applied: '🎯',
      mastered: '⭐',
    };
    return icons[tier] ?? '';
  }

  // =========================================================================
  // Expand/Collapse Handlers
  // =========================================================================

  /**
   * Toggle chapter expansion state.
   */
  toggleChapter(chapterId: string): void {
    const chapter = this.chapters.find(c => c.chapter.id === chapterId);
    if (chapter) {
      chapter.isExpanded = !chapter.isExpanded;
    }
  }

  /**
   * Toggle module expansion state.
   */
  toggleModule(chapterId: string, moduleId: string): void {
    const chapter = this.chapters.find(c => c.chapter.id === chapterId);
    if (chapter) {
      const mod = chapter.modules.find(m => m.module.id === moduleId);
      if (mod) {
        mod.isExpanded = !mod.isExpanded;
      }
    }
  }

  /**
   * Toggle section expansion state.
   */
  toggleSection(chapterId: string, moduleId: string, sectionId: string): void {
    const chapter = this.chapters.find(c => c.chapter.id === chapterId);
    if (chapter) {
      const mod = chapter.modules.find(m => m.module.id === moduleId);
      if (mod) {
        const sec = mod.sections.find(s => s.section.id === sectionId);
        if (sec) {
          sec.isExpanded = !sec.isExpanded;
        }
      }
    }
  }

  /**
   * Expand all chapters, modules, and sections.
   */
  expandAll(): void {
    for (const chapter of this.chapters) {
      chapter.isExpanded = true;
      for (const mod of chapter.modules) {
        mod.isExpanded = true;
        for (const sec of mod.sections) {
          sec.isExpanded = true;
        }
      }
    }
  }

  /**
   * Collapse all chapters, modules, and sections.
   */
  collapseAll(): void {
    for (const chapter of this.chapters) {
      chapter.isExpanded = false;
      for (const mod of chapter.modules) {
        mod.isExpanded = false;
        for (const sec of mod.sections) {
          sec.isExpanded = false;
        }
      }
    }
  }

  // =========================================================================
  // Hierarchical Structure Helpers
  // =========================================================================

  /**
   * Build module displays from a chapter, handling both formats:
   * - 4-level: chapters → modules → sections → conceptIds
   * - 2-level: chapters → steps (creates synthetic module/sections)
   */
  private buildModuleDisplaysFromChapter(chapter: PathChapter): ModuleDisplay[] {
    // Check if chapter has direct steps (2-level format)
    if (chapter.steps && chapter.steps.length > 0) {
      return this.buildModuleDisplaysFromSteps(chapter);
    }
    // Use 4-level format (modules → sections)
    return this.buildModuleDisplays(chapter.modules || []);
  }

  /**
   * Build synthetic module displays from chapter steps (2-level format).
   * Creates a single module with a single section containing all steps as concepts.
   */
  private buildModuleDisplaysFromSteps(chapter: PathChapter): ModuleDisplay[] {
    const steps = chapter.steps || [];
    const conceptIds = steps.map(s => s.resourceId);

    // Create a synthetic section
    const syntheticSection: PathSection = {
      id: `${chapter.id}-section`,
      title: chapter.title,
      order: 0,
      conceptIds,
    };

    // Create a synthetic module
    const syntheticModule: PathModule = {
      id: `${chapter.id}-module`,
      title: chapter.title,
      order: 0,
      sections: [syntheticSection],
    };

    const sectionDisplay: SectionDisplay = {
      section: syntheticSection,
      concepts: steps.map(step => {
        const inferredType = inferContentTypeFromId(step.resourceId);
        return {
          conceptId: step.resourceId,
          title: step.stepTitle,
          isCompleted: false, // TODO: Load from progress
          isGlobalCompletion: false,
          icon: getIconForContent(step.resourceId, inferredType),
          contentType: inferredType,
        };
      }),
      totalConcepts: steps.length,
      completedConcepts: 0, // TODO: Load from progress
      completionPercentage: 0,
      isExpanded: true,
    };

    return [
      {
        module: syntheticModule,
        sections: [sectionDisplay],
        totalConcepts: steps.length,
        completedConcepts: 0,
        completionPercentage: 0,
        isExpanded: true,
      },
    ];
  }

  /**
   * Build module display objects from chapter modules.
   */
  private buildModuleDisplays(modules: PathModule[]): ModuleDisplay[] {
    return modules.map(module => {
      const sections = this.buildSectionDisplays(module.sections || []);
      const totalConcepts = sections.reduce((sum, s) => sum + s.totalConcepts, 0);
      const completedConcepts = sections.reduce((sum, s) => sum + s.completedConcepts, 0);
      const completionPercentage =
        totalConcepts > 0 ? Math.round((completedConcepts / totalConcepts) * 100) : 0;

      return {
        module,
        sections,
        totalConcepts,
        completedConcepts,
        completionPercentage,
        isExpanded: true, // Default expanded for detailed view
      };
    });
  }

  /**
   * Build section display objects from module sections.
   */
  private buildSectionDisplays(sections: PathSection[]): SectionDisplay[] {
    return sections.map(section => {
      const concepts = this.buildConceptDisplays(section.conceptIds);
      const totalConcepts = concepts.length;
      const completedConcepts = concepts.filter(c => c.isCompleted || c.isGlobalCompletion).length;
      const completionPercentage =
        totalConcepts > 0 ? Math.round((completedConcepts / totalConcepts) * 100) : 0;

      return {
        section,
        concepts,
        totalConcepts,
        completedConcepts,
        completionPercentage,
        isExpanded: true, // Default expanded for detailed view
      };
    });
  }

  /**
   * Build concept display objects from concept IDs.
   * Uses conceptProgress data for completion status, utility functions for icons.
   */
  private buildConceptDisplays(conceptIds: string[]): ConceptDisplay[] {
    return conceptIds.map(conceptId => {
      const conceptData = this.conceptProgress.find(c => c.conceptId === conceptId);
      const inferredType = inferContentTypeFromId(conceptId);

      return {
        conceptId,
        title: conceptData?.title ?? this.formatConceptTitle(conceptId),
        isCompleted: this.isConceptCompleted(conceptId),
        isGlobalCompletion: this.isConceptGloballyComplete(conceptId),
        icon: getIconForContent(conceptId, inferredType),
        contentType: inferredType,
      };
    });
  }

  /**
   * Check if a concept was completed in a different path.
   */
  private isConceptGloballyComplete(conceptId: string): boolean {
    const conceptData = this.conceptProgress.find(c => c.conceptId === conceptId);
    if (!conceptData) return false;
    // If completed but not in this path's progress, it's global
    return conceptData.completionPercentage === 100 && !this.isConceptCompleted(conceptId);
  }

  /**
   * Format a concept ID into a human-readable title.
   * Converts kebab-case to Title Case as a fallback.
   */
  private formatConceptTitle(conceptId: string): string {
    return conceptId
      .split('-')
      .map(word => word.charAt(0).toUpperCase() + word.slice(1))
      .join(' ');
  }

  /**
   * Check if a concept has been completed by the user.
   * Uses conceptProgress data loaded from the path service.
   */
  private isConceptCompleted(conceptId: string): boolean {
    const conceptData = this.conceptProgress.find(c => c.conceptId === conceptId);
    return conceptData ? conceptData.completionPercentage === 100 : false;
  }

  /**
   * Navigate to a specific concept within the path context.
   * Finds the step that references this concept and navigates to the step view.
   */
  goToConcept(conceptId: string): void {
    // Find the step that references this concept
    const step = this.path?.steps.find(s => s.resourceId === conceptId);
    if (step) {
      this.router.navigate([this.PATH_ROUTE, this.pathId, 'step', step.order]);
    } else {
      // Fallback to direct resource view if no matching step found
      this.router.navigate(['/lamad/resource', conceptId]);
    }
  }

  /**
   * Navigate to a section by going to its first concept.
   */
  goToSection(conceptId: string | undefined): void {
    if (conceptId) {
      this.goToConcept(conceptId);
    }
  }
}
