import { Component } from '@angular/core';

import { HealthIndicatorComponent } from '../../elohim/components/health-indicator/health-indicator.component';

/** Surfaces the (previously unmounted) HealthIndicatorComponent — holochain /
 *  indexedDb / blobCache / network checks. Works in both deployment contexts. */
@Component({
  selector: 'app-health-lens',
  standalone: true,
  imports: [HealthIndicatorComponent],
  template: `<app-health-indicator />`,
})
export class HealthLensComponent {}
