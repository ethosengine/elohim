import { describe, it, expect, vi, beforeEach } from 'vitest';
import { TestBed } from '@angular/core/testing';

import { SignalAccumulationService } from './signal-accumulation.service';
import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import type { SignalAggregateView } from '@elohim/storage-client/generated';

const mockGovernanceApi = {
  getSignalAggregate: vi.fn(),
};

function makeAggregate(
  total: number,
  consensus: number,
  participants = total
): SignalAggregateView {
  return {
    entityType: 'content',
    entityId: 'test-1',
    totalSignals: BigInt(total),
    uniqueParticipants: BigInt(participants),
    byType: {},
    byValue: {},
    consensusStrength: consensus,
  };
}

describe('SignalAccumulationService', () => {
  let service: SignalAccumulationService;

  beforeEach(() => {
    vi.clearAllMocks();
    TestBed.configureTestingModule({
      providers: [
        SignalAccumulationService,
        { provide: GovernanceApiService, useValue: mockGovernanceApi },
      ],
    });
    service = TestBed.inject(SignalAccumulationService);
  });

  it('below minimum signals — nothing triggered', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(5, 0.5));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.readyForSensemaking).toBe(false);
    expect(status.controversyDetected).toBe(false);
    expect(status.settled).toBe(false);
  });

  it('ready for sensemaking — enough signals, low consensus', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(25, 0.5));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.readyForSensemaking).toBe(true);
  });

  it('high consensus blocks sensemaking', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(25, 0.8));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.readyForSensemaking).toBe(false);
  });

  it('controversy detected — many signals, very low consensus', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(15, 0.2));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.controversyDetected).toBe(true);
  });

  it('not enough signals for controversy', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(5, 0.1));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.controversyDetected).toBe(false);
  });

  it('settled — high signals and high consensus', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(35, 0.9));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.settled).toBe(true);
  });

  it('not enough signals for settled', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(20, 0.9));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.settled).toBe(false);
  });

  it('not enough consensus for settled', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(35, 0.7));

    const status = await service.getAccumulationStatus('content', 'test-1');

    expect(status.settled).toBe(false);
  });

  it('edge case: exactly at sensemaking thresholds — not triggered', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(20, 0.7));

    const status = await service.getAccumulationStatus('content', 'test-1');

    // 20 >= 20 is true, but 0.7 < 0.7 is false
    expect(status.readyForSensemaking).toBe(false);
  });

  it('edge case: exactly at controversy thresholds — not triggered', async () => {
    mockGovernanceApi.getSignalAggregate.mockResolvedValue(makeAggregate(10, 0.3));

    const status = await service.getAccumulationStatus('content', 'test-1');

    // 10 >= 10 is true, but 0.3 < 0.3 is false
    expect(status.controversyDetected).toBe(false);
  });
});
