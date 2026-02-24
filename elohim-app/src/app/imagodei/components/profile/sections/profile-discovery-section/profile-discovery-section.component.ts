import { Component, inject, input, output } from '@angular/core';
import { Router } from '@angular/router';

// @coverage: 20.0% (2026-02-24)

import {
  type DiscoveryResult,
  getFrameworkDisplayName,
  getCategoryIcon,
} from '@app/lamad/quiz-engine/models/discovery-assessment.model';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

@Component({
  selector: 'app-profile-discovery-section',
  standalone: true,
  templateUrl: './profile-discovery-section.component.html',
  styleUrls: ['./profile-discovery-section.component.css'],
})
export class ProfileDiscoverySectionComponent {
  private readonly discoveryService = inject(DiscoveryAttestationService);
  private readonly router = inject(Router);

  readonly results = input.required<DiscoveryResult[]>();

  readonly navigateToDiscovery = output<void>();

  getFrameworkName(result: DiscoveryResult): string {
    return getFrameworkDisplayName(result.framework);
  }

  getCategoryIcon(result: DiscoveryResult): string {
    return getCategoryIcon(result.category);
  }

  getBadgeColor(result: DiscoveryResult): string {
    return this.discoveryService.getBadgeDisplay(result).color;
  }

  hasLink(result: DiscoveryResult): boolean {
    return !!result.contentNodeId;
  }

  navigateToResult(result: DiscoveryResult): void {
    if (result.contentNodeId) {
      void this.router.navigate(['/lamad/resource', result.contentNodeId]);
    } else {
      this.navigateToDiscovery.emit();
    }
  }
}
