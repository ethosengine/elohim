import { Injectable } from '@angular/core';

// @coverage: 100.0% (2026-02-05)

import { BehaviorSubject, Observable } from 'rxjs';

import { PathContext, DetourInfo } from '../models/exploration-context.model';

/**
 * PathContextService - Maintains path context during exploration detours.
 *
 * When a learner is navigating a learning path and decides to explore
 * related content, this service tracks where they came from so they
 * can easily return to their position in the path.
 *
 * Supports nested detours (exploring from an exploration) via a stack.
 *
 * Usage:
 * ```typescript
 * // When entering a path
 * pathContextService.enterPath({
 *   pathId: 'governance-intro',
 *   pathTitle: 'Introduction to Governance',
 *   stepIndex: 3,
 *   totalSteps: 10,
 *   returnRoute: ['/lamad/path', 'governance-intro', 'step', '3']
 * });
 *
 * // When user clicks "explore related"
 * pathContextService.startDetour({
 *   fromContentId: 'separation-of-powers',
 *   toContentId: 'appeals-process',
 *   detourType: 'related',
 *   timestamp: new Date().toISOString()
 * });
 *
 * // When user clicks "return to path"
 * const returnRoute = pathContextService.returnToPath();
 * if (returnRoute) {
 *   router.navigate(returnRoute);
 * }
 * ```
 */
@Injectable({ providedIn: 'root' })
export class PathContextService {
  /** Stack of path contexts (supports nested path navigation) */
  private contextStack: PathContext[] = [];

  /** Observable for current active context */
  private readonly activeContext$ = new BehaviorSubject<PathContext | null>(null);

  /**
   * Observable stream of the current path context.
   * Components can subscribe to react to context changes.
   */
  get context$(): Observable<PathContext | null> {
    return this.activeContext$.asObservable();
  }

  /**
   * Get the current context synchronously (for template bindings).
   */
  get currentContext(): PathContext | null {
    return this.activeContext$.value;
  }

  /**
   * Check if we're currently within a path.
   */
  get hasPathContext(): boolean {
    return this.contextStack.length > 0;
  }

  /**
   * Check if we're currently in a detour from a path.
   */
  get isInDetour(): boolean {
    const current = this.contextStack[this.contextStack.length - 1];
    return (current?.detourStack?.length ?? 0) > 0;
  }

  /**
   * Get current detour depth.
   */
  get detourDepth(): number {
    const current = this.contextStack[this.contextStack.length - 1];
    return current?.detourStack?.length ?? 0;
  }

  /**
   * Enter a learning path. Called when navigating to a path step.
   *
   * @param context - The path context to enter
   */
  enterPath(context: PathContext): void {
    // Initialize detour stack if not present
    context.detourStack ??= [];

    // Check if we're already in this path (update position)
    const existingIndex = this.contextStack.findIndex(c => c.pathId === context.pathId);
    if (existingIndex >= 0) {
      // Update existing context with new position
      this.contextStack[existingIndex] = {
        ...this.contextStack[existingIndex],
        stepIndex: context.stepIndex,
        chapterTitle: context.chapterTitle,
        returnRoute: context.returnRoute,
      };
    } else {
      // Push new path context
      this.contextStack.push(context);
    }

    this.activeContext$.next(this.contextStack[this.contextStack.length - 1]);
  }

  /**
   * Update the current path position without changing path.
   * Called when navigating between steps within the same path.
   *
   * @param stepIndex - New step index
   * @param chapterTitle - Optional new chapter title
   */
  updatePosition(stepIndex: number, chapterTitle?: string): void {
    const current = this.contextStack[this.contextStack.length - 1];
    if (!current) return;

    current.stepIndex = stepIndex;
    if (chapterTitle !== undefined) {
      current.chapterTitle = chapterTitle;
    }
    current.returnRoute = ['/lamad/path', current.pathId, 'step', String(stepIndex)];

    this.activeContext$.next(current);
  }

