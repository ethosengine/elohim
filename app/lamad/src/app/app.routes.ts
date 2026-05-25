import { Routes } from '@angular/router';

import { LAMAD_ROUTES } from './lamad.routes';

// B18d: wire lamad routes into the standalone app bundle.
// LAMAD_ROUTES has path '' as the layout root; the bundle is served at /lamad/
// so Angular Router matches from the bundle's own base href, not /lamad/.
export const routes: Routes = LAMAD_ROUTES;
