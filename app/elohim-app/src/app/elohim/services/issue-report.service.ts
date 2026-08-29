import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import { generateId } from '../utils/id-generator';

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

/** lamad content type for a filed issue report. */
const ISSUE_REPORT_TYPE = 'issue-report';

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
      id: generateId('issue'),
      title: `Issue: ${truncatedTitle}`,
      schemaVersion: 1,
      description: input.description,
      contentType: ISSUE_REPORT_TYPE,
      contentFormat: 'text',
      contentBody: '',
      blobHash: null,
      blobCid: null,
      contentSizeBytes: null,
      tags: [ISSUE_REPORT_TYPE, category],
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
    return this.storageApi.getContents({ contentType: ISSUE_REPORT_TYPE });
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
        promotedFrom: ISSUE_REPORT_TYPE,
      },
    });
  }
}
