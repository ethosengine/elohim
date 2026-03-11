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
import { NotificationService } from '../../../core/notifications/notification.service';
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
            <button class="btn btn-sm btn-secondary" (click)="refreshPeers()" [disabled]="refreshing()" data-testid="federation-refresh">
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
              data-testid="federation-peer-url"
            />
            <button class="btn btn-sm btn-primary" (click)="addPeer()" [disabled]="addingPeer() || !newPeerUrl" data-testid="federation-add-peer">
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
  styleUrl: './federation-tab.component.css',
})
export class FederationTabComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);
  private readonly notify = inject(NotificationService);

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
        this.notify.error(result?.message ?? 'Failed to add peer');
      }
    } catch {
      this.notify.error('Failed to add peer');
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
        this.notify.error(result?.message ?? 'Failed to remove peer');
      }
    } catch {
      this.notify.error('Failed to remove peer');
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
      this.notify.error('Failed to refresh peers');
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
