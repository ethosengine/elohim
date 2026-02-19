/**
 * Federation Tab Component
 *
 * Grid of federated doorway cards showing name, URL, region,
 * status, and latency. Highlights self. P2P peer table below.
 */

import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DoorwayAdminService } from '../../../services/doorway-admin.service';
import {
  FederatedDoorway,
  P2PPeer,
  formatBytes,
} from '../../../models/doorway.model';

@Component({
  selector: 'app-federation-tab',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="federation-tab">
      @if (loading()) {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading federation data...</p>
        </div>
      } @else {
        <!-- Doorway Grid -->
        <section class="doorway-section">
          <h3>Federated Doorways ({{ doorways().length }})</h3>
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

      .peers-table th {
        background: #1f2937;
      }

      .peers-table tbody tr:hover {
        background: #374151;
      }
    }
  `],
})
export class FederationTabComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);

  readonly loading = signal(true);
  readonly doorways = signal<FederatedDoorway[]>([]);
  readonly peers = signal<P2PPeer[]>([]);

  readonly formatBytesHelper = formatBytes;

  ngOnInit(): void {
    this.loadData();
  }

  async loadData(): Promise<void> {
    this.loading.set(true);

    try {
      const [doorwaysRes, peersRes] = await Promise.all([
        this.adminService.getFederationDoorways().toPromise(),
        this.adminService.getP2PPeers().toPromise(),
      ]);

      if (doorwaysRes) {
        this.doorways.set(doorwaysRes.doorways);
      }
      if (peersRes) {
        this.peers.set(peersRes.peers);
      }
    } catch {
      // Errors handled by service fallbacks
    } finally {
      this.loading.set(false);
    }
  }
}
