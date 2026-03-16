// schedule.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { HttpClient, HttpParams } from '@angular/common/http';
import { of } from 'rxjs';
import { vi, describe, it, expect, beforeEach } from 'vitest';

import { ScheduleService } from './schedule.service';

describe('ScheduleService', () => {
  let service: ScheduleService;
  let httpMock: Record<string, ReturnType<typeof vi.fn>>;

  beforeEach(() => {
    httpMock = {
      get: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
      patch: vi.fn().mockReturnValue(of({})),
    };

    TestBed.configureTestingModule({
      providers: [
        ScheduleService,
        { provide: HttpClient, useValue: httpMock },
      ],
    });
    service = TestBed.inject(ScheduleService);
  });

  it('should GET schedule by entity', () => {
    service.getSchedule('content', 'cid-123').subscribe();
    expect(httpMock.get).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules'),
      expect.objectContaining({ params: expect.anything() }),
    );
  });

  it('should POST to create schedule', () => {
    const input = {
      entityType: 'content',
      entityId: 'cid-123',
      scheduledAt: '2026-03-20T09:00:00Z',
      expiresAt: null,
      rrule: null,
    };
    service.createSchedule(input).subscribe();
    expect(httpMock.post).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules'),
      input,
    );
  });

  it('should PATCH to update schedule', () => {
    service.updateSchedule('sched-1', { expiresAt: '2026-04-20T00:00:00Z' }).subscribe();
    expect(httpMock.patch).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules/sched-1'),
      { expiresAt: '2026-04-20T00:00:00Z' },
    );
  });

  it('should GET due schedules', () => {
    service.getDueSchedules('2026-03-20T00:00:00Z').subscribe();
    expect(httpMock.get).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules'),
      expect.objectContaining({ params: expect.anything() }),
    );
  });

  it('should POST to advance occurrence', () => {
    service.advanceOccurrence('sched-1').subscribe();
    expect(httpMock.post).toHaveBeenCalledWith(
      expect.stringContaining('/api/v1/schedules/sched-1/advance'),
      {},
    );
  });
});
