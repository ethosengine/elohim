/**
 * Angular providers for ElohimClient
 *
 * Provides dependency injection integration for Angular applications.
 * Configure the client mode based on your environment.
 */

import { InjectionToken, Provider, FactoryProvider } from '@angular/core';
import { ElohimClient, ElohimClientConfig, ClientMode, ReachLevel } from './index';

/**
 * Injection token for ElohimClient
 */
export const ELOHIM_CLIENT = new InjectionToken<ElohimClient>('ElohimClient');

/**
 * Injection token for client configuration
 */
export const ELOHIM_CLIENT_CONFIG = new InjectionToken<ElohimClientConfig>('ElohimClientConfig');

/**
 * Factory function for creating ElohimClient
 */
export function elohimClientFactory(config: ElohimClientConfig): ElohimClient {
  return new ElohimClient(config);
}

/**
 * Provider for ElohimClient with configuration
 *
 * @example
 * ```typescript
 * // In your app.module.ts or app.config.ts
 * import { provideElohimClient } from '@elohim/service/client/angular-provider';
 * import { environment } from './environments/environment';
 *
 * // Browser mode (doorway-dependent)
 * providers: [
 *   provideElohimClient({
 *     mode: {
 *       type: 'browser',
 *       doorwayUrl: environment.doorwayUrl,
 *       apiKey: environment.apiKey,
 *     },
 *   }),
 * ]
 *
 * // Or configure dynamically
 * providers: [
 *   provideElohimClient(getClientConfig(environment)),
 * ]
 * ```
 */
export function provideElohimClient(config: ElohimClientConfig): Provider[] {
  return [
    { provide: ELOHIM_CLIENT_CONFIG, useValue: config },
    {
      provide: ELOHIM_CLIENT,
      useFactory: elohimClientFactory,
      deps: [ELOHIM_CLIENT_CONFIG],
    } as FactoryProvider,
  ];
}

/**
 * Provider for anonymous browser client
 *
 * @example
 * ```typescript
 * providers: [
 *   provideAnonymousBrowserClient('https://doorway.example.com'),
 * ]
 * ```
 */
export function provideAnonymousBrowserClient(doorwayUrl: string): Provider[] {
  return provideElohimClient({
    mode: { type: 'browser', doorway: { url: doorwayUrl } },
    agentReach: ReachLevel.Commons,
  });
}

/**
 * Helper to detect client mode from environment
 *
 * @example
 * ```typescript
 * import { detectClientMode, provideElohimClient } from '@elohim/service/client/angular-provider';
 *
 * providers: [
 *   provideElohimClient({
 *     mode: detectClientMode(environment),
 *   }),
 * ]
 * ```
 */
export function detectClientMode(environment: {
  /** Primary doorway URL */
  doorwayUrl?: string;
  /** Fallback doorway URLs */
  doorwayFallbacks?: string[];
  /** API key for doorway */
  apiKey?: string;
  /** Force Tauri mode detection */
  tauri?: boolean;
  /** Personal elohim-node URLs (for Tauri sync) */
  nodeUrls?: string[];
}): ClientMode {
  // Tauri mode (detected via window.__TAURI__)
  if (environment.tauri || typeof (globalThis as any).__TAURI__ !== 'undefined') {
    const tauri = (globalThis as any).__TAURI__;
    return {
      type: 'tauri',
      invoke: tauri?.invoke ?? (() => Promise.reject(new Error('Tauri not available'))),
      doorway: environment.doorwayUrl
        ? {
            url: environment.doorwayUrl,
            fallbacks: environment.doorwayFallbacks,
            apiKey: environment.apiKey,
          }
        : undefined,
      nodes: environment.nodeUrls?.length
        ? { urls: environment.nodeUrls, preferOverDoorway: true }
        : undefined,
    };
  }

  // Default: Browser mode (doorway-dependent)
  return {
    type: 'browser',
    doorway: {
      url: environment.doorwayUrl ?? 'http://localhost:8080',
      fallbacks: environment.doorwayFallbacks,
      apiKey: environment.apiKey,
    },
  };
}
