/**
 * Local ElohimClient provider
 *
 * This file bridges the Angular version mismatch between elohim-app
 * and elohim-library by re-exporting the InjectionToken using the
 * app's Angular version.
 */

import { InjectionToken, Provider } from '@angular/core';
import { ElohimClient, ElohimClientConfig, detectClientMode, ReachLevel } from '@elohim/service/client';

/**
 * Local injection token for ElohimClient
 * Uses app's Angular version to avoid InjectionToken mismatch
 */
export const ELOHIM_CLIENT = new InjectionToken<ElohimClient>('ElohimClient');

/**
 * Factory function for creating ElohimClient
 */
export function elohimClientFactory(config: ElohimClientConfig): ElohimClient {
  return new ElohimClient(config);
}

/**
 * Provider for ElohimClient with configuration
 */
export function provideElohimClient(config: ElohimClientConfig): Provider[] {
  return [
    {
      provide: ELOHIM_CLIENT,
      useFactory: () => elohimClientFactory(config),
    },
  ];
}

// Re-export class and functions (values)
export { ElohimClient, detectClientMode, ReachLevel };

// Re-export types
export type { ElohimClientConfig };
