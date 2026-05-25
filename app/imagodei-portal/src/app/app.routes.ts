import { Routes } from '@angular/router';

// Both routes render AppComponent — the URL params and path drive which
// portal mode (login vs consent) is displayed inside the portal-shell.
// AppComponent.ngOnInit() inspects window.location.search to distinguish flows.
export const routes: Routes = [
  { path: '', children: [] },
  { path: 'consent', children: [] },
];
