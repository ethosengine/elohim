/**
 * Doorway Account Component
 *
 * Self-service account page for hosted humans.
 * Shows usage gauges, account info, agency pipeline progress,
 * and graduation CTA. Adapts based on JWT claims (steward vs hosted).
 */

import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { DoorwayAdminService } from '../../services/doorway-admin.service';
import {
  AccountResponse,
  AgencyStep,
  quotaGaugeColor,
  formatBytes,
} from '../../models/doorway.model';

/** Agency pipeline step definition */
interface PipelineStep {
  key: AgencyStep;
  label: string;
  description: string;
}

const AGENCY_STEPS: PipelineStep[] = [
  { key: 'hosted', label: 'Hosted', description: 'Account created on a doorway' },
  { key: 'key_export', label: 'Key Export', description: 'Exported your cryptographic keys' },
  { key: 'install_app', label: 'Install App', description: 'Running Elohim locally' },
  { key: 'steward', label: 'Steward', description: 'Full network participant' },
];

@Component({
  selector: 'app-doorway-account',
  standalone: true,
  imports: [CommonModule, RouterLink],
  template: `
    <div class="account-page">
      <header class="page-header">
        <h1>My Account</h1>
        <a routerLink="/" class="back-link">Back to doorway</a>
      </header>

      <!-- Loading -->
      @if (loading()) {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading account...</p>
        </div>
      }

      <!-- Error -->
      @if (error()) {
        <div class="error-state">
          <p>{{ error() }}</p>
          <button class="btn-secondary" (click)="loadAccount()">Retry</button>
        </div>
      }

      @if (account()) {
        <!-- Account Info -->
        <section class="card account-info">
          <h2>Account</h2>
          <div class="info-grid">
            <div class="info-item">
              <span class="info-label">Identifier</span>
              <span class="info-value">{{ account()?.identifier }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Status</span>
              <span class="status-badge" [class.active]="account()?.isActive">
                {{ account()?.isActive ? 'Active' : 'Inactive' }}
              </span>
            </div>
            <div class="info-item">
              <span class="info-label">Member Since</span>
              <span class="info-value">{{ account()?.createdAt | date:'mediumDate' }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Last Login</span>
              <span class="info-value">{{ account()?.lastLoginAt | date:'medium' }}</span>
            </div>
            <div class="info-item">
              <span class="info-label">Doorway</span>
              <span class="info-value">
                {{ account()?.doorwayName }}
                @if (account()?.doorwayRegion) {
                  <span class="region-tag">{{ account()?.doorwayRegion }}</span>
                }
              </span>
            </div>
          </div>
        </section>

        <!-- Steward context banner -->
        @if (account()?.isSteward && !account()?.hasLocalConductor) {
          <div class="context-banner">
            <span class="banner-icon">&#9432;</span>
            <div class="banner-text">
              <strong>Accessing through {{ account()?.doorwayName }}</strong>
              <p>Your local conductor is not connected. You can download your key bundle or re-provision below.</p>
            </div>
          </div>
        }

        <!-- Usage Gauges -->
        <section class="card usage-section">
          <h2>Usage</h2>
          <div class="gauge-grid">
            <!-- Storage -->
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">Storage</span>
                <span class="gauge-value" [style.color]="storageColor()">
                  {{ storagePercent() | number:'1.0-0' }}%
                </span>
              </div>
              <div class="gauge-bar">
                <div
                  class="gauge-fill"
                  [style.width.%]="Math.min(storagePercent(), 100)"
                  [style.background]="storageColor()">
                </div>
              </div>
              <span class="gauge-detail">
                {{ account()?.usage?.storageMb | number:'1.1-1' }} MB /
                {{ account()?.quota?.storageLimitMb | number:'1.0-0' }} MB
              </span>
            </div>

            <!-- Queries -->
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">Daily Queries</span>
                <span class="gauge-value" [style.color]="queriesColor()">
                  {{ queriesPercent() | number:'1.0-0' }}%
                </span>
              </div>
              <div class="gauge-bar">
                <div
                  class="gauge-fill"
                  [style.width.%]="Math.min(queriesPercent(), 100)"
                  [style.background]="queriesColor()">
                </div>
              </div>
              <span class="gauge-detail">
                {{ account()?.usage?.projectionQueries | number }} /
                {{ account()?.quota?.dailyQueryLimit | number }} queries
              </span>
            </div>

            <!-- Bandwidth -->
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">Daily Bandwidth</span>
                <span class="gauge-value" [style.color]="bandwidthColor()">
                  {{ bandwidthPercent() | number:'1.0-0' }}%
                </span>
              </div>
              <div class="gauge-bar">
                <div
                  class="gauge-fill"
                  [style.width.%]="Math.min(bandwidthPercent(), 100)"
                  [style.background]="bandwidthColor()">
                </div>
              </div>
              <span class="gauge-detail">
                {{ account()?.usage?.bandwidthMb | number:'1.1-1' }} MB /
                {{ account()?.quota?.dailyBandwidthLimitMb | number:'1.0-0' }} MB
              </span>
            </div>
          </div>
        </section>

        <!-- Agency Pipeline Progress -->
        <section class="card pipeline-section">
          <h2>Agency Pipeline</h2>
          <p class="pipeline-subtitle">Your journey from hosted human to steward</p>
          <div class="pipeline-stepper">
            @for (step of agencySteps; track step.key; let i = $index) {
              <div class="step" [class.completed]="isStepCompleted(step.key)" [class.current]="isCurrentStep(step.key)">
                <div class="step-marker">
                  @if (isStepCompleted(step.key)) {
                    <span class="check">&#10003;</span>
                  } @else {
                    <span class="step-number">{{ i + 1 }}</span>
                  }
                </div>
                <div class="step-content">
                  <span class="step-label">{{ step.label }}</span>
                  <span class="step-desc">{{ step.description }}</span>
                </div>
                @if (i < agencySteps.length - 1) {
                  <div class="step-connector" [class.completed]="isStepCompleted(step.key)"></div>
                }
              </div>
            }
          </div>
        </section>

        <!-- Graduation CTA -->
        @if (!account()?.isSteward) {
          <section class="card graduation-cta">
            <h2>Graduate to Steward</h2>
            <p>As a steward, you run your own node and contribute to the network.</p>
            <div class="requirements">
              <h3>Requirements</h3>
              <ul>
                <li [class.met]="true">
                  <span class="req-check">{{ true ? '&#10003;' : '&#10007;' }}</span>
                  Active hosted account
                </li>
                <li [class.met]="account()?.hasExportedKey">
                  <span class="req-check">{{ account()?.hasExportedKey ? '&#10003;' : '&#10007;' }}</span>
                  Export cryptographic keys
                </li>
                <li [class.met]="account()?.hasLocalConductor">
                  <span class="req-check">{{ account()?.hasLocalConductor ? '&#10003;' : '&#10007;' }}</span>
                  Install and run Elohim locally
                </li>
              </ul>
            </div>
          </section>
        }

        <!-- Profile link -->
        <div class="profile-link-section">
          <a href="/identity/profile" class="profile-link">View full profile in Elohim App</a>
        </div>
      }
    </div>
  `,
  styles: [`
    .account-page {
      max-width: 800px;
      margin: 0 auto;
      padding: 2rem 1.5rem;
      font-family: system-ui, -apple-system, sans-serif;
    }

    .page-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 2rem;

      h1 {
        font-size: 1.75rem;
        font-weight: 600;
        margin: 0;
        color: var(--text-primary, #111827);
      }

      .back-link {
        color: var(--primary, #6366f1);
        text-decoration: none;
        font-size: 0.875rem;

        &:hover { text-decoration: underline; }
      }
    }

    .loading-state, .error-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 4rem 2rem;
      color: var(--text-secondary, #6b7280);
    }

    .spinner {
      width: 32px;
      height: 32px;
      border: 3px solid #e5e7eb;
      border-top-color: #6366f1;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-bottom: 1rem;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .btn-secondary {
      padding: 0.5rem 1rem;
      background: white;
      border: 1px solid var(--border-color, #d1d5db);
      border-radius: 0.375rem;
      cursor: pointer;
      margin-top: 1rem;
    }

    .card {
      background: white;
      border: 1px solid var(--border-color, #e5e7eb);
      border-radius: 0.75rem;
      padding: 1.5rem;
      margin-bottom: 1.5rem;

      h2 {
        font-size: 1.125rem;
        font-weight: 600;
        margin: 0 0 1rem;
        color: var(--text-primary, #111827);
      }
    }

    .info-grid {
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
      gap: 1rem;
    }

    .info-item {
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .info-label {
      font-size: 0.75rem;
      color: var(--text-secondary, #6b7280);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .info-value {
      font-size: 0.875rem;
      color: var(--text-primary, #111827);
      font-weight: 500;
    }

    .status-badge {
      display: inline-block;
      padding: 0.125rem 0.5rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 500;
      width: fit-content;
      background: #fee2e2;
      color: #dc2626;

      &.active {
        background: #d1fae5;
        color: #059669;
      }
    }

    .region-tag {
      display: inline-block;
      margin-left: 0.5rem;
      padding: 0.125rem 0.375rem;
      border-radius: 0.25rem;
      background: #eff6ff;
      color: #1d4ed8;
      font-size: 0.75rem;
    }

    .context-banner {
      display: flex;
      gap: 0.75rem;
      padding: 1rem 1.25rem;
      margin-bottom: 1.5rem;
      background: #eff6ff;
      border: 1px solid #bfdbfe;
      border-radius: 0.75rem;

      .banner-icon {
        font-size: 1.25rem;
        color: #3b82f6;
        flex-shrink: 0;
      }

      .banner-text {
        strong {
          display: block;
          font-size: 0.875rem;
          color: #1e40af;
          margin-bottom: 0.25rem;
        }

        p {
          font-size: 0.8125rem;
          color: #1e40af;
          margin: 0;
          opacity: 0.8;
        }
      }
    }

    .gauge-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 1.5rem;
    }

    .gauge-item {
      display: flex;
      flex-direction: column;
      gap: 0.375rem;
    }

    .gauge-header {
      display: flex;
      justify-content: space-between;
      align-items: baseline;
    }

    .gauge-label {
      font-size: 0.875rem;
      color: var(--text-secondary, #6b7280);
    }

    .gauge-value {
      font-size: 1.125rem;
      font-weight: 600;
    }

    .gauge-bar {
      height: 8px;
      background: #e5e7eb;
      border-radius: 4px;
      overflow: hidden;
    }

    .gauge-fill {
      height: 100%;
      border-radius: 4px;
      transition: width 0.3s ease;
    }

    .gauge-detail {
      font-size: 0.75rem;
      color: var(--text-secondary, #6b7280);
    }

    .pipeline-section {
      .pipeline-subtitle {
        font-size: 0.875rem;
        color: var(--text-secondary, #6b7280);
        margin: -0.5rem 0 1.5rem;
      }
    }

    .pipeline-stepper {
      display: flex;
      flex-direction: column;
      gap: 0;
    }

    .step {
      display: flex;
      align-items: flex-start;
      gap: 1rem;
      position: relative;
      padding-bottom: 1.5rem;

      &:last-child {
        padding-bottom: 0;
      }
    }

    .step-marker {
      width: 32px;
      height: 32px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 0.875rem;
      font-weight: 600;
      flex-shrink: 0;
      background: #e5e7eb;
      color: #6b7280;
      position: relative;
      z-index: 1;

      .completed & {
        background: #10b981;
        color: white;
      }

      .current & {
        background: #6366f1;
        color: white;
      }
    }

    .check {
      font-size: 0.875rem;
    }

    .step-content {
      display: flex;
      flex-direction: column;
      padding-top: 0.25rem;
    }

    .step-label {
      font-size: 0.875rem;
      font-weight: 600;
      color: var(--text-primary, #111827);
    }

    .step-desc {
      font-size: 0.75rem;
      color: var(--text-secondary, #6b7280);
    }

    .step-connector {
      position: absolute;
      left: 15px;
      top: 32px;
      width: 2px;
      height: calc(100% - 32px);
      background: #e5e7eb;

      &.completed {
        background: #10b981;
      }
    }

    .graduation-cta {
      background: linear-gradient(135deg, #f5f3ff 0%, #eff6ff 100%);

      p {
        font-size: 0.875rem;
        color: var(--text-secondary, #6b7280);
        margin: 0 0 1rem;
      }

      h3 {
        font-size: 0.875rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
        color: var(--text-primary, #111827);
      }

      ul {
        list-style: none;
        padding: 0;
        margin: 0;
      }

      li {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.375rem 0;
        font-size: 0.875rem;
        color: var(--text-secondary, #6b7280);

        &.met {
          color: var(--text-primary, #111827);
        }
      }

      .req-check {
        font-size: 0.875rem;
        width: 1.25rem;
        text-align: center;

        .met & { color: #10b981; }
      }
    }

    .profile-link-section {
      text-align: center;
      padding: 1rem 0 2rem;
    }

    .profile-link {
      color: var(--primary, #6366f1);
      text-decoration: none;
      font-size: 0.875rem;

      &:hover { text-decoration: underline; }
    }

    @media (prefers-color-scheme: dark) {
      .card {
        background: #1f2937;
        border-color: #374151;
      }

      .gauge-bar {
        background: #374151;
      }

      .context-banner {
        background: #1e3a5f;
        border-color: #1e40af;
      }

      .graduation-cta {
        background: linear-gradient(135deg, #1f1b3a 0%, #1e293b 100%);
      }

      .region-tag {
        background: #1e3a5f;
        color: #93c5fd;
      }
    }
  `],
})
export class DoorwayAccountComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);

  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly account = signal<AccountResponse | null>(null);

  readonly agencySteps = AGENCY_STEPS;
  readonly Math = Math;
  readonly formatBytesHelper = formatBytes;

  readonly storagePercent = computed(() => this.account()?.quota?.storagePercentUsed ?? 0);
  readonly queriesPercent = computed(() => this.account()?.quota?.queriesPercentUsed ?? 0);
  readonly bandwidthPercent = computed(() => this.account()?.quota?.bandwidthPercentUsed ?? 0);

  readonly storageColor = computed(() => quotaGaugeColor(this.storagePercent()));
  readonly queriesColor = computed(() => quotaGaugeColor(this.queriesPercent()));
  readonly bandwidthColor = computed(() => quotaGaugeColor(this.bandwidthPercent()));

  ngOnInit(): void {
    this.loadAccount();
  }

  async loadAccount(): Promise<void> {
    this.loading.set(true);
    this.error.set(null);

    try {
      const account = await this.adminService.getAccount().toPromise();
      if (account) {
        this.account.set(account);
      } else {
        this.error.set('Unable to load account. Please sign in.');
      }
    } catch {
      this.error.set('Failed to load account data');
    } finally {
      this.loading.set(false);
    }
  }

  isStepCompleted(step: AgencyStep): boolean {
    const acct = this.account();
    if (!acct) return false;
    switch (step) {
      case 'hosted': return true;
      case 'key_export': return acct.hasExportedKey;
      case 'install_app': return acct.hasLocalConductor;
      case 'steward': return acct.isSteward;
      default: return false;
    }
  }

  isCurrentStep(step: AgencyStep): boolean {
    const acct = this.account();
    if (!acct) return step === 'hosted';
    if (step === 'steward' && !acct.isSteward && acct.hasLocalConductor) return true;
    if (step === 'install_app' && !acct.hasLocalConductor && acct.hasExportedKey) return true;
    if (step === 'key_export' && !acct.hasExportedKey) return true;
    return false;
  }
}
