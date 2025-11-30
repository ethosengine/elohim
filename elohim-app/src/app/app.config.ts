import { ApplicationConfig, provideZoneChangeDetection, importProvidersFrom, APP_INITIALIZER, inject } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';

import { routes } from './app.routes';

// Content I/O plugins - these register themselves when imported
import { MarkdownIOModule } from './lamad/content-io/plugins/markdown/markdown-io.module';
import { GherkinIOModule } from './lamad/content-io/plugins/gherkin/gherkin-io.module';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(),
    // Import Content I/O plugin modules to ensure they self-register
    importProvidersFrom(MarkdownIOModule, GherkinIOModule)
  ]
};
