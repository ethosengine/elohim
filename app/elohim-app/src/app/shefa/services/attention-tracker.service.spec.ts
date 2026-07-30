import { TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';

import { AttentionTrackerService } from './attention-tracker.service';

describe('AttentionTrackerService', () => {
  let service: AttentionTrackerService;
  let httpSpy: { post: ReturnType<typeof vi.fn> };

  const MOCK_ACK = { accepted: true };

  beforeEach(() => {
    httpSpy = {
      post: vi.fn().mockReturnValue(of(MOCK_ACK)),
    };

    TestBed.configureTestingModule({
      providers: [AttentionTrackerService, { provide: HttpClient, useValue: httpSpy }],
    });
    service = TestBed.inject(AttentionTrackerService);
  });

  describe('trackContentView + trackContentLeave', () => {
    it('posts to attention/tending on leave with elapsed time', () => {
      service.trackContentView('concept-trust');
      service.trackContentLeave('concept-trust');

      expect(httpSpy.post).toHaveBeenCalledOnce();
      const [url, body] = httpSpy.post.mock.calls[0];
      expect(url).toBe('/api/v1/attention/tending');
      expect(body.classification).toBe('values-forward');
      expect(JSON.parse(body.filterSubjectJson).contentId).toBe('concept-trust');
      expect(body.elapsedMs).toBeGreaterThanOrEqual(0);
      expect(body.ttlSeconds).toBeGreaterThanOrEqual(3600);
      expect(JSON.parse(body.contextJson).pillar).toBe('shefa');
    });

    it('does NOT post if trackContentLeave called without trackContentView', () => {
      service.trackContentLeave('concept-trust');
      expect(httpSpy.post).not.toHaveBeenCalled();
    });

    it('clears mount time after trackContentLeave (no double-send)', () => {
      service.trackContentView('concept-trust');
      service.trackContentLeave('concept-trust');
      service.trackContentLeave('concept-trust');

      expect(httpSpy.post).toHaveBeenCalledOnce();
    });

    it('tracks separate content nodes independently', () => {
      service.trackContentView('concept-trust');
      service.trackContentView('concept-governance');
      service.trackContentLeave('concept-trust');
      service.trackContentLeave('concept-governance');

      expect(httpSpy.post).toHaveBeenCalledTimes(2);
      const calls = httpSpy.post.mock.calls;
      expect(JSON.parse(calls[0][1].filterSubjectJson).contentId).toBe('concept-trust');
      expect(JSON.parse(calls[1][1].filterSubjectJson).contentId).toBe('concept-governance');
    });

    it('second trackContentView for same content does not overwrite mount time', () => {
      service.trackContentView('concept-trust');
      service.trackContentView('concept-trust'); // re-mount without leave — ignored
      service.trackContentLeave('concept-trust');

      expect(httpSpy.post).toHaveBeenCalledOnce();
    });
  });

  describe('ngOnDestroy', () => {
    it('clears mount times on destroy', () => {
      service.trackContentView('concept-trust');
      service.ngOnDestroy();

      service.trackContentLeave('concept-trust');
      expect(httpSpy.post).not.toHaveBeenCalled();
    });
  });
});
