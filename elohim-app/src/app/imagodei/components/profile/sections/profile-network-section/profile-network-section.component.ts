import { CommonModule } from '@angular/common';
import { Component, input, output } from '@angular/core';

import { type EdgeNodeDisplayInfo } from '@app/elohim/models/holochain-connection.model';

import { type AgencyState, type ConnectionStatus } from '../../../../models/agency.model';

@Component({
  selector: 'app-profile-network-section',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-network-section.component.html',
  styleUrls: ['./profile-network-section.component.css'],
})
export class ProfileNetworkSectionComponent {
  readonly agencyState = input.required<AgencyState>();
  readonly connectionStatus = input.required<ConnectionStatus>();
  readonly edgeNodeInfo = input.required<EdgeNodeDisplayInfo>();

  readonly reconnect = output<void>();

  isConnected(): boolean {
    return this.connectionStatus().state === 'connected';
  }

  getLocationIcon(location: string): string {
    const icons: Record<string, string> = {
      'browser-memory': 'memory',
      'browser-storage': 'storage',
      'hosted-server': 'cloud',
      'local-holochain': 'smartphone',
      dht: 'lan',
      'encrypted-backup': 'lock',
    };
    return icons[location] ?? 'folder';
  }

  truncateHash(hash: string | null): string {
    if (!hash) return 'N/A';
    if (hash.length <= 16) return hash;
    return `${hash.substring(0, 8)}...${hash.substring(hash.length - 4)}`;
  }

  async copyToClipboard(value: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      // Clipboard API not available
    }
  }
}
