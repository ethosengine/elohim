/**
 * AttentionTrackerService Tests
 *
 * Updated for M-REA-2: AttentionTrackerService now posts to
 * POST /api/v1/attention/tending via AttentionTendingApiService.
 * EventService dependency removed — content-view tracking IS
 * AttentionTending, not a separate EconomicEvent (F-REA-2 retirement).
 *
 * Behaviour changes from M-REA-2:
 * - No sessionViewed Set: substrate deduplicates via re-tend (refresh_tending_ttl).
 *   Tests that asserted deduplication-via-Set now test that the route IS called
 *   for each visit (substrate decides whether to deduplicate).
 * - No dwell timer: elapsed time is sent on leave; the route qualifies.
 *   Tests that asserted "fires at 3000ms, not on leave" now test that the POST
 *   happens on leave, passing the elapsed time (substrate gates dwell).
 * - trackContentLeave sends elapsedMs; the service no longer suppresses calls
 *   below 3000ms (that's now the route's job).
 */

import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { AttentionTrackerService, AGENT_CONTEXT } from './attention-tracker.service';
import { AttentionTendingApiService } from './attention-tending-api.service';

describe('AttentionTrackerService', () => {
  let service: AttentionTrackerService;
  let attentionApiSpy: { postTending: ReturnType<typeof vi.fn> };
  let agentContextMock: { getCurrentAgentId: ReturnType<typeof vi.fn> };

  const MOCK_AGENT_ID = 'agent-maya-123';
  const MOCK_ACK = { accepted: true, actionHash: 'abc123' } as any;

  beforeEach(() => {
    attentionApiSpy = {
      postTending: vi.fn().mockReturnValue(of(MOCK_ACK)),
    };
    agentContextMock = {
      getCurrentAgentId: vi.fn().mockReturnValue(MOCK_AGENT_ID),
    };

    TestBed.configureTestingModule({
      providers: [
        AttentionTrackerService,
        { provide: AttentionTendingApiService, useValue: attentionApiSpy },
        { provide: AGENT_CONTEXT, useValue: agentContextMock },
      ],
    });
    service = TestBed.inject(AttentionTrackerService);
  });

  describe('trackContentView + trackContentLeave', () => {
    it('posts to attention/tending on leave with elapsed time', () => {
      service.trackContentView('concept-trust');
      // Brief pause (not measured here — Date.now() is live, not faked)
      service.trackContentLeave('concept-trust');

      expect(attentionApiSpy.postTending).toHaveBeenCalledOnce();
      const callArg = attentionApiSpy.postTending.mock.calls[0][0];
      expect(callArg.classification).toBe('values-forward');
      expect(callArg.filterSubjectJson).toContain('concept-trust');
      expect(callArg.elapsedMs).toBeGreaterThanOrEqual(0);
      expect(callArg.ttlSeconds).toBeGreaterThanOrEqual(3600);
    });

    it('does NOT post if trackContentLeave called without trackContentView', () => {
      // Leaving content that was never tracked — no mount time recorded.
      service.trackContentLeave('concept-trust');
      expect(attentionApiSpy.postTending).not.toHaveBeenCalled();
    });

    it('clears mount time after trackContentLeave (no double-send)', () => {
      service.trackContentView('concept-trust');
      service.trackContentLeave('concept-trust');
      service.trackContentLeave('concept-trust'); // second leave — no mount time

      expect(attentionApiSpy.postTending).toHaveBeenCalledOnce();
    });

    it('tracks separate content nodes independently', () => {
      service.trackContentView('concept-trust');
      service.trackContentView('concept-governance');
      service.trackContentLeave('concept-trust');
      service.trackContentLeave('concept-governance');

      expect(attentionApiSpy.postTending).toHaveBeenCalledTimes(2);
      const calls = attentionApiSpy.postTending.mock.calls;
      const firstContent = JSON.parse(calls[0][0].filterSubjectJson);
      const secondContent = JSON.parse(calls[1][0].filterSubjectJson);
      expect(firstContent.contentId).toBe('concept-trust');
      expect(secondContent.contentId).toBe('concept-governance');
    });

    it('second trackContentView for same content does not overwrite mount time', () => {
      service.trackContentView('concept-trust');
      const firstMount = Date.now();

      // Simulate re-mount without leave — mount time should not be reset.
      service.trackContentView('concept-trust');

      service.trackContentLeave('concept-trust');
      expect(attentionApiSpy.postTending).toHaveBeenCalledOnce();
    });
  });

  describe('ngOnDestroy', () => {
    it('clears mount times on destroy', () => {
      service.trackContentView('concept-trust');
      service.ngOnDestroy();

      // After destroy, leave should not call postTending (mount time cleared).
      service.trackContentLeave('concept-trust');
      expect(attentionApiSpy.postTending).not.toHaveBeenCalled();
    });
  });
});
