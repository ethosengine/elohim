import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { IssueReportService, type IssueReportInput } from './issue-report.service';
import { StorageApiService } from './storage-api.service';

describe('IssueReportService', () => {
  let service: IssueReportService;
  let storageApiSpy: {
    createContent: ReturnType<typeof vi.fn>;
    getContents: ReturnType<typeof vi.fn>;
    updateContent: ReturnType<typeof vi.fn>;
  };

  const mockCreatedReport = {
    id: 'report-123',
    contentType: 'issue-report',
    title: 'Issue: Something broke',
    description: 'It broke when I clicked the button',
    contentBody: '',
    contentFormat: 'text',
    tags: ['issue-report', 'bug'],
    metadata: {
      category: 'bug',
      severity: 'error',
      diagnostics: { logs: [], correlationIds: [] },
      resolutionStatus: 'open',
    },
    reach: 'community',
    createdAt: '2026-03-15T12:00:00Z',
    updatedAt: '2026-03-15T12:00:00Z',
  };

  beforeEach(() => {
    storageApiSpy = {
      createContent: vi.fn().mockReturnValue(of(mockCreatedReport)),
      getContents: vi.fn().mockReturnValue(of([mockCreatedReport])),
      updateContent: vi.fn().mockReturnValue(of(mockCreatedReport)),
    };

    TestBed.configureTestingModule({
      providers: [
        IssueReportService,
        { provide: StorageApiService, useValue: storageApiSpy },
        { provide: HttpClient, useValue: { post: vi.fn().mockReturnValue(of({})) } },
      ],
    });

    service = TestBed.inject(IssueReportService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should create issue report as content node with contentType issue-report', () => {
    const input: IssueReportInput = {
      description: 'Something broke',
      category: 'bug',
      severity: 'error',
      diagnostics: {
        logs: [],
        environment: {} as never,
        context: {} as never,
        correlationIds: [],
        collectedAt: '',
      },
    };

    service.createReport(input).subscribe();

    expect(storageApiSpy.createContent).toHaveBeenCalledWith(
      expect.objectContaining({
        contentType: 'issue-report',
        title: expect.stringContaining('Something broke'),
        description: 'Something broke',
      }),
    );
  });

  it('should store diagnostics, category, severity in metadata', () => {
    const input: IssueReportInput = {
      description: 'Error loading content',
      category: 'bug',
      severity: 'warning',
      diagnostics: {
        logs: [],
        environment: {} as never,
        context: {} as never,
        correlationIds: ['c-1'],
        collectedAt: '',
      },
    };

    service.createReport(input).subscribe();

    const call = storageApiSpy.createContent.mock.calls[0][0];
    expect(call.metadata.category).toBe('bug');
    expect(call.metadata.severity).toBe('warning');
    expect(call.metadata.resolutionStatus).toBe('open');
    expect(call.metadata.diagnostics.correlationIds).toEqual(['c-1']);
  });

  it('should tag with issue-report and category', () => {
    const input: IssueReportInput = {
      description: 'Feature idea',
      category: 'feature-request',
      severity: 'info',
      diagnostics: {
        logs: [],
        environment: {} as never,
        context: {} as never,
        correlationIds: [],
        collectedAt: '',
      },
    };

    service.createReport(input).subscribe();

    const call = storageApiSpy.createContent.mock.calls[0][0];
    expect(call.tags).toContain('issue-report');
    expect(call.tags).toContain('feature-request');
  });

  it('should list reports by querying contentType=issue-report', () => {
    service.listReports().subscribe();

    expect(storageApiSpy.getContents).toHaveBeenCalledWith(
      expect.objectContaining({ contentType: 'issue-report' }),
    );
  });

  it('should update resolution status via updateContent', () => {
    service.updateResolution('report-123', 'resolved').subscribe();

    expect(storageApiSpy.updateContent).toHaveBeenCalledWith('report-123', {
      metadata: { resolutionStatus: 'resolved' },
    });
  });
});
