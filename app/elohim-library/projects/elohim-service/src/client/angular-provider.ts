/**
 * Angular providers for ElohimClient
 *
 * Provides dependency injection integration for Angular applications.
 * Configure the client mode based on your environment.
 */

import { InjectionToken, Provider, FactoryProvider } from '@angular/core';

import { ConfiguredDoorwayResolver } from './doorway-address-resolver';

import { ElohimClient, ReachLevel } from './index';

import type { DoorwayAddressResolver } from './doorway-address-resolver';
import type { ElohimClientConfig, ClientMode } from './index';

/**
 * Injection token for ElohimClient
 */
export const ELOHIM_CLIENT = new InjectionToken<ElohimClient>('ElohimClient');

/**
 * Injection token for client configuration
 */
export const ELOHIM_CLIENT_CONFIG = new InjectionToken<ElohimClientConfig>('ElohimClientConfig');

/** Shared resolver seam used by SDK and Angular HTTP traffic. */
export const DOORWAY_ADDRESS_RESOLVER = new InjectionToken<DoorwayAddressResolver>(
  'DoorwayAddressResolver'
);

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
  const doorway = config.mode.doorway;
  const resolver =
    config.doorwayResolver ??
    (doorway
      ? new ConfiguredDoorwayResolver([
          {
            identity: doorway.identity ?? doorway.url,
            primaryUrl: doorway.url,
            fallbackUrls: doorway.fallbacks,
          },
        ])
      : new ConfiguredDoorwayResolver([]));
  const resolvedConfig = { ...config, doorwayResolver: resolver };

  return [
    { provide: DOORWAY_ADDRESS_RESOLVER, useValue: resolver },
    { provide: ELOHIM_CLIENT_CONFIG, useValue: resolvedConfig },
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
// `window` type comes from src/types/browser-globals.d.ts

/**
 * Helper to detect client mode from environment
 *
 * Environment-agnostic: when the doorway lives at a different origin than the
 * page (a development workspace runtime publishing per-endpoint hostnames), the
 * CALLER resolves that origin and passes it as `doorwayUrl`. See
 * `app/workspace-runtime/` — the library must not know any workspace vendor.
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
  /** Stable doorway identity key; defaults to doorwayUrl during migration. */
  doorwayIdentity?: string;
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
  if (environment.tauri || (globalThis as any).__TAURI__ !== undefined) {
    const tauri = (globalThis as any).__TAURI__;
    return {
      type: 'tauri',
      invoke: tauri?.invoke ?? (async () => Promise.reject(new Error('Tauri not available'))),
      doorway: environment.doorwayUrl
        ? {
            identity: environment.doorwayIdentity,
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

  // Browser mode (doorway-dependent).
  //
  // A caller that wants /db/* proxied THROUGH the doorway simply passes no
  // storageUrl — that is the choice the removed vendor branch used to make on
  // the caller's behalf, and it is the caller's to make.
  return {
    type: 'browser',
    doorway: {
      identity: environment.doorwayIdentity,
      url: environment.doorwayUrl ?? 'http://localhost:8080',
      fallbacks: environment.doorwayFallbacks,
      apiKey: environment.apiKey,
    },
    storageUrl: environment.storageUrl,
  };
}
