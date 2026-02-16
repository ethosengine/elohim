import { CommonModule } from '@angular/common';
import { Component, input, output } from '@angular/core';

import {
  type HostingCostSummary,
  type KeyBackupStatus,
  type NodeOperatorHostingIncome,
} from '../../../../models/identity.model';

@Component({
  selector: 'app-profile-data-section',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-data-section.component.html',
  styleUrls: ['./profile-data-section.component.css'],
})
export class ProfileDataSectionComponent {
  readonly isAuthenticated = input(false);
  readonly hostingCost = input<HostingCostSummary | null>(null);
  readonly nodeOperatorIncome = input<NodeOperatorHostingIncome | null>(null);
  readonly keyBackup = input<KeyBackupStatus | null>(null);

  readonly exportData = output<void>();

  formatBackupDate(dateString: string | undefined): string {
    if (!dateString) return 'Unknown';
    try {
      return new Date(dateString).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    } catch {
      return dateString;
    }
  }

  formatBackupMethod(method: string | undefined): string {
    const labels: Record<string, string> = {
      'seed-phrase': 'Seed Phrase',
      'encrypted-file': 'Encrypted File',
      'hardware-backup': 'Hardware Backup',
      'social-recovery': 'Social Recovery',
    };
    return method ? (labels[method] ?? method) : 'Unknown';
  }

  formatStorageBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
}
