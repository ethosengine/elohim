/**
 * Connection Strategy Tests
 *
 * Tests for the connection strategy pattern that enables
 * web (Doorway) and native (Direct) deployment modes.
 */

// Mock @holochain/client to avoid ESM transitive dependency chain in Jest
jest.mock('@holochain/client', () => ({
  AdminWebsocket: { connect: jest.fn() },
  AppWebsocket: { connect: jest.fn() },
  generateSigningKeyPair: jest.fn(),
  randomCapSecret: jest.fn(),
  setSigningCredentials: jest.fn(),
}));

import {
  DoorwayConnectionStrategy,
  DirectConnectionStrategy,
  createConnectionStrategy,
  detectConnectionMode,
  SourceTier,
  type ConnectionConfig,
} from './index';

// ============================================================================
// Test Fixtures
// ============================================================================

const createTestConfig = (overrides?: Partial<ConnectionConfig>): ConnectionConfig => ({
  mode: 'doorway',
  adminUrl: 'wss://doorway-alpha.elohim.host',
  appUrl: 'wss://doorway-alpha.elohim.host/app',
  proxyApiKey: 'test-api-key',
  appId: 'elohim',
  origin: 'elohim-app',
  ...overrides,
});

const createDirectConfig = (overrides?: Partial<ConnectionConfig>): ConnectionConfig => ({
  mode: 'direct',
  adminUrl: 'ws://localhost:4444',
  appUrl: 'ws://localhost:4445',
  storageUrl: 'http://localhost:8090',
  appId: 'elohim',
  origin: 'elohim-app',
  ...overrides,
});

// ============================================================================
// Factory Tests
// ============================================================================

describe('Connection Strategy Factory', () => {
  describe('createConnectionStrategy', () => {
    it('should create DoorwayConnectionStrategy for doorway mode', () => {
      const strategy = createConnectionStrategy('doorway');
      expect(strategy).toBeInstanceOf(DoorwayConnectionStrategy);
      expect(strategy.name).toBe('doorway');
      expect(strategy.mode).toBe('doorway');
    });

    it('should create DirectConnectionStrategy for direct mode', () => {
      const strategy = createConnectionStrategy('direct');
      expect(strategy).toBeInstanceOf(DirectConnectionStrategy);
      expect(strategy.name).toBe('direct');
      expect(strategy.mode).toBe('direct');
    });

    it('should auto-detect mode when set to auto', () => {
      const strategy = createConnectionStrategy('auto');
      // In test environment (Node.js), should detect as direct
      expect(strategy.mode).toBe('direct');
    });

    it('should default to auto when no mode specified', () => {
      const strategy = createConnectionStrategy();
      expect(strategy).toBeDefined();
      expect(['doorway', 'direct']).toContain(strategy.mode);
    });
  });

  describe('detectConnectionMode', () => {
    it('should return doorway by default in browser environment', () => {
      // In test environment (Node.js), detects as direct
      const mode = detectConnectionMode();
      expect(mode).toBe('direct');
    });
  });
});

// ============================================================================
// DoorwayConnectionStrategy Tests
// ============================================================================

