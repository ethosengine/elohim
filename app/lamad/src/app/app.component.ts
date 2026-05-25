import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

/**
 * Lamad bundle root component.
 *
 * The lamad bundle is served at /lamad/ with <base href="/lamad/">.
 * Routes are defined in app.routes.ts and imported from lamad.routes.ts.
 * The LamadLayoutComponent (loaded at route '') provides the actual shell with
 * ElohimNavigatorComponent (cross-pillar via @app/elohim path alias — B18c MVP).
 *
 * B18d: router-outlet at root delegates immediately to LAMAD_ROUTES.
 */
@Component({
  selector: 'lamad-root',
  standalone: true,
  imports: [RouterOutlet],
  template: `<router-outlet />`,
})
export class AppComponent {}
