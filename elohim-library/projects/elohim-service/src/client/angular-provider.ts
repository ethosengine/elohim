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

// Browser window reference (safely handles SSR/Node environments)
declare const window: Window & typeof globalThis | undefined;

/**
 * Detect if running in Eclipse Che environment
 */
function isEclipseChe(): boolean {
  if (typeof window === 'undefined') return false;
  const hostname = window.location.hostname;
  return hostname.includes('.code.ethosengine.com') || hostname.includes('.devspaces.');
}

/**
 * Get the Che hc-dev endpoint URL for doorway access
 *
 * In Eclipse Che, the browser runs on the user's machine but services
 * run in the remote workspace. The dev-proxy routes requests through
 * Che's infrastructure.
 *
 * Example:
 * - Angular: mbd06b-gmail-com-elohim-devspace-angular-dev.code.ethosengine.com
 * - hc-dev:  mbd06b-gmail-com-elohim-devspace-hc-dev.code.ethosengine.com
 */
function getCheHcDevUrl(): string {
  if (typeof window === 'undefined') return '';
  const hostname = window.location.hostname.replace(/-angular-dev\./, '-hc-dev.');
  return `https://${hostname}`;
}

/**
 * Get the Che hc-storage endpoint URL for direct storage access
 *
 * Example:
 * - Angular:     mbd06b-gmail-com-elohim-devspace-angular-dev.code.ethosengine.com
 * - hc-storage:  mbd06b-gmail-com-elohim-devspace-hc-storage.code.ethosengine.com
 */
function getCheStorageUrl(): string {
  if (typeof window === 'undefined') return '';
  const hostname = window.location.hostname.replace(/-angular-dev\./, '-hc-storage.');
  return `https://${hostname}`;
}

/**
 * Helper to detect client mode from environment
 *
 * Automatically detects Eclipse Che and routes through the hc-dev endpoint.
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
  /** Direct storage URL for /db/* routes in browser mode (bypasses doorway) */
  storageUrl?: string;
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

  // Eclipse Che: Use hc-dev endpoint for doorway access
  // The browser runs on user's machine, services run in remote workspace
  // Doorway proxies /db/* to storage internally, so we don't need a separate storageUrl
  if (isEclipseChe()) {
    const cheUrl = getCheHcDevUrl();
    console.log('[ElohimClient] Detected Eclipse Che, using hc-dev endpoint:', cheUrl);
    console.log('[ElohimClient] Doorway will proxy /db/* routes to storage internally');
    return {
      type: 'browser',
      doorway: {
        url: cheUrl,
        fallbacks: environment.doorwayFallbacks,
        apiKey: environment.apiKey,
      },
      // Don't set storageUrl - doorway proxies /db/* to storage
      // The hc-storage endpoint has SSL issues that may block browser requests
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
    storageUrl: environment.storageUrl,
  };
}
