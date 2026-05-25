import { bootstrapApplication } from '@angular/platform-browser';
import 'elohim-core/register';
import 'elohim-imagodei/register';

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig).catch((err: unknown) =>
  console.error('Application bootstrap failed:', err)
);
