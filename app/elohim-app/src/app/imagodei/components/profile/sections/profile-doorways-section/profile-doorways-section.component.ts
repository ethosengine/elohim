import { CommonModule } from '@angular/common';
import { Component, computed, inject, input, output, signal } from '@angular/core';

// @coverage: 2.9% (2026-02-24)

import { type DoorwayWithHealth } from '../../../../models/doorway.model';
import { type IdentityMode } from '../../../../models/identity.model';
import { AuthService } from '../../../../services';

export interface DoorwayRegistrationContext {
  identifier: string | null;
  registeredSince: string | null;
  credentialStorage: 'browser' | 'device' | null;
}

@Component({
  selector: 'app-profile-doorways-section',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-doorways-section.component.html',
  styleUrls: ['./profile-doorways-section.component.css'],
})
export class ProfileDoorwaysSectionComponent {
  private readonly authService = inject(AuthService);

  readonly doorways = input.required<DoorwayWithHealth[]>();
  readonly activeDoorwayId = input<string | null>(null);
  readonly identityMode = input<IdentityMode>('hosted');
  readonly registrationContext = input<DoorwayRegistrationContext | null>(null);

  readonly showRecoveryContext = computed(() => {
    const mode = this.identityMode();
    return mode === 'steward' || mode === 'migrating';
  });

  readonly onlineDoorwayCount = computed(() => {
    return this.doorways().filter(d => d.isReachable).length;
  });

  readonly setAsPrimary = output<string>();
  readonly validateDoorway = output<string>();
  readonly addDoorway = output<string>();

  readonly showAddDoorway = signal(false);
  readonly newDoorwayUrl = signal('');
  readonly validating = signal(false);
  readonly validationError = signal<string | null>(null);
  readonly validationResult = signal<{ name: string; url: string } | null>(null);

  onDoorwayUrlInput(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.newDoorwayUrl.set(input.value);
  }

  toggleAddDoorway(): void {
    this.showAddDoorway.update(v => !v);
    this.validationError.set(null);
    this.validationResult.set(null);
    this.newDoorwayUrl.set('');
  }

  onValidate(): void {
    const url = this.newDoorwayUrl().trim();
    if (url) {
      this.validateDoorway.emit(url);
    }
  }

  onAdd(): void {
    const result = this.validationResult();
    if (result) {
      this.addDoorway.emit(result.url);
      this.toggleAddDoorway();
    }
  }

  /**
   * Navigate to doorway-app with a session transfer token so Matthew
   * doesn't have to log in again. Falls back to plain URL if token
   * fetch fails.
   */
  async visitDoorway(doorwayUrl: string): Promise<void> {
    const token = this.authService.token();
    if (token) {
      try {
        const res = await fetch(`${doorwayUrl}/auth/session-token`, {
          headers: { Authorization: `Bearer ${token}` },
        });
        if (res.ok) {
          const data = (await res.json()) as { sessionToken?: string };
          if (data.sessionToken) {
            window.open(
              `${doorwayUrl}/threshold?session_token=${encodeURIComponent(data.sessionToken)}`,
              '_blank',
              'noopener'
            );
            return;
          }
        }
      } catch {
        // Fall through to plain navigation
      }
    }
    // Fallback: open without token (user will need to log in)
    window.open(`${doorwayUrl}/threshold`, '_blank', 'noopener');
  }

  formatDate(iso: string): string {
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
}
