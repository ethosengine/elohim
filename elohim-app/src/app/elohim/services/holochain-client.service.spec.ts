import { TestBed } from '@angular/core/testing';
import { HolochainClientService } from './holochain-client.service';

/**
 * Unit tests for HolochainClientService
 *
 * Note: These tests mock the @holochain/client WebSocket connections
 * since actual conductor connectivity requires a running Edge Node.
 */
describe('HolochainClientService', () => {
  let service: HolochainClientService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [HolochainClientService],
    });
    service = TestBed.inject(HolochainClientService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('initial state', () => {
    it('should start in disconnected state', () => {
      expect(service.state()).toBe('disconnected');
    });

    it('should not be connected initially', () => {
      expect(service.isConnected()).toBeFalse();
    });

    it('should have no error initially', () => {
      expect(service.error()).toBeUndefined();
    });

    it('should expose connection signal', () => {
      const connection = service.connection();
      expect(connection.state).toBe('disconnected');
      expect(connection.adminWs).toBeNull();
      expect(connection.appWs).toBeNull();
      expect(connection.agentPubKey).toBeNull();
      expect(connection.cellId).toBeNull();
    });
  });

  describe('callZome', () => {
    it('should return error when not connected', async () => {
      const result = await service.callZome({
        zomeName: 'content_store',
        fnName: 'get_content',
        payload: { id: 'test' },
      });

      expect(result.success).toBeFalse();
      expect(result.error).toContain('Not connected');
    });
  });

  describe('disconnect', () => {
    it('should reset state to initial on disconnect', async () => {
      await service.disconnect();

      expect(service.state()).toBe('disconnected');
      expect(service.isConnected()).toBeFalse();
    });
  });
});
