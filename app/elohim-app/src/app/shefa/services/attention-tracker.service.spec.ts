import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { AttentionTendingApiService } from '@elohim/rea-runtime';

import { AttentionTrackerService } from './attention-tracker.service';

describe('AttentionTrackerService', () => {
  let service: AttentionTrackerService;
  let attentionApiSpy: { postTending: ReturnType<typeof vi.fn> };

  const MOCK_ACK = { accepted: true, actionHash: 'abc123' } as any;

  beforeEach(() => {
    attentionApiSpy = {
      postTending: vi.fn().mockReturnValue(of(MOCK_ACK)),
    };

    TestBed.configureTestingModule({
      providers: [
        AttentionTrackerService,
        { provide: AttentionTendingApiService, useValue: attentionApiSpy },
      ],
    });
    service = TestBed.inject(AttentionTrackerService);
  });

  describe('trackContentView + trackContentLeave', () => {
    it('posts to attention/tending on leave with elapsed time', () => {
      service.trackContentView('concept-trust');
      service.trackContentLeave('concept-trust');

      expect(attentionApiSpy.postTending).toHaveBeenCalledOnce();
      const callArg = attentionApiSpy.postTending.mock.calls[0][0];
      expect(callArg.classification).toBe('values-forward');
      expect(callArg.filterSubjectJson).toContain('concept-trust');
      expect(callArg.elapsedMs).toBeGreaterThanOrEqual(0);
      expect(callArg.ttlSeconds).toBeGreaterThanOrEqual(3600);
      expect(JSON.parse(callArg.contextJson).pillar).toBe('shefa');
    });

    it('does NOT post if trackContentLeave called without trackContentView', () => {
      service.trackContentLeave('concept-trust');
      expect(attentionApiSpy.postTending).not.toHaveBeenCalled();
    });

    it('clears mount time after trackContentLeave (no double-send)', () => {
      service.trackContentView('concept-trust');
      service.trackContentLeave('concept-trust');
      service.trackContentLeave('concept-trust');

      expect(attentionApiSpy.postTending).toHaveBeenCalledOnce();
    });

    it('tracks separate content nodes independently', () => {
      service.trackContentView('concept-trust');
      service.trackContentView('concept-governance');
      service.trackContentLeave('concept-trust');
      service.trackContentLeave('concept-governance');

      expect(attentionApiSpy.postTending).toHaveBeenCalledTimes(2);
      const calls = attentionApiSpy.postTending.mock.calls;
      expect(JSON.parse(calls[0][0].filterSubjectJson).contentId).toBe('concept-trust');
      expect(JSON.parse(calls[1][0].filterSubjectJson).contentId).toBe('concept-governance');
    });

    it('second trackContentView for same content does not overwrite mount time', () => {
      service.trackContentView('concept-trust');
      service.trackContentView('concept-trust'); // re-mount without leave — ignored
      service.trackContentLeave('concept-trust');

      expect(attentionApiSpy.postTending).toHaveBeenCalledOnce();
    });
  });

  describe('ngOnDestroy', () => {
    it('clears mount times on destroy', () => {
      service.trackContentView('concept-trust');
      service.ngOnDestroy();

      service.trackContentLeave('concept-trust');
      expect(attentionApiSpy.postTending).not.toHaveBeenCalled();
    });
  });
});
