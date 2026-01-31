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

import { isPlatformBrowser } from '@angular/common';
import { ClassProvider, InjectionToken, Provider, PLATFORM_ID } from '@angular/core';

// @coverage: 25.0% (2026-01-31)

import {
  type IConnectionStrategy,
  createConnectionStrategy,
  DoorwayConnectionStrategy,
  DirectConnectionStrategy,
} from '@elohim/service/connection';

import { environment } from '../../../environments/environment';

import type { ConnectionMode } from '../../../environments/environment.types';

/**
 * Injection token for the connection strategy.
 *
 * The factory creates the appropriate strategy based on:
 * 1. Environment configuration (connectionMode)
 * 2. Runtime detection (Tauri, Node.js, browser)
 */
export const CONNECTION_STRATEGY = new InjectionToken<IConnectionStrategy>('ConnectionStrategy', {
  providedIn: 'root',
  factory: () => {
    // Get connection mode from environment config
    const mode: ConnectionMode = environment.holochain?.connectionMode ?? 'auto';

    // Create strategy based on mode
    return createConnectionStrategy(mode);
  },
});

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
export function provideDoorwayStrategy(): ClassProvider {
  return {
    provide: CONNECTION_STRATEGY,
    useClass: DoorwayConnectionStrategy,
  };
}

/**
 * Provider that forces Direct mode (useful for testing).
 */
export function provideDirectStrategy(): ClassProvider {
  return {
    provide: CONNECTION_STRATEGY,
    useClass: DirectConnectionStrategy,
  };
}

/**
 * Provider factory that respects SSR (server-side rendering).
 * Always uses Doorway mode on server since direct connection is not available.
 */
// Provider return type varies based on use; explicit return type is Provider
// eslint-disable-next-line sonarjs/function-return-type
export function provideConnectionStrategySSR(): Provider {
  return {
    provide: CONNECTION_STRATEGY,
    useFactory: (platformId: object) => {
      // On server, always use doorway (can't detect Tauri/Node.js)
      if (!isPlatformBrowser(platformId)) {
        return new DoorwayConnectionStrategy();
      }

      // On browser, use environment config
      const mode: ConnectionMode = environment.holochain?.connectionMode ?? 'auto';
      return createConnectionStrategy(mode);
    },
    deps: [PLATFORM_ID],
  };
}
