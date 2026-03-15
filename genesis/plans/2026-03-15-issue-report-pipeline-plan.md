# Issue Report Pipeline — Implementation Plan (v2, content-node approach)

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a "Report Issue" entry to the governance feedback menu that silently collects diagnostics (logs, environment, health, route context), sends them through the gate conversation, and persists the report as a content node with `contentType: 'issue-report'` — using the same content CRUD that Avodah uses for work-stories.

**Architecture:** No new Rust code. Issue reports are content nodes (`contentType: 'issue-report'`) stored via existing `POST /db/content` and queried via `GET /db/content?contentType=issue-report`. Diagnostics, severity, category, and resolution status live in `metadata`. New `DiagnosticCollectorService` in Angular gathers runtime context. `FeedbackType` extended to include `'report'`. `StorageApiService.handleError()` wired to `LoggerService` so HTTP failures appear in diagnostic bundles. Future promotion to work-story is just `PATCH /db/content/{id}` with `contentType: 'work-story'`.

**Tech Stack:** Angular 19 (signals, OnPush, Vitest), existing elohim-storage content API

**Design doc:** `genesis/plans/2026-03-15-issue-report-pipeline-design.md`

**Avodah synergy:** Same pattern as `AvodahApiService` — store domain objects as content nodes, use `metadata` for domain-specific fields, use existing `createContent()` / `updateContent()` / `getContents()`. Issue report → work-story promotion is `updateContent(id, { contentType: 'work-story', metadata: { projectId, status: 'todo' } })`.

---

### Task 1: StorageApiService — error logging in handleError

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/storage-api.service.ts`

**Step 1: Add LoggerService injection**

At the top of the `StorageApiService` class, add:

```typescript
private readonly logger = inject(LoggerService);
```

Add import: `import { LoggerService } from './logger.service';`

**Step 2: Update handleError to log**

Replace the existing `handleError`:

```typescript
private handleError(operation: string, error: unknown): Observable<never> {
  const message = error instanceof Error ? error.message : String(error);
  const status = (error as Record<string, unknown>)['status'];
  const url = (error as Record<string, unknown>)['url'];

  this.logger.error(`${operation} failed`, error instanceof Error ? error : undefined, {
    operation,
    status: status as number,
    url: url as string,
  });

  return throwError(() => new Error(`${operation} failed: ${message}`));
}
```

**Step 3: Verify build**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "storage-api" 2>&1 | tail -10`
Expected: Existing tests pass (logger is providedIn: 'root', auto-available)

**Step 4: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/storage-api.service.ts
git commit -m "feat(elohim): wire LoggerService into StorageApiService.handleError for diagnostic capture"
```

---

### Task 2: DiagnosticCollectorService — tests

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/diagnostic-collector.service.spec.ts`

**Step 1: Write failing tests**

```typescript
import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { of, throwError } from 'rxjs';
import { vi } from 'vitest';

import { LoggerService, type LogEntry } from './logger.service';
import { DiagnosticCollectorService, type DiagnosticBundle } from './diagnostic-collector.service';

describe('DiagnosticCollectorService', () => {
  let service: DiagnosticCollectorService;
  let loggerMock: { getRecentLogs: ReturnType<typeof vi.fn> };
  let routerMock: { url: string };
  let httpMock: { get: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    loggerMock = {
      getRecentLogs: vi.fn().mockReturnValue([]),
    };
    routerMock = { url: '/learn/path/123/node/456' };
    httpMock = {
      get: vi.fn().mockReturnValue(of({ status: 'ok', blobs: 10, bytes: 1024 })),
    };

    TestBed.configureTestingModule({
      providers: [
        DiagnosticCollectorService,
        { provide: LoggerService, useValue: loggerMock },
        { provide: Router, useValue: routerMock },
        { provide: HttpClient, useValue: httpMock },
      ],
    });

    service = TestBed.inject(DiagnosticCollectorService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should include current route in context', async () => {
    const bundle = await service.collect();
    expect(bundle.context.url).toBe('/learn/path/123/node/456');
  });

  it('should include logs from LoggerService', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'error', message: 'test error' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.logs).toEqual(mockLogs);
  });

  it('should filter logs to warn and error levels', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'debug', message: 'noise' },
      { timestamp: '2026-03-15T10:00:01Z', level: 'info', message: 'info' },
      { timestamp: '2026-03-15T10:00:02Z', level: 'warn', message: 'warning' },
      { timestamp: '2026-03-15T10:00:03Z', level: 'error', message: 'error' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.logs.length).toBe(2);
    expect(bundle.logs[0].level).toBe('warn');
    expect(bundle.logs[1].level).toBe('error');
  });

  it('should include environment info', async () => {
    const bundle = await service.collect();
    expect(bundle.environment.platform).toBeDefined();
    expect(bundle.environment.userAgent).toBeDefined();
  });

  it('should fetch health snapshot', async () => {
    const bundle = await service.collect();
    expect(httpMock.get).toHaveBeenCalled();
    expect(bundle.environment.storageHealth).toEqual({ status: 'ok', blobs: 10, bytes: 1024 });
  });

  it('should handle health fetch failure gracefully', async () => {
    httpMock.get.mockReturnValue(throwError(() => new Error('network error')));

    const bundle = await service.collect();
    expect(bundle.environment.storageHealth).toBeNull();
  });

  it('should extract unique correlation IDs from logs', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'error', message: 'fail', correlationId: 'corr-1' },
      { timestamp: '2026-03-15T10:00:01Z', level: 'error', message: 'fail2', correlationId: 'corr-1' },
      { timestamp: '2026-03-15T10:00:02Z', level: 'warn', message: 'warn', correlationId: 'corr-2' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.correlationIds).toEqual(['corr-1', 'corr-2']);
  });

  it('should include collectedAt timestamp', async () => {
    const bundle = await service.collect();
    expect(bundle.collectedAt).toBeDefined();
    expect(new Date(bundle.collectedAt).getTime()).toBeGreaterThan(0);
  });
});
```

