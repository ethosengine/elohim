import { bootstrapApplication } from '@angular/platform-browser';
import 'elohim-core/register'; // Side-effect: registers <elohim-page-chrome>, <elohim-epr-link>, etc.

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig).catch((err: unknown) =>
  console.error('Application bootstrap failed:', err)
);
