import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { StorageApiService } from './storage-api.service';

import type { DiagnosticBundle } from './diagnostic-collector.service';
import type { ContentWithTagsView, CreateContentInputView } from '@elohim/storage-client/generated';

export interface IssueReportInput {
  description: string;
  category?: string;
  severity?: string;
  diagnostics: DiagnosticBundle;
  contextUrl?: string;
}

export type ResolutionStatus = 'open' | 'investigating' | 'resolved' | 'wont-fix';

@Injectable({ providedIn: 'root' })
export class IssueReportService {
  private readonly storageApi = inject(StorageApiService);

  createReport(input: IssueReportInput): Observable<ContentWithTagsView> {
    const category = input.category ?? 'bug';
    const severity = input.severity ?? 'info';
    const truncatedTitle =
      input.description.length > 80
        ? input.description.substring(0, 77) + '...'
        : input.description;

    const contentInput: CreateContentInputView = {
      id: `issue-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      title: `Issue: ${truncatedTitle}`,
      schemaVersion: 1,
      description: input.description,
      contentType: 'issue-report',
      contentFormat: 'text',
      contentBody: '',
      blobHash: null,
      blobCid: null,
      contentSizeBytes: null,
      tags: ['issue-report', category],
      metadata: {
        category,
        severity,
        resolutionStatus: 'open',
        diagnostics: input.diagnostics as unknown as Record<string, never>,
        contextUrl: input.contextUrl ?? input.diagnostics.context.url,
        linkedGithubUrl: null,
        linkedWorkStoryId: null,
      },
      reach: null,
      createdBy: null,
    };

    return this.storageApi.createContent(contentInput);
  }

  listReports(): Observable<ContentWithTagsView[]> {
    return this.storageApi.getContents({ contentType: 'issue-report' });
  }

  updateResolution(reportId: string, status: ResolutionStatus): Observable<ContentWithTagsView> {
    return this.storageApi.updateContent(reportId, {
      metadata: { resolutionStatus: status },
    });
  }

  promoteToWorkStory(reportId: string, projectId: string): Observable<ContentWithTagsView> {
    return this.storageApi.updateContent(reportId, {
      metadata: {
        projectId,
        status: 'todo',
        promotedFrom: 'issue-report',
      },
    });
  }
}