**Step 2: Commit (tests will fail — TDD red phase)**

```bash
git add app/elohim-app/src/app/elohim/services/diagnostic-collector.service.spec.ts
git commit -m "test(elohim): add failing tests for DiagnosticCollectorService"
```

---

### Task 3: DiagnosticCollectorService — implementation

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/diagnostic-collector.service.ts`

**Step 1: Write the service**

```typescript
import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { firstValueFrom, timeout, catchError, of } from 'rxjs';

import { LoggerService, type LogEntry } from './logger.service';

export interface DiagnosticBundle {
  logs: LogEntry[];
  environment: {
    platform: 'browser' | 'tauri';
    userAgent: string;
    appVersion: string;
    storageHealth: Record<string, unknown> | null;
  };
  context: {
    url: string;
    eprId: string | null;
    avodahProject: string | null;
    avodahStory: string | null;
  };
  correlationIds: string[];
  collectedAt: string;
}

@Injectable({ providedIn: 'root' })
export class DiagnosticCollectorService {
  private readonly logger = inject(LoggerService);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);

  async collect(): Promise<DiagnosticBundle> {
    const allLogs = this.logger.getRecentLogs();
    const logs = allLogs.filter(
      (l) => l.level === 'warn' || l.level === 'error',
    );

    const correlationIds = [
      ...new Set(
        logs
          .map((l) => l.correlationId)
          .filter((id): id is string => id != null),
      ),
    ];

    const isTauri =
      'window' in globalThis &&
      '__TAURI__' in (globalThis as Record<string, unknown>);

    let storageHealth: Record<string, unknown> | null = null;
    try {
      storageHealth = await firstValueFrom(
        this.http
          .get<Record<string, unknown>>('/health')
          .pipe(
            timeout(5000),
            catchError(() => of(null)),
          ),
      );
    } catch {
      // Health fetch failed — diagnostic context, not critical
    }

    return {
      logs,
      environment: {
        platform: isTauri ? 'tauri' : 'browser',
        userAgent: navigator.userAgent,
        appVersion: '0.1.0',
        storageHealth,
      },
      context: {
        url: this.router.url,
        eprId: null,
        avodahProject: null,
        avodahStory: null,
      },
      correlationIds,
      collectedAt: new Date().toISOString(),
    };
  }
}
```

**Step 2: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "diagnostic-collector"`
Expected: All 9 tests PASS

**Step 3: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/diagnostic-collector.service.ts
git commit -m "feat(elohim): implement DiagnosticCollectorService — gathers logs, health, route context"
```

---

### Task 4: Barrel exports

**Files:**
- Modify: `app/elohim-app/src/app/elohim/services/index.ts`

**Step 1: Add exports**

```typescript
export { DiagnosticCollectorService } from './diagnostic-collector.service';
export type { DiagnosticBundle } from './diagnostic-collector.service';
```

**Step 2: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/index.ts
git commit -m "chore(elohim): export DiagnosticCollectorService from services barrel"
```