  /**
   * Start a detour from the current path position.
   * Called when user clicks on related content to explore.
   *
   * @param detourInfo - Information about the detour
   */
  startDetour(detourInfo: DetourInfo): void {
    const current = this.contextStack[this.contextStack.length - 1];
    if (!current) {
      // Not in a path context - detour without path context is just navigation
      return;
    }

    // Initialize detour stack if needed
    current.detourStack ??= [];

    current.detourStack.push(detourInfo);
    this.activeContext$.next(current);
  }

  /**
   * Return from the current detour.
   * Pops the latest detour and returns the route to navigate to.
   *
   * @returns Route segments to navigate to, or null if not in a detour
   */
  returnFromDetour(): string[] | null {
    const current = this.contextStack[this.contextStack.length - 1];
    if (!current?.detourStack?.length) {
      return null;
    }

    current.detourStack.pop();
    this.activeContext$.next(current);

    // If there are more detours, return to the previous detour's content
    if (current.detourStack.length > 0) {
      const previousDetour = current.detourStack[current.detourStack.length - 1];
      return ['/lamad/resource', previousDetour.toContentId];
    }

    // No more detours - return to the path
    return current.returnRoute;
  }

  /**
   * Return directly to the path, clearing all detours.
   *
   * @returns Route segments to navigate to the path, or null if not in a path
   */
  returnToPath(): string[] | null {
    const current = this.contextStack[this.contextStack.length - 1];
    if (!current) {
      return null;
    }

    // Clear all detours
    current.detourStack = [];
    this.activeContext$.next(current);

    return current.returnRoute;
  }

  /**
   * Exit the current path entirely.
   * Called when navigating away from a path.
   */
  exitPath(): void {
    this.contextStack.pop();
    const newContext = this.contextStack[this.contextStack.length - 1] ?? null;
    this.activeContext$.next(newContext);
  }

  /**
   * Exit all paths and clear context.
   */
  clearAll(): void {
    this.contextStack = [];
    this.activeContext$.next(null);
  }

  /**
   * Get breadcrumb trail for current context.
   * Useful for displaying navigation history.
   */
  getBreadcrumbs(): BreadcrumbItem[] {
    const current = this.contextStack[this.contextStack.length - 1];
    if (!current) return [];

    const breadcrumbs: BreadcrumbItem[] = [
      {
        label: 'Paths',
        route: ['/lamad'],
      },
      {
        label: current.pathTitle,
        route: ['/lamad/path', current.pathId],
      },
    ];

    if (current.chapterTitle) {
      breadcrumbs.push({
        label: current.chapterTitle,
        route: current.returnRoute,
      });
    }

    breadcrumbs.push({
      label: `Step ${current.stepIndex + 1}`,
      route: current.returnRoute,
      isCurrent: !current.detourStack?.length,
    });

    // Add detour breadcrumbs
    if (current.detourStack) {
      for (let i = 0; i < current.detourStack.length; i++) {
        const detour = current.detourStack[i];
        breadcrumbs.push({
          label: `Exploring: ${detour.toContentId}`,
          route: ['/lamad/resource', detour.toContentId],
          isDetour: true,
          isCurrent: i === current.detourStack.length - 1,
        });
      }
    }

    return breadcrumbs;
  }

  /**
   * Get a summary of the current context for display.
   */
  getContextSummary(): PathContextSummary | null {
    const current = this.contextStack[this.contextStack.length - 1];
    if (!current) return null;

    return {
      pathTitle: current.pathTitle,
      stepIndex: current.stepIndex,
      totalSteps: current.totalSteps,
      chapterTitle: current.chapterTitle,
      detourCount: current.detourStack?.length ?? 0,
      returnRoute: current.returnRoute,
    };
  }
}

/**
 * Breadcrumb item for navigation display.
 */
export interface BreadcrumbItem {
  label: string;
  route: string[];
  isDetour?: boolean;
  isCurrent?: boolean;
}

/**
 * Summary of current path context.
 */
export interface PathContextSummary {
  pathTitle: string;
  stepIndex: number;
  totalSteps: number;
  chapterTitle?: string;
  detourCount: number;
  returnRoute: string[];
}
