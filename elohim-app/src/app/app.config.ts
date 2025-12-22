import { ApplicationConfig, provideZoneChangeDetection, importProvidersFrom } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';

import { routes } from './app.routes';

// Content I/O module with unified format plugins
import { ContentIOModuleWithPlugins } from './lamad/content-io/content-io.module';

// Shefa metrics and custodian selection services
import { CustodianCommitmentService } from './elohim/services/custodian-commitment.service';
import { PerformanceMetricsService } from './elohim/services/performance-metrics.service';
import { ShefaService } from './elohim/services/shefa.service';
import { CustodianSelectionService } from './elohim/services/custodian-selection.service';
import { CustodianMetricsReporterService } from './elohim/services/custodian-metrics-reporter.service';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(),
    // Import ContentIO module with built-in format plugins (Markdown, Gherkin)
    importProvidersFrom(ContentIOModuleWithPlugins),
    // Shefa metrics and custodian selection services
    CustodianCommitmentService,
    PerformanceMetricsService,
    ShefaService,
    CustodianSelectionService,
    CustodianMetricsReporterService
  ]
};
