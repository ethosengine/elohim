import { inject, Injectable } from '@angular/core';

import { firstValueFrom } from 'rxjs';

import { UpdateContentPatch } from '../../elohim/interfaces';
import { StorageApiService } from '../../elohim/services/storage-api.service';
import { ContentNode, ContentRelationshipType } from '../../lamad/models/content-node.model';
import { WorkStoryStatus } from '../models/work-story.model';

import type { ContentWithTagsView } from '@elohim/storage-client/generated';

function toContentNode(view: ContentWithTagsView): ContentNode {
  return {
    id: view.id,
    contentType: view.contentType as ContentNode['contentType'],
    title: view.title,
    description: view.description ?? '',
    content: view.contentBody ?? '',
    contentFormat: view.contentFormat as ContentNode['contentFormat'],
    tags: view.tags ?? [],
    relatedNodeIds: [],
    metadata: (view.metadata as Record<string, unknown>) ?? {},
    createdAt: view.createdAt,
    updatedAt: view.updatedAt,
  };
}

@Injectable({ providedIn: 'root' })
export class AvodahApiService {
  private readonly storageApi = inject(StorageApiService);

  async getProjects(): Promise<ContentNode[]> {
    const views = await firstValueFrom(
      this.storageApi.getContents({ contentType: 'work-project' })
    );
    return views.map(toContentNode);
  }

  async getStoriesForProject(projectId: string): Promise<ContentNode[]> {
    const views = await firstValueFrom(this.storageApi.getContents({ contentType: 'work-story' }));
    return views
      .filter(v => (v.metadata as Record<string, unknown> | null)?.['projectId'] === projectId)
      .map(toContentNode);
  }

  async updateStoryStatus(storyId: string, status: string, isTerminal = false): Promise<void> {
    await firstValueFrom(this.storageApi.updateContent(storyId, { metadata: { status } }));
    if (isTerminal) {
      await firstValueFrom(
        this.storageApi.createEconomicEvent({
          action: 'work',
          contentId: storyId,
        } as never)
      );
    }
  }

  async updateStoryField(storyId: string, patch: UpdateContentPatch): Promise<void> {
    await firstValueFrom(this.storageApi.updateContent(storyId, patch));
  }

  async createStory(
    projectId: string,
    title: string,
    status: WorkStoryStatus = 'backlog'
  ): Promise<ContentNode> {
    // eslint-disable-next-line sonarjs/pseudo-random -- ID suffix for local uniqueness, not security
    const id = `story-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const view = await firstValueFrom(
      this.storageApi.createContent({
        id,
        title,
        schemaVersion: 1,
        contentType: 'work-story',
        contentFormat: 'text',
        reach: 'private',
        metadata: {
          projectId,
          status,
          visibility: 'private',
          priority: 'medium',
        },
        tags: [],
      } as never)
    );
    return toContentNode(view);
  }

  async getAttachments(storyId: string): Promise<{ relationshipId: string; content: ContentNode }[]> {
    const relationships = await firstValueFrom(
      this.storageApi.getRelationships({
        sourceId: storyId,
        relationshipType: ContentRelationshipType.ATTACHED_TO,
      }),
    );
    if (relationships.length === 0) return [];

    const allContent = await firstValueFrom(this.storageApi.getContents({}));
    const contentMap = new Map(allContent.map(v => [v.id, toContentNode(v)]));

    return relationships
      .filter(r => contentMap.has(r.targetId))
      .map(r => ({ relationshipId: r.id, content: contentMap.get(r.targetId)! }));
  }

  async attachContent(storyId: string, contentId: string): Promise<void> {
    await firstValueFrom(
      this.storageApi.createRelationship({
        sourceId: storyId,
        targetId: contentId,
        relationshipType: ContentRelationshipType.ATTACHED_TO,
        confidence: 1,
        inferenceSource: 'author',
      }),
    );
  }

  async detachContent(relationshipId: string): Promise<void> {
    await firstValueFrom(this.storageApi.deleteRelationship(relationshipId));
  }
}
