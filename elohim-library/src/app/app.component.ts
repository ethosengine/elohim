import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

/**
 * Minimal shell — this application target exists solely to provide
 * Angular compiler context for the Storybook builder (`browserTarget`).
 * The actual component library is developed and showcased via Storybook
 * at port 6006: `pnpm run storybook`
 */
@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet],
  template: `<router-outlet />`,
})
export class AppComponent {}
