/**
 * ScopeSequenceLensProvider - Lamad's path hierarchy lens for the resource explorer.
 *
 * Projects ContentNode resources through the learning path hierarchy:
 * Path -> Chapter -> Module -> Section -> Concepts (leaf files)
 *
 * This is a lamad-specific view over shefa-stewarded content resources.
 * Registered with the elohim LensRegistry at construction.
 */

import { Injectable, inject } from '@angular/core';

import { Observable, of, map, shareReplay } from 'rxjs';

import { LensRegistryService } from '@app/elohim/services/lens-registry.service';
import { getContentIcon } from '@app/lamad/utils/content-icons';

import { PathService } from './path.service';

import type { ExplorerBreadcrumb, ExplorerNode, LensProvider } from '@app/shefa/models';

/** Default file icon */
const DEFAULT_FILE_ICON = getContentIcon();

@Injectable({ providedIn: 'root' })
export class ScopeSequenceLensProvider implements LensProvider {
  private readonly lensRegistry = inject(LensRegistryService);
  private readonly pathService = inject(PathService);

  readonly type = 'scope-sequence';
  readonly label = 'Scope & Sequence';
  readonly icon = 'school';
  readonly description = 'Browse by learning path hierarchy: Paths > Chapters > Concepts';
  readonly appliesTo = ['content' as const];

  private treeCache$: Observable<ExplorerNode[]> | null = null;

  constructor() {
    this.lensRegistry.register(this);
  }

  getTree(): Observable<ExplorerNode[]> {
    this.treeCache$ ??= this.buildPathTree().pipe(shareReplay(1));
    return this.treeCache$;
  }

  getChildren(folderId: string): Observable<ExplorerNode[]> {
    // Path folder -> chapters (or flat steps)
    if (folderId.startsWith('path:')) {
      const pathId = folderId.replace('path:', '');
      return this.buildPathChildren(pathId);
    }

    // Chapter folder -> modules or direct steps
    if (folderId.startsWith('chapter:')) {
      const [pathId, chapterId] = folderId.replace('chapter:', '').split('/');
      return this.buildChapterChildren(pathId, chapterId);
    }

    // Module folder -> sections
    if (folderId.startsWith('module:')) {
      const [pathId, , moduleId] = folderId.replace('module:', '').split('/');
      return this.buildModuleChildren(pathId, moduleId);
    }

    // Section folder -> concept files
    if (folderId.startsWith('section:')) {
      const parts = folderId.replace('section:', '').split('/');
      const pathId = parts[0];
      const sectionId = parts.at(-1)!;
      return this.buildSectionChildren(pathId, sectionId);
    }

    return of([]);
  }

  getBreadcrumbs(nodeId: string): Observable<ExplorerBreadcrumb[]> {
    const crumbs: ExplorerBreadcrumb[] = [{ id: null, title: 'Scope & Sequence' }];

    if (nodeId.startsWith('path:')) {
      const pathId = nodeId.replace('path:', '');
      return this.pathService.listPaths().pipe(
        map(index => {
          const path = index.paths.find(p => p.id === pathId);
          crumbs.push({ id: nodeId, title: path?.title ?? pathId });
          return crumbs;
        })
      );
    }

    // For deeper levels, build from the path hierarchy
    if (
      nodeId.startsWith('chapter:') ||
      nodeId.startsWith('module:') ||
      nodeId.startsWith('section:')
    ) {
      const prefix = nodeId.split(':')[0];
      const idPart = nodeId.replace(`${prefix}:`, '');
      const pathId = idPart.split('/')[0];

      return this.pathService.getPath(pathId).pipe(
        map(path => {
          crumbs.push({ id: `path:${pathId}`, title: path.title });

          if (prefix === 'chapter' || prefix === 'module' || prefix === 'section') {
            const chapterId = idPart.split('/')[1];
            const chapter = path.chapters?.find(c => c.id === chapterId);
            if (chapter) {
              crumbs.push({ id: `chapter:${pathId}/${chapterId}`, title: chapter.title });
            }
          }

          if (prefix === 'module' || prefix === 'section') {
            const moduleId = idPart.split('/')[2];
            const chapter = path.chapters?.find(c => c.modules?.some(m => m.id === moduleId));
            const mod = chapter?.modules?.find(m => m.id === moduleId);
            if (mod) {
              crumbs.push({
                id: `module:${pathId}/${chapter!.id}/${moduleId}`,
                title: mod.title,
              });
            }
          }

          if (prefix === 'section') {
            const sectionId = idPart.split('/')[3] ?? idPart.split('/')[2];
            const section = path.chapters
              ?.flatMap(c => c.modules ?? [])
              .flatMap(m => m.sections)
              .find(s => s.id === sectionId);
            if (section) {
              crumbs.push({ id: nodeId, title: section.title });
            }
          }

          return crumbs;
        })
      );
    }

    return of(crumbs);
  }

  /** Invalidate cached tree */
  invalidateCache(): void {
    this.treeCache$ = null;
  }

