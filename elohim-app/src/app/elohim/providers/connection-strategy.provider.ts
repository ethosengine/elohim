/**
 * Angular DI Provider for Connection Strategy
 *
 * Provides the appropriate connection strategy based on environment configuration.
 * Uses InjectionToken with factory for lazy initialization.
 *
 * Usage:
 * ```typescript
 * @Component({...})
 * export class MyComponent {
 *   private strategy = inject(CONNECTION_STRATEGY);
 *
 *   async connect() {
 *     const result = await this.strategy.connect(config);
 *   }
 * }
 * ```
 *
 * @packageDocumentation
 */

import { InjectionToken, Provider, inject, PLATFORM_ID } from '@angular/core';
import { isPlatformBrowser } from '@angular/common';

import { environment } from '../../../environments/environment';
import type { ConnectionMode } from '../../../environments/environment.types';

import {
  type IConnectionStrategy,
  createConnectionStrategy,
  DoorwayConnectionStrategy,
  DirectConnectionStrategy,
} from '../../../../../elohim-library/projects/elohim-service/src/connection';

/**
 * Injection token for the connection strategy.
 *
 * The factory creates the appropriate strategy based on:
 * 1. Environment configuration (connectionMode)
 * 2. Runtime detection (Tauri, Node.js, browser)
 */
export const CONNECTION_STRATEGY = new InjectionToken<IConnectionStrategy>(
  'ConnectionStrategy',
  {
    providedIn: 'root',
    factory: () => {
      // Get connection mode from environment config
      const mode: ConnectionMode = environment.holochain?.connectionMode ?? 'auto';

      // Create strategy based on mode
      const strategy = createConnectionStrategy(mode);

      console.log(`[CONNECTION_STRATEGY] Created ${strategy.name} strategy (mode: ${mode})`);

      return strategy;
    },
  }
);

/**
 * Provider for explicit strategy injection (e.g., in tests).
 *
 * @param mode - Connection mode to use
 * @returns Provider configuration
 *
 * @example
 * ```typescript
 * // In test module
 * TestBed.configureTestingModule({
 *   providers: [
 *     provideConnectionStrategy('doorway'),
 *   ],
 * });
 * ```
 */
export function provideConnectionStrategy(mode: ConnectionMode = 'auto'): Provider {
  return {
    provide: CONNECTION_STRATEGY,
    useFactory: () => createConnectionStrategy(mode),
  };
}

/**
 * Provider that forces Doorway mode (useful for testing).
 */
export function provideDoorwayStrategy(): Provider {
  return {
    provide: CONNECTION_STRATEGY,
    useClass: DoorwayConnectionStrategy,
  };
}

/**
 * Provider that forces Direct mode (useful for testing).
 */
export function provideDirectStrategy(): Provider {
  return {
    provide: CONNECTION_STRATEGY,
    useClass: DirectConnectionStrategy,
  };
}

/**
 * Provider factory that respects SSR (server-side rendering).
 * Always uses Doorway mode on server since direct connection is not available.
 */
export function provideConnectionStrategySSR(): Provider {
  return {
    provide: CONNECTION_STRATEGY,
    useFactory: (platformId: object) => {
      // On server, always use doorway (can't detect Tauri/Node.js)
      if (!isPlatformBrowser(platformId)) {
        console.log('[CONNECTION_STRATEGY] SSR detected, using doorway strategy');
        return new DoorwayConnectionStrategy();
      }

      // On browser, use environment config
      const mode: ConnectionMode = environment.holochain?.connectionMode ?? 'auto';
      return createConnectionStrategy(mode);
    },
    deps: [PLATFORM_ID],
  };
}
