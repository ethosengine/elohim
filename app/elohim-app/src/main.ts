import { bootstrapApplication } from '@angular/platform-browser';

import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig).catch((err: unknown) =>
  console.error('Application bootstrap failed:', err)
);

// Register apps Service Worker for offline HTML5 app delivery.
// main.ts is the BROWSER entry point -- SSR boots through main.server.ts and
// never reaches this file -- so `navigator` and `window` are present by
// construction here, not merely guarded.
if ('serviceWorker' in navigator) {
  // eslint-disable-next-line no-restricted-syntax -- browser-only entry point; SSR uses main.server.ts
  window.addEventListener('load', () => {
    // eslint-disable-next-line no-restricted-syntax -- browser-only entry point; SSR uses main.server.ts
    navigator.serviceWorker
      .register('/apps-sw.js', { scope: '/apps/' })
      // eslint-disable-next-line no-console -- one-time boot diagnostic naming the SW scope, which is the field that actually goes wrong
      .then(reg => console.log('[apps-sw] registered, scope:', reg.scope))
      .catch(err => console.warn('[apps-sw] registration failed:', err));
  });
}
