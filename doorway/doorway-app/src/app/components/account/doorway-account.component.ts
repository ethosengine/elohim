/**
 * Doorway Account Component
 *
 * Self-service account page for hosted humans.
 * Shows usage gauges, account info, agency pipeline progress,
 * and graduation CTA. Adapts based on JWT claims (steward vs hosted).
 */

import {
  Component,
  OnInit,
  inject,
  signal,
  computed,
  ChangeDetectionStrategy,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { DoorwayAdminService } from '../../services/doorway-admin.service';
import {
  AccountResponse,
  AgencyStep,
  PortalHostResponse,
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
        <a routerLink="/" class="back-link" data-testid="account-back">Back to doorway</a>
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
          <button class="btn-secondary" (click)="loadAccount()" data-testid="account-retry">
            Retry
          </button>
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
              <!-- The /auth/account wire carries no isActive (that flag is an
                   admin-surface concern); a successful bearer-authenticated
                   account probe implies the account is active. -->
              <span class="status-badge active">Active</span>
            </div>
            <div class="info-item">
              <span class="info-label">Member Since</span>
              <span class="info-value">
                {{ $safeNavigationMigration(account()?.createdAt) | date: 'mediumDate' }}
              </span>
            </div>
            <div class="info-item">
              <span class="info-label">Last Login</span>
              <span class="info-value">
                {{ $safeNavigationMigration(account()?.lastLoginAt) | date: 'medium' }}
              </span>
            </div>
          </div>
        </section>

        <!-- Steward context banner -->
        @if (stewardAccessingThroughDoorway()) {
          <div class="context-banner">
            <span class="banner-icon">&#9432;</span>
            <div class="banner-text">
              <strong>Accessing through this doorway</strong>
              <p>
                Your local conductor is not connected. You can download your key bundle or
                re-provision below.
              </p>
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
                  {{ storagePercent() | number: '1.0-0' }}%
                </span>
              </div>
              <div class="gauge-bar">
                <div
                  class="gauge-fill"
                  [style.width.%]="Math.min(storagePercent(), 100)"
                  [style.background]="storageColor()"
                ></div>
              </div>
              <span class="gauge-detail">
                {{ formatBytesHelper(account()?.storageBytes ?? 0) }} /
                {{ formatBytesHelper(account()?.storageLimit ?? 0) }}
              </span>
            </div>

            <!-- Queries -->
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">Daily Queries</span>
                <span class="gauge-value" [style.color]="queriesColor()">
                  {{ queriesPercent() | number: '1.0-0' }}%
                </span>
              </div>
              <div class="gauge-bar">
                <div
                  class="gauge-fill"
                  [style.width.%]="Math.min(queriesPercent(), 100)"
                  [style.background]="queriesColor()"
                ></div>
              </div>
              <span class="gauge-detail">
                {{ $safeNavigationMigration(account()?.projectionQueries) | number }} /
                {{ $safeNavigationMigration(account()?.dailyQueryLimit) | number }} queries
              </span>
            </div>

            <!-- Bandwidth -->
            <div class="gauge-item">
              <div class="gauge-header">
                <span class="gauge-label">Daily Bandwidth</span>
                <span class="gauge-value" [style.color]="bandwidthColor()">
                  {{ bandwidthPercent() | number: '1.0-0' }}%
                </span>
              </div>
              <div class="gauge-bar">
                <div
                  class="gauge-fill"
                  [style.width.%]="Math.min(bandwidthPercent(), 100)"
                  [style.background]="bandwidthColor()"
                ></div>
              </div>
              <span class="gauge-detail">
                {{ formatBytesHelper(account()?.bandwidthBytes ?? 0) }} /
                {{ formatBytesHelper(account()?.dailyBandwidthLimit ?? 0) }}
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
              <div
                class="step"
                [class.completed]="isStepCompleted(step.key)"
                [class.current]="isCurrentStep(step.key)"
              >
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
                <li [class.met]="account()?.keyExported">
                  <span class="req-check">
                    {{ account()?.keyExported ? '&#10003;' : '&#10007;' }}
                  </span>
                  Export cryptographic keys
                </li>
                <li [class.met]="runsOwnConductor()">
                  <span class="req-check">{{ runsOwnConductor() ? '&#10003;' : '&#10007;' }}</span>
                  Install and run Elohim locally
                </li>
              </ul>
            </div>
          </section>
        }

        <!-- Manage from your steward -->
        @if (portalHostUrl(); as hostUrl) {
          <section class="card portal-host">
            <h2>Manage from your steward</h2>
            <p>Your account is also reachable from your peer-native client.</p>
            <button class="btn-primary" (click)="openSteward()" data-testid="portal-host-redirect">
              Manage from your steward →
            </button>
          </section>
        }

        <!-- Profile link -->
        <div class="profile-link-section">
          <a href="/identity/profile" class="profile-link">View full profile in Elohim App</a>
        </div>
      }
    </div>
  `,
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './doorway-account.component.css',
})
export class DoorwayAccountComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);

  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly account = signal<AccountResponse | null>(null);

  readonly agencySteps = AGENCY_STEPS;
  readonly Math = Math;
  readonly formatBytesHelper = formatBytes;

  readonly storagePercent = computed(() => this.account()?.storagePercent ?? 0);
  readonly queriesPercent = computed(() => this.account()?.queriesPercent ?? 0);
  readonly bandwidthPercent = computed(() => this.account()?.bandwidthPercent ?? 0);

  /**
   * The wire carries no hasLocalConductor flag. The closest wire-true signal
   * is conductorId: it names the doorway conductor hosting this account's
   * cell, and is absent when the human runs their own conductor.
   */
  readonly runsOwnConductor = computed(() => {
    const acct = this.account();
    return !!acct && !acct.conductorId;
  });

  /** Steward whose cell is still doorway-hosted (no local conductor). */
  readonly stewardAccessingThroughDoorway = computed(() => {
    const acct = this.account();
    return !!acct?.isSteward && !!acct.conductorId;
  });

  readonly storageColor = computed(() => quotaGaugeColor(this.storagePercent()));
  readonly queriesColor = computed(() => quotaGaugeColor(this.queriesPercent()));
  readonly bandwidthColor = computed(() => quotaGaugeColor(this.bandwidthPercent()));

  private readonly portalHostSignal = signal<PortalHostResponse | null>(null);
  readonly portalHostUrl = computed(() => this.portalHostSignal()?.hostUrl ?? null);

  ngOnInit(): void {
    this.loadAccount();
    this.loadPortalHost();
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

  async loadPortalHost(): Promise<void> {
    try {
      const resp = await this.adminService.getPortalHostUrl();
      this.portalHostSignal.set(resp);
    } catch {
      this.portalHostSignal.set(null);
    }
  }

  async openSteward(): Promise<void> {
    const url = this.portalHostUrl();
    if (!url) return;
    const { sessionToken } = await this.adminService.mintSessionToken();
    window.location.href = `${url}?session_token=${encodeURIComponent(sessionToken)}`;
  }

  isStepCompleted(step: AgencyStep): boolean {
    const acct = this.account();
    if (!acct) return false;
    switch (step) {
      case 'hosted':
        return true;
      case 'key_export':
        return acct.keyExported;
      case 'install_app':
        return this.runsOwnConductor();
      case 'steward':
        return acct.isSteward;
      default:
        return false;
    }
  }

  isCurrentStep(step: AgencyStep): boolean {
    const acct = this.account();
    if (!acct) return step === 'hosted';
    if (step === 'steward' && !acct.isSteward && this.runsOwnConductor()) return true;
    if (step === 'install_app' && !this.runsOwnConductor() && acct.keyExported) return true;
    if (step === 'key_export' && !acct.keyExported) return true;
    return false;
  }
}
