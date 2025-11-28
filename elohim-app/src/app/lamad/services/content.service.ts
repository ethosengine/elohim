import { Injectable } from '@angular/core';
import { Observable, of, forkJoin } from 'rxjs';
import { map, switchMap, catchError } from 'rxjs/operators';
import { DataLoaderService } from './data-loader.service';
import { ContentNode, ContentType, ContentReach, ContentPreview } from '../models/content-node.model';
import { LearningPath } from '../models/learning-path.model';
import { AgentService } from './agent.service';

/**
 * Content index entry (metadata only).
 */
export interface ContentIndexEntry {
  id: string;
  title: string;
  description: string;
  contentType: ContentType;
  tags: string[];
}

/**
 * Path reference - where content appears in a path.
 * Used for "appears in paths" back-links (Wikipedia-style).
 */
export interface PathReference {
  path: LearningPath;
  stepIndex: number;
  stepNarrative?: string;
}

/**
 * Content access result - combines content with access status.
 */
export interface ContentAccessResult {
  /** Whether the agent can access this content */
  canAccess: boolean;

  /** The content (if accessible) */
  content?: ContentNode;

  /** If not accessible, why */
  reason?: 'unauthenticated' | 'insufficient-reach' | 'not-found';

  /** What reach level is required */
  requiredReach?: ContentReach;

  /** Agent's current reach level for this content */
  agentReach?: ContentReach;
}

/**
 * Reach hierarchy for access control.
 * Higher index = broader reach = more accessible.
 */
const REACH_HIERARCHY: ContentReach[] = [
  'private',
  'invited',
  'local',
  'community',
  'federated',
  'commons'
];

/**
 * ContentService - Accesses Territory resources directly (outside path context).
 *
 * Use cases:
 * - Direct resource viewing (via URL) with reach-based access control
 * - "Appears in paths" back-links (Wikipedia-style)
 * - Search functionality
 * - Related resource lookup
 *
 * Access Control (Bidirectional Trust):
 * - Content has a `reach` level (private → commons)
 * - Agents have attestations that grant them reach levels
 * - Access is granted when agent's reach >= content's reach
 * - Commons content is accessible to everyone
 *
 * Note: For path-based navigation, use PathService.getPathStep() instead.
 */
@Injectable({ providedIn: 'root' })
export class ContentService {
  constructor(
    private dataLoader: DataLoaderService,
    private agentService: AgentService
  ) {}

  /**
   * Get content by ID.
   * This is the direct access method - prefer PathService for journey navigation.
   */
  getContent(resourceId: string): Observable<ContentNode> {
    return this.dataLoader.getContent(resourceId);
  }

  /**
   * Get related resources (shallow metadata).
   * Does NOT load full content for related nodes - just their IDs.
   * The caller should load specific related resources as needed.
   */
  getRelatedResourceIds(resourceId: string): Observable<string[]> {
    return this.dataLoader.getContent(resourceId).pipe(
      map(node => node.relatedNodeIds || [])
    );
  }

  /**
   * Get a related resource by loading it.
   * Use sparingly - prefer lazy loading patterns.
   */
  getRelatedResource(resourceId: string, relatedIndex: number): Observable<ContentNode | null> {
    return this.getRelatedResourceIds(resourceId).pipe(
      switchMap(relatedIds => {
        if (relatedIndex < 0 || relatedIndex >= relatedIds.length) {
          return of(null);
        }
        return this.dataLoader.getContent(relatedIds[relatedIndex]).pipe(
          catchError(() => of(null))
        );
      })
    );
  }

