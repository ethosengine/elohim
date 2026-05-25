import { provideHttpClient } from '@angular/common/http';
import { ApplicationConfig, provideZoneChangeDetection } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideAnimations } from '@angular/platform-browser/animations';

import { routes } from './app.routes';
import { ELOHIM_ENV } from '@elohim/service';
import { environment } from '../environments/environment';

/**
 * Lamad bundle application config.
 *
 * B18d: Provides HTTP client, router (wired to LAMAD_ROUTES via app.routes.ts),
 * and animations. The lamad bundle is served at /lamad/ with <base href="/lamad/">.
 *
 * B18d omnibar note:
 * When spec Task B6 lands <elohim-page-chrome>, the lamad toolbar becomes a
 * standalone component with `host: { 'slot': 'omnibar' }` that slots into the
 * page chrome custom element. For now, ElohimNavigatorComponent (cross-pillar
 * via @app/elohim path alias) provides the shell from within lamad-layout.
 *
 * TODO(B18d-followup): Extract lamad toolbar as a standalone slot='omnibar'
 * component once elohim-page-chrome (Task B6) is available.
 */
export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(),
    provideAnimations(),
    // ELOHIM_ENV — maps lamad-local environment to the @elohim/service token contract
    {
      provide: ELOHIM_ENV,
      useValue: {
        production: environment.production,
        doorwayUrl: environment.client?.doorwayUrl,
      },
    },
  ],
};
