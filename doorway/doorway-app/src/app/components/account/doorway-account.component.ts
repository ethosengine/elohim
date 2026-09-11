/**
 * Doorway Account Component
 *
 * Self-service account page for hosted humans.
 * Shows usage gauges, account info, agency pipeline progress,
 * and graduation CTA. Adapts based on JWT claims (steward vs hosted).
 */

import {
  ChangeDetectionStrategy,
  Component,
  OnInit,
  inject,
  signal,
  computed,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { HttpErrorResponse } from '@angular/common/http';
import { Router, RouterLink } from '@angular/router';
import { AuthStateService } from '../../services/auth-state.service';
import { DoorwayAdminService } from '../../services/doorway-admin.service';
import {
  AccountResponse,
  AgencyStep,
  HostedCellFacts,
  PortalHostResponse,
  quotaGaugeColor,
  formatBytes,
} from '../../models/doorway.model';

/**
 * What `GET /auth/account` answers a hosted human: the generated wire view
 * plus the hosting facts S2 Task 13 adds to it, read optionally so the page
 * renders correctly against a doorway whose wire has not caught up.
 */
type HostedAccount = AccountResponse & HostedCellFacts;

/**
 * Say back what the doorway refused, in the human's own terms. The doorway
 * owns the judgement; this only chooses the words for it.
 */
function closeRefusalMessage(refusal: unknown): string {
  if (refusal instanceof HttpErrorResponse) {
    const body = refusal.error as { code?: string; message?: string } | null;
    if (body?.code === 'CONFIRMATION_MISMATCH') {
      return 'That is not the identifier on this account. Type it exactly as it is shown above.';
    }
    if (body?.message) return body.message;
  }
  return 'The doorway could not close this account, and nothing was changed. Please try again.';
}

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
              <span class="info-value" data-testid="account-identifier">
                {{ account()?.identifier }}
              </span>
            </div>
            @if (displayName(); as name) {
              <div class="info-item">
                <span class="info-label">Name</span>
                <span class="info-value">{{ name }}</span>
              </div>
            }
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

        <!--
          Hosted by a household. Being hosted is something a household
          undertook, not a favour that can quietly stop — so the page says who
          is lending the machine and until when. Each row renders only when the
          doorway actually carries that fact: absent, never blank.
        -->
        @if (hostedByHousehold() || hostedUntil()) {
          <section class="card hosting-strip">
            <h2>Hosted by a household</h2>
            <p class="hosting-lede">
              Your cell — your own record chain — runs on someone else's machine, and they promised
              it where anyone can check.
            </p>
            <div class="info-grid">
              @if (hostedByHousehold(); as household) {
                <div class="info-item">
                  <span class="info-label">Hosted by</span>
                  <span class="info-value" data-testid="account-hosted-by-household">
                    {{ household }}
                  </span>
                </div>
              }
              @if (hostedUntil(); as until) {
                <div class="info-item">
                  <span class="info-label">Promised until</span>
                  <span class="info-value" data-testid="account-hosted-until">
                    {{ until | date: 'mediumDate' }}
                  </span>
                </div>
              }
            </div>
          </section>
        }

        <!-- Hosted-steward context banner: the in-between state -->
        @if (stewardAccessingThroughDoorway()) {
          <div class="context-banner" data-testid="account-hosted-steward-banner">
            <span class="banner-icon">&#9432;</span>
            <div class="banner-text">
              <strong>Hosted Steward</strong>
              <p>
                Accessing through {{ gatewayDomain() }}. Your local conductor is not connected, so
                this doorway is temporarily hosting your cell. You can download your key bundle or
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

        <!--
          Close this account. Offered to every signed-in human, because trying
          the network must cost nothing you cannot take back. The page states
          plainly what the doorway reclaims and what the network keeps: a notary
          keeps what it witnessed, so closing is honest, not amnesia.
        -->
        <section class="card close-account">
          <h2>Close this account</h2>
          <p>
            Closing stops this doorway hosting you. It uninstalls the cell it runs on your behalf,
            withdraws the promise of the machine lending it, and stops your sign-in working.
          </p>
          <p class="close-keeps">
            What the network keeps: anything you already wrote that the network witnessed stays
            witnessed. Closing ends the hosting, it does not unsay what was said.
          </p>

          @if (closeRequested()) {
            <div class="close-confirm">
              <label class="close-label" for="close-confirm-input">
                Type
                <strong>{{ account()?.identifier }}</strong>
                to confirm.
              </label>
              <input
                id="close-confirm-input"
                class="close-input"
                type="text"
                autocomplete="off"
                spellcheck="false"
                [attr.aria-invalid]="closeError() ? 'true' : null"
                [value]="confirmIdentifier()"
                (input)="onConfirmIdentifierInput($event)"
                data-testid="account-close-confirm-input"
              />
              @if (closeError(); as message) {
                <p class="close-error" role="alert" data-testid="account-close-error">
                  {{ message }}
                </p>
              }
              <div class="close-actions">
                <button
                  class="btn-danger"
                  type="button"
                  [disabled]="closing()"
                  (click)="confirmClose()"
                  data-testid="account-close-confirm"
                >
                  {{ closing() ? 'Closing…' : 'Close my account' }}
                </button>
                <button
                  class="btn-secondary"
                  type="button"
                  [disabled]="closing()"
                  (click)="cancelClose()"
                >
                  Keep my account
                </button>
              </div>
            </div>
          } @else {
            <button
              class="btn-danger"
              type="button"
              (click)="beginClose()"
              data-testid="account-close-begin"
            >
              Close this account
            </button>
          }
        </section>

        <!-- Profile link -->
        <div class="profile-link-section">
          <a href="/identity/profile" class="profile-link">View full profile in Elohim App</a>
        </div>
      }
    </div>
  `,
  styleUrl: './doorway-account.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DoorwayAccountComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);
  private readonly authState = inject(AuthStateService);
  private readonly router = inject(Router);

  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly account = signal<HostedAccount | null>(null);

  /** Closure is a deliberate two-step: begin, then type the identifier back. */
  readonly closeRequested = signal(false);
  readonly confirmIdentifier = signal('');
  readonly closing = signal(false);
  readonly closeError = signal<string | null>(null);

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

  /**
   * Steward whose cell is still doorway-hosted (no local conductor) — the
   * "hosted steward" in-between state the elohim-app agency badge names.
   */
  readonly stewardAccessingThroughDoorway = computed(() => {
    const acct = this.account();
    return !!acct?.isSteward && !!acct.conductorId;
  });

  /**
   * The domain accounts on this doorway actually live at.
   * doorway-alpha.elohim.host serves accounts at alpha.elohim.host — same
   * derivation as threshold-login's gatewayDomain().
   */
  readonly gatewayDomain = computed(() => {
    const hostname = window.location.hostname;
    return hostname.startsWith('doorway-') ? hostname.replace(/^doorway-/, '') : hostname;
  });

  /** The name the human asked to be known by, when the wire carries it. */
  readonly displayName = computed(() => this.account()?.displayName ?? null);

  /**
   * The household lending the machine this human's cell runs on.
   *
   * eslint-disable-next-line sonarjs/todo-tag -- names the backend contract this waits on
   * TODO(hosted-household-name-wire): `GET /auth/account` carries NO household
   * name — `conductorId` (the pool conductor the cell was placed on) is the
   * only wire-true fact about who is hosting, so it stands in here. The honest
   * value is the pool conductor's steward's own display name, which the truth
   * layer owes: the doorway knows the conductor, the conductor names its
   * steward, and `07-hosted-by-a-household.feature` asks for that name "in the
   * words a person uses". Until then this row names the machine, not the
   * household. Do NOT substitute the doorway's own name — the doorway arranges
   * the hosting, the steward's machine performs it, and the story is explicit
   * that they are not the same party. Never render `hostedCellGrantCid` here:
   * it is the notary's opaque handle, never a name to show a person.
   */
  readonly hostedByHousehold = computed(() => this.account()?.conductorId ?? null);

  /** RFC3339 instant the hosting is promised until (S2 Task 13's grant bound). */
  readonly hostedUntil = computed(() => this.account()?.hostedCellValidUntil ?? null);

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

  /** Reveal the confirmation step. Nothing is called until it is answered. */
  beginClose(): void {
    this.closeRequested.set(true);
    this.closeError.set(null);
  }

  cancelClose(): void {
    this.closeRequested.set(false);
    this.confirmIdentifier.set('');
    this.closeError.set(null);
  }

  onConfirmIdentifierInput(event: Event): void {
    this.confirmIdentifier.set((event.target as HTMLInputElement).value);
  }

  /**
   * Ask the doorway to close the account.
   *
   * Whether the typed identifier matches is the DOORWAY's judgement, not this
   * page's — it answers 400 `CONFIRMATION_MISMATCH` when it does not, and that
   * refusal is what `hosted-human/05-leaving.feature` checks. Comparing the
   * strings here instead would move a piece of account truth into the browser
   * and let a page with a stale identifier refuse a human their own exit.
   *
   * Deliberately a two-callback `.then`, not `await`: under zone.js a rejection
   * handled by a native-await catch is inspected before V8 attaches the
   * thenable job, and the handled 400 false-flags as an uncaught rejection in
   * the browser console.
   */
  confirmClose(): void {
    if (this.closing()) return;
    this.closing.set(true);
    this.closeError.set(null);

    this.adminService.closeAccount(this.confirmIdentifier()).then(
      () => {
        this.closing.set(false);
        // The session is already gone on the doorway side — drop it locally
        // and return to the signed-out landing.
        this.authState.clearLocalSession();
        void this.router.navigate(['/']);
      },
      (refusal: unknown) => {
        this.closing.set(false);
        this.closeError.set(closeRefusalMessage(refusal));
      }
    );
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
        // A steward whose cell is still doorway-hosted has NOT finished this
        // step — they are a hosted steward. Ticking it complete made
        // doorway/account claim they were further along than the elohim-app
        // agency badge ("Hosted Steward") says.
        return acct.isSteward && !this.stewardAccessingThroughDoorway();
      default:
        return false;
    }
  }

  isCurrentStep(step: AgencyStep): boolean {
    const acct = this.account();
    if (!acct) return step === 'hosted';
    if (step === 'steward' && this.stewardAccessingThroughDoorway()) return true;
    if (step === 'steward' && !acct.isSteward && this.runsOwnConductor()) return true;
    if (step === 'install_app' && !this.runsOwnConductor() && acct.keyExported) return true;
    if (step === 'key_export' && !acct.keyExported) return true;
    return false;
  }
}
