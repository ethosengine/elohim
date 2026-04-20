import { bootstrapApplication } from '@angular/platform-browser';

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig).catch((err: unknown) =>
  console.error('Application bootstrap failed:', err)
);

// Register apps Service Worker for offline HTML5 app delivery
if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker
      .register('/apps-sw.js', { scope: '/apps/' })
      .then(reg => console.log('[apps-sw] registered, scope:', reg.scope))
      .catch(err => console.warn('[apps-sw] registration failed:', err));
  });
}