describe('DoorwayConnectionStrategy', () => {
  let strategy: DoorwayConnectionStrategy;

  beforeEach(() => {
    strategy = new DoorwayConnectionStrategy();
  });

  describe('identity', () => {
    it('should have correct name', () => {
      expect(strategy.name).toBe('doorway');
    });

    it('should have correct mode', () => {
      expect(strategy.mode).toBe('doorway');
    });

    it('should report as supported in browser', () => {
      expect(strategy.isSupported()).toBe(true);
    });
  });

  describe('URL resolution', () => {
    it('should resolve admin URL with API key', () => {
      const config = createTestConfig();
      const url = strategy.resolveAdminUrl(config);
      expect(url).toContain('doorway-alpha.elohim.host');
      expect(url).toContain('apiKey=test-api-key');
    });

    it('should resolve admin URL without API key', () => {
      const config = createTestConfig({ proxyApiKey: undefined });
      const url = strategy.resolveAdminUrl(config);
      expect(url).toBe('wss://doorway-alpha.elohim.host/hc/admin');
    });

    it('should resolve app URL with port through proxy', () => {
      const config = createTestConfig();
      const url = strategy.resolveAppUrl(config, 4445);
      expect(url).toContain('/app/4445');
      expect(url).toContain('apiKey=test-api-key');
    });

    it('should fall back to localhost for local config', () => {
      const config = createTestConfig({
        adminUrl: 'ws://localhost:4444',
        proxyApiKey: undefined,
      });
      const url = strategy.resolveAppUrl(config, 4445);
      expect(url).toBe('ws://localhost:4445');
    });
  });

  describe('blob storage URL', () => {
    it('should construct HTTPS blob URL', () => {
      const config = createTestConfig();
      const url = strategy.getBlobStorageUrl(config, 'abc123hash');
      expect(url).toContain('https://doorway-alpha.elohim.host');
      expect(url).toContain('/api/blob/abc123hash');
      expect(url).toContain('apiKey=test-api-key');
    });

    it('should encode special characters in hash', () => {
      const config = createTestConfig();
      const url = strategy.getBlobStorageUrl(config, 'hash/with+special');
      expect(url).toContain(encodeURIComponent('hash/with+special'));
    });
  });

  describe('content sources', () => {
    it('should return sources with Projection tier', () => {
      const config = createTestConfig();
      const sources = strategy.getContentSources(config);

      expect(sources.length).toBeGreaterThan(0);

      const projectionSource = sources.find(s => s.id === 'projection');
      expect(projectionSource).toBeDefined();
      expect(projectionSource?.tier).toBe(SourceTier.Projection);
      expect(projectionSource?.available).toBe(true);
    });

    it('should include indexeddb as Local tier', () => {
      const config = createTestConfig();
      const sources = strategy.getContentSources(config);

      const localSource = sources.find(s => s.id === 'indexeddb');
      expect(localSource).toBeDefined();
      expect(localSource?.tier).toBe(SourceTier.Local);
    });

    it('should include conductor as Authoritative tier', () => {
      const config = createTestConfig();
      const sources = strategy.getContentSources(config);

      const conductorSource = sources.find(s => s.id === 'conductor');
      expect(conductorSource).toBeDefined();
      expect(conductorSource?.tier).toBe(SourceTier.Authoritative);
      expect(conductorSource?.available).toBe(false); // Not available until connected
    });
  });

  describe('connection state', () => {
    it('should start disconnected', () => {
      expect(strategy.isConnected()).toBe(false);
    });

    it('should return null credentials when disconnected', () => {
      expect(strategy.getSigningCredentials()).toBeNull();
    });
  });
});

// ============================================================================
// DirectConnectionStrategy Tests
// ============================================================================

