import { CommonModule } from '@angular/common';
import { Component, computed, input } from '@angular/core';

import {
  type HostingAccount,
  type UsageLevel,
  getUsageLevel,
} from '../../../../models/hosting-account.model';

export interface AgencyStep {
  label: string;
  complete: boolean;
  active: boolean;
}

@Component({
  selector: 'app-profile-hosting-section',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-hosting-section.component.html',
  styleUrls: ['./profile-hosting-section.component.css'],
})
export class ProfileHostingSectionComponent {
  readonly account = input.required<HostingAccount>();
  readonly doorwayUrl = input<string | null>(null);

  readonly storageLevel = computed<UsageLevel>(() => getUsageLevel(this.account().storagePercent));
  readonly queriesLevel = computed<UsageLevel>(() => getUsageLevel(this.account().queriesPercent));
  readonly bandwidthLevel = computed<UsageLevel>(() =>
    getUsageLevel(this.account().bandwidthPercent)
  );

  readonly accountUrl = computed(() => {
    const url = this.doorwayUrl();
    return url ? `${url}/threshold/account` : null;
  });

  readonly agencySteps = computed<AgencyStep[]>(() => {
    const acct = this.account();
    const isSteward = acct.isSteward;
    const keyExported = acct.keyExported;

    return [
      { label: 'Hosted', complete: true, active: !keyExported && !isSteward },
      { label: 'Key Export', complete: keyExported, active: keyExported && !isSteward },
      { label: 'Install App', complete: isSteward, active: false },
      { label: 'Steward', complete: isSteward, active: isSteward },
    ];
  });

  formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }

  formatDate(iso: string | null): string {
    if (!iso) return 'Never';
    try {
      return new Date(iso).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    } catch {
      return iso;
    }
  }

  formatNumber(n: number): string {
    return n.toLocaleString();
  }
}
