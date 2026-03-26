/**
 * ResourceExplorerService - Shefa's native resource exploration service.
 *
 * Provides the "Folders" lens (default) that organizes resources by category
 * and content type. Also manages explorer state and coordinates with the
 * LensRegistry for cross-pillar lens switching.
 *
 * Phase 1: Only the "content" resource category is populated.
 * Future categories (compute, storage, property, etc.) show placeholders.
 */

import { Injectable, inject } from '@angular/core';

import { Observable, of, map, shareReplay } from 'rxjs';

import { LensRegistryService } from '@app/elohim/services/lens-registry.service';
import { ContentService } from '@app/lamad/services/content.service';

import type {
  ExplorerBreadcrumb,
  ExplorerNode,
  LensProvider,
  ResourceCategory,
} from '../models/resource-explorer.model';
import type { ContentType } from '@app/lamad/models/content-node.model';

/** Helper: convert folder title to a URL-safe key */
function folderKey(title: string): string {
  return title.toLowerCase().replace(/\s+/g, '-');
}

/**
 * Content type groupings for the folders lens.
 * Maps a display folder to the content types it contains.
 */
export const CONTENT_TYPE_FOLDERS: { title: string; icon: string; types: ContentType[] }[] = [
  {
    title: 'Articles',
    icon: 'article',
    types: ['concept', 'epic', 'article', 'reference', 'example'],
  },
  { title: 'Assessments', icon: 'quiz', types: ['assessment'] },
  { title: 'Collectives', icon: 'groups', types: ['collective'] },
  { title: 'Scenarios', icon: 'checklist', types: ['scenario'] },
  { title: 'People', icon: 'people', types: ['human', 'role'] },
  {
    title: 'Activities',
    icon: 'directions_run',
    types: ['exercise', 'reflection', 'discussion'],
  },
  { title: 'Courses', icon: 'school', types: ['lesson', 'path'] },
];

/**
 * Resource category folders for the top level.
 * Phase 1: only "Content" is populated.
 */
const RESOURCE_CATEGORY_FOLDERS: {
  category: ResourceCategory;
  title: string;
  icon: string;
}[] = [
  { category: 'content', title: 'Content', icon: 'library_books' },
  { category: 'compute', title: 'Compute', icon: 'memory' },
  { category: 'storage', title: 'Storage', icon: 'cloud' },
  { category: 'property', title: 'Property', icon: 'real_estate_agent' },
  { category: 'energy', title: 'Energy', icon: 'bolt' },
  { category: 'knowledge', title: 'Knowledge', icon: 'school' },
  { category: 'financial', title: 'Financial', icon: 'account_balance' },
];

@Injectable({ providedIn: 'root' })
export class ResourceExplorerService implements LensProvider {
  private readonly lensRegistry = inject(LensRegistryService);
  private readonly contentService = inject(ContentService);

  readonly type = 'folders';
  readonly label = 'Folders';
  readonly icon = 'folder';
  readonly description = 'Browse resources by category and content type';
  readonly appliesTo: ResourceCategory[] = [
    'content',
    'compute',
    'storage',
    'property',
    'energy',
    'knowledge',
    'financial',
  ];

  private treeCache$: Observable<ExplorerNode[]> | null = null;

  constructor() {
    // Self-register with the lens registry
    this.lensRegistry.register(this);
  }

  getTree(): Observable<ExplorerNode[]> {
    this.treeCache$ ??= this.buildRootTree().pipe(shareReplay(1));
    return this.treeCache$;
  }

  getChildren(folderId: string): Observable<ExplorerNode[]> {
    // Root category folder -> content type sub-folders
    if (folderId.startsWith('category:')) {
      const category = folderId.replace('category:', '') as ResourceCategory;
      if (category === 'content') {
        return this.buildContentTypeFolders();
      }
      // Future categories return empty
      return of([]);
    }

    // Content type folder -> individual content nodes
    if (folderId.startsWith('type:')) {
      const typeKey = folderId.replace('type:', '');
      const folder = CONTENT_TYPE_FOLDERS.find(f => folderKey(f.title) === typeKey);
      if (folder) {
        return this.buildContentFilesForTypes(folderId, folder.types);
      }
      return of([]);
    }

    return of([]);
  }

