import { CommonModule } from '@angular/common';
import { Component } from '@angular/core';

import { environment } from '../../../environments/environment';

/** Read-only view of build-time FeatureFlags. chrome://flags-style transparency,
 *  not a runtime override framework (YAGNI). Notes the useGraphqlTopology drift. */
@Component({
  selector: 'app-flags-lens',
  standalone: true,
  imports: [CommonModule],
  template: `
    <dl class="debug-kv">
      <ng-container *ngFor="let f of flags">
        <dt>{{ f.key }}</dt>
        <dd>{{ f.value }}</dd>
      </ng-container>
    </dl>
    <p class="flags-note">
      Build-time flags (no runtime override). Note: <code>useGraphqlTopology</code> is
      documented "default false" but set <code>true</code> in every environment build.
    </p>
  `,
  styles: [
    `.debug-kv { display: grid; grid-template-columns: max-content 1fr; gap: 0.25rem 1rem; }
     dt { font-weight: 600; } dd { margin: 0; font-family: monospace; }
     .flags-note { opacity: 0.7; font-size: 0.85rem; margin-top: 0.75rem; }`,
  ],
})
export class FlagsLensComponent {
  readonly flags = Object.entries(environment.features ?? {}).map(([key, value]) => ({
    key,
    value: String(value),
  }));
}
