import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { StorageApiService } from './storage-api.service';
import { GateService } from './gate.service';

describe('StorageApiService — Comments', () => {
  let service: StorageApiService;
  let httpMock: HttpTestingController;
  let gateService: GateService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), StorageApiService],
    });

    service = TestBed.inject(StorageApiService);
    httpMock = TestBed.inject(HttpTestingController);
    gateService = TestBed.inject(GateService);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ===========================================================================
  // createComment
  // ===========================================================================

  describe('createComment', () => {
    it('should POST to /api/v1/comments with contentId and body', fakeAsync(() => {
      service.createComment('content-1', 'Great article!').subscribe();

      const req = httpMock.expectOne(
        request =>
          request.method === 'POST' && request.url.includes('/api/v1/comments')
      );

      expect(req.request.body).toEqual({
        contentId: 'content-1',
        body: 'Great article!',
      });

      req.flush({ data: { id: 'comment-1', contentId: 'content-1', body: 'Great article!' } });
      tick();
    }));

    it('should extract gate from successful response', fakeAsync(() => {
      const handleSpy = vi.spyOn(gateService, 'handleGateResponse');

      const mockGate = {
        passed: true,
        humanId: 'human-1',
        action: 'comment',
        gate: 'mastery',
        evaluatedAt: '2026-03-15T00:00:00Z',
      };

      service.createComment('content-1', 'Nice!').subscribe();

      const req = httpMock.expectOne(request => request.url.includes('/api/v1/comments'));
      req.flush({
        data: { id: 'comment-1' },
        gate: mockGate,
      });
      tick();

      expect(handleSpy).toHaveBeenCalledWith(mockGate);
    }));

    it('should handle 409 pause error with gate in error body', fakeAsync(() => {
      const handleSpy = vi.spyOn(gateService, 'handleGateResponse');

      const mockGate = {
        passed: false,
        humanId: 'human-1',
        action: 'comment',
        gate: 'mastery',
        pauseReason: 'Mastery threshold not met',
        evaluatedAt: '2026-03-15T00:00:00Z',
      };

      let errorReceived = false;

      service.createComment('content-1', 'Blocked').subscribe({
        error: () => {
          errorReceived = true;
        },
      });

      const req = httpMock.expectOne(request => request.url.includes('/api/v1/comments'));
      req.flush(
        { data: null, gate: mockGate },
        { status: 409, statusText: 'Conflict' }
      );
      tick();

      expect(errorReceived).toBe(true);
      expect(handleSpy).toHaveBeenCalledWith(mockGate);
    }));

    it('should handle non-gate errors', fakeAsync(() => {
      let error: Error | null = null;

      service.createComment('content-1', 'Fails').subscribe({
        error: (err: Error) => {
          error = err;
        },
      });

      const req = httpMock.expectOne(request => request.url.includes('/api/v1/comments'));
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('createComment failed');
    }));
  });

  // ===========================================================================
  // getComments
  // ===========================================================================

  describe('getComments', () => {
    it('should GET with contentId query param', fakeAsync(() => {
      const mockComments = [
        { id: 'comment-1', contentId: 'content-1', body: 'Hello' },
        { id: 'comment-2', contentId: 'content-1', body: 'World' },
      ];

      service.getComments('content-1').subscribe(comments => {
        expect(comments.length).toBe(2);
        expect(comments[0]).toEqual(mockComments[0]);
      });

      const req = httpMock.expectOne(request => {
        return (
          request.method === 'GET' &&
          request.url.includes('/api/v1/comments') &&
          request.params.get('contentId') === 'content-1'
        );
      });
      req.flush(mockComments);
      tick();
    }));

    it('should handle fetch errors', fakeAsync(() => {
      let error: Error | null = null;

      service.getComments('content-1').subscribe({
        error: (err: Error) => {
          error = err;
        },
      });

      const req = httpMock.expectOne(request => request.url.includes('/api/v1/comments'));
      req.error(new ProgressEvent('error'), { status: 500 });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('getComments failed');
    }));
  });
});
