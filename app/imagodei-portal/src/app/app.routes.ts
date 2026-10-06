import { Routes } from '@angular/router';

// Every route renders AppComponent — the URL params and path drive which
// portal mode (login, consent, device approval) is displayed inside the
// portal-shell. AppComponent.ngOnInit() inspects window.location to
// distinguish flows. `consent/device/preview` and `identity/preview` are
// development-only pages; an optimized build ignores them.
export const routes: Routes = [
  { path: '', children: [] },
  { path: 'consent', children: [] },
  { path: 'consent/device', children: [] },
  { path: 'consent/device/preview', children: [] },
  { path: 'identity/preview', children: [] },
];
