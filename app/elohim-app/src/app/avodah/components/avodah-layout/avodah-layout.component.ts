import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { ElohimNavigatorComponent } from '@app/elohim/components/elohim-navigator/elohim-navigator.component';

/**
 * AvodahLayoutComponent - Layout wrapper for the Avodah work management pillar
 *
 * Uses the unified ElohimNavigator for consistent navigation across all contexts.
 */
@Component({
  selector: 'app-avodah-layout',
  standalone: true,
  imports: [RouterOutlet, ElohimNavigatorComponent],
  template: `
    <div class="avodah-container">
      <app-elohim-navigator [context]="'avodah'" [showSearch]="true">
        <div class="avodah-main">
          <router-outlet></router-outlet>
        </div>
      </app-elohim-navigator>
    </div>
  `,
  styles: [
    `
      .avodah-container {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
        background: var(--lamad-bg-primary, #0f0f1a);
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .avodah-main {
        flex: 1;
        display: flex;
      }
    `,
  ],
})
export class AvodahLayoutComponent {}
