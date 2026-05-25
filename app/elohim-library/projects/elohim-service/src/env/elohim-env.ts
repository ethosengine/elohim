import { InjectionToken } from '@angular/core';

/**
 * Minimum environment surface needed by services in @elohim/service.
 *
 * Angular services in this library cannot import from `src/environments/`
 * (app-local paths). Instead they inject this token. Each consuming app
 * (elohim-app, lamad) registers its concrete environment at bootstrap:
 *
 *   { provide: ELOHIM_ENV, useValue: environment as ElohimEnv }
 *
 * Keep this interface lean — only the env keys the migrated services
 * actually access. Resist the temptation to expose the full Environment
 * interface; that lives in elohim-app and is app-specific.
 */
export interface ElohimEnv {
  production: boolean;
  /** Doorway API base URL (defaults to same origin if not set) */
  doorwayUrl?: string;
  /** Holochain connection config subset needed by StorageClientService */
  holochain?: {
    adminUrl: string;
    appUrl: string;
    authUrl?: string;
    proxyApiKey?: string;
    useLocalProxy?: boolean;
    connectionMode?: 'auto' | 'doorway' | 'direct';
    storageUrl?: string;
  };
  /** Projection API configuration */
  projectionApi?: {
    enabled: boolean;
    baseUrl?: string;
    timeout?: number;
    cacheMode?: 'prefer-projection' | 'prefer-holochain';
  };
}

export const ELOHIM_ENV = new InjectionToken<ElohimEnv>('ELOHIM_ENV', {
  providedIn: 'root',
  factory: () => ({ production: false }),
});
