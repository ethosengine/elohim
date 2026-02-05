/**
 * Connection Strategy Provider Tests
 *
 * Tests the Angular DI provider factory for connection strategies.
 * Validates that the correct strategy is created based on environment configuration
 * and that the provider functions work correctly in different deployment modes.
 *
 * Coverage targets:
 * - InjectionToken factory with environment mode detection
 * - Provider factory functions (provideConnectionStrategy, etc.)
 * - SSR (server-side rendering) mode detection
 * - Strategy selection based on ConnectionMode
 */

import { TestBed } from '@angular/core/testing';
import { PLATFORM_ID } from '@angular/core';

import {
  CONNECTION_STRATEGY,
  provideConnectionStrategy,
  provideDoorwayStrategy,
  provideDirectStrategy,
  provideConnectionStrategySSR,
} from './connection-strategy.provider';

import {
  type IConnectionStrategy,
  DoorwayConnectionStrategy,
  DirectConnectionStrategy,
  SourceTier,
} from '@elohim/service/connection';

import { environment } from '../../../environments/environment';

describe('ConnectionStrategyProvider', () => {
  describe('CONNECTION_STRATEGY token', () => {
    it('should create strategy using environment config', () => {
      TestBed.configureTestingModule({});
      const strategy = TestBed.inject(CONNECTION_STRATEGY);

      expect(strategy).toBeDefined();
      expect(strategy.connect).toBeDefined();
      expect(strategy.name).toBeDefined();
      expect(strategy.mode).toBeDefined();
    });

    it('should use auto mode from environment by default', () => {
      // environment.holochain.connectionMode is 'auto' by default
      TestBed.configureTestingModule({});
      const strategy = TestBed.inject(CONNECTION_STRATEGY);

      // Auto mode should detect environment and create appropriate strategy
      // In test environment (Node.js), it should create DirectConnectionStrategy
      expect(strategy).toBeDefined();
    });

    it('should handle missing holochain config gracefully', () => {
      // Mock environment without holochain config
      const originalHolochain = environment.holochain;
      (environment as any).holochain = undefined;

      try {
        TestBed.configureTestingModule({});
        const strategy = TestBed.inject(CONNECTION_STRATEGY);

        // Should default to 'auto' mode when config is missing
        expect(strategy).toBeDefined();
      } finally {
        // Restore original config
        (environment as any).holochain = originalHolochain;
      }
    });
  });

  describe('provideConnectionStrategy', () => {
    it('should create strategy with specified mode', () => {
      TestBed.configureTestingModule({
        providers: [provideConnectionStrategy('doorway')],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeDefined();
      expect(strategy.mode).toBe('doorway');
    });

    it('should default to auto mode when no mode specified', () => {
      TestBed.configureTestingModule({
        providers: [provideConnectionStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeDefined();
      // Auto mode should detect environment
      expect(strategy.mode).toBeDefined();
    });

    it('should create direct mode strategy when specified', () => {
      TestBed.configureTestingModule({
        providers: [provideConnectionStrategy('direct')],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeDefined();
      expect(strategy.mode).toBe('direct');
    });

    it('should override default token factory', () => {
      // First get default strategy
      TestBed.configureTestingModule({});
      const defaultStrategy = TestBed.inject(CONNECTION_STRATEGY);

      // Reset TestBed and provide explicit strategy
      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideConnectionStrategy('doorway')],
      });
      const explicitStrategy = TestBed.inject(CONNECTION_STRATEGY);

      // Both should be defined but may differ based on environment
      expect(defaultStrategy).toBeDefined();
      expect(explicitStrategy).toBeDefined();
      expect(explicitStrategy.mode).toBe('doorway');
    });
  });

  describe('provideDoorwayStrategy', () => {
    it('should create DoorwayConnectionStrategy instance', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeInstanceOf(DoorwayConnectionStrategy);
      expect(strategy.name).toBe('doorway');
      expect(strategy.mode).toBe('doorway');
    });

    it('should provide strategy with correct interface', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(typeof strategy.connect).toBe('function');
      expect(typeof strategy.disconnect).toBe('function');
      expect(typeof strategy.isSupported).toBe('function');
      expect(typeof strategy.isConnected).toBe('function');
      expect(typeof strategy.getContentSources).toBe('function');
      expect(typeof strategy.getStorageBaseUrl).toBe('function');
      expect(typeof strategy.getBlobStorageUrl).toBe('function');
      expect(typeof strategy.resolveAdminUrl).toBe('function');
      expect(typeof strategy.resolveAppUrl).toBe('function');
    });
  });

  describe('provideDirectStrategy', () => {
    it('should create DirectConnectionStrategy instance', () => {
      TestBed.configureTestingModule({
        providers: [provideDirectStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeInstanceOf(DirectConnectionStrategy);
      expect(strategy.name).toBe('direct');
      expect(strategy.mode).toBe('direct');
    });

    it('should provide strategy with correct interface', () => {
      TestBed.configureTestingModule({
        providers: [provideDirectStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(typeof strategy.connect).toBe('function');
      expect(typeof strategy.disconnect).toBe('function');
      expect(typeof strategy.isSupported).toBe('function');
      expect(typeof strategy.isConnected).toBe('function');
      expect(typeof strategy.getContentSources).toBe('function');
      expect(typeof strategy.getStorageBaseUrl).toBe('function');
      expect(typeof strategy.getBlobStorageUrl).toBe('function');
      expect(typeof strategy.resolveAdminUrl).toBe('function');
      expect(typeof strategy.resolveAppUrl).toBe('function');
    });
  });

  describe('provideConnectionStrategySSR', () => {
    it('should use doorway strategy on server platform', () => {
      TestBed.configureTestingModule({
        providers: [
          provideConnectionStrategySSR(),
          { provide: PLATFORM_ID, useValue: 'server' },
        ],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeInstanceOf(DoorwayConnectionStrategy);
      expect(strategy.mode).toBe('doorway');
    });

    it('should use environment config on browser platform', () => {
      TestBed.configureTestingModule({
        providers: [
          provideConnectionStrategySSR(),
          { provide: PLATFORM_ID, useValue: 'browser' },
        ],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy).toBeDefined();
      // Should use environment.holochain.connectionMode (which is 'auto')
    });

    it('should handle missing holochain config in browser mode', () => {
      const originalHolochain = environment.holochain;
      (environment as any).holochain = undefined;

      try {
        TestBed.configureTestingModule({
          providers: [
            provideConnectionStrategySSR(),
            { provide: PLATFORM_ID, useValue: 'browser' },
          ],
        });

        const strategy = TestBed.inject(CONNECTION_STRATEGY);
        expect(strategy).toBeDefined();
        // Should default to 'auto' mode
      } finally {
        (environment as any).holochain = originalHolochain;
      }
    });

    it('should always use doorway on server regardless of environment config', () => {
      // Even if environment says 'direct', SSR should force doorway
      const originalMode = environment.holochain?.connectionMode;
      if (environment.holochain) {
        environment.holochain.connectionMode = 'direct';
      }

      try {
        TestBed.configureTestingModule({
          providers: [
            provideConnectionStrategySSR(),
            { provide: PLATFORM_ID, useValue: 'server' },
          ],
        });

        const strategy = TestBed.inject(CONNECTION_STRATEGY);
        expect(strategy).toBeInstanceOf(DoorwayConnectionStrategy);
        expect(strategy.mode).toBe('doorway');
      } finally {
        if (environment.holochain && originalMode) {
          environment.holochain.connectionMode = originalMode;
        }
      }
    });
  });

  describe('Strategy Interface Compliance', () => {
    it('should provide strategy with all required methods', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);

      // Check all IConnectionStrategy interface methods
      const requiredMethods = [
        'connect',
        'disconnect',
        'isSupported',
        'isConnected',
        'getContentSources',
        'getStorageBaseUrl',
        'getBlobStorageUrl',
        'resolveAdminUrl',
        'resolveAppUrl',
        'getSigningCredentials',
      ];

      requiredMethods.forEach((method) => {
        expect(typeof (strategy as any)[method]).toBe('function');
      });
    });

    it('should provide strategy with all required properties', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);

      // Check all required properties
      expect(strategy.name).toBeDefined();
      expect(strategy.mode).toBeDefined();
      expect(typeof strategy.name).toBe('string');
      expect(typeof strategy.mode).toBe('string');
    });
  });

  describe('Provider Singleton Behavior', () => {
    it('should provide singleton instance by default', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy1 = TestBed.inject(CONNECTION_STRATEGY);
      const strategy2 = TestBed.inject(CONNECTION_STRATEGY);

      // Should be the same instance
      expect(strategy1).toBe(strategy2);
    });

    it('should create new instances across TestBed resets', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });
      const strategy1 = TestBed.inject(CONNECTION_STRATEGY);

      TestBed.resetTestingModule();
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });
      const strategy2 = TestBed.inject(CONNECTION_STRATEGY);

      // Should be different instances after reset
      expect(strategy1).not.toBe(strategy2);
    });
  });

  describe('Edge Cases', () => {
    it('should handle multiple provider configurations', () => {
      // If multiple providers are configured, last one wins
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy(), provideDirectStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      // Last provider (direct) should be used
      expect(strategy).toBeInstanceOf(DirectConnectionStrategy);
    });

    it('should work with empty environment config', () => {
      const originalEnv = { ...environment };
      (environment as any).holochain = null;

      try {
        TestBed.configureTestingModule({
          providers: [provideConnectionStrategy()],
        });

        const strategy = TestBed.inject(CONNECTION_STRATEGY);
        expect(strategy).toBeDefined();
      } finally {
        Object.assign(environment, originalEnv);
      }
    });
  });

  describe('Strategy Method Behavior', () => {
    it('should return false for isConnected on new instance', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy.isConnected()).toBe(false);
    });

    it('should return true for isSupported in doorway mode', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      // Doorway is always supported (browser-based)
      expect(strategy.isSupported()).toBe(true);
    });

    it('should return null for signing credentials when not connected', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      expect(strategy.getSigningCredentials()).toBeNull();
    });

    it('should return content sources configuration', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'doorway' as const,
        adminUrl: 'wss://doorway-dev.elohim.host',
        appUrl: 'wss://doorway-dev.elohim.host',
        appId: 'elohim',
      };

      const sources = strategy.getContentSources(mockConfig);
      expect(Array.isArray(sources)).toBe(true);
      expect(sources.length).toBeGreaterThan(0);

      // Each source should have required properties
      sources.forEach((source) => {
        expect(source.id).toBeDefined();
        expect(source.tier).toBeDefined();
        expect(source.priority).toBeDefined();
        expect(Array.isArray(source.contentTypes)).toBe(true);
      });
    });

    it('should generate storage base URL', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'doorway' as const,
        adminUrl: 'wss://doorway-dev.elohim.host',
        appUrl: 'wss://doorway-dev.elohim.host',
        appId: 'elohim',
      };

      const baseUrl = strategy.getStorageBaseUrl(mockConfig);
      expect(typeof baseUrl).toBe('string');
      expect(baseUrl.length).toBeGreaterThan(0);
    });

    it('should generate blob storage URL with hash', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'doorway' as const,
        adminUrl: 'wss://doorway-dev.elohim.host',
        appUrl: 'wss://doorway-dev.elohim.host',
        appId: 'elohim',
      };

      const blobUrl = strategy.getBlobStorageUrl(mockConfig, 'sha256-abc123');
      expect(typeof blobUrl).toBe('string');
      expect(blobUrl).toContain('sha256-abc123');
    });

    it('should resolve admin URL from config', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'doorway' as const,
        adminUrl: 'wss://doorway-dev.elohim.host',
        appUrl: 'wss://doorway-dev.elohim.host',
        appId: 'elohim',
        proxyApiKey: 'test-key',
      };

      const adminUrl = strategy.resolveAdminUrl(mockConfig);
      expect(typeof adminUrl).toBe('string');
      expect(adminUrl.length).toBeGreaterThan(0);
    });

    it('should resolve app URL with port', () => {
      TestBed.configureTestingModule({
        providers: [provideDoorwayStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'doorway' as const,
        adminUrl: 'wss://doorway-dev.elohim.host',
        appUrl: 'wss://doorway-dev.elohim.host',
        appId: 'elohim',
      };

      const appUrl = strategy.resolveAppUrl(mockConfig, 8888);
      expect(typeof appUrl).toBe('string');
      expect(appUrl.length).toBeGreaterThan(0);
    });
  });

  describe('Direct Strategy Specifics', () => {
    it('should use different content sources for direct mode', () => {
      TestBed.configureTestingModule({
        providers: [provideDirectStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'direct' as const,
        adminUrl: 'ws://localhost:4444',
        appUrl: 'ws://localhost:4445',
        appId: 'elohim',
        storageUrl: 'http://localhost:8090',
      };

      const sources = strategy.getContentSources(mockConfig);
      expect(Array.isArray(sources)).toBe(true);

      // Direct mode should skip Projection tier
      const projectionSources = sources.filter((s) => s.tier === SourceTier.Projection);
      expect(projectionSources.length).toBe(0);
    });

    it('should use storage URL from config for direct mode', () => {
      TestBed.configureTestingModule({
        providers: [provideDirectStrategy()],
      });

      const strategy = TestBed.inject(CONNECTION_STRATEGY);
      const mockConfig = {
        mode: 'direct' as const,
        adminUrl: 'ws://localhost:4444',
        appUrl: 'ws://localhost:4445',
        appId: 'elohim',
        storageUrl: 'http://localhost:8090',
      };

      const baseUrl = strategy.getStorageBaseUrl(mockConfig);
      expect(baseUrl).toContain('localhost');
    });
  });
});
