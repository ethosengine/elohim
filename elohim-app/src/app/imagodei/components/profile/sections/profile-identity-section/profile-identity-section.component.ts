import { CommonModule } from '@angular/common';
import { Component, input } from '@angular/core';

import {
  type HumanProfile,
  type ProfileReach,
  getReachLabel,
} from '../../../../models/identity.model';

@Component({
  selector: 'app-profile-identity-section',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-identity-section.component.html',
  styleUrls: ['./profile-identity-section.component.css'],
})
export class ProfileIdentitySectionComponent {
  readonly profile = input.required<HumanProfile>();

  getReachLabel(reach: ProfileReach | undefined): string {
    return reach ? getReachLabel(reach) : 'Not set';
  }

  formatDate(dateString: string | undefined): string {
    if (!dateString) return 'Unknown';
    try {
      return new Date(dateString).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    } catch {
      return dateString;
    }
  }
}