  getBreadcrumbs(nodeId: string): Observable<ExplorerBreadcrumb[]> {
    const crumbs: ExplorerBreadcrumb[] = [{ id: null, title: 'My Resources' }];

    if (nodeId.startsWith('category:')) {
      const category = nodeId.replace('category:', '');
      const catFolder = RESOURCE_CATEGORY_FOLDERS.find(f => f.category === category);
      crumbs.push({ id: nodeId, title: catFolder?.title ?? category });
      return of(crumbs);
    }

    if (nodeId.startsWith('type:')) {
      crumbs.push({ id: 'category:content', title: 'Content' });
      const typeKey = nodeId.replace('type:', '');
      const typeFolder = CONTENT_TYPE_FOLDERS.find(f => folderKey(f.title) === typeKey);
      crumbs.push({ id: nodeId, title: typeFolder?.title ?? typeKey });
      return of(crumbs);
    }

    return of(crumbs);
  }

  /** Invalidate cached tree (call when content changes) */
  invalidateCache(): void {
    this.treeCache$ = null;
  }

  private buildRootTree(): Observable<ExplorerNode[]> {
    return this.contentService.getAllContentTypes().pipe(
      map(raw => {
        const availableTypes = Array.isArray(raw) ? raw : [];
        return RESOURCE_CATEGORY_FOLDERS.map((cat, index) => {
          const hasContent = cat.category === 'content' && availableTypes.length > 0;
          return {
            id: `category:${cat.category}`,
            title: cat.title,
            nodeType: 'folder' as const,
            folderLevel: 'category' as const,
            resourceCategory: cat.category,
            parentId: null,
            childIds: hasContent ? CONTENT_TYPE_FOLDERS.map(f => `type:${folderKey(f.title)}`) : [],
            itemCount: hasContent ? availableTypes.length : 0,
            icon: cat.icon,
            order: index,
          };
        });
      })
    );
  }

  private buildContentTypeFolders(): Observable<ExplorerNode[]> {
    return this.contentService.getAllContentTypes().pipe(
      map(availableTypes => {
        const safe = Array.isArray(availableTypes) ? availableTypes : [];
        const typeSet = new Set(safe);
        return CONTENT_TYPE_FOLDERS.filter(folder => folder.types.some(t => typeSet.has(t))).map(
          (folder, index) => ({
            id: `type:${folderKey(folder.title)}`,
            title: folder.title,
            nodeType: 'folder' as const,
            folderLevel: 'type' as const,
            resourceCategory: 'content' as const,
            parentId: 'category:content',
            childIds: [],
            itemCount: 0,
            icon: folder.icon,
            order: index,
          })
        );
      })
    );
  }

  private buildContentFilesForTypes(
    parentId: string,
    types: ContentType[]
  ): Observable<ExplorerNode[]> {
    return this.contentService.searchContent('').pipe(
      map(entries => {
        const safe = Array.isArray(entries) ? entries : [];
        const typeSet = new Set<string>(types);
        return safe
          .filter(entry => typeSet.has(entry.contentType))
          .map((entry, index) => ({
            id: `file:${entry.id}`,
            title: entry.title || entry.id,
            description: String(entry.description ?? ''),
            nodeType: 'file' as const,
            resourceCategory: 'content' as const,
            resourceId: entry.id,
            contentType: entry.contentType,
            parentId,
            childIds: [],
            itemCount: 0,
            icon: entry.contentType === 'collective' ? 'groups' : 'description',
            order: index,
          }));
      })
    );
  }
}
