/* eslint-disable @typescript-eslint/require-await -- Observable→Promise bridging */
import { Injectable, inject } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { StorageApiService } from '../../elohim/services/storage-api.service';
import { ContentMetadata, ContentNode } from '../../lamad/models/content-node.model';

import type { WorkStoryStatus } from '../models/work-story.model';

import type { ContentWithTagsView } from '@elohim/storage-client/generated';

// TODO: [HOLOCHAIN-ZOME] writes currently go direct to storage (same as seed workflow).
// Route through conductor once the work-story zome is implemented.

/**
 * Map a storage ContentWithTagsView to the app's ContentNode domain type.
 * The wire format is already camelCase with parsed JSON — no transformation needed,
 * just field projection.
 */
function toContentNode(view: ContentWithTagsView): ContentNode {
  return {
    id: view.id,
    contentType: view.contentType,
    title: view.title,
    description: view.description ?? '',
    content: view.contentBody ?? '',
    contentFormat: view.contentFormat,
    tags: view.tags,
    relatedNodeIds: [],
    metadata: (view.metadata ?? {}) as ContentMetadata,
    reach: view.reach,
    createdAt: view.createdAt,
    updatedAt: view.updatedAt,
  };
}

@Injectable({ providedIn: 'root' })
export class AvodahApiService {
  private readonly storageApi = inject(StorageApiService);

  async getProjects(): Promise<ContentNode[]> {
    const views = await firstValueFrom(
      this.storageApi.getContents({ contentType: 'work-project' }),
    );
    return views.map(toContentNode);
  }

  async getStoriesForProject(projectId: string): Promise<ContentNode[]> {
    const views = await firstValueFrom(
      this.storageApi.getContents({ contentType: 'work-story' }),
    );
    return views
      .map(toContentNode)
      .filter(
        n => (n.metadata as Record<string, unknown>)['projectId'] === projectId,
      );
  }

  /**
   * Update a story's status.
   *
   * @param isTerminal - set true when moving to a done-state column (`isTerminal: true`
   *   in the project's BoardColumn config). This triggers an economic event in shefa.
   */
  async updateStoryStatus(
    storyId: string,
    status: WorkStoryStatus,
    isTerminal = false,
  ): Promise<void> {
    await firstValueFrom(
      this.storageApi.updateContent(storyId, { metadata: { status } }),
    );

    if (isTerminal) {
      // REA transition: done → economic event settles the work record
      await firstValueFrom(
        this.storageApi.createEconomicEvent({
          action: 'work',
          provider: storyId,
          receiver: storyId,
          contentId: storyId,
          lamadEventType: 'work-complete',
        }),
      );
    }
  }
}
