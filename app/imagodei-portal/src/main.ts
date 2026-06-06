import { bootstrapApplication } from '@angular/platform-browser';
import 'elohim-core/register';
import 'elohim-imagodei/register';
import { installEprLinkInterceptor } from 'elohim-core';

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

// Cross-bundle safety net (§12.3): content-authored/legacy anchors get the
// EPR-native handoff. Base href /auth/portal/ makes the default ownsPath
// heuristic correct — default (non-explicit) install is the right semantics.
installEprLinkInterceptor();

bootstrapApplication(AppComponent, appConfig).catch((err: unknown) =>
  console.error('Application bootstrap failed:', err)
);