  /**
   * Search content by query string.
   * Searches title and tags (from index, not full content).
   */
  searchContent(query: string): Observable<ContentIndexEntry[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        if (!query || query.trim().length === 0) {
          return index.nodes || [];
        }

        const lowerQuery = query.toLowerCase().trim();
        return (index.nodes || []).filter((node: ContentIndexEntry) =>
          node.title.toLowerCase().includes(lowerQuery) ||
          node.description?.toLowerCase().includes(lowerQuery) ||
          node.tags?.some((tag: string) => tag.toLowerCase().includes(lowerQuery))
        );
      })
    );
  }

  /**
   * Filter content by type.
   */
  getContentByType(contentType: ContentType): Observable<ContentIndexEntry[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        return (index.nodes || []).filter(
          (node: ContentIndexEntry) => node.contentType === contentType
        );
      })
    );
  }

  /**
   * Filter content by tag.
   */
  getContentByTag(tag: string): Observable<ContentIndexEntry[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        const lowerTag = tag.toLowerCase();
        return (index.nodes || []).filter((node: ContentIndexEntry) =>
          node.tags?.some((t: string) => t.toLowerCase() === lowerTag)
        );
      })
    );
  }

  /**
   * Get all unique tags from the content index.
   */
  getAllTags(): Observable<string[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        const tagSet = new Set<string>();
        (index.nodes || []).forEach((node: ContentIndexEntry) => {
          node.tags?.forEach((tag: string) => tagSet.add(tag));
        });
        return Array.from(tagSet).sort();
      })
    );
  }

  /**
   * Get all unique content types from the content index.
   */
  getAllContentTypes(): Observable<ContentType[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => {
        const typeSet = new Set<ContentType>();
        (index.nodes || []).forEach((node: ContentIndexEntry) => {
          if (node.contentType) {
            typeSet.add(node.contentType);
          }
        });
        return Array.from(typeSet);
      })
    );
  }

  // =========================================================================
  // Access Control (Bidirectional Trust Model)
  // =========================================================================

  /**
   * Get content with access control check.
   * Returns both the content and access status.
   *
   * Access rules:
   * - 'commons' content: accessible to everyone (even unauthenticated)
   * - 'federated' content: accessible to any authenticated agent
   * - 'community' content: requires community membership or attestation
   * - 'local' content: requires local attestation
   * - 'invited' content: requires explicit invitation
   * - 'private' content: only accessible to author
   */
  getContentWithAccessCheck(resourceId: string): Observable<ContentAccessResult> {
    return this.dataLoader.getContent(resourceId).pipe(
      map(content => {
        const contentReach = content.reach || 'commons'; // Default to commons for legacy content
        const agentReach = this.getAgentReachLevel();

        // Check if agent can access this content
        const canAccess = this.canAccessContent(contentReach, agentReach, content);

        if (canAccess) {
          return {
            canAccess: true,
            content,
            agentReach
          } as ContentAccessResult;
        } else {
          const reason: 'unauthenticated' | 'insufficient-reach' =
            agentReach === 'private' ? 'unauthenticated' : 'insufficient-reach';
          return {
            canAccess: false,
            reason,
            requiredReach: contentReach,
            agentReach
          } as ContentAccessResult;
        }
      }),
      catchError(() => of({
        canAccess: false,
        reason: 'not-found' as const
      }))
    );
  }

  /**
   * Check if an agent can access content based on reach levels.
   */
  private canAccessContent(
    contentReach: ContentReach,
    agentReach: ContentReach,
    content: ContentNode
  ): boolean {
    // Commons is always accessible
    if (contentReach === 'commons') {
      return true;
    }

    // Check if agent is the author
    const currentAgentId = this.agentService.getCurrentAgentId();
    if (content.authorId && content.authorId === currentAgentId) {
      return true;
    }

    // Check if agent is in invited list
    if (contentReach === 'invited' && content.invitedAgentIds?.includes(currentAgentId)) {
      return true;
    }

    // Compare reach hierarchy
    const contentReachIndex = REACH_HIERARCHY.indexOf(contentReach);
    const agentReachIndex = REACH_HIERARCHY.indexOf(agentReach);

    return agentReachIndex >= contentReachIndex;
  }

  /**
   * Get the current agent's reach level based on their attestations.
   */
  private getAgentReachLevel(): ContentReach {
    const attestations = this.agentService.getAttestations();

    // Check attestations to determine reach level
    if (attestations.includes('commons-contributor') || attestations.includes('governance-ratifier')) {
      return 'commons';
    }
    if (attestations.includes('federated-member') || attestations.includes('peer-reviewer')) {
      return 'federated';
    }
    if (attestations.includes('community-member') || attestations.includes('path-completion:elohim-protocol')) {
      return 'community';
    }
    if (attestations.includes('local-member')) {
      return 'local';
    }

    // Default: authenticated but no special attestations
    const currentAgent = this.agentService.getCurrentAgentId();
    if (currentAgent) {
      return 'community'; // Authenticated users get community access by default
    }

    return 'private'; // Unauthenticated
  }

  // =========================================================================
  // Path Back-Links ("Appears in Paths" - Wikipedia-style)
  // =========================================================================

  /**
   * Find which paths contain a given resource.
   * Returns paths along with the step index where this resource appears.
   *
   * This enables the "appears in paths" feature - when viewing content directly,
   * users can see all the curated journeys that include this content.
   */
  getContainingPaths(resourceId: string): Observable<PathReference[]> {
    return this.dataLoader.getPathIndex().pipe(
      switchMap(index => {
        if (!index.paths || index.paths.length === 0) {
          return of([]);
        }

        // Load all paths and check which contain this resource
        const pathLoads = index.paths.map(pathEntry =>
          this.dataLoader.getPath(pathEntry.id).pipe(
            map(path => this.findResourceInPath(path, resourceId)),
            catchError(() => of(null))
          )
        );

        return forkJoin(pathLoads).pipe(
          map(results => results.filter((ref): ref is PathReference => ref !== null))
        );
      })
    );
  }

  /**
   * Check if a path contains a resource and return the reference.
   */
  private findResourceInPath(path: LearningPath, resourceId: string): PathReference | null {
    const stepIndex = path.steps.findIndex(step => step.resourceId === resourceId);

    if (stepIndex === -1) {
      return null;
    }

    return {
      path,
      stepIndex,
      stepNarrative: path.steps[stepIndex].stepNarrative
    };
  }

  /**
   * Get a summary of paths containing this resource (lightweight).
   * Returns just path metadata without loading full paths.
   */
  getContainingPathsSummary(resourceId: string): Observable<Array<{
    pathId: string;
    pathTitle: string;
    stepIndex: number;
  }>> {
    return this.getContainingPaths(resourceId).pipe(
      map(refs => refs.map(ref => ({
        pathId: ref.path.id,
        pathTitle: ref.path.title,
        stepIndex: ref.stepIndex
      })))
    );
  }

  // =========================================================================
  // Category-Level Content Discovery (Rich Media Composability)
  // =========================================================================

  /**
   * Get all content previews for a specific category.
   * Returns lightweight preview data for videos, organizations, books, etc.
   *
   * Use case: Displaying rich media cards on a category overview page.
   */
  getContentPreviewsForCategory(category: string): Observable<ContentPreview[]> {
    return this.dataLoader.getContentIndex().pipe(
      map(index => this.filterAndMapToPreview(index.nodes || [], category))
    );
  }

  /**
   * Get content previews filtered by type within a category.
   * E.g., get all videos related to the governance category.
   */
  getContentPreviewsByTypeForCategory(
    category: string,
    contentType: ContentType
  ): Observable<ContentPreview[]> {
    return this.getContentPreviewsForCategory(category).pipe(
      map(previews => previews.filter(p => p.contentType === contentType))
    );
  }

  /**
   * Get all rich media content for a category (videos, audio, organizations, books).
   * Excludes structural content types like features, scenarios, concepts.
   */
  getRichMediaForCategory(category: string): Observable<{
    videos: ContentPreview[];
    organizations: ContentPreview[];
    books: ContentPreview[];
    tools: ContentPreview[];
  }> {
    return this.getContentPreviewsForCategory(category).pipe(
      map(previews => ({
        videos: previews.filter(p => p.contentType === 'video'),
        organizations: previews.filter(p => p.contentType === 'organization'),
        books: previews.filter(p => p.contentType === 'book-chapter'),
        tools: previews.filter(p => p.contentType === 'tool')
      }))
    );
  }

  /**
   * Get related content previews for a specific node.
   * Loads preview data for all related nodes.
   */
  getRelatedContentPreviews(resourceId: string): Observable<ContentPreview[]> {
    return this.dataLoader.getContent(resourceId).pipe(
      switchMap(node => {
        const relatedIds = node.relatedNodeIds || [];
        if (relatedIds.length === 0) {
          return of([]);
        }

        return this.dataLoader.getContentIndex().pipe(
          map(index => {
            const nodes = index.nodes || [];
            return nodes
              .filter((n: any) => relatedIds.includes(n.id))
              .map((n: any) => this.mapIndexEntryToPreview(n));
          })
        );
      }),
      catchError(() => of([]))
    );
  }

  /**
   * Search content with preview data returned.
   * Returns ContentPreview[] instead of ContentIndexEntry[] for richer UI.
   */
  searchContentWithPreviews(query: string): Observable<ContentPreview[]> {
    return this.searchContent(query).pipe(
      map(entries => entries.map(e => this.mapIndexEntryToPreview(e)))
    );
  }

  /**
   * Get all content of a specific type as previews.
   */
  getContentPreviewsByType(contentType: ContentType): Observable<ContentPreview[]> {
    return this.getContentByType(contentType).pipe(
      map(entries => entries.map(e => this.mapIndexEntryToPreview(e)))
    );
  }

  // =========================================================================
  // Preview Mapping Helpers
  // =========================================================================

  /**
   * Filter content index entries by category and map to ContentPreview.
   */
  private filterAndMapToPreview(nodes: any[], categoryId: string): ContentPreview[] {
    // Normalize categoryId (handle both "governance" and "governance-epic" formats)
    const normalizedCategoryId = categoryId.replace('-epic', '').replace('_', '-');

    return nodes
      .filter(node => {
        // Match by category
        const category = node.category?.replace('_', '-');
        if (category === normalizedCategoryId) return true;

        // Match by tag (e.g., "governance", "category:governance")
        const tags = node.tags || [];
        return tags.some((tag: string) => {
          const normalizedTag = tag.replace('epic:', '').replace('category:', '').replace('_', '-');
          return normalizedTag === normalizedCategoryId;
        });
      })
      .map(node => this.mapIndexEntryToPreview(node));
  }

  /**
   * Map a content index entry to ContentPreview.
   */
  private mapIndexEntryToPreview(entry: any): ContentPreview {
    const preview: ContentPreview = {
      id: entry.id,
      title: entry.name || entry.title, // Prefer 'name' for display if available
      description: entry.description || '',
      contentType: entry.contentType,
      tags: entry.tags || [],
      url: entry.url,
      name: entry.name,
      publisher: entry.publisher,
      category: entry.category,
      isPlayable: ['video', 'audio'].includes(entry.contentType),
      // contributorPresenceId will be populated when ContributorPresence data is generated
      contributorPresenceId: entry.contributorPresenceId
    };

    // Generate YouTube thumbnail if URL is a YouTube link
    if (entry.url && entry.url.includes('youtube.com')) {
      const videoId = this.extractYouTubeId(entry.url);
      if (videoId) {
        preview.thumbnailUrl = `https://img.youtube.com/vi/${videoId}/mqdefault.jpg`;
      }
    }

    return preview;
  }

  /**
   * Extract YouTube video ID from URL.
   */
  private extractYouTubeId(url: string): string | null {
    // Handle various YouTube URL formats
    // https://www.youtube.com/watch?v=VIDEO_ID
    // https://youtu.be/VIDEO_ID
    // https://www.youtube.com/c/CHANNEL (no video ID)
    const patterns = [
      /youtube\.com\/watch\?v=([^&]+)/,
      /youtu\.be\/([^?]+)/,
      /youtube\.com\/embed\/([^?]+)/
    ];

    for (const pattern of patterns) {
      const match = url.match(pattern);
      if (match) {
        return match[1];
      }
    }

    return null;
  }
}