---

### Task 5: IssueReportService — Avodah-pattern content-node API

**Files:**
- Create: `app/elohim-app/src/app/elohim/services/issue-report.service.ts`
- Create: `app/elohim-app/src/app/elohim/services/issue-report.service.spec.ts`

**Step 1: Write failing tests**

```typescript
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
      diagnostics: { logs: [], environment: {} as never, context: {} as never, correlationIds: [], collectedAt: '' },
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
      diagnostics: { logs: [], environment: {} as never, context: {} as never, correlationIds: ['c-1'], collectedAt: '' },
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
      diagnostics: { logs: [], environment: {} as never, context: {} as never, correlationIds: [], collectedAt: '' },
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
```

**Step 2: Write the service**

```typescript
import { Injectable, inject } from '@angular/core';
import { Observable, map } from 'rxjs';

import { StorageApiService } from './storage-api.service';
import type { DiagnosticBundle } from './diagnostic-collector.service';
import type { ContentWithTagsView, CreateContentInputView } from '@elohim/storage-client';

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
    const truncatedTitle = input.description.length > 80
      ? input.description.substring(0, 77) + '...'
      : input.description;

    const contentInput: CreateContentInputView = {
      id: `issue-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      title: `Issue: ${truncatedTitle}`,
      description: input.description,
      contentType: 'issue-report',
      contentFormat: 'text',
      contentBody: '',
      tags: ['issue-report', category],
      metadata: {
        category,
        severity,
        resolutionStatus: 'open',
        diagnostics: input.diagnostics,
        contextUrl: input.contextUrl ?? input.diagnostics.context.url,
        linkedGithubUrl: null,
        linkedWorkStoryId: null,
      },
    };

    return this.storageApi.createContent(contentInput);
  }

  listReports(): Observable<ContentWithTagsView[]> {
    return this.storageApi.getContents({ contentType: 'issue-report' });
  }

  updateResolution(
    reportId: string,
    status: ResolutionStatus,
  ): Observable<ContentWithTagsView> {
    return this.storageApi.updateContent(reportId, {
      metadata: { resolutionStatus: status },
    });
  }

  promoteToWorkStory(
    reportId: string,
    projectId: string,
  ): Observable<ContentWithTagsView> {
    return this.storageApi.updateContent(reportId, {
      metadata: {
        projectId,
        status: 'todo',
        promotedFrom: 'issue-report',
      },
    });
  }
}
```

**Step 3: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "issue-report"`
Expected: All 6 tests PASS

**Step 4: Add to barrel**

In `app/elohim-app/src/app/elohim/services/index.ts`:

```typescript
export { IssueReportService } from './issue-report.service';
export type { IssueReportInput, ResolutionStatus } from './issue-report.service';
```

**Step 5: Commit**

```bash
git add app/elohim-app/src/app/elohim/services/issue-report.service.ts
git add app/elohim-app/src/app/elohim/services/issue-report.service.spec.ts
git add app/elohim-app/src/app/elohim/services/index.ts
git commit -m "feat(elohim): add IssueReportService — stores reports as content nodes (Avodah pattern)"
```

---

### Task 6: Extend FeedbackType + wire modal for 'report'

**Files:**
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-modal.component.spec.ts`
- Modify: `app/elohim-app/src/app/elohim/components/gate-feedback/gate-feedback-trigger.component.spec.ts`

**Step 1: Extend FeedbackType**

In `gate-feedback-modal.component.ts`:

```typescript
export type FeedbackType = 'flag' | 'challenge' | 'feedback' | 'report';
```

Add to maps:

```typescript
const TITLE_MAP: Record<string, string> = {
  flag: 'Flag Content',
  challenge: 'Challenge Content',
  feedback: 'Share Feedback',
  report: 'Report Issue',
};

const PLACEHOLDER_MAP: Record<string, string> = {
  flag: 'Describe the issue...',
  challenge: 'State your case...',
  feedback: 'Share your thoughts...',
  report: 'What happened?',
};
```

**Step 2: Add diagnostic collection to modal**

Inject `DiagnosticCollectorService` and `IssueReportService`. Collect diagnostics when feedbackType is 'report'. Route the API call accordingly:

```typescript
import { DiagnosticCollectorService, type DiagnosticBundle } from '../../services/diagnostic-collector.service';
import { IssueReportService } from '../../services/issue-report.service';

