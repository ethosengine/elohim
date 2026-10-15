import { Component, inject, input, output } from '@angular/core';
import { Router } from '@angular/router';

// @coverage: 100.0% (2026-03-03)

import {
  type DiscoveryResult,
  getFrameworkDisplayName,
  getCategoryIcon,
} from '@app/lamad/quiz-engine/models/discovery-assessment.model';
import { DiscoveryAttestationService } from '@app/lamad/quiz-engine/services/discovery-attestation.service';

import { eprToUniversalHref } from '@elohim/service';

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
      // The shell owns the universal address, so an in-bundle navigation is right.
      void this.router.navigateByUrl(
        eprToUniversalHref({ id: result.contentNodeId, tier: 'head' })
      );
    } else {
      this.navigateToDiscovery.emit();
    }
  }
}
