/**
 * Federation Tab Component
 *
 * Three sections:
 * 1. Configured Peers — admin controls for add/remove/refresh federation peer URLs
 * 2. Discovered Doorways — read-only card grid from DHT + peer cache
 * 3. P2P Peers — table of libp2p peer connections
 */

import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { DoorwayAdminService } from '../../../services/doorway-admin.service';
import {
  FederatedDoorway,
  FederationPeerConfig,
  P2PPeer,
  formatBytes,
} from '../../../models/doorway.model';

@Component({
  selector: 'app-federation-tab',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: `
    <div class="federation-tab">
      @if (loading()) {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading federation data...</p>
        </div>
      } @else {
        <!-- Configured Peers (Admin) -->
        <section class="peers-config-section">
          <div class="section-header">
            <h3>Configured Peers ({{ peerConfig().length }})</h3>
            <button class="btn btn-sm btn-secondary" (click)="refreshPeers()" [disabled]="refreshing()">
              {{ refreshing() ? 'Refreshing...' : 'Refresh' }}
            </button>
          </div>

          <!-- Add peer input -->
          <div class="add-peer-row">
            <input
              type="text"
              class="peer-url-input"
              placeholder="https://doorway-example.elohim.host"
              [(ngModel)]="newPeerUrl"
              (keyup.enter)="addPeer()"
              [disabled]="addingPeer()"
            />
            <button class="btn btn-sm btn-primary" (click)="addPeer()" [disabled]="addingPeer() || !newPeerUrl">
              {{ addingPeer() ? 'Adding...' : 'Add Peer' }}
            </button>
          </div>

          @if (peerConfig().length > 0) {
            <table class="config-table">
              <thead>
                <tr>
                  <th>URL</th>
                  <th>Status</th>
                  <th>Doorway</th>
                  <th>Region</th>
                  <th>Capabilities</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                @for (peer of peerConfig(); track peer.url) {
                  <tr>
                    <td class="url-cell">{{ peer.url }}</td>
                    <td>
                      <span class="status-dot" [class.reachable]="peer.reachable" [class.unreachable]="!peer.reachable"></span>
                      {{ peer.reachable ? 'Reachable' : 'Unreachable' }}
                    </td>
                    <td>{{ peer.doorwayId ?? '-' }}</td>
                    <td>{{ peer.region ?? '-' }}</td>
                    <td>
                      @for (cap of peer.capabilities; track cap) {
                        <span class="cap-badge">{{ cap }}</span>
                      }
                      @if (peer.capabilities.length === 0) {
                        <span class="text-muted">-</span>
                      }
                    </td>
                    <td>
                      <button
                        class="btn btn-sm btn-danger"
                        (click)="removePeer(peer.url)"
                        [disabled]="removingPeer() === peer.url"
                      >
                        {{ removingPeer() === peer.url ? 'Removing...' : 'Remove' }}
                      </button>
                    </td>
                  </tr>
                }
              </tbody>
            </table>
          } @else {
            <div class="empty-state">No federation peers configured. Add a peer URL above to start federating.</div>
          }
        </section>

        <!-- Doorway Grid -->
        <section class="doorway-section">
          <h3>Discovered Doorways ({{ doorways().length }})</h3>
          <div class="doorway-grid">
            @for (dw of doorways(); track dw.id) {
              <div class="doorway-card" [class.self]="dw.isSelf">
                <div class="dw-header">
                  <span class="dw-status" [class]="dw.status"></span>
                  <span class="dw-name">{{ dw.name || dw.id }}</span>
                  @if (dw.isSelf) {
                    <span class="self-tag">Self</span>
                  }
                </div>
                <div class="dw-details">
                  <div class="dw-detail">
                    <span class="label">URL</span>
                    <span class="value url">{{ dw.url }}</span>
                  </div>
                  @if (dw.region) {
                    <div class="dw-detail">
                      <span class="label">Region</span>
                      <span class="value">{{ dw.region }}</span>
                    </div>
                  }
                  <div class="dw-detail">
                    <span class="label">Status</span>
                    <span class="value status-text" [class]="dw.status">{{ dw.status | titlecase }}</span>
                  </div>
                  @if (dw.latencyMs !== null) {
                    <div class="dw-detail">
                      <span class="label">Latency</span>
                      <span class="value">{{ dw.latencyMs }}ms</span>
                    </div>
                  }
                  <div class="dw-detail">
                    <span class="label">Humans</span>
                    <span class="value">{{ dw.humansServed | number }}</span>
                  </div>
                  <div class="dw-detail">
                    <span class="label">Content</span>
                    <span class="value">{{ dw.contentAvailable | number }}</span>
                  </div>
                </div>
                @if (dw.capabilities.length > 0) {
                  <div class="dw-caps">
                    @for (cap of dw.capabilities; track cap) {
                      <span class="cap-badge">{{ cap }}</span>
                    }
                  </div>
                }
              </div>
            } @empty {
              <div class="empty-state">No federated doorways found</div>
            }
          </div>
        </section>

        <!-- P2P Peers Table -->
        <section class="peers-section">
          <h3>P2P Peers ({{ peers().length }})</h3>
          @if (peers().length > 0) {
            <table class="peers-table">
              <thead>
                <tr>
                  <th>Peer ID</th>
                  <th>State</th>
                  <th>Latency</th>
                  <th>Connected Since</th>
                  <th>Sent</th>
                  <th>Received</th>
                </tr>
              </thead>
              <tbody>
                @for (peer of peers(); track peer.peerId) {
                  <tr>
                    <td class="peer-id">{{ peer.peerId | slice:0:16 }}...</td>
                    <td>
                      <span class="conn-badge" [class]="peer.connectionState">
                        {{ peer.connectionState | titlecase }}
                      </span>
                    </td>
                    <td>{{ peer.latencyMs !== null ? peer.latencyMs + 'ms' : '-' }}</td>
                    <td>{{ peer.connectedSince | date:'short' }}</td>
                    <td>{{ formatBytesHelper(peer.bytesSent) }}</td>
                    <td>{{ formatBytesHelper(peer.bytesReceived) }}</td>
                  </tr>
                }
              </tbody>
            </table>
          } @else {
            <div class="empty-state">No P2P peers connected</div>
          }
        </section>
      }
    </div>
  `,
  styles: [`
    .federation-tab {
      padding: 1rem 0;
    }

    .loading-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 3rem;
      color: var(--text-secondary, #6b7280);
    }

    .spinner {
      width: 32px;
      height: 32px;
      border: 3px solid #e5e7eb;
      border-top-color: #3b82f6;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-bottom: 1rem;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    h3 {
      font-size: 1rem;
      font-weight: 600;
      color: var(--text-primary, #111827);
      margin: 0 0 1rem;
    }

    /* Configured Peers Section */
    .peers-config-section {
      margin-bottom: 2rem;
    }

    .section-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 0.75rem;

      h3 { margin: 0; }
    }

    .add-peer-row {
      display: flex;
      gap: 0.5rem;
      margin-bottom: 1rem;
    }

    .peer-url-input {
      flex: 1;
      padding: 0.5rem 0.75rem;
      border: 1px solid var(--border-color, #d1d5db);
      border-radius: 0.375rem;
      font-size: 0.875rem;
      background: white;
      color: var(--text-primary, #111827);

      &:focus {
        outline: none;
        border-color: #6366f1;
        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
      }
    }

    .config-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;
      margin-bottom: 1rem;

      th, td {
        padding: 0.625rem 0.75rem;
        text-align: left;
        border-bottom: 1px solid var(--border-color, #e5e7eb);
      }

      th {
        background: #f9fafb;
        font-weight: 500;
        color: var(--text-secondary, #6b7280);
        font-size: 0.8125rem;
      }

      tbody tr:hover {
        background: #f9fafb;
      }

      .url-cell {
        font-family: monospace;
        font-size: 0.8125rem;
        word-break: break-all;
        max-width: 300px;
      }
    }

    .status-dot {
      display: inline-block;
      width: 8px;
      height: 8px;
      border-radius: 50%;
      margin-right: 0.375rem;

      &.reachable { background: #10b981; }
      &.unreachable { background: #ef4444; }
    }

    .text-muted {
      color: var(--text-secondary, #9ca3af);
    }

    /* Buttons */
    .btn {
      cursor: pointer;
      border: none;
      border-radius: 0.375rem;
      font-weight: 500;
      white-space: nowrap;

      &:disabled {
        opacity: 0.6;
        cursor: not-allowed;
      }
    }

    .btn-sm {
      padding: 0.375rem 0.75rem;
      font-size: 0.8125rem;
    }

    .btn-primary {
      background: #6366f1;
      color: white;

      &:hover:not(:disabled) { background: #4f46e5; }
    }

    .btn-secondary {
      background: #f3f4f6;
      color: #374151;
      border: 1px solid #d1d5db;

      &:hover:not(:disabled) { background: #e5e7eb; }
    }

    .btn-danger {
      background: #fef2f2;
      color: #dc2626;
      border: 1px solid #fecaca;

      &:hover:not(:disabled) { background: #fee2e2; }
    }

    /* Doorway Grid */
    .doorway-section {
      margin-bottom: 2rem;
    }

    .doorway-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
      gap: 1rem;
    }

    .doorway-card {
      background: white;
      border: 1px solid var(--border-color, #e5e7eb);
      border-radius: 0.5rem;
      padding: 1rem;

      &.self {
        border-color: #6366f1;
        background: #fafafe;
      }
    }

    .dw-header {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      margin-bottom: 0.75rem;
    }

    .dw-status {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: #9ca3af;
      flex-shrink: 0;

      &.active, &.online { background: #10b981; }
      &.degraded { background: #f59e0b; }
      &.offline { background: #ef4444; }
    }

    .dw-name {
      font-weight: 600;
      font-size: 0.875rem;
      color: var(--text-primary, #111827);
    }

    .self-tag {
      margin-left: auto;
      padding: 0.125rem 0.375rem;
      border-radius: 0.25rem;
      background: #e0e7ff;
      color: #4338ca;
      font-size: 0.6875rem;
      font-weight: 600;
    }

    .dw-details {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 0.375rem 1rem;
    }

    .dw-detail {
      display: flex;
      flex-direction: column;

      .label {
        font-size: 0.6875rem;
        color: var(--text-secondary, #6b7280);
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .value {
        font-size: 0.8125rem;
        color: var(--text-primary, #111827);

        &.url {
          font-size: 0.75rem;
          word-break: break-all;
          color: var(--text-secondary, #6b7280);
        }

        &.status-text {
          &.active, &.online { color: #059669; }
          &.degraded { color: #d97706; }
          &.offline { color: #dc2626; }
        }
      }
    }

    .dw-caps {
      display: flex;
      flex-wrap: wrap;
      gap: 0.375rem;
      margin-top: 0.75rem;
    }

    .cap-badge {
      padding: 0.125rem 0.375rem;
      border-radius: 0.25rem;
      background: #f3f4f6;
      color: #6b7280;
      font-size: 0.6875rem;
    }

    /* P2P Peers */
    .peers-section {
      margin-top: 2rem;
    }

    .peers-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;

      th, td {
        padding: 0.75rem 1rem;
        text-align: left;
        border-bottom: 1px solid var(--border-color, #e5e7eb);
      }

      th {
        background: #f9fafb;
        font-weight: 500;
        color: var(--text-secondary, #6b7280);
      }

      tbody tr:hover {
        background: #f9fafb;
      }

      .peer-id {
        font-family: monospace;
        font-size: 0.8125rem;
      }
    }

    .conn-badge {
      display: inline-block;
      padding: 0.25rem 0.5rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 500;

      &.connected { background: #d1fae5; color: #065f46; }
      &.connecting { background: #fef3c7; color: #92400e; }
      &.disconnected { background: #f3f4f6; color: #4b5563; }
    }

    .empty-state {
      text-align: center;
      padding: 2rem;
      color: var(--text-secondary, #6b7280);
    }

    @media (prefers-color-scheme: dark) {
      .doorway-card {
        background: #1f2937;
        border-color: #374151;

        &.self {
          border-color: #6366f1;
          background: #1e1b3a;
        }
      }

      .cap-badge {
        background: #374151;
        color: #9ca3af;
      }

      .self-tag {
        background: #312e81;
        color: #a5b4fc;
      }

      .config-table th,
      .peers-table th {
        background: #1f2937;
      }

      .config-table tbody tr:hover,
      .peers-table tbody tr:hover {
        background: #374151;
      }

      .peer-url-input {
        background: #1f2937;
        border-color: #374151;
        color: #e5e7eb;
      }

      .btn-secondary {
        background: #374151;
        color: #e5e7eb;
        border-color: #4b5563;

        &:hover:not(:disabled) { background: #4b5563; }
      }

      .btn-danger {
        background: #451a1a;
        border-color: #7f1d1d;

        &:hover:not(:disabled) { background: #7f1d1d; }
      }
    }
  `],
})
export class FederationTabComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);

  readonly loading = signal(true);
  readonly doorways = signal<FederatedDoorway[]>([]);
  readonly peers = signal<P2PPeer[]>([]);
  readonly peerConfig = signal<FederationPeerConfig[]>([]);

  // Admin control state
  newPeerUrl = '';
  readonly addingPeer = signal(false);
  readonly removingPeer = signal<string | null>(null);
  readonly refreshing = signal(false);

  readonly formatBytesHelper = formatBytes;

  ngOnInit(): void {
    this.loadData();
  }

  async loadData(): Promise<void> {
    this.loading.set(true);

    try {
      const [doorwaysRes, peersRes, configRes] = await Promise.all([
        this.adminService.getFederationDoorways().toPromise(),
        this.adminService.getP2PPeers().toPromise(),
        this.adminService.getFederationPeerConfig().toPromise(),
      ]);

      if (doorwaysRes) {
        this.doorways.set(doorwaysRes.doorways);
      }
      if (peersRes) {
        this.peers.set(peersRes.peers);
      }
      if (configRes) {
        this.peerConfig.set(configRes.peers);
      }
    } catch {
      // Errors handled by service fallbacks
    } finally {
      this.loading.set(false);
    }
  }

  async addPeer(): Promise<void> {
    const url = this.newPeerUrl.trim();
    if (!url) return;

    this.addingPeer.set(true);
    try {
      const result = await this.adminService.addFederationPeer(url).toPromise();
      if (result?.success) {
        this.newPeerUrl = '';
        await this.reloadPeerConfig();
      } else {
        alert(result?.message ?? 'Failed to add peer');
      }
    } catch {
      alert('Failed to add peer');
    } finally {
      this.addingPeer.set(false);
    }
  }

  async removePeer(url: string): Promise<void> {
    this.removingPeer.set(url);
    try {
      const result = await this.adminService.removeFederationPeer(url).toPromise();
      if (result?.success) {
        await this.reloadPeerConfig();
      } else {
        alert(result?.message ?? 'Failed to remove peer');
      }
    } catch {
      alert('Failed to remove peer');
    } finally {
      this.removingPeer.set(null);
    }
  }

  async refreshPeers(): Promise<void> {
    this.refreshing.set(true);
    try {
      await this.adminService.refreshFederationPeers().toPromise();
      await this.reloadPeerConfig();
    } catch {
      alert('Failed to refresh peers');
    } finally {
      this.refreshing.set(false);
    }
  }

  private async reloadPeerConfig(): Promise<void> {
    const configRes = await this.adminService.getFederationPeerConfig().toPromise();
    if (configRes) {
      this.peerConfig.set(configRes.peers);
    }
    // Also reload discovered doorways since they may have changed
    const doorwaysRes = await this.adminService.getFederationDoorways().toPromise();
    if (doorwaysRes) {
      this.doorways.set(doorwaysRes.doorways);
    }
  }
}