// In class:
private readonly diagnosticCollector = inject(DiagnosticCollectorService);
private readonly issueReportService = inject(IssueReportService);
private diagnosticBundle: DiagnosticBundle | null = null;

constructor() {
  effect(() => {
    if (this.feedbackType() === 'report') {
      this.diagnosticCollector.collect().then((bundle) => {
        this.diagnosticBundle = bundle;
      });
    }
  });
}

// Update apiCall:
readonly apiCall = (text: string, context: MutationContext): Observable<unknown> => {
  if (context['category'] === 'report' && this.diagnosticBundle) {
    return this.issueReportService.createReport({
      description: text,
      diagnostics: this.diagnosticBundle,
      contextUrl: this.diagnosticBundle.context.url,
    });
  }
  return this.storageApi.createComment(context['contentId'] as string, text);
};
```

**Step 3: Add 'Report Issue' to trigger menu**

In `gate-feedback-trigger.component.ts`:

```typescript
const MENU_ITEMS: MenuItem[] = [
  { type: 'flag', label: 'Flag' },
  { type: 'challenge', label: 'Challenge' },
  { type: 'feedback', label: 'Feedback' },
  { type: 'report', label: 'Report Issue' },
];
```

**Step 4: Add tests**

In modal spec, add:

```typescript
it('should render "Report Issue" title for report type', () => {
  fixture.componentRef.setInput('feedbackType', 'report');
  fixture.detectChanges();
  const title = fixture.nativeElement.querySelector('[data-testid="feedback-modal-title"]');
  expect(title.textContent.trim()).toBe('Report Issue');
});

it('should set placeholder to "What happened?" for report type', () => {
  fixture.componentRef.setInput('feedbackType', 'report');
  fixture.detectChanges();
  const textarea = fixture.nativeElement.querySelector('[data-testid="artifact-textarea"]');
  expect(textarea.getAttribute('placeholder')).toBe('What happened?');
});
```

In trigger spec, update:

```typescript
it('should show four menu items', () => {
  // click trigger...
  const items = fixture.nativeElement.querySelectorAll('[data-testid^="feedback-menu-item-"]');
  expect(items.length).toBe(4);
});

it('should show Flag, Challenge, Feedback, Report Issue labels', () => {
  // click trigger...
  const labels = Array.from(
    fixture.nativeElement.querySelectorAll('[data-testid^="feedback-menu-item-"]'),
  ).map((el: Element) => (el as HTMLElement).textContent?.trim());
  expect(labels).toEqual(['Flag', 'Challenge', 'Feedback', 'Report Issue']);
});
```

**Step 5: Run tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts "gate-feedback"`
Expected: All tests pass (existing + new)

**Step 6: Commit**

```bash
git add app/elohim-app/src/app/elohim/components/gate-feedback/
git commit -m "feat(elohim): add 'Report Issue' to feedback menu with diagnostic collection"
```

---

### Task 7: Full test suite + lint

**Step 1: Run all Angular tests**

Run: `cd app/elohim-app && pnpm exec vitest run --config vite.config.ts`
Expected: All tests pass

**Step 2: Run lint**

Run: `cd app/elohim-app && pnpm run lint`
Expected: No new lint errors

**Step 3: Fix any issues, commit if needed**

---

## What Changed from v1

| v1 (dedicated table) | v2 (content-node) |
|---|---|
| New `issue_reports` migration | **Eliminated** — uses existing content table |
| New Diesel models | **Eliminated** |
| New Rust view types | **Eliminated** |
| New Rust CRUD module | **Eliminated** |
| New API routes | **Eliminated** — uses existing `/db/content` |
| TypeScript type generation | **Eliminated** |
| 11 tasks | **7 tasks** (all Angular-only) |

## Future Seams

### Screenshot Capture
- **Auto-capture (A):** `html2canvas` library, capture viewport on "Report Issue" click
- **User-provided (B):** Clipboard paste / drag-and-drop, upload as blob
- Both for AI agent and human review

### Agent Code Awareness
- Agent has codebase map as a tool, not payload
- Route-to-component mapping is agent investigation
- Backend log correlation via correlation IDs from diagnostic bundle

### Avodah Promotion
- Issue report → work-story: `updateContent(id, { contentType: 'work-story', metadata: { projectId, status: 'todo' } })`
- Already wired as `IssueReportService.promoteToWorkStory()`
- REA event on resolution: same pattern as Avodah terminal status
