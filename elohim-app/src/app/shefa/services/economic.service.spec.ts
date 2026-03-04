/**
 * Economic Service Tests
 *
 * Tests the economic service which provides access to hREA EconomicEvent
 * operations via Holochain, with caching and query capabilities.
 */

import { TestBed } from '@angular/core/testing';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import { EconomicService, CreateEconomicEventInput } from './economic.service';
import { vi } from 'vitest';

describe('EconomicService', () => {
  let service: EconomicService;
  let holochainClientMock: any;

  beforeEach(() => {
    holochainClientMock = {
      callZome: vi.fn(),
      isConnected: vi.fn(),
    };

    // Default: not connected
    holochainClientMock.isConnected.mockReturnValue(false);
    holochainClientMock.callZome.mockReturnValue(
      Promise.resolve({ success: false, error: 'Not connected' })
    );

    TestBed.configureTestingModule({
      providers: [
        EconomicService,
        { provide: HolochainClientService, useValue: holochainClientMock },
      ],
    });
    service = TestBed.inject(EconomicService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('availability', () => {
    it('should start as not available', () => {
      expect(service.isAvailable()).toBe(false);
      expect(service.available()).toBe(false);
    });

    it('should report ready as false when not available', () => {
      expect(service.ready()).toBe(false);
    });

    describe('testAvailability', () => {
      it('should set available to true when zome responds successfully', async () => {
        holochainClientMock.callZome.mockReturnValue(Promise.resolve({ success: true, data: [] }));

        const result = await service.testAvailability();

        expect(result).toBe(true);
        expect(service.isAvailable()).toBe(true);
        expect(holochainClientMock.callZome).toHaveBeenCalledWith({
          zomeName: 'content_store',
          fnName: 'get_events_by_provider',
          payload: 'test-availability',
        });
      });

      it('should set available to false when zome call fails', async () => {
        holochainClientMock.callZome.mockReturnValue(
          Promise.resolve({ success: false, error: 'Zome not found' })
        );

        const result = await service.testAvailability();

        expect(result).toBe(false);
        expect(service.isAvailable()).toBe(false);
      });

      it('should handle exceptions gracefully', async () => {
        holochainClientMock.callZome.mockReturnValue(Promise.reject(new Error('Connection lost')));

        const result = await service.testAvailability();

        expect(result).toBe(false);
        expect(service.isAvailable()).toBe(false);
      });
    });
  });

  describe('getEventsForAgent', () => {
    it('should return empty array when service not available', () =>
      new Promise<void>(done => {
        service.getEventsForAgent('agent-1').subscribe(result => {
          expect(result).toEqual([]);
          expect(holochainClientMock.callZome).not.toHaveBeenCalled();
          done();
        });
      }));

    it('should return empty array for unavailable service with different directions', () =>
      new Promise<void>(done => {
        service.getEventsForAgent('agent-1', 'provider').subscribe(() => {
          service.getEventsForAgent('agent-1', 'receiver').subscribe(() => {
            service.getEventsForAgent('agent-1', 'both').subscribe(result => {
              expect(result).toEqual([]);
              done();
            });
          });
        });
      }));
  });

  describe('getEventsByAction', () => {
    it('should return empty array when service not available', () =>
      new Promise<void>(done => {
        service.getEventsByAction('use').subscribe(result => {
          expect(result).toEqual([]);
          expect(holochainClientMock.callZome).not.toHaveBeenCalled();
          done();
        });
      }));

    it('should have getEventsByAction method', () => {
      expect(service.getEventsByAction).toBeDefined();
      expect(typeof service.getEventsByAction).toBe('function');
    });
  });

  describe('getEventsByLamadType', () => {
    it('should return empty array when service not available', () =>
      new Promise<void>(done => {
        service.getEventsByLamadType('content-view').subscribe(result => {
          expect(result).toEqual([]);
          done();
        });
      }));

    it('should have getEventsByLamadType method', () => {
      expect(service.getEventsByLamadType).toBeDefined();
      expect(typeof service.getEventsByLamadType).toBe('function');
    });
  });

  describe('createEvent', () => {
    const input: CreateEconomicEventInput = {
      action: 'use',
      providerId: 'agent-1',
      receiverId: 'agent-2',
      resourceQuantityValue: 5,
    };

    it('should throw error when service not available', () => {
      expect(() => service.createEvent(input)).toThrowError('Economic service not available');
    });

    it('should have createEvent method', () => {
      expect(service.createEvent).toBeDefined();
      expect(typeof service.createEvent).toBe('function');
    });
  });

  describe('clearCache', () => {
    it('should clear cache without error', () => {
      expect(() => service.clearCache()).not.toThrow();
    });

    it('should have clearCache method', () => {
      expect(service.clearCache).toBeDefined();
      expect(typeof service.clearCache).toBe('function');
    });
  });
});