  private buildPathTree(): Observable<ExplorerNode[]> {
    return this.pathService.listPaths().pipe(
      map(index =>
        index.paths.map((entry, i) => ({
          id: `path:${entry.id}`,
          title: entry.title,
          description: entry.description,
          nodeType: 'folder' as const,
          folderLevel: 'path' as const,
          resourceCategory: 'content' as const,
          parentId: null,
          childIds: [],
          itemCount: entry.stepCount,
          icon: 'route',
          order: i,
          metadata: {
            difficulty: entry.difficulty,
            estimatedDuration: entry.estimatedDuration,
          },
        }))
      )
    );
  }

  private buildPathChildren(pathId: string): Observable<ExplorerNode[]> {
    return this.pathService.getPath(pathId).pipe(
      map(path => {
        // Hierarchical path: chapters
        if (path.chapters && path.chapters.length > 0) {
          return path.chapters.map((chapter, i) => ({
            id: `chapter:${pathId}/${chapter.id}`,
            title: chapter.title,
            description: chapter.description,
            nodeType: 'folder' as const,
            folderLevel: 'chapter' as const,
            resourceCategory: 'content' as const,
            parentId: `path:${pathId}`,
            childIds: [],
            itemCount: this.countChapterItems(chapter),
            icon: 'menu_book',
            order: chapter.order ?? i,
          }));
        }

        // Flat path: steps as files
        return path.steps.map((step, i) => ({
          id: `file:${step.resourceId}`,
          title: step.stepTitle || step.resourceId,
          nodeType: 'file' as const,
          resourceCategory: 'content' as const,
          resourceId: step.resourceId,
          contentType: step.stepType ?? 'content',
          parentId: `path:${pathId}`,
          childIds: [],
          itemCount: 0,
          icon: DEFAULT_FILE_ICON,
          order: step.order ?? i,
        }));
      })
    );
  }

  private buildChapterChildren(pathId: string, chapterId: string): Observable<ExplorerNode[]> {
    return this.pathService.getPath(pathId).pipe(
      map(path => {
        const chapter = path.chapters?.find(c => c.id === chapterId);
        if (!chapter) return [];

        // 4-level: chapter has modules
        if (chapter.modules && chapter.modules.length > 0) {
          return chapter.modules.map((mod, i) => ({
            id: `module:${pathId}/${chapterId}/${mod.id}`,
            title: mod.title,
            description: mod.description,
            nodeType: 'folder' as const,
            folderLevel: 'module' as const,
            resourceCategory: 'content' as const,
            parentId: `chapter:${pathId}/${chapterId}`,
            childIds: [],
            itemCount: mod.sections.reduce((sum, s) => sum + s.conceptIds.length, 0),
            icon: 'view_module',
            order: mod.order ?? i,
          }));
        }

        // 2-level: chapter has direct steps
        return (chapter.steps ?? []).map((step, i) => ({
          id: `file:${step.resourceId}`,
          title: step.stepTitle || step.resourceId,
          nodeType: 'file' as const,
          resourceCategory: 'content' as const,
          resourceId: step.resourceId,
          parentId: `chapter:${pathId}/${chapterId}`,
          childIds: [],
          itemCount: 0,
          icon: DEFAULT_FILE_ICON,
          order: step.order ?? i,
        }));
      })
    );
  }

  private buildModuleChildren(pathId: string, moduleId: string): Observable<ExplorerNode[]> {
    return this.pathService.getPath(pathId).pipe(
      map(path => {
        const mod = path.chapters?.flatMap(c => c.modules ?? []).find(m => m.id === moduleId);
        if (!mod) return [];

        return mod.sections.map((section, i) => ({
          id: `section:${pathId}/${moduleId}/${section.id}`,
          title: section.title,
          description: section.description,
          nodeType: 'folder' as const,
          folderLevel: 'section' as const,
          resourceCategory: 'content' as const,
          parentId: `module:${pathId}/*/${moduleId}`,
          childIds: [],
          itemCount: section.conceptIds.length,
          icon: 'bookmark',
          order: section.order ?? i,
        }));
      })
    );
  }

  private buildSectionChildren(pathId: string, sectionId: string): Observable<ExplorerNode[]> {
    return this.pathService.getPath(pathId).pipe(
      map(path => {
        const section = path.chapters
          ?.flatMap(c => c.modules ?? [])
          .flatMap(m => m.sections)
          .find(s => s.id === sectionId);
        if (!section) return [];

        return section.conceptIds.map((conceptId, i) => ({
          id: `file:${conceptId}`,
          title: conceptId,
          nodeType: 'file' as const,
          resourceCategory: 'content' as const,
          resourceId: conceptId,
          parentId: `section:${pathId}/*/${sectionId}`,
          childIds: [],
          itemCount: 0,
          icon: DEFAULT_FILE_ICON,
          order: i,
        }));
      })
    );
  }

  private countChapterItems(chapter: {
    modules?: { sections: { conceptIds: string[] }[] }[];
    steps?: unknown[];
  }): number {
    if (chapter.modules) {
      return chapter.modules.reduce(
        (sum, mod) => sum + mod.sections.reduce((s, sec) => s + sec.conceptIds.length, 0),
        0
      );
    }
    return chapter.steps?.length ?? 0;
  }
}
