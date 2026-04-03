import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { AttentionTrackerService } from './attention-tracker.service';
import { EventService } from './event.service';
import { AgentService } from '@app/elohim/services/agent.service';

describe('AttentionTrackerService', () => {
  let service: AttentionTrackerService;
  let eventServiceMock: Record<string, ReturnType<typeof vi.fn>>;
  let agentServiceMock: Record<string, ReturnType<typeof vi.fn>>;

  const MOCK_AGENT_ID = 'agent-maya-123';
  const MOCK_EVENT = { id: 'evt-1' } as any;

  beforeEach(() => {
    eventServiceMock = {
      recordContentInteraction: vi.fn().mockReturnValue(of(MOCK_EVENT)),
      recordContentView: vi.fn().mockReturnValue(of(MOCK_EVENT)),
      recordContentComplete: vi.fn().mockReturnValue(of(MOCK_EVENT)),
      hasViewed: vi.fn().mockReturnValue(of(false)),
      getViewCount: vi.fn().mockReturnValue(of(0)),
      getCompletionCount: vi.fn().mockReturnValue(of(0)),
    };
    agentServiceMock = {
      getCurrentAgentId: vi.fn().mockReturnValue(MOCK_AGENT_ID),
    };

    TestBed.configureTestingModule({
      providers: [
        AttentionTrackerService,
        { provide: EventService, useValue: eventServiceMock },
        { provide: AgentService, useValue: agentServiceMock },
      ],
    });
    service = TestBed.inject(AttentionTrackerService);
  });

  describe('trackContentView', () => {
    it('records a view event after dwell threshold', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      expect(eventServiceMock['recordContentInteraction']).toHaveBeenCalledWith(
        MOCK_AGENT_ID,
        'concept-trust',
        'content-view',
      );
    }));

    it('does NOT record a view event for bounce (under threshold)', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(2000);
      service.trackContentLeave('concept-trust');

      expect(eventServiceMock['recordContentInteraction']).not.toHaveBeenCalled();
    }));

    it('deduplicates views within same session', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      expect(eventServiceMock['recordContentInteraction']).toHaveBeenCalledTimes(1);
    }));

    it('records separate events for different content', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      service.trackContentView('concept-governance');
      tick(3000);
      service.trackContentLeave('concept-governance');

      expect(eventServiceMock['recordContentInteraction']).toHaveBeenCalledTimes(2);
    }));

    it('records the view event at threshold time, not on leave', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);

      // Event fires at threshold, before leave
      expect(eventServiceMock['recordContentInteraction']).toHaveBeenCalledTimes(1);

      service.trackContentLeave('concept-trust');
    }));
  });

  describe('getSessionViewedIds', () => {
    it('returns empty set initially', () => {
      expect(service.getSessionViewedIds().size).toBe(0);
    });

    it('includes content IDs after qualified views', fakeAsync(() => {
      service.trackContentView('concept-trust');
      tick(3000);
      service.trackContentLeave('concept-trust');

      expect(service.getSessionViewedIds().has('concept-trust')).toBe(true);
    }));
  });
});
