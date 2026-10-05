import { Routes } from '@angular/router';

// Every route renders AppComponent — the URL params and path drive which
// portal mode (login, consent, device approval) is displayed inside the
// portal-shell. AppComponent.ngOnInit() inspects window.location to
// distinguish flows. `consent/device/preview` is a development-only page;
// in an optimized build AppComponent ignores it (see devicePageFor).
export const routes: Routes = [
  { path: '', children: [] },
  { path: 'consent', children: [] },
  { path: 'consent/device', children: [] },
  { path: 'consent/device/preview', children: [] },
];
