import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { LamadEventService } from './lamad-event.service';
import { EventService } from '@app/shefa/services/event.service';

describe('LamadEventService', () => {
  let service: LamadEventService;
  let eventServiceSpy: { recordContentInteraction: ReturnType<typeof vi.fn> };

  const MOCK_EVENT = { id: 'evt-1' } as any;
  const AGENT = 'agent-1';
  const CONTENT = 'content-1';

  beforeEach(() => {
    eventServiceSpy = {
      recordContentInteraction: vi.fn().mockReturnValue(of(MOCK_EVENT)),
    };

    TestBed.configureTestingModule({
      providers: [
        LamadEventService,
        { provide: EventService, useValue: eventServiceSpy },
      ],
    });
    service = TestBed.inject(LamadEventService);
  });

  it('creates', () => {
    expect(service).toBeTruthy();
  });

  it('recordQuizSubmit calls recordContentInteraction with quiz-submit', () => {
    service.recordQuizSubmit(AGENT, CONTENT, 'quiz-1', true, 85);
    expect(eventServiceSpy.recordContentInteraction).toHaveBeenCalled();
  });

  it('recordAssessmentComplete calls recordContentInteraction', () => {
    service.recordAssessmentComplete(AGENT, CONTENT, 'assess-1', 90);
    expect(eventServiceSpy.recordContentInteraction).toHaveBeenCalled();
  });

  it('recordPathStepComplete delegates to EventService', () => {
    service.recordPathStepComplete(AGENT, 'path-1', 'step-1');
    expect(eventServiceSpy.recordContentInteraction).toHaveBeenCalled();
  });
});