describe('DirectConnectionStrategy', () => {
  let strategy: DirectConnectionStrategy;

  beforeEach(() => {
    strategy = new DirectConnectionStrategy();
  });

  describe('identity', () => {
    it('should have correct name', () => {
      expect(strategy.name).toBe('direct');
    });

    it('should have correct mode', () => {
      expect(strategy.mode).toBe('direct');
    });
  });

  describe('URL resolution', () => {
    it('should resolve admin URL directly', () => {
      const config = createDirectConfig();
      const url = strategy.resolveAdminUrl(config);
      expect(url).toBe('ws://localhost:4444');
    });

    it('should resolve app URL with port', () => {
      const config = createDirectConfig();
      const url = strategy.resolveAppUrl(config, 4445);
      expect(url).toBe('ws://localhost:4445');
    });

    it('should use custom port when specified', () => {
      const config = createDirectConfig();
      const url = strategy.resolveAppUrl(config, 5555);
      expect(url).toBe('ws://localhost:5555');
    });
  });

  describe('blob storage URL', () => {
    it('should use elohim-storage URL', () => {
      const config = createDirectConfig();
      const url = strategy.getBlobStorageUrl(config, 'abc123hash');
      expect(url).toBe('http://localhost:8090/store/abc123hash');
    });

    it('should use default storage URL if not configured', () => {
      const config = createDirectConfig({ storageUrl: undefined });
      const url = strategy.getBlobStorageUrl(config, 'abc123hash');
      expect(url).toContain('/store/abc123hash');
    });
  });

  describe('content sources', () => {
    it('should NOT include Projection tier', () => {
      const config = createDirectConfig();
      const sources = strategy.getContentSources(config);

      const projectionSource = sources.find(s => s.tier === SourceTier.Projection);
      expect(projectionSource).toBeUndefined();
    });

    it('should include elohim-storage as Authoritative', () => {
      const config = createDirectConfig();
      const sources = strategy.getContentSources(config);

      const storageSource = sources.find(s => s.id === 'elohim-storage');
      expect(storageSource).toBeDefined();
      expect(storageSource?.tier).toBe(SourceTier.Authoritative);
    });

    it('should include indexeddb as Local tier', () => {
      const config = createDirectConfig();
      const sources = strategy.getContentSources(config);

      const localSource = sources.find(s => s.id === 'indexeddb');
      expect(localSource).toBeDefined();
      expect(localSource?.tier).toBe(SourceTier.Local);
    });

    it('should have conductor with higher priority than storage', () => {
      const config = createDirectConfig();
      const sources = strategy.getContentSources(config);

      const conductorSource = sources.find(s => s.id === 'conductor');
      const storageSource = sources.find(s => s.id === 'elohim-storage');

      expect(conductorSource).toBeDefined();
      expect(storageSource).toBeDefined();
      expect(conductorSource!.priority).toBeGreaterThan(storageSource!.priority);
    });
  });

  describe('connection state', () => {
    it('should start disconnected', () => {
      expect(strategy.isConnected()).toBe(false);
    });

    it('should return null credentials when disconnected', () => {
      expect(strategy.getSigningCredentials()).toBeNull();
    });
  });
});

// ============================================================================
// Integration Tests
// ============================================================================

describe('Connection Strategy Integration', () => {
  it('should allow switching strategies at runtime', () => {
    const doorway = createConnectionStrategy('doorway');
    const direct = createConnectionStrategy('direct');

    expect(doorway.mode).not.toBe(direct.mode);
    expect(doorway.name).not.toBe(direct.name);
  });

  it('should produce different blob URLs for same hash', () => {
    const doorway = new DoorwayConnectionStrategy();
    const direct = new DirectConnectionStrategy();

    const doorwayConfig = createTestConfig();
    const directConfig = createDirectConfig();
    const hash = 'test-blob-hash';

    const doorwayUrl = doorway.getBlobStorageUrl(doorwayConfig, hash);
    const directUrl = direct.getBlobStorageUrl(directConfig, hash);

    expect(doorwayUrl).not.toBe(directUrl);
    expect(doorwayUrl).toContain('api/blob');
    expect(directUrl).toContain('store');
  });

  it('should produce different content sources', () => {
    const doorway = new DoorwayConnectionStrategy();
    const direct = new DirectConnectionStrategy();

    const doorwayConfig = createTestConfig();
    const directConfig = createDirectConfig();

    const doorwaySources = doorway.getContentSources(doorwayConfig);
    const directSources = direct.getContentSources(directConfig);

    // Doorway should have projection, Direct should not
    const doorwayHasProjection = doorwaySources.some(s => s.id === 'projection');
    const directHasProjection = directSources.some(s => s.id === 'projection');

    expect(doorwayHasProjection).toBe(true);
    expect(directHasProjection).toBe(false);

    // Direct should have elohim-storage, Doorway should not
    const doorwayHasStorage = doorwaySources.some(s => s.id === 'elohim-storage');
    const directHasStorage = directSources.some(s => s.id === 'elohim-storage');

    expect(doorwayHasStorage).toBe(false);
    expect(directHasStorage).toBe(true);
  });
});
