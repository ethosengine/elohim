import { TestBed } from '@angular/core/testing';

import { AgentService } from '@app/elohim/services/agent.service';
import {
  EconomicEventsApiService,
  SignalEmitService,
  type SignalEmitResult,
  type SignalIntent,
} from '@app/shefa';

import { SignalHarnessService } from './signal-harness.service';

describe('SignalHarnessService — EPR write-through path (Task C.3)', () => {
  let harness: SignalHarnessService;
  let signalEmit: { tryEmit: ReturnType<typeof vi.fn> };
  let economicEvents: { createEconomicEvent: ReturnType<typeof vi.fn> };
  let agentService: { getCurrentAgentId: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    signalEmit = { tryEmit: vi.fn() };
    economicEvents = { createEconomicEvent: vi.fn().mockResolvedValue(undefined) };
    agentService = { getCurrentAgentId: vi.fn().mockReturnValue('agent-cid-test') };

    TestBed.configureTestingModule({
      providers: [
        SignalHarnessService,
        { provide: SignalEmitService, useValue: signalEmit },
        { provide: EconomicEventsApiService, useValue: economicEvents },
        { provide: AgentService, useValue: agentService },
      ],
    });
    harness = TestBed.inject(SignalHarnessService);
  });

  // The harness reads coupling from LAMAD_COUPLING_MAP. Use a content type
  // that exists in the map and produces an `onComplete` value flow.
  // 'concept' is canonical and has produce-on-complete coupling.
  const node = { id: 'node-1', contentType: 'concept', contentFormat: 'markdown' };
  const event = { type: 'view', passed: true } as const;

  it('emits via SignalEmitService when intent is shefa/EconomicEvent', async () => {
    const ok: SignalEmitResult = {
      status: 'emitted',
      response: { eprCid: 'cid-1', eventCid: 'cid-1' },
    };
    signalEmit.tryEmit.mockResolvedValue(ok);

    await harness.onRendererComplete(node, event);

    expect(signalEmit.tryEmit).toHaveBeenCalledTimes(1);
    const intent = signalEmit.tryEmit.mock.calls[0][0] as SignalIntent;
    expect(intent.pillar).toBe('shefa');
    expect(intent.signalType).toBe('EconomicEvent');
    expect(intent.agentCid).toBe('agent-cid-test');
    expect(intent.couplingRefs.knowledge).toBe('node-1');
    // Legacy path was NOT called.
    expect(economicEvents.createEconomicEvent).not.toHaveBeenCalled();
  });

  it('falls back to legacy path on 503 (write-through OFF)', async () => {
    const fallback: SignalEmitResult = {
      status: 'fallback',
      reason: 'write-through OFF for shefa/EconomicEvent',
    };
    signalEmit.tryEmit.mockResolvedValue(fallback);

    await harness.onRendererComplete(node, event);

    expect(signalEmit.tryEmit).toHaveBeenCalledTimes(1);
    expect(economicEvents.createEconomicEvent).toHaveBeenCalledTimes(1);
  });

  it('throws on non-503 errors from signal-emit', async () => {
    const err: SignalEmitResult = {
      status: 'error',
      status_code: 400,
      message: 'invalid agent cid',
    };
    signalEmit.tryEmit.mockResolvedValue(err);

    await expect(harness.onRendererComplete(node, event)).rejects.toThrow(/HTTP 400/);
    expect(economicEvents.createEconomicEvent).not.toHaveBeenCalled();
  });
});
