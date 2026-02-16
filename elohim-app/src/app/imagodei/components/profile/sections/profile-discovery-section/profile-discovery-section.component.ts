import { Component, input, output } from '@angular/core';

import {
  type DiscoveryResult,
  getFrameworkDisplayName,
  getCategoryIcon,
} from '@app/lamad/quiz-engine/models/discovery-assessment.model';

@Component({
  selector: 'app-profile-discovery-section',
  standalone: true,
  templateUrl: './profile-discovery-section.component.html',
  styleUrls: ['./profile-discovery-section.component.css'],
})
export class ProfileDiscoverySectionComponent {
  readonly results = input.required<DiscoveryResult[]>();

  readonly navigateToDiscovery = output<void>();

  getFrameworkName(result: DiscoveryResult): string {
    return getFrameworkDisplayName(result.framework);
  }

  getCategoryIcon(result: DiscoveryResult): string {
    return getCategoryIcon(result.category);
  }
}
