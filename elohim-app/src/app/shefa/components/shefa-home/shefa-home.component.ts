import { Component, OnInit, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { EconomicService } from '../../services/economic.service';
import { AppreciationService, AppreciationDisplay } from '../../services/appreciation.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { EconomicEvent, REAAction } from '@app/elohim/models';

/**
 * ShefaHomeComponent - Economic Dashboard
 *
 * Shefa (Hebrew: abundance, flow) is the economic layer of the Elohim Protocol.
 *
 * This dashboard displays:
 * - Connection status to Holochain
 * - Economic event statistics
 * - Recent value flows
 * - Appreciation/recognition activity
 */
@Component({
  selector: 'app-shefa-home',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <div class="shefa-dashboard">
      <!-- Header -->
      <div class="dashboard-header">
        <div class="header-content">
          <h1>Shefa Dashboard</h1>
          <p class="subtitle">Economics of Human Flourishing</p>
        </div>
        <div class="connection-status" [class.connected]="isConnected()" [class.disconnected]="!isConnected()">
          <span class="status-dot"></span>
          {{ isConnected() ? 'Connected' : 'Disconnected' }}
        </div>
      </div>

      <!-- Loading State -->
      <div class="loading-overlay" *ngIf="loading()">
        <div class="spinner"></div>
        <p>Loading economic data...</p>
      </div>

      <!-- Stats Grid -->
      <div class="stats-grid" *ngIf="!loading()">
        <div class="stat-card">
          <div class="stat-icon">&#x1F4CA;</div>
          <div class="stat-content">
            <div class="stat-value">{{ totalEvents() }}</div>
            <div class="stat-label">Economic Events</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon">&#x2764;&#xFE0F;</div>
          <div class="stat-content">
            <div class="stat-value">{{ totalAppreciations() }}</div>
            <div class="stat-label">Appreciations</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon">&#x1F91D;</div>
          <div class="stat-content">
            <div class="stat-value">{{ uniqueAgents() }}</div>
            <div class="stat-label">Active Agents</div>
          </div>
        </div>

        <div class="stat-card">
          <div class="stat-icon">&#x2728;</div>
          <div class="stat-content">
            <div class="stat-value">{{ totalRecognition() | number:'1.0-0' }}</div>
            <div class="stat-label">Recognition Points</div>
          </div>
        </div>
      </div>

      <!-- Two Column Layout -->
      <div class="dashboard-columns" *ngIf="!loading()">
        <!-- Recent Events -->
        <div class="dashboard-section">
          <div class="section-header">
            <h2>Recent Economic Events</h2>
            <span class="event-count">{{ events().length }} events</span>
          </div>

          <div class="events-list" *ngIf="events().length > 0; else noEvents">
            <div class="event-item" *ngFor="let event of events().slice(0, 10)">
              <div class="event-icon" [ngClass]="getActionClass(event.action)">
                {{ getActionIcon(event.action) }}
              </div>
              <div class="event-details">
                <div class="event-action">{{ formatAction(event.action) }}</div>
                <div class="event-parties">
                  <span class="provider">{{ shortenId(event.provider) }}</span>
                  <span class="arrow">→</span>
                  <span class="receiver">{{ shortenId(event.receiver) }}</span>
                </div>
                <div class="event-meta">
                  <span class="event-quantity" *ngIf="event.resourceQuantity">
                    {{ event.resourceQuantity.hasNumericalValue }} {{ event.resourceQuantity.hasUnit }}
                  </span>
                  <span class="event-time">{{ formatTime(event.hasPointInTime) }}</span>
                </div>
              </div>
            </div>
          </div>

          <ng-template #noEvents>
            <div class="empty-state">
              <div class="empty-icon">&#x1F4ED;</div>
              <p>No economic events yet</p>
              <p class="empty-hint">Events will appear here as value flows through the network</p>
            </div>
          </ng-template>
        </div>

        <!-- Recent Appreciations -->
        <div class="dashboard-section">
          <div class="section-header">
            <h2>Recent Appreciations</h2>
            <span class="event-count">{{ appreciations().length }} appreciations</span>
          </div>

          <div class="events-list" *ngIf="appreciations().length > 0; else noAppreciations">
            <div class="appreciation-item" *ngFor="let appreciation of appreciations().slice(0, 10)">
              <div class="appreciation-icon">&#x1F49C;</div>
              <div class="appreciation-details">
                <div class="appreciation-flow">
                  <span class="appreciator">{{ shortenId(appreciation.appreciatedBy) }}</span>
                  <span class="arrow">→</span>
                  <span class="appreciated">{{ shortenId(appreciation.appreciationTo) }}</span>
                </div>
                <div class="appreciation-meta">
                  <span class="appreciation-value">
                    {{ appreciation.quantityValue }} {{ appreciation.quantityUnit }}
                  </span>
                  <span class="appreciation-time">{{ formatTime(appreciation.createdAt) }}</span>
                </div>
                <div class="appreciation-note" *ngIf="appreciation.note">
                  "{{ appreciation.note }}"
                </div>
              </div>
            </div>
          </div>

          <ng-template #noAppreciations>
            <div class="empty-state">
              <div class="empty-icon">&#x2764;&#xFE0F;</div>
              <p>No appreciations yet</p>
              <p class="empty-hint">Recognition will flow as learners engage with content</p>
            </div>
          </ng-template>
        </div>
      </div>

      <!-- Action Buttons -->
      <div class="dashboard-actions" *ngIf="!loading()">
        <button class="action-btn primary" (click)="refreshData()">
          &#x1F504; Refresh Data
        </button>
        <button class="action-btn" (click)="testConnection()">
          &#x1F50C; Test Connection
        </button>
        <a routerLink="/lamad" class="action-btn">
          &#x1F4DA; Explore Lamad
        </a>
      </div>

      <!-- Error State -->
      <div class="error-banner" *ngIf="error()">
        <span class="error-icon">&#x26A0;&#xFE0F;</span>
        {{ error() }}
        <button class="dismiss-btn" (click)="dismissError()">Dismiss</button>
      </div>
    </div>
  `,
  styles: [`
    .shefa-dashboard {
      max-width: 1200px;
      margin: 0 auto;
      padding: 2rem;
      position: relative;
    }

    /* Header */
    .dashboard-header {
      display: flex;
      justify-content: space-between;
      align-items: flex-start;
      margin-bottom: 2rem;
      flex-wrap: wrap;
      gap: 1rem;
    }

    .header-content h1 {
      font-size: 2rem;
      margin: 0;
      color: var(--lamad-text-primary, #f8fafc);
    }

    .subtitle {
      color: var(--lamad-text-muted, #64748b);
      margin: 0.25rem 0 0 0;
    }

    .connection-status {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.5rem 1rem;
      border-radius: 20px;
      font-size: 0.875rem;
      font-weight: 500;
    }

    .connection-status.connected {
      background: rgba(34, 197, 94, 0.1);
      color: #22c55e;
    }

    .connection-status.disconnected {
      background: rgba(239, 68, 68, 0.1);
      color: #ef4444;
    }

    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: currentColor;
    }

    /* Loading */
    .loading-overlay {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 4rem;
      color: var(--lamad-text-muted, #64748b);
    }

    .spinner {
      width: 40px;
      height: 40px;
      border: 3px solid var(--lamad-border, rgba(99, 102, 241, 0.2));
      border-top-color: var(--lamad-accent-primary, #6366f1);
      border-radius: 50%;
      animation: spin 1s linear infinite;
      margin-bottom: 1rem;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    /* Stats Grid */
    .stats-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 1rem;
      margin-bottom: 2rem;
    }

    .stat-card {
      background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
      border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      border-radius: 12px;
      padding: 1.5rem;
      display: flex;
      align-items: center;
      gap: 1rem;
      transition: transform 0.2s, border-color 0.2s;
    }

    .stat-card:hover {
      transform: translateY(-2px);
      border-color: var(--lamad-accent-primary, #6366f1);
    }

    .stat-icon {
      font-size: 2rem;
      width: 50px;
      height: 50px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: var(--lamad-bg-primary, #0f0f1a);
      border-radius: 10px;
    }

    .stat-value {
      font-size: 1.75rem;
      font-weight: 600;
      color: var(--lamad-text-primary, #f8fafc);
    }

    .stat-label {
      font-size: 0.875rem;
      color: var(--lamad-text-muted, #64748b);
    }

    /* Dashboard Columns */
    .dashboard-columns {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
      gap: 1.5rem;
      margin-bottom: 2rem;
    }

    @media (max-width: 900px) {
      .dashboard-columns {
        grid-template-columns: 1fr;
      }
    }

    .dashboard-section {
      background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
      border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      border-radius: 12px;
      overflow: hidden;
    }

    .section-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 1rem 1.5rem;
      border-bottom: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
      background: var(--lamad-bg-primary, rgba(15, 15, 26, 0.5));
    }

    .section-header h2 {
      margin: 0;
      font-size: 1rem;
      font-weight: 600;
      color: var(--lamad-text-primary, #f8fafc);
    }

    .event-count {
      font-size: 0.75rem;
      color: var(--lamad-text-muted, #64748b);
      background: var(--lamad-bg-primary, #0f0f1a);
      padding: 0.25rem 0.75rem;
      border-radius: 10px;
    }

    /* Events List */
    .events-list {
      max-height: 400px;
      overflow-y: auto;
    }

    .event-item, .appreciation-item {
      display: flex;
      gap: 1rem;
      padding: 1rem 1.5rem;
      border-bottom: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.05));
      transition: background 0.2s;
    }

    .event-item:hover, .appreciation-item:hover {
      background: var(--lamad-bg-primary, rgba(15, 15, 26, 0.5));
    }

    .event-item:last-child, .appreciation-item:last-child {
      border-bottom: none;
    }

    .event-icon, .appreciation-icon {
      width: 36px;
      height: 36px;
      display: flex;
      align-items: center;
      justify-content: center;
      background: var(--lamad-bg-primary, #0f0f1a);
      border-radius: 8px;
      font-size: 1.25rem;
      flex-shrink: 0;
    }

    .event-icon.use { background: rgba(59, 130, 246, 0.2); }
    .event-icon.produce { background: rgba(34, 197, 94, 0.2); }
    .event-icon.transfer { background: rgba(168, 85, 247, 0.2); }
    .event-icon.consume { background: rgba(239, 68, 68, 0.2); }
    .event-icon.raise { background: rgba(251, 191, 36, 0.2); }

    .event-details, .appreciation-details {
      flex: 1;
      min-width: 0;
    }

    .event-action {
      font-weight: 600;
      color: var(--lamad-text-primary, #f8fafc);
      margin-bottom: 0.25rem;
      text-transform: capitalize;
    }

    .event-parties, .appreciation-flow {
      font-size: 0.875rem;
      color: var(--lamad-text-secondary, #e2e8f0);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }

    .arrow {
      color: var(--lamad-text-muted, #64748b);
    }

    .provider, .appreciator {
      color: var(--lamad-accent-primary, #6366f1);
    }

    .receiver, .appreciated {
      color: #22c55e;
    }

    .event-meta, .appreciation-meta {
      font-size: 0.75rem;
      color: var(--lamad-text-muted, #64748b);
      margin-top: 0.25rem;
      display: flex;
      gap: 1rem;
    }

    .appreciation-note {
      font-size: 0.75rem;
      color: var(--lamad-text-secondary, #e2e8f0);
      font-style: italic;
      margin-top: 0.25rem;
      opacity: 0.8;
    }

    /* Empty State */
    .empty-state {
      text-align: center;
      padding: 3rem 1.5rem;
      color: var(--lamad-text-muted, #64748b);
    }

    .empty-icon {
      font-size: 3rem;
      margin-bottom: 1rem;
      opacity: 0.5;
    }

    .empty-state p {
      margin: 0;
    }

    .empty-hint {
      font-size: 0.875rem;
      margin-top: 0.5rem !important;
      opacity: 0.7;
    }

    /* Action Buttons */
    .dashboard-actions {
      display: flex;
      gap: 1rem;
      flex-wrap: wrap;
      justify-content: center;
    }

    .action-btn {
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      padding: 0.75rem 1.5rem;
      background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
      border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.2));
      color: var(--lamad-text-secondary, #e2e8f0);
      text-decoration: none;
      border-radius: 8px;
      font-size: 0.875rem;
      font-weight: 500;
      cursor: pointer;
      transition: all 0.2s;
    }

    .action-btn:hover {
      background: var(--lamad-accent-primary, #6366f1);
      border-color: var(--lamad-accent-primary, #6366f1);
      color: white;
    }

    .action-btn.primary {
      background: var(--lamad-accent-primary, #6366f1);
      border-color: var(--lamad-accent-primary, #6366f1);
      color: white;
    }

    .action-btn.primary:hover {
      background: var(--lamad-accent-secondary, #4f46e5);
    }

    /* Error Banner */
    .error-banner {
      position: fixed;
      bottom: 2rem;
      left: 50%;
      transform: translateX(-50%);
      background: rgba(239, 68, 68, 0.9);
      color: white;
      padding: 1rem 1.5rem;
      border-radius: 8px;
      display: flex;
      align-items: center;
      gap: 1rem;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
      z-index: 1000;
    }

    .error-icon {
      font-size: 1.25rem;
    }

    .dismiss-btn {
      background: rgba(255, 255, 255, 0.2);
      border: none;
      color: white;
      padding: 0.25rem 0.75rem;
      border-radius: 4px;
      cursor: pointer;
      font-size: 0.75rem;
    }

    .dismiss-btn:hover {
      background: rgba(255, 255, 255, 0.3);
    }
  `]
})
export class ShefaHomeComponent implements OnInit {
  // State signals
  loading = signal(true);
  error = signal<string | null>(null);

  // Data signals
  events = signal<EconomicEvent[]>([]);
  appreciations = signal<AppreciationDisplay[]>([]);

  // Computed stats
  totalEvents = computed(() => this.events().length);
  totalAppreciations = computed(() => this.appreciations().length);
  uniqueAgents = computed(() => {
    const agents = new Set<string>();
    this.events().forEach(e => {
      agents.add(e.provider);
      agents.add(e.receiver);
    });
    this.appreciations().forEach(a => {
      agents.add(a.appreciatedBy);
      agents.add(a.appreciationTo);
    });
    return agents.size;
  });
  totalRecognition = computed(() => {
    return this.appreciations().reduce((sum, a) => sum + a.quantityValue, 0);
  });

  // Connection status
  isConnected = computed(() => this.holochainClient.isConnected());

  constructor(
    private readonly economicService: EconomicService,
    private readonly appreciationService: AppreciationService,
    private readonly holochainClient: HolochainClientService
  ) {}

  ngOnInit(): void {
    this.loadData();
  }

  async loadData(): Promise<void> {
    this.loading.set(true);
    this.error.set(null);

    try {
      // Test availability first
      await this.economicService.testAvailability();
      await this.appreciationService.testAvailability();

      // If services are available, try to load some data
      if (this.economicService.isAvailable()) {
        // Try to load events for current agent (or a sample)
        this.economicService.getEventsForAgent('current', 'both').subscribe({
          next: (events) => this.events.set(events),
          error: (err) => console.warn('Could not load events:', err)
        });
      }

      if (this.appreciationService.isAvailable()) {
        // Try to load appreciations
        this.appreciationService.getAppreciationsFor('current').subscribe({
          next: (appreciations) => this.appreciations.set(appreciations),
          error: (err) => console.warn('Could not load appreciations:', err)
        });
      }

      // If not connected, show demo data
      if (!this.isConnected()) {
        this.loadDemoData();
      }
    } catch (err) {
      console.error('Failed to load economic data:', err);
      this.error.set('Failed to connect to Holochain. Showing demo data.');
      this.loadDemoData();
    } finally {
      this.loading.set(false);
    }
  }

  loadDemoData(): void {
    // Demo economic events
    const demoEvents: EconomicEvent[] = [
      {
        id: 'demo-event-1',
        action: 'use' as REAAction,
        provider: 'agent-learner-001',
        receiver: 'content-node-abc',
        hasPointInTime: new Date(Date.now() - 3600000).toISOString(),
        state: 'validated',
        resourceQuantity: { hasNumericalValue: 1, hasUnit: 'view' },
      },
      {
        id: 'demo-event-2',
        action: 'produce' as REAAction,
        provider: 'agent-contributor-002',
        receiver: 'content-node-xyz',
        hasPointInTime: new Date(Date.now() - 7200000).toISOString(),
        state: 'validated',
        resourceQuantity: { hasNumericalValue: 1, hasUnit: 'content-node' },
      },
      {
        id: 'demo-event-3',
        action: 'raise' as REAAction,
        provider: 'agent-learner-003',
        receiver: 'contributor-presence-001',
        hasPointInTime: new Date(Date.now() - 10800000).toISOString(),
        state: 'validated',
        resourceQuantity: { hasNumericalValue: 5, hasUnit: 'recognition-points' },
      },
    ];

    // Demo appreciations
    const demoAppreciations: AppreciationDisplay[] = [
      {
        id: 'demo-appreciation-1',
        appreciationOf: 'content-node-abc',
        appreciatedBy: 'agent-learner-001',
        appreciationTo: 'contributor-presence-001',
        quantityValue: 10,
        quantityUnit: 'recognition-points',
        note: 'Great explanation of REA concepts!',
        createdAt: new Date(Date.now() - 1800000).toISOString(),
      },
      {
        id: 'demo-appreciation-2',
        appreciationOf: 'learning-path-xyz',
        appreciatedBy: 'agent-learner-002',
        appreciationTo: 'contributor-presence-002',
        quantityValue: 25,
        quantityUnit: 'recognition-points',
        note: null,
        createdAt: new Date(Date.now() - 5400000).toISOString(),
      },
    ];

    this.events.set(demoEvents);
    this.appreciations.set(demoAppreciations);
  }

  refreshData(): void {
    this.loadData();
  }

  async testConnection(): Promise<void> {
    this.loading.set(true);
    try {
      const connected = await this.holochainClient.testAdminConnection();
      if (connected.success) {
        this.error.set(null);
        await this.loadData();
      } else {
        this.error.set('Could not connect to Holochain conductor');
      }
    } catch (err) {
      this.error.set('Connection test failed');
    } finally {
      this.loading.set(false);
    }
  }

  dismissError(): void {
    this.error.set(null);
  }

  // Formatting helpers
  formatAction(action: string): string {
    const actionLabels: Record<string, string> = {
      use: 'Used Resource',
      produce: 'Produced',
      consume: 'Consumed',
      transfer: 'Transferred',
      raise: 'Recognition',
      lower: 'Reduced',
      cite: 'Cited',
      work: 'Work Performed',
      'deliver-service': 'Service Delivered',
    };
    return actionLabels[action] || action;
  }

  getActionIcon(action: string): string {
    const icons: Record<string, string> = {
      use: '&#x1F441;',
      produce: '&#x2728;',
      consume: '&#x1F525;',
      transfer: '&#x1F4E6;',
      raise: '&#x2B06;',
      lower: '&#x2B07;',
      cite: '&#x1F4DD;',
      work: '&#x1F6E0;',
      'deliver-service': '&#x1F91D;',
    };
    return icons[action] || '&#x25CF;';
  }

  getActionClass(action: string): string {
    return action.replace('-', '_');
  }

  shortenId(id: string): string {
    if (!id) return 'Unknown';
    if (id.length <= 16) return id;
    return `${id.slice(0, 8)}...${id.slice(-4)}`;
  }

  formatTime(isoString: string): string {
    try {
      const date = new Date(isoString);
      const now = new Date();
      const diff = now.getTime() - date.getTime();

      if (diff < 60000) return 'Just now';
      if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
      if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
      return date.toLocaleDateString();
    } catch {
      return 'Unknown';
    }
  }
}
