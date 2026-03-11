import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { EconomicEventsApiService } from './economic-events-api.service';
import type {
  AppreciationDisplay,
  CreateAppreciationInput,
  CreateEconomicEventInput,
  EconomicEvent,
} from '../interfaces/economic-event-factory.interface';

describe('EconomicEventsApiService', () => {
  let service: EconomicEventsApiService;
  let httpMock: HttpTestingController;

  const stubEvent: EconomicEvent = {
    id: 'evt-1',
    eventType: 'transfer',
    timestamp: '2026-03-07T00:00:00Z',
    providerId: 'agent-a',
    receiverId: 'agent-b',
    quantity: 10,
    unit: 'credits',
    action: 'transfer',
    metadata: {},
    state: { status: 'validated', timestamp: '2026-03-07T00:00:00Z' },
    createdAt: '2026-03-07T00:00:00Z',
    createdBy: 'agent-a',
  };

  const stubAppreciation: AppreciationDisplay = {
    id: 'app-1',
    appreciationOf: 'evt-1',
    appreciatedBy: 'agent-a',
    appreciationTo: 'agent-b',
    quantityValue: 5,
    quantityUnit: 'recognition-points',
    note: null,
    createdAt: '2026-03-07T00:00:00Z',
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        EconomicEventsApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(EconomicEventsApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('getEventsByProvider calls GET with provider param', async () => {
    const promise = service.getEventsByProvider('agent-a');
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/economic-events' && r.params.get('provider') === 'agent-a'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubEvent]);
    expect(await promise).toEqual([stubEvent]);
  });

  it('getEventsByReceiver calls GET with receiver param', async () => {
    const promise = service.getEventsByReceiver('agent-b');
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/economic-events' && r.params.get('receiver') === 'agent-b'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubEvent]);
    expect(await promise).toEqual([stubEvent]);
  });

  it('getEventsByAction calls GET with action param', async () => {
    const promise = service.getEventsByAction('transfer');
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/economic-events' && r.params.get('action') === 'transfer'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubEvent]);
    expect(await promise).toEqual([stubEvent]);
  });

  it('getEventsByLamadType calls GET with lamadType param', async () => {
    const promise = service.getEventsByLamadType('mastery');
    const req = httpMock.expectOne(
      (r) => r.url === '/api/v1/economic-events' && r.params.get('lamadType') === 'mastery'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubEvent]);
    expect(await promise).toEqual([stubEvent]);
  });

  it('createEconomicEvent calls POST with payload', async () => {
    const payload: CreateEconomicEventInput = {
      action: 'transfer',
      providerId: 'agent-a',
      receiverId: 'agent-b',
      resourceQuantityValue: 10,
      resourceQuantityUnit: 'credits',
    };
    const promise = service.createEconomicEvent(payload);
    const req = httpMock.expectOne('/api/v1/economic-events');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(payload);
    req.flush(stubEvent);
    expect(await promise).toEqual(stubEvent);
  });

  it('getAppreciationsFor calls GET with for param', async () => {
    const promise = service.getAppreciationsFor('evt-1');
    const req = httpMock.expectOne(
      (r) =>
        r.url === '/api/v1/economic-events/appreciations' && r.params.get('for') === 'evt-1'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubAppreciation]);
    expect(await promise).toEqual([stubAppreciation]);
  });

  it('getAppreciationsBy calls GET with by param', async () => {
    const promise = service.getAppreciationsBy('agent-a');
    const req = httpMock.expectOne(
      (r) =>
        r.url === '/api/v1/economic-events/appreciations' && r.params.get('by') === 'agent-a'
    );
    expect(req.request.method).toBe('GET');
    req.flush([stubAppreciation]);
    expect(await promise).toEqual([stubAppreciation]);
  });

  it('createAppreciation calls POST with payload', async () => {
    const payload: CreateAppreciationInput = {
      appreciationOf: 'evt-1',
      appreciationTo: 'agent-b',
      quantityValue: 5,
      quantityUnit: 'recognition-points',
    };
    const promise = service.createAppreciation(payload);
    const req = httpMock.expectOne('/api/v1/economic-events/appreciations');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(payload);
    req.flush(stubAppreciation);
    expect(await promise).toEqual(stubAppreciation);
  });
});
