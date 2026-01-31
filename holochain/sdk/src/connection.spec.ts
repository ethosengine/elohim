/**
 * Connection Manager Tests
 *
 * Tests for HolochainConnection class covering:
 * - Connection lifecycle
 * - Cell ID resolution
 * - Error handling
 * - Concurrent connection attempts
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { HolochainConnection, createConnection } from './connection.js';
import { AdminWebsocket, AppWebsocket } from '@holochain/client';
import type { ConnectionConfig } from './types.js';

// Mock @holochain/client
vi.mock('@holochain/client', () => ({
  AdminWebsocket: {
    connect: vi.fn(),
  },
  AppWebsocket: {
    connect: vi.fn(),
  },
}));

describe('HolochainConnection', () => {
  let mockAdminWs: any;
  let mockAppWs: any;
  let config: ConnectionConfig;

  beforeEach(() => {
    // Reset mocks
    vi.clearAllMocks();

    // Setup mock AdminWebsocket
    mockAdminWs = {
      listApps: vi.fn(),
      client: { close: vi.fn() },
    };

    // Setup mock AppWebsocket
    mockAppWs = {
      callZome: vi.fn(),
      client: { close: vi.fn() },
    };

    // Default config
    config = {
      adminUrl: 'ws://localhost:8888/admin',
      appUrl: 'ws://localhost:8888/app/4445',
      appId: 'elohim',
      roleId: 'elohim',
    };

    // Setup default successful connection flow
    (AdminWebsocket.connect as any).mockResolvedValue(mockAdminWs);
    (AppWebsocket.connect as any).mockResolvedValue(mockAppWs);

    mockAdminWs.listApps.mockResolvedValue([
      {
        installed_app_id: 'elohim',
        cell_info: {
          elohim: [
            {
              type: 'provisioned',
              value: {
                cell_id: [
                  new Uint8Array([1, 2, 3]), // DNA hash
                  new Uint8Array([4, 5, 6]), // Agent pubkey
                ],
              },
            },
          ],
        },
      },
    ]);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Constructor', () => {
    it('should create instance with provided config', () => {
      const conn = new HolochainConnection(config);
      expect(conn).toBeInstanceOf(HolochainConnection);
    });

    it('should apply default values for missing config', () => {
      const minimalConfig = {
        adminUrl: 'ws://localhost:8888/admin',
      };
      const conn = new HolochainConnection(minimalConfig);
      expect(conn).toBeInstanceOf(HolochainConnection);

      // Should use defaults internally
      const state = conn.getState();
      expect(state.appId).toBe('elohim');
    });

    it('should merge provided config with defaults', () => {
      const customConfig = {
        ...config,
        timeout: 60000,
      };
      const conn = new HolochainConnection(customConfig);
      expect(conn).toBeInstanceOf(HolochainConnection);
    });
  });

  describe('connect()', () => {
    it('should connect successfully with valid config', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();

      expect(AdminWebsocket.connect).toHaveBeenCalledWith({
        url: new URL(config.adminUrl),
      });
      expect(mockAdminWs.listApps).toHaveBeenCalledWith({});
      expect(AppWebsocket.connect).toHaveBeenCalled();
      expect(conn.isConnected()).toBe(true);
    });

    it('should resolve cell ID from app info', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();

      const cellId = conn.getCellId();
      expect(cellId).toBeDefined();
      expect(cellId[0]).toBeInstanceOf(Uint8Array);
      expect(cellId[1]).toBeInstanceOf(Uint8Array);
    });

    it('should throw error if app not installed', async () => {
      mockAdminWs.listApps.mockResolvedValue([
        { installed_app_id: 'other-app', cell_info: {} },
      ]);

      const conn = new HolochainConnection(config);
      await expect(conn.connect()).rejects.toThrow(
        "App 'elohim' not installed"
      );
    });

    it('should throw error if no cells found for role', async () => {
      mockAdminWs.listApps.mockResolvedValue([
        {
          installed_app_id: 'elohim',
          cell_info: {
            elohim: [], // Empty cell array
          },
        },
      ]);

      const conn = new HolochainConnection(config);
      await expect(conn.connect()).rejects.toThrow(
        "No cells found for role 'elohim'"
      );
    });

    it('should throw error if no provisioned cell found', async () => {
      mockAdminWs.listApps.mockResolvedValue([
        {
          installed_app_id: 'elohim',
          cell_info: {
            elohim: [
              {
                type: 'cloned', // Not provisioned
                value: { cell_id: [new Uint8Array(), new Uint8Array()] },
              },
            ],
          },
        },
      ]);

      const conn = new HolochainConnection(config);
      await expect(conn.connect()).rejects.toThrow(
        "No provisioned cell found for role 'elohim'"
      );
    });

    it('should handle legacy cell_info format', async () => {
      mockAdminWs.listApps.mockResolvedValue([
        {
          installed_app_id: 'elohim',
          cell_info: {
            elohim: [
              {
                // Legacy format without 'type'
                provisioned: {
                  cell_id: [new Uint8Array([1, 2, 3]), new Uint8Array([4, 5, 6])],
                },
              },
            ],
          },
        },
      ]);

      const conn = new HolochainConnection(config);
      await conn.connect();
      expect(conn.isConnected()).toBe(true);
    });

    it('should derive app URL from admin URL when not provided', async () => {
      const configNoAppUrl = {
        adminUrl: 'ws://localhost:8888/admin',
        appId: 'elohim',
        roleId: 'elohim',
      };

      const conn = new HolochainConnection(configNoAppUrl);
      await conn.connect();

      expect(AppWebsocket.connect).toHaveBeenCalledWith({
        url: new URL('ws://localhost:8888/app/4445'),
      });
    });

    it('should handle non-proxy admin URL', async () => {
      const directConfig = {
        adminUrl: 'ws://localhost:4444',
        appId: 'elohim',
        roleId: 'elohim',
      };

      const conn = new HolochainConnection(directConfig);
      await conn.connect();

      // Should derive app URL with port 4445
      expect(AppWebsocket.connect).toHaveBeenCalled();
    });

    it('should prevent concurrent connection attempts', async () => {
      const conn = new HolochainConnection(config);

      // Start multiple connections simultaneously
      const promise1 = conn.connect();
      const promise2 = conn.connect();
      const promise3 = conn.connect();

      await Promise.all([promise1, promise2, promise3]);

      // Should only connect once
      expect(AdminWebsocket.connect).toHaveBeenCalledTimes(1);
    });

    it('should allow reconnection after disconnect', async () => {
      const conn = new HolochainConnection(config);

      await conn.connect();
      expect(conn.isConnected()).toBe(true);

      await conn.disconnect();
      expect(conn.isConnected()).toBe(false);

      await conn.connect();
      expect(conn.isConnected()).toBe(true);
      expect(AdminWebsocket.connect).toHaveBeenCalledTimes(2);
    });
  });

  describe('disconnect()', () => {
    it('should close both websockets', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      await conn.disconnect();

      expect(mockAppWs.client.close).toHaveBeenCalled();
      expect(mockAdminWs.client.close).toHaveBeenCalled();
      expect(conn.isConnected()).toBe(false);
    });

    it('should handle disconnect when not connected', async () => {
      const conn = new HolochainConnection(config);
      await expect(conn.disconnect()).resolves.not.toThrow();
    });

    it('should clear cell ID on disconnect', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      expect(conn.getCellId()).toBeDefined();

      await conn.disconnect();
      expect(() => conn.getCellId()).toThrow('Not connected');
    });
  });

  describe('getState()', () => {
    it('should return disconnected state initially', () => {
      const conn = new HolochainConnection(config);
      const state = conn.getState();

      expect(state.isConnected).toBe(false);
      expect(state.adminUrl).toBe(config.adminUrl);
      expect(state.cellId).toBeNull();
    });

    it('should return connected state after connection', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();

      const state = conn.getState();
      expect(state.isConnected).toBe(true);
      expect(state.adminUrl).toBe(config.adminUrl);
      expect(state.appId).toBe(config.appId);
      expect(state.cellId).not.toBeNull();
    });
  });

  describe('isConnected()', () => {
    it('should return false when not connected', () => {
      const conn = new HolochainConnection(config);
      expect(conn.isConnected()).toBe(false);
    });

    it('should return true when connected', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      expect(conn.isConnected()).toBe(true);
    });

    it('should return false after disconnect', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      await conn.disconnect();
      expect(conn.isConnected()).toBe(false);
    });
  });

  describe('getAdminWs()', () => {
    it('should return AdminWebsocket when connected', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      const ws = conn.getAdminWs();
      expect(ws).toBe(mockAdminWs);
    });

    it('should throw error when not connected', () => {
      const conn = new HolochainConnection(config);
      expect(() => conn.getAdminWs()).toThrow('Not connected');
    });
  });

  describe('getAppWs()', () => {
    it('should return AppWebsocket when connected', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      const ws = conn.getAppWs();
      expect(ws).toBe(mockAppWs);
    });

    it('should throw error when not connected', () => {
      const conn = new HolochainConnection(config);
      expect(() => conn.getAppWs()).toThrow('Not connected');
    });
  });

  describe('getCellId()', () => {
    it('should return cell ID when connected', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();
      const cellId = conn.getCellId();
      expect(cellId).toBeDefined();
      expect(cellId).toHaveLength(2);
    });

    it('should throw error when not connected', () => {
      const conn = new HolochainConnection(config);
      expect(() => conn.getCellId()).toThrow('Not connected');
    });
  });

  describe('callZome()', () => {
    it('should call zome function successfully', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();

      const mockResponse = { result: 'success' };
      mockAppWs.callZome.mockResolvedValue(mockResponse);

      const result = await conn.callZome('content_store', 'get_content_stats', null);

      expect(mockAppWs.callZome).toHaveBeenCalledWith({
        cell_id: expect.any(Array),
        zome_name: 'content_store',
        fn_name: 'get_content_stats',
        payload: null,
      });
      expect(result).toEqual(mockResponse);
    });

    it('should throw error when not connected', async () => {
      const conn = new HolochainConnection(config);
      await expect(
        conn.callZome('content_store', 'get_content_stats', null)
      ).rejects.toThrow('Not connected');
    });

    it('should pass payload to zome call', async () => {
      const conn = new HolochainConnection(config);
      await conn.connect();

      const payload = { id: 'test-content' };
      await conn.callZome('content_store', 'get_content_by_id', payload);

      expect(mockAppWs.callZome).toHaveBeenCalledWith(
        expect.objectContaining({ payload })
      );
    });
  });

  describe('createConnection()', () => {
    it('should create connection instance', () => {
      const conn = createConnection(config);
      expect(conn).toBeInstanceOf(HolochainConnection);
    });
  });
});
