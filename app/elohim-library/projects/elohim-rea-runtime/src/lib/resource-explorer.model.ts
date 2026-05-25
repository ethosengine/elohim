/**
 * Resource Explorer Models
 *
 * The resource explorer provides a "Google Drive"-like browsing experience
 * over stewarded resources. Resources include ContentNodes (learning content),
 * compute capacity, storage, property, energy, and more.
 *
 * Key concepts:
 * - **ResourceCategory**: What kind of resource (content, compute, storage...)
 * - **ExplorerNode**: Virtual tree node projected by a lens (folder or file)
 * - **LensProvider**: A pillar-specific projection that organizes resources into a tree
 * - **ExplorerBreadcrumb**: Navigation trail segment
 *
 * The explorer is a shefa concern (stewardship). Lamad provides a "Scope & Sequence"
 * lens over content resources. Other pillars can register additional lenses.
 */

import { Observable } from 'rxjs';

/**
 * ResourceCategory - Extensible classification of stewarded resources.
 *
 * Content is just one of many resource types. The explorer model supports
 * any resource that can be browsed, organized, and stewarded.
 */
export type ResourceCategory =
  | 'content'
  | 'compute'
  | 'storage'
  | 'property'
  | 'energy'
  | 'knowledge'
  | 'financial'
  | 'reputation';

/**
 * FolderLevel - Semantic level within a lens hierarchy.
 *
 * Different lenses use different folder levels. The scope-sequence lens uses
 * path/chapter/module/section. The folders lens uses category/type/tag.
 */
export type FolderLevel =
  | 'root'
  | 'category'
  | 'path'
  | 'chapter'
  | 'module'
  | 'section'
  | 'type'
  | 'tag';

/**
 * ExplorerNode - A virtual node in the explorer tree.
 *
 * Lenses project the resource graph into trees of ExplorerNodes.
 * A node is either a "folder" (expandable container) or a "file" (leaf resource).
 * The same underlying resource can appear in multiple lenses and folders.
 */
export interface ExplorerNode {
  /** Unique ID within this lens projection (e.g., 'path:elohim-protocol', 'type:video') */
  id: string;

  /** Display title */
  title: string;

  /** Optional description */
  description?: string;

  /** Folder (expandable) or file (leaf resource) */
  nodeType: 'folder' | 'file';

  /** Semantic level in the hierarchy */
  folderLevel?: FolderLevel;

  /** Which resource category this node belongs to */
  resourceCategory: ResourceCategory;

  /** For files: the underlying resource ID (ContentNode ID, device ID, etc.) */
  resourceId?: string;

  /** For content files: the content type for icon/behavior selection */
  contentType?: string;

  /** For content files: the content format */
  contentFormat?: string;

  /** Parent node ID (null for root-level nodes) */
  parentId: string | null;

  /** Child node IDs (empty for files) */
  childIds: string[];

  /** Count of leaf items (recursive for folders) */
  itemCount: number;

  /** Display icon (Material Icons name or emoji) */
  icon: string;

  /** Sort order within parent */
  order: number;

  /** Extensible metadata (difficulty, duration, tags, etc.) */
  metadata?: Record<string, unknown>;
}

/**
 * ExplorerBreadcrumb - One segment of the navigation trail.
 */
export interface ExplorerBreadcrumb {
  /** Node ID (null for the root) */
  id: string | null;

  /** Display title */
  title: string;
}

/**
 * LensProvider - Interface for pillar-specific resource projections.
 *
 * Each pillar can register lenses that project the resource graph into
 * different tree structures. The explorer queries the LensRegistry for
 * available lenses and delegates tree building to the active provider.
 */
export interface LensProvider {
  /** Unique lens type identifier (e.g., 'scope-sequence', 'folders') */
  readonly type: string;

  /** Human-readable label */
  readonly label: string;

  /** Material Icons name */
  readonly icon: string;

  /** Brief description for tooltips */
  readonly description: string;

  /** Which resource categories this lens can organize */
  readonly appliesTo: ResourceCategory[];

  /** Get root-level nodes for the tree */
  getTree(): Observable<ExplorerNode[]>;

  /** Get children of a folder node (lazy loading) */
  getChildren(folderId: string): Observable<ExplorerNode[]>;

  /** Build breadcrumb trail from root to the given node */
  getBreadcrumbs(nodeId: string): Observable<ExplorerBreadcrumb[]>;
}

/**
 * LensDefinition - Lightweight descriptor for the lens switcher UI.
 * Extracted from LensProvider so the UI doesn't need the full provider.
 */
export interface LensDefinition {
  type: string;
  label: string;
  icon: string;
  description: string;
  appliesTo: ResourceCategory[];
}
