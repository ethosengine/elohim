/**
 * Appreciation Service Tests
 *
 * Tests the appreciation service which provides access to hREA Appreciation
 * operations via Holochain, with Signal-based state management and caching.
 */

import { TestBed } from '@angular/core/testing';

import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

import {
  AppreciationService,
  AppreciationDisplay,
  CreateAppreciationInput,
} from './appreciation.service';

describe('AppreciationService', () => {
  let service: AppreciationService;
  let holochainClientMock: jasmine.SpyObj<HolochainClientService>;

  const mockAppreciation: AppreciationDisplay = {
    id: 'appreciation-1',
    appreciationOf: 'content-1',
    appreciatedBy: 'agent-1',
    appreciationTo: 'agent-2',
    quantityValue: 1,
    quantityUnit: 'recognition-points',
    note: null,
    createdAt: new Date().toISOString(),
  };

  beforeEach(() => {
    holochainClientMock = jasmine.createSpyObj('HolochainClientService', ['callZome', 'isConnected']);

    // Default: not connected
    holochainClientMock.isConnected.and.returnValue(false);
    holochainClientMock.callZome.and.returnValue(
      Promise.resolve({ success: false, error: 'Not connected' })
    );

    TestBed.configureTestingModule({
      providers: [
        AppreciationService,
        { provide: HolochainClientService, useValue: holochainClientMock },
      ],
    });
    service = TestBed.inject(AppreciationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ==========================================================================
  // Availability Tests
  // ==========================================================================

  describe('availability', () => {
    it('should start as not available', () => {
      expect(service.isAvailable()).toBeFalse();
      expect(service.available()).toBeFalse();
    });

    it('should report ready as false when not connected', () => {
      expect(service.ready()).toBeFalse();
    });

    describe('testAvailability', () => {
      it('should set available to true when zome responds successfully', async () => {
        holochainClientMock.callZome.and.returnValue(
          Promise.resolve({ success: true, data: [] })
        );

        const result = await service.testAvailability();

        expect(result).toBeTrue();
        expect(service.isAvailable()).toBeTrue();
        expect(holochainClientMock.callZome).toHaveBeenCalledWith({
          zomeName: 'content_store',
          fnName: 'get_appreciations_for',
          payload: 'test-availability',
        });
      });

      it('should set available to false when zome call fails', async () => {
        holochainClientMock.callZome.and.returnValue(
          Promise.resolve({ success: false, error: 'Zome not found' })
        );

        const result = await service.testAvailability();

        expect(result).toBeFalse();
        expect(service.isAvailable()).toBeFalse();
      });

      it('should handle exceptions gracefully', async () => {
        holochainClientMock.callZome.and.returnValue(Promise.reject(new Error('Connection lost')));

        const result = await service.testAvailability();

        expect(result).toBeFalse();
        expect(service.isAvailable()).toBeFalse();
      });
    });
  });

  // ==========================================================================
  // Query Tests - getAppreciationsFor
  // ==========================================================================

  describe('getAppreciationsFor', () => {
    it('should return empty array when service not available', done => {
      service.getAppreciationsFor('entity-1').subscribe(result => {
        expect(result).toEqual([]);
        expect(holochainClientMock.callZome).not.toHaveBeenCalled();
        done();
      });
    });

    it('should fetch appreciations when service is available', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      // Now set up the actual response
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [
            {
              actionHash: new Uint8Array([1, 2, 3]),
              appreciation: {
                id: 'appreciation-1',
                appreciationOf: 'entity-1',
                appreciatedBy: 'agent-1',
                appreciationTo: 'agent-2',
                quantityValue: 5,
                quantityUnit: 'points',
                note: 'Great work!',
                createdAt: '2026-01-01T00:00:00Z',
              },
            },
          ],
        })
      );

      const result = await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });

      expect(result.length).toBe(1);
      expect(result[0].appreciationOf).toBe('entity-1');
      expect(result[0].quantityValue).toBe(5);
      expect(result[0].note).toBe('Great work!');
    });

    it('should cache results for same entity', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      // First call
      await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });

      // Second call - should use cache
      await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });

      // callZome should only be called for the availability test and once for data
      // (not twice for entity-1)
      const dataCalls = holochainClientMock.callZome.calls
        .all()
        .filter(call => call.args[0].fnName === 'get_appreciations_for');
      expect(dataCalls.length).toBe(2); // availability test + one data fetch
    });

    it('should handle errors gracefully', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      // Set up error response
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'Network error' })
      );

      const result = await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });

      expect(result).toEqual([]);
    });

    it('should sort results by date descending', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [
            {
              actionHash: new Uint8Array([1]),
              appreciation: {
                id: 'old',
                appreciationOf: 'entity-1',
                appreciatedBy: 'agent-1',
                appreciationTo: 'agent-2',
                quantityValue: 1,
                quantityUnit: 'points',
                note: null,
                createdAt: '2026-01-01T00:00:00Z',
              },
            },
            {
              actionHash: new Uint8Array([2]),
              appreciation: {
                id: 'new',
                appreciationOf: 'entity-1',
                appreciatedBy: 'agent-1',
                appreciationTo: 'agent-2',
                quantityValue: 1,
                quantityUnit: 'points',
                note: null,
                createdAt: '2026-01-15T00:00:00Z',
              },
            },
          ],
        })
      );

      const result = await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });

      expect(result[0].id).toBe('new'); // Newer first
      expect(result[1].id).toBe('old');
    });
  });

  // ==========================================================================
  // Query Tests - getAppreciationsBy
  // ==========================================================================

  describe('getAppreciationsBy', () => {
    it('should return empty array when service not available', done => {
      service.getAppreciationsBy('agent-1').subscribe(result => {
        expect(result).toEqual([]);
        done();
      });
    });

    it('should fetch appreciations given by an agent', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: [
            {
              actionHash: new Uint8Array([1, 2, 3]),
              appreciation: {
                id: 'appreciation-1',
                appreciationOf: 'content-1',
                appreciatedBy: 'agent-1',
                appreciationTo: 'agent-2',
                quantityValue: 3,
                quantityUnit: 'points',
                note: null,
                createdAt: '2026-01-01T00:00:00Z',
              },
            },
          ],
        })
      );

      const result = await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsBy('agent-1').subscribe(resolve);
      });

      expect(result.length).toBe(1);
      expect(result[0].appreciatedBy).toBe('agent-1');
    });
  });

  // ==========================================================================
  // Create Tests - appreciate
  // ==========================================================================

  describe('appreciate', () => {
    const input: CreateAppreciationInput = {
      appreciationOf: 'content-1',
      appreciationTo: 'agent-2',
      quantityValue: 1,
      quantityUnit: 'recognition-points',
      note: 'Thank you!',
    };

    it('should throw error when service not available', () => {
      expect(() => service.appreciate(input)).toThrowError('Appreciation service not available');
    });

    it('should create appreciation when service is available', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: {
            actionHash: new Uint8Array([1, 2, 3]),
            appreciation: {
              id: 'new-appreciation',
              appreciationOf: input.appreciationOf,
              appreciatedBy: 'current-agent',
              appreciationTo: input.appreciationTo,
              quantityValue: input.quantityValue,
              quantityUnit: input.quantityUnit,
              note: input.note,
              createdAt: '2026-01-25T00:00:00Z',
            },
          },
        })
      );

      const result = await new Promise<AppreciationDisplay>(resolve => {
        service.appreciate(input).subscribe(resolve);
      });

      expect(result.id).toBe('new-appreciation');
      expect(result.note).toBe('Thank you!');
      expect(holochainClientMock.callZome).toHaveBeenCalledWith({
        zomeName: 'content_store',
        fnName: 'create_appreciation',
        payload: {
          appreciation_of: input.appreciationOf,
          appreciation_to: input.appreciationTo,
          quantity_value: input.quantityValue,
          quantity_unit: input.quantityUnit,
          note: input.note,
        },
      });
    });

    it('should handle null note', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      const inputWithoutNote: CreateAppreciationInput = {
        appreciationOf: 'content-1',
        appreciationTo: 'agent-2',
        quantityValue: 1,
        quantityUnit: 'points',
      };

      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({
          success: true,
          data: {
            actionHash: new Uint8Array([1]),
            appreciation: {
              id: 'appreciation-1',
              appreciationOf: 'content-1',
              appreciatedBy: 'agent-1',
              appreciationTo: 'agent-2',
              quantityValue: 1,
              quantityUnit: 'points',
              note: null,
              createdAt: '2026-01-25T00:00:00Z',
            },
          },
        })
      );

      await new Promise<AppreciationDisplay>(resolve => {
        service.appreciate(inputWithoutNote).subscribe(resolve);
      });

      expect(holochainClientMock.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          payload: jasmine.objectContaining({
            note: null,
          }),
        })
      );
    });

    it('should throw error when zome call fails', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'Validation failed' })
      );

      await expectAsync(
        new Promise<AppreciationDisplay>((resolve, reject) => {
          service.appreciate(input).subscribe({ next: resolve, error: reject });
        })
      ).toBeRejectedWithError('Validation failed');
    });
  });

  // ==========================================================================
  // Cache Management Tests
  // ==========================================================================

  describe('clearCache', () => {
    it('should clear both caches', async () => {
      // Make service available
      holochainClientMock.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      await service.testAvailability();

      // Populate caches
      await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });
      await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsBy('agent-1').subscribe(resolve);
      });

      // Clear
      service.clearCache();

      // Make new calls - these should create new cache entries
      await new Promise<AppreciationDisplay[]>(resolve => {
        service.getAppreciationsFor('entity-1').subscribe(resolve);
      });

      // Verify the zome was called again for entity-1
      const entityCalls = holochainClientMock.callZome.calls
        .all()
        .filter(
          call =>
            call.args[0].fnName === 'get_appreciations_for' &&
            call.args[0].payload === 'entity-1'
        );
      expect(entityCalls.length).toBe(2); // Before clear + after clear
    });
  });
});
