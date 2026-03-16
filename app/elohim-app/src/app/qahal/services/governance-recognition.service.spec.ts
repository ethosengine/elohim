/**
 * GovernanceRecognitionService Tests
 *
 * Tests weight mapping from mechanism levels and that recordParticipation
 * calls the recognition API with the correct trigger payload.
 */

import { TestBed } from '@angular/core/testing';

import { describe, expect, it, vi } from 'vitest';
import { of, throwError } from 'rxjs';

import { RecognitionApiService } from '@app/elohim/services/recognition-api.service';

import { GovernanceRecognitionService } from './governance-recognition.service';

describe('GovernanceRecognitionService', () => {
  let service: GovernanceRecognitionService;
  let recognitionApiMock: { distribute: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    recognitionApiMock = {
      distribute: vi.fn().mockReturnValue(
        of({
          contentId: 'entity-1',
          triggerEventType: 'governance-vote',
          rawAmount: 0.7,
          weightedAmount: 0.7,
          distributions: [],
          economicEventIds: [],
          limitsApplied: [],
        }),
      ),
    };

    TestBed.configureTestingModule({
      providers: [
        GovernanceRecognitionService,
        { provide: RecognitionApiService, useValue: recognitionApiMock },
      ],
    });

    service = TestBed.inject(GovernanceRecognitionService);
  });

  describe('weight mapping', () => {
    const expectedWeights: [number, number][] = [
      [0, 0.1],
      [1, 0.1],
      [2, 0.3],
      [3, 0.5],
      [4, 0.7],
      [5, 0.7],
      [6, 0.8],
      [7, 1.0],
    ];

    for (const [level, expectedWeight] of expectedWeights) {
      it(`maps level ${level} to weight ${expectedWeight}`, async () => {
        await service.recordParticipation({
          entityType: 'content',
          entityId: 'entity-1',
          humanId: 'human-1',
          mechanismLevel: level,
          participationType: 'vote',
        });

        expect(recognitionApiMock.distribute).toHaveBeenCalledWith({
          contentId: 'entity-1',
          eventType: 'governance-vote',
          rawAmount: expectedWeight,
          triggeredBy: 'human-1',
        });
      });
    }

    it('defaults to 0.1 for unknown levels', async () => {
      await service.recordParticipation({
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-1',
        mechanismLevel: 99,
        participationType: 'reaction',
      });

      expect(recognitionApiMock.distribute).toHaveBeenCalledWith(
        expect.objectContaining({ rawAmount: 0.1 }),
      );
    });
  });

  describe('recordParticipation', () => {
    it('calls recognition API with correct trigger for reaction', async () => {
      await service.recordParticipation({
        entityType: 'content',
        entityId: 'entity-42',
        humanId: 'human-matthew',
        mechanismLevel: 1,
        participationType: 'reaction',
      });

      expect(recognitionApiMock.distribute).toHaveBeenCalledOnce();
      expect(recognitionApiMock.distribute).toHaveBeenCalledWith({
        contentId: 'entity-42',
        eventType: 'governance-reaction',
        rawAmount: 0.1,
        triggeredBy: 'human-matthew',
      });
    });

    it('calls recognition API with correct trigger for graduated-feedback', async () => {
      await service.recordParticipation({
        entityType: 'proposal',
        entityId: 'prop-7',
        humanId: 'human-jessica',
        mechanismLevel: 2,
        participationType: 'graduated-feedback',
      });

      expect(recognitionApiMock.distribute).toHaveBeenCalledWith({
        contentId: 'prop-7',
        eventType: 'governance-graduated-feedback',
        rawAmount: 0.3,
        triggeredBy: 'human-jessica',
      });
    });

    it('calls recognition API with correct trigger for block', async () => {
      await service.recordParticipation({
        entityType: 'content',
        entityId: 'entity-1',
        humanId: 'human-1',
        mechanismLevel: 6,
        participationType: 'block',
      });

      expect(recognitionApiMock.distribute).toHaveBeenCalledWith({
        contentId: 'entity-1',
        eventType: 'governance-block',
        rawAmount: 0.8,
        triggeredBy: 'human-1',
      });
    });
  });

  describe('error handling', () => {
    it('propagates errors from recognition API (caller is responsible for catching)', async () => {
      recognitionApiMock.distribute.mockReturnValue(
        throwError(() => new Error('Network failure')),
      );

      await expect(
        service.recordParticipation({
          entityType: 'content',
          entityId: 'entity-1',
          humanId: 'human-1',
          mechanismLevel: 4,
          participationType: 'vote',
        }),
      ).rejects.toThrow('Network failure');
    });
  });
});
